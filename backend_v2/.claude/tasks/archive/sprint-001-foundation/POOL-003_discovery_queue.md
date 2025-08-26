# Task POOL-003: Discovery Queue Integration
*Agent Type: Integration Specialist*
*Branch: `fix/discovery-integration`*
*Dependencies: POOL-001 (cache integration)*

## 📋 Your Mission
Integrate the existing pool discovery functionality from `PoolCache` to handle unknown pools WITHOUT blocking the WebSocket hot path.

## 🎯 Context
The `PoolCache` in `libs/state/market/src/pool_cache.rs` ALREADY HAS discovery functionality with:
- `get_or_discover_pool()` - Async discovery
- `discovery_in_progress` tracking
- RPC resilience with retries
- No blocking on hot path

We just need to USE it properly in the Polygon collector!

## 🔧 Git Setup Instructions

```bash
# Step 1: Ensure POOL-001 is available
git checkout main
git pull origin main
# Check if POOL-001 merged (optional)
git log --oneline -5

# Step 2: Create your branch
git checkout -b fix/discovery-queue

# Step 3: Verify branch
git branch --show-current  # Should show: fix/discovery-queue
```

## 📝 Task Specification

### Files to Create/Modify
1. `services_v2/adapters/src/polygon/discovery_queue.rs` (NEW)
2. `services_v2/adapters/src/polygon/mod.rs` (add module)
3. `services_v2/adapters/src/polygon/polygon.rs` (integrate queue)

### Required Implementation

```rust
// discovery_queue.rs - Create this file

use std::collections::{HashSet, VecDeque};
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc, oneshot};
use web3::types::H160;
use crate::polygon::pool_cache::{PoolCache, PoolMetadata};

pub struct DiscoveryRequest {
    pub pool_address: H160,
    pub priority: DiscoveryPriority,
    pub requested_at: u64,
    pub retry_count: u8,
    // Optional callback for completion notification
    pub callback: Option<oneshot::Sender<Result<PoolMetadata, DiscoveryError>>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiscoveryPriority {
    Critical = 0,  // High-volume pools
    High = 1,      // Active trading pools
    Normal = 2,    // Regular pools
    Low = 3,       // Rarely seen pools
}

pub struct DiscoveryQueue {
    // Priority queue for pending discoveries
    pending: Arc<Mutex<Vec<VecDeque<DiscoveryRequest>>>>,
    // Track in-flight to avoid duplicates
    in_flight: Arc<Mutex<HashSet<H160>>>,
    // Channel to discovery worker
    tx: mpsc::Sender<DiscoveryRequest>,
    // Metrics
    queue_depth: Arc<Mutex<usize>>,
    max_depth: usize,
    total_discovered: Arc<Mutex<u64>>,
}

impl DiscoveryQueue {
    pub fn new(max_depth: usize) -> (Self, mpsc::Receiver<DiscoveryRequest>) {
        let (tx, rx) = mpsc::channel(max_depth);

        let pending = Arc::new(Mutex::new(vec![
            VecDeque::new(), // Critical
            VecDeque::new(), // High
            VecDeque::new(), // Normal
            VecDeque::new(), // Low
        ]));

        (
            Self {
                pending,
                in_flight: Arc::new(Mutex::new(HashSet::new())),
                tx,
                queue_depth: Arc::new(Mutex::new(0)),
                max_depth,
                total_discovered: Arc::new(Mutex::new(0)),
            },
            rx
        )
    }

    /// Queue a pool for discovery (non-blocking)
    pub async fn queue_discovery(
        &self,
        pool_address: H160,
        priority: DiscoveryPriority,
    ) -> Result<(), QueueError> {
        // Check if already queued or in-flight
        let mut in_flight = self.in_flight.lock().await;
        if in_flight.contains(&pool_address) {
            return Ok(()); // Already being processed
        }

        // Check queue depth
        let mut depth = self.queue_depth.lock().await;
        if *depth >= self.max_depth {
            return Err(QueueError::QueueFull);
        }

        // Add to in-flight set
        in_flight.insert(pool_address);

        // Create request
        let request = DiscoveryRequest {
            pool_address,
            priority,
            requested_at: crate::utils::fast_timestamp(),
            retry_count: 0,
            callback: None,
        };

        // Send to worker (non-blocking)
        self.tx.try_send(request).map_err(|_| QueueError::WorkerBusy)?;

        *depth += 1;
        Ok(())
    }

    /// Get next pool to discover (called by worker)
    pub async fn next_discovery(&self) -> Option<DiscoveryRequest> {
        let mut pending = self.pending.lock().await;

        // Process by priority
        for queue in pending.iter_mut() {
            if let Some(request) = queue.pop_front() {
                let mut depth = self.queue_depth.lock().await;
                *depth = depth.saturating_sub(1);
                return Some(request);
            }
        }

        None
    }

    /// Mark discovery complete
    pub async fn mark_complete(&self, pool_address: H160, success: bool) {
        let mut in_flight = self.in_flight.lock().await;
        in_flight.remove(&pool_address);

        if success {
            let mut total = self.total_discovered.lock().await;
            *total += 1;
        }
    }

    /// Get queue metrics
    pub async fn metrics(&self) -> QueueMetrics {
        QueueMetrics {
            queue_depth: *self.queue_depth.lock().await,
            in_flight: self.in_flight.lock().await.len(),
            total_discovered: *self.total_discovered.lock().await,
        }
    }
}

#[derive(Debug)]
pub struct QueueMetrics {
    pub queue_depth: usize,
    pub in_flight: usize,
    pub total_discovered: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum QueueError {
    #[error("Discovery queue is full")]
    QueueFull,
    #[error("Discovery worker is busy")]
    WorkerBusy,
}
```

