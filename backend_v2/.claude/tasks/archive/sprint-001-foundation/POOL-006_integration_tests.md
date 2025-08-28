# Task POOL-006: Comprehensive Integration Testing
*Agent Type: Test Engineer*
*Branch: `test/pool-integration-validation`*
*Dependencies: ALL POOL tasks (001-005)*

## 📋 Your Mission
Create comprehensive integration tests that validate the entire pool address discovery and TLV integration system works correctly end-to-end.

## 🎯 Context
All components are now built:
- Pool cache with persistence (POOL-001)
- Event extraction (POOL-002)
- Discovery queue (POOL-003)
- RPC discovery (POOL-004)
- TLV integration (POOL-005)

Now we need to PROVE it all works together!

## 🔧 Git Setup Instructions

```bash
# Step 1: Ensure all POOL tasks are merged
git checkout main
git pull origin main

# Verify all branches merged
git log --oneline --grep="POOL-" -10

# Step 2: Create test branch
git checkout -b test/pool-integration-validation

# Step 3: Confirm branch
git branch --show-current  # Should show: test/pool-integration-validation
```

## 📝 Test Specification

### Test Files to Create
1. `tests/e2e/pool_integration_test.rs` - Main integration test
2. `tests/e2e/pool_discovery_flow_test.rs` - Discovery flow test
3. `tests/e2e/pool_cache_persistence_test.rs` - Persistence test
4. `tests/e2e/pool_performance_test.rs` - Performance validation

### Test 1: End-to-End Pool Discovery Flow
```rust
// tests/e2e/pool_integration_test.rs

use torq_protocol_v2::tlv::PoolSwapTLV;
use torq_services_v2::polygon::UnifiedPolygonCollector;
use torq_state_market::pool_cache::PoolCache;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::time::{sleep, Duration};

#[tokio::test]
async fn test_complete_pool_discovery_flow() {
    // Setup
    let temp_dir = TempDir::new().unwrap();
    let config = create_test_config(temp_dir.path());

    // Initialize collector with all components
    let collector = UnifiedPolygonCollector::new(config).await.unwrap();

    // Test Case 1: Known pool (should hit cache)
    let known_pool = H160::from_str("0x45dda9cb7c25131df268515131f647d726f50608").unwrap();
    let event = create_mock_swap_event(known_pool);

    // Process event
    collector.process_swap_event(&event).await.unwrap();

    // Verify TLV sent with real addresses
    let sent_messages = collector.get_sent_messages();
    assert_eq!(sent_messages.len(), 1);

    let tlv = parse_pool_swap_tlv(&sent_messages[0]);
    assert_ne!(tlv.token0, [0u8; 20], "Token0 should not be placeholder");
    assert_ne!(tlv.token1, [0u8; 20], "Token1 should not be placeholder");

    // Test Case 2: Unknown pool (should queue for discovery)
    let unknown_pool = H160::from_str("0x1234567890abcdef1234567890abcdef12345678").unwrap();
    let event = create_mock_swap_event(unknown_pool);

    let initial_queue_depth = collector.get_metrics().discovery_queue_depth;
    collector.process_swap_event(&event).await.unwrap();

    // Verify queued for discovery
    let new_queue_depth = collector.get_metrics().discovery_queue_depth;
    assert_eq!(new_queue_depth, initial_queue_depth + 1, "Pool should be queued");

    // Wait for discovery to complete (with timeout)
    let start = Instant::now();
    while collector.pool_cache.get(&unknown_pool.as_bytes()).await.is_none() {
        if start.elapsed() > Duration::from_secs(10) {
            panic!("Discovery timeout");
        }
        sleep(Duration::from_millis(100)).await;
    }

    // Verify pool now in cache
    let pool_info = collector.pool_cache.get(&unknown_pool.as_bytes()).await.unwrap();
    assert_eq!(pool_info.pool_address, unknown_pool.as_bytes());

    // Test Case 3: High-volume scenario
    let pools = generate_random_pools(100);
    let mut events = Vec::new();
    for pool in &pools {
        events.push(create_mock_swap_event(*pool));
    }

    // Process all events
    for event in events {
        collector.process_swap_event(&event).await.unwrap();
    }

    // Verify metrics
    let metrics = collector.get_metrics();
    assert!(metrics.cache_hit_rate > 0.0, "Should have some cache hits");
    assert!(metrics.unknown_pools_queued > 0, "Should have queued some pools");

    // Cleanup
    collector.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_cache_persistence_across_restarts() {
    let temp_dir = TempDir::new().unwrap();
    let cache_path = temp_dir.path().join("test_cache.tlv");

    // Phase 1: Create cache and add pools
    {
        let cache = PoolCache::with_persistence(cache_path.clone(), 137);

        // Add some pools
        for i in 0..10 {
            let pool_info = create_test_pool_info(i);
            cache.insert(pool_info).await.unwrap();
        }

        // Force snapshot
        cache.force_snapshot().await.unwrap();

        // Verify pools in cache
        assert_eq!(cache.size(), 10);
    }

    // Phase 2: Restart and verify persistence
    {
        let cache = PoolCache::with_persistence(cache_path.clone(), 137);

        // Load from disk
        let loaded = cache.load_from_disk().await.unwrap();
        assert_eq!(loaded, 10, "Should load 10 pools from disk");

        // Verify pools still accessible
        for i in 0..10 {
            let pool_address = create_test_address(i);
            let pool_info = cache.get(&pool_address).await;
            assert!(pool_info.is_some(), "Pool {} should be in cache", i);
        }
    }
}

#[tokio::test]
async fn test_concurrent_discovery_handling() {
    let cache = Arc::new(PoolCache::with_default_config());
    let (queue, rx) = DiscoveryQueue::new(1000);
    let queue = Arc::new(queue);

    // Start discovery worker
    let worker_cache = cache.clone();
    let worker_queue = queue.clone();
    tokio::spawn(async move {
        discovery_worker(rx, worker_queue, worker_cache).await;
    });

    // Queue many pools concurrently
    let mut handles = Vec::new();
    for i in 0..100 {
        let queue_clone = queue.clone();
        let handle = tokio::spawn(async move {
            let pool = create_test_address(i);
            queue_clone.queue_discovery(
                H160::from_slice(&pool),
                DiscoveryPriority::Normal
            ).await
        });
        handles.push(handle);
    }

    // Wait for all queuing to complete
    for handle in handles {
        handle.await.unwrap().unwrap();
    }

    // Verify queue metrics
    let metrics = queue.metrics().await;
    assert!(metrics.queue_depth > 0 || metrics.in_flight > 0,
            "Should have pools queued or in flight");

    // No duplicates should be queued
    assert!(metrics.queue_depth + metrics.in_flight <= 100,
            "Should not have duplicates");
}

#[tokio::test]
async fn test_performance_requirements() {
    let cache = PoolCache::with_default_config();

    // Pre-populate cache
    for i in 0..10000 {
        let pool_info = create_test_pool_info(i);
        cache.insert(pool_info).await.unwrap();
    }

    // Test cache lookup performance
    let start = Instant::now();
    let iterations = 100_000;

    for i in 0..iterations {
        let pool_address = create_test_address(i % 10000);
        let _ = cache.get(&pool_address).await;
    }

    let elapsed = start.elapsed();
    let per_lookup = elapsed / iterations;

    assert!(per_lookup < Duration::from_micros(1),
            "Cache lookup should be <1μs, was {:?}", per_lookup);

    // Test TLV construction performance
    let builder = PoolTLVBuilder::new(Arc::new(cache));
    let start = Instant::now();

    for i in 0..iterations {
        let pool = H160::from_slice(&create_test_address(i % 10000));
        let _ = builder.build_pool_swap_tlv(
            pool,
            timestamp(),
            1000,
            2000,
            None,
            100000,
            None,
        ).await;
    }

    let elapsed = start.elapsed();
    let per_build = elapsed / iterations;

    assert!(per_build < Duration::from_micros(35),
            "TLV build should be <35μs, was {:?}", per_build);
}

// Helper functions
fn create_test_config(cache_dir: &Path) -> Config {
    Config {
        cache_dir: cache_dir.to_path_buf(),
        rpc_url: "https://polygon-rpc.com".to_string(),
        ws_url: "wss://polygon-ws.com".to_string(),
        ..Default::default()
    }
}

fn create_test_pool_info(index: u32) -> PoolInfo {
    PoolInfo {
        pool_address: create_test_address(index),
        token0: create_test_address(index * 2),
        token1: create_test_address(index * 2 + 1),
        token0_decimals: 18,
        token1_decimals: 6,
        pool_type: DEXProtocol::UniswapV2,
        fee_tier: Some(30),
        discovered_at: timestamp(),
        venue: VenueId::Polygon,
        last_seen: timestamp(),
    }
}

fn create_test_address(seed: u32) -> [u8; 20] {
    let mut addr = [0u8; 20];
    addr[..4].copy_from_slice(&seed.to_be_bytes());
    addr
}
```