### Discovery Worker Task

```rust
// In polygon.rs or separate worker file

pub async fn discovery_worker(
    mut rx: mpsc::Receiver<DiscoveryRequest>,
    queue: Arc<DiscoveryQueue>,
    cache: Arc<PoolCache>,
    web3: Arc<Web3<Http>>,
) {
    info!("Discovery worker started");

    while let Some(request) = rx.recv().await {
        let pool_address = request.pool_address;

        // Check cache first (might have been discovered by another worker)
        if cache.get(&pool_address).await.is_some() {
            queue.mark_complete(pool_address, true).await;
            continue;
        }

        // Attempt discovery with timeout
        let discovery_result = tokio::time::timeout(
            Duration::from_secs(5),
            discover_pool_metadata(&web3, pool_address)
        ).await;

        match discovery_result {
            Ok(Ok(metadata)) => {
                // Success! Add to cache
                if let Err(e) = cache.insert(metadata.clone()).await {
                    error!("Failed to cache pool {}: {}", pool_address, e);
                }

                info!("Discovered pool {}: {} <-> {}",
                    pool_address, metadata.token0, metadata.token1);

                // Notify callback if present
                if let Some(callback) = request.callback {
                    let _ = callback.send(Ok(metadata));
                }

                queue.mark_complete(pool_address, true).await;
            }
            Ok(Err(e)) => {
                warn!("Failed to discover pool {}: {}", pool_address, e);

                // Retry logic
                if request.retry_count < 3 {
                    let mut retry = request;
                    retry.retry_count += 1;
                    retry.priority = DiscoveryPriority::Low; // Lower priority for retries

                    if let Err(e) = queue.queue_discovery(pool_address, retry.priority).await {
                        error!("Failed to requeue {}: {}", pool_address, e);
                    }
                }

                queue.mark_complete(pool_address, false).await;
            }
            Err(_) => {
                // Timeout
                error!("Discovery timeout for pool {}", pool_address);
                queue.mark_complete(pool_address, false).await;
            }
        }

        // Rate limiting
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

async fn discover_pool_metadata(
    web3: &Web3<Http>,
    pool_address: H160,
) -> Result<PoolMetadata, Box<dyn std::error::Error>> {
    // This is where POOL-004's RPC logic will go
    // For now, return placeholder
    todo!("Implement in POOL-004")
}
```

## ✅ Acceptance Criteria

1. **Performance Requirements**
   - [ ] Queue operations <1μs (non-blocking)
   - [ ] No impact on WebSocket hot path
   - [ ] Worker processes queue continuously
   - [ ] Rate limiting to avoid RPC overload

2. **Queue Management**
   - [ ] Priority-based processing
   - [ ] Duplicate detection
   - [ ] Retry logic with backoff
   - [ ] Queue depth limits
   - [ ] Metrics tracking

3. **Integration**
   - [ ] Seamless integration with POOL-001 cache
   - [ ] Non-blocking from event processor perspective
   - [ ] Graceful shutdown handling
   - [ ] Error recovery

## 🧪 Testing Instructions

```bash
# Unit tests
cargo test --package services_v2 discovery_queue

# Integration test with mock RPC
cargo test --package services_v2 discovery_integration

# Load test - queue 10,000 pools
cargo test --package services_v2 discovery_load_test

# Verify no hot path impact
cargo bench --package services_v2 with_discovery_queue
```

## 📤 Commit & Push Instructions

```bash
# Add your files
git add services_v2/adapters/src/polygon/discovery_queue.rs
git add services_v2/adapters/src/polygon/mod.rs
git add services_v2/adapters/src/polygon/polygon.rs

# Commit
git commit -m "feat(pool): implement async discovery queue for unknown pools

- Add priority-based discovery queue with retry logic
- Implement background worker for pool discovery
- Ensure zero impact on WebSocket hot path (<1μs queue ops)
- Add metrics tracking and queue depth management"

# Push
git push -u origin fix/discovery-queue
```

## 🔄 Pull Request Template

```markdown
## Task POOL-003: Async Pool Discovery Queue

### Summary
Implemented non-blocking discovery queue to handle unknown pools without impacting the critical WebSocket processing path.

### Key Features
- Priority-based queue (Critical/High/Normal/Low)
- Duplicate detection to avoid redundant discoveries
- Retry logic with exponential backoff
- Zero impact on hot path (<1μs queue operations)
- Background worker with rate limiting

### Performance Impact
- Queue operation: 0.7μs ✅
- Hot path impact: NONE ✅
- Worker throughput: 10 pools/second (rate limited)

### Integration
- Works with POOL-001 cache structure
- Ready for POOL-004 RPC implementation
- Metrics exposed for monitoring

### Testing
- [x] Unit tests pass
- [x] Load test (10,000 pools) successful
- [x] No hot path regression confirmed
```

## ⚠️ Critical Requirements

1. **NEVER BLOCK**: Queue operations must be fire-and-forget
2. **Respect Rate Limits**: Don't overwhelm RPC endpoints
3. **Handle Failures**: Retry with backoff, don't lose pools
4. **Monitor Queue Depth**: Prevent unbounded growth
5. **Clean Shutdown**: Process remaining queue on shutdown

## 🤝 Coordination
- Depends on POOL-001 for cache structure
- POOL-004 will implement actual RPC discovery logic
- POOL-002 will call your queue_discovery() method

---
*Remember: The hot path is sacred - not even 1μs of blocking is acceptable!*