### Test 2: Integration Test Script
```bash
#!/bin/bash
# tests/e2e/run_integration_tests.sh

set -e

echo "Running Pool Integration Tests..."

# Test 1: Unit tests for each component
echo "Testing individual components..."
cargo test --package torq-state-market pool_cache
cargo test --package services_v2 discovery_queue
cargo test --package services_v2 event_extraction

# Test 2: Integration tests
echo "Running integration tests..."
cargo test --test pool_integration_test -- --nocapture
cargo test --test pool_discovery_flow_test -- --nocapture
cargo test --test pool_cache_persistence_test -- --nocapture

# Test 3: Performance benchmarks
echo "Running performance benchmarks..."
cargo bench --bench pool_cache_bench
cargo bench --bench tlv_construction_bench

# Test 4: Load test with real data
echo "Running load test..."
cargo run --bin pool_load_test -- \
    --pools 10000 \
    --events-per-pool 100 \
    --duration 60

# Test 5: Memory leak check
echo "Checking for memory leaks..."
valgrind --leak-check=full \
    cargo run --bin polygon_collector_test -- --iterations 1000

echo "All tests passed!"
```

## ✅ Acceptance Criteria

1. **Functional Tests**
   - [ ] Pool discovery flow works end-to-end
   - [ ] Cache persistence verified
   - [ ] Concurrent discovery handled correctly
   - [ ] No placeholder addresses in output

2. **Performance Tests**
   - [ ] Cache lookup <1μs confirmed
   - [ ] TLV construction <35μs confirmed
   - [ ] 1M msg/s throughput achievable
   - [ ] Memory usage <50MB for 10k pools

3. **Reliability Tests**
   - [ ] Graceful handling of RPC failures
   - [ ] Recovery from corrupted cache
   - [ ] No memory leaks detected
   - [ ] Proper cleanup on shutdown

4. **Integration Tests**
   - [ ] All components work together
   - [ ] Metrics accurately tracked
   - [ ] Error paths tested
   - [ ] Edge cases covered

## 🧪 Testing Instructions

```bash
# Run all integration tests
./tests/e2e/run_integration_tests.sh

# Run specific test suites
cargo test --test pool_integration_test
cargo test --test pool_performance_test

# Run with real Polygon data
POLYGON_RPC_URL=https://polygon-rpc.com \
    cargo test --test pool_integration_test -- --ignored

# Generate test coverage report
cargo tarpaulin --out Html --output-dir coverage/
```

## 🔄 Rollback Instructions

Tests don't need rollback, but if tests reveal issues:

```bash
# Document failures
echo "Test failures found:" > test_failures.md
cargo test 2>&1 | grep FAILED >> test_failures.md

# Create issue for each failure
gh issue create --title "Pool Integration Test Failure" \
    --body-file test_failures.md

# Revert problematic changes
git revert <problematic-commit>
git push origin main
```

## 📤 Commit & Push Instructions

```bash
# Stage test files
git add tests/e2e/pool_integration_test.rs
git add tests/e2e/pool_discovery_flow_test.rs
git add tests/e2e/pool_cache_persistence_test.rs
git add tests/e2e/pool_performance_test.rs
git add tests/e2e/run_integration_tests.sh

# Commit
git commit -m "test(pool): comprehensive integration tests for pool system

- End-to-end discovery flow validation
- Cache persistence verification
- Performance requirement tests
- Concurrent operation handling
- Load and stress testing"

# Push
git push -u origin test/pool-integration-validation
```

## 🔄 Pull Request Template

```markdown
## Task POOL-006: Comprehensive Integration Testing

### Summary
Complete test suite validating entire pool discovery and TLV integration system.

### Test Coverage
- ✅ End-to-end discovery flow
- ✅ Cache persistence across restarts
- ✅ Concurrent discovery handling
- ✅ Performance requirements (<1μs cache, <35μs TLV)
- ✅ Load testing with 1M events
- ✅ Memory leak detection

### Results
- All functional tests: PASS
- Performance benchmarks: PASS
- Load test (1M events): PASS
- Memory usage: 42MB for 10k pools ✅

### Validation
- [x] No placeholder addresses in output
- [x] Cache hit rate >99% after warmup
- [x] Discovery queue processes all pools
- [x] Graceful degradation on failures

### Ready for Production
- [x] All tests passing
- [x] Performance targets met
- [x] No memory leaks
- [x] Edge cases covered
```

## ⚠️ Important Notes

1. **Test with Real Data**: Use actual Polygon events when possible
2. **Monitor Resources**: Watch memory and CPU during load tests
3. **Test Failures**: Each failure path needs coverage
4. **Concurrent Testing**: Ensure thread safety
5. **Performance Baseline**: Document current performance for regression detection

## 🤝 Coordination
- Final validation before production deployment
- Must pass before considering system complete
- Provides confidence in entire integration

---
*Comprehensive testing ensures production reliability!*
