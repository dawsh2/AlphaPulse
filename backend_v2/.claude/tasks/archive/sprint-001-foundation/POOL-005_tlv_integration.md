# Task POOL-005: TLV Message Integration with Real Addresses
*Agent Type: Protocol Integration Specialist*
*Branch: `fix/tlv-pool-integration`*
*Dependencies: POOL-001 (cache), POOL-002 (event extraction)*

## 📋 Your Mission
Integrate the real pool/token addresses from cache and event extraction into TLV message construction, replacing all placeholder addresses.

## 🎯 Context
Now that we have:
- Pool cache with real addresses (POOL-001)
- Event extraction getting pool addresses (POOL-002)
- Discovery queue for unknown pools (POOL-003)
- RPC discovery implementation (POOL-004)

We need to wire it all together in the TLV message construction!

## 🔧 Git Setup Instructions

```bash
# Step 1: Ensure dependencies are available
git checkout main
git pull origin main

# Step 2: Create your feature branch
git checkout -b fix/tlv-pool-integration

# Step 3: Confirm branch
git branch --show-current  # Should show: fix/tlv-pool-integration
```

## 📝 Task Specification

### Files to Modify
1. `services_v2/adapters/src/polygon/polygon.rs` - Complete integration
2. `services_v2/adapters/src/polygon/tlv_builder.rs` - Create TLV builder helper

### Required Implementation

#### Step 1: Create TLV Builder Helper
```rust
// Create services_v2/adapters/src/polygon/tlv_builder.rs

use torq_protocol_v2::{
    tlv::{PoolSwapTLV, TradeTLV, QuoteTLV},
    MessageHeader, TLVType,
};
use torq_state_market::pool_cache::{PoolCache, PoolInfo};
use std::sync::Arc;
use web3::types::H160;

pub struct PoolTLVBuilder {
    cache: Arc<PoolCache>,
}

impl PoolTLVBuilder {
    pub fn new(cache: Arc<PoolCache>) -> Self {
        Self { cache }
    }

    /// Build PoolSwapTLV with real addresses from cache
    pub async fn build_pool_swap_tlv(
        &self,
        pool_address: H160,
        timestamp: u64,
        amount0: i128,
        amount1: i128,
        sqrt_price: Option<u128>,
        liquidity: u128,
        tick: Option<i32>,
    ) -> Result<Option<PoolSwapTLV>, Box<dyn std::error::Error>> {
        // Try to get pool info from cache
        let pool_info = match self.cache.get(&pool_address.as_bytes().try_into()?).await {
            Some(info) => info,
            None => {
                // Queue for discovery but don't block
                warn!("Unknown pool {}, queueing for discovery", pool_address);
                self.cache.queue_discovery(pool_address.as_bytes().try_into()?).await?;
                return Ok(None); // Skip this message for now
            }
        };

        // Build TLV with REAL addresses
        let tlv = PoolSwapTLV {
            timestamp,
            pool_address: pool_info.pool_address,
            token0: pool_info.token0,
            token1: pool_info.token1,
            token0_decimals: pool_info.token0_decimals,
            token1_decimals: pool_info.token1_decimals,
            amount0,
            amount1,
            sqrt_price_x96: sqrt_price.unwrap_or(0),
            liquidity,
            tick: tick.unwrap_or(0),
            fee_tier: pool_info.fee_tier.unwrap_or(30),
            protocol: pool_info.pool_type as u8,
            reserved: [0u8; 3],
        };

        Ok(Some(tlv))
    }

    /// Build TradeTLV from swap event
    pub async fn build_trade_tlv(
        &self,
        pool_address: H160,
        timestamp: u64,
        price: i64,
        amount: i64,
        side: u8,
    ) -> Result<Option<TradeTLV>, Box<dyn std::error::Error>> {
        // Get pool info to determine instrument
        let pool_info = match self.cache.get(&pool_address.as_bytes().try_into()?).await {
            Some(info) => info,
            None => {
                self.cache.queue_discovery(pool_address.as_bytes().try_into()?).await?;
                return Ok(None);
            }
        };

        // Create instrument ID from pool
        let instrument_id = create_pool_instrument_id(&pool_info);

        let tlv = TradeTLV {
            timestamp,
            instrument_id,
            price,
            amount,
            side,
            venue: VenueId::Polygon as u8,
            flags: 0,
        };

        Ok(Some(tlv))
    }
}

fn create_pool_instrument_id(pool_info: &PoolInfo) -> InstrumentId {
    // Use bijective ID construction
    InstrumentId::from_pool(
        pool_info.venue,
        pool_info.pool_address,
        pool_info.token0,
        pool_info.token1,
        pool_info.fee_tier,
    )
}
```

#### Step 2: Update Polygon Collector Integration
```rust
// In services_v2/adapters/src/polygon/polygon.rs

use crate::polygon::tlv_builder::PoolTLVBuilder;

impl UnifiedPolygonCollector {
    // Update the constructor
    pub async fn new(config: Config) -> Result<Self> {
        // ... existing code ...

        // Initialize pool cache with persistence
        let cache_path = config.cache_dir.join("polygon_pools.tlv");
        let pool_cache = Arc::new(
            PoolCache::with_persistence(cache_path, 137) // 137 = Polygon chain ID
        );

        // Load existing cache from disk
        if let Err(e) = pool_cache.load_from_disk().await {
            warn!("Failed to load pool cache: {}", e);
        }

        // Create TLV builder
        let tlv_builder = Arc::new(PoolTLVBuilder::new(pool_cache.clone()));

        // Initialize discovery queue (from POOL-003)
        let (discovery_queue, discovery_rx) = DiscoveryQueue::new(1000);

        // Start discovery worker
        let worker_cache = pool_cache.clone();
        let worker_queue = Arc::new(discovery_queue.clone());
        tokio::spawn(async move {
            discovery_worker(discovery_rx, worker_queue, worker_cache).await;
        });

        Ok(Self {
            // ... existing fields ...
            pool_cache,
            tlv_builder,
            discovery_queue: Arc::new(discovery_queue),
        })
    }

    // Update process_swap_event
    async fn process_swap_event(&self, log: &Log) -> Result<()> {
        let pool_address = log.address;
        let timestamp = self.get_block_timestamp(log.block_number).await?;

        // Parse swap amounts based on protocol (from POOL-002)
        let (amount0, amount1, sqrt_price, liquidity, tick) =
            self.parse_swap_data(log)?;

        // Build TLV with real addresses
        match self.tlv_builder.build_pool_swap_tlv(
            pool_address,
            timestamp,
            amount0,
            amount1,
            sqrt_price,
            liquidity,
            tick,
        ).await? {
            Some(tlv) => {
                // Send to relay
                self.send_tlv_message(TLVType::PoolSwap, &tlv)?;

                // Update metrics
                self.messages_sent.fetch_add(1, Ordering::Relaxed);
            }
            None => {
                // Pool not in cache, discovery queued
                self.unknown_pools.fetch_add(1, Ordering::Relaxed);
            }
        }

        Ok(())
    }

    // Graceful shutdown
    pub async fn shutdown(self) -> Result<()> {
        info!("Shutting down Polygon collector...");

        // Save cache to disk
        self.pool_cache.force_snapshot().await?;

        // Shutdown discovery queue
        self.discovery_queue.shutdown().await?;

        // Close WebSocket
        self.ws_stream.close().await?;

        Ok(())
    }
}
```

#### Step 3: Add Metrics and Monitoring
```rust
// Add metrics structure
#[derive(Debug, Clone)]
pub struct IntegrationMetrics {
    pub total_events: u64,
    pub tlv_messages_sent: u64,
    pub unknown_pools_queued: u64,
    pub cache_hit_rate: f64,
    pub discovery_queue_depth: usize,
}

impl UnifiedPolygonCollector {
    pub fn get_metrics(&self) -> IntegrationMetrics {
        let cache_stats = self.pool_cache.get_stats();
        let queue_metrics = self.discovery_queue.metrics();

        IntegrationMetrics {
            total_events: self.total_events.load(Ordering::Relaxed),
            tlv_messages_sent: self.messages_sent.load(Ordering::Relaxed),
            unknown_pools_queued: self.unknown_pools.load(Ordering::Relaxed),
            cache_hit_rate: cache_stats.hit_rate(),
            discovery_queue_depth: queue_metrics.queue_depth,
        }
    }
}
```

## ✅ Acceptance Criteria

1. **Integration Complete**
   - [ ] All TLV messages use real addresses
   - [ ] No more `[0u8; 20]` placeholders in production
   - [ ] Cache integration working smoothly
   - [ ] Discovery queue processing unknown pools

2. **Performance Maintained**
   - [ ] Hot path still <35μs with cache hits
   - [ ] Non-blocking on cache misses
   - [ ] Graceful degradation for unknown pools
   - [ ] Cache persistence working

3. **Monitoring**
   - [ ] Metrics exposed for monitoring
   - [ ] Cache hit rate visible
   - [ ] Discovery queue depth tracked
   - [ ] Unknown pool rate measured

## 🧪 Testing Instructions

```bash
# Integration test
cargo test --package services_v2 tlv_integration

# End-to-end test with real Polygon events
cargo run --bin polygon_integration_test

# Monitor metrics
cargo run --bin polygon_collector -- --show-metrics

# Verify cache persistence
cargo run --bin verify_cache_persistence
```

## 🔄 Rollback Instructions

If integration causes issues:

```bash
# Immediate rollback
git revert HEAD
git push origin main

# Or rollback to before integration
git checkout main
git reset --hard <commit-before-pool-005>
git push --force-with-lease origin main

# Clear corrupted cache if needed
rm -f /var/lib/torq/polygon_pools.tlv
rm -f /var/lib/torq/polygon_pools.tlv.journal

# Restart services
systemctl restart torq-polygon-collector
```

## 📤 Commit & Push Instructions

```bash
# Stage changes
git add services_v2/adapters/src/polygon/polygon.rs
git add services_v2/adapters/src/polygon/tlv_builder.rs

# Commit
git commit -m "feat(tlv): integrate real addresses into TLV messages

- Wire together cache, discovery, and TLV construction
- Replace all placeholder addresses with real data
- Add metrics for monitoring integration health
- Implement graceful degradation for unknown pools"

# Push
git push -u origin fix/tlv-pool-integration
```

## 🔄 Pull Request Template

```markdown
## Task POOL-005: TLV Integration with Real Addresses

### Summary
Complete integration of pool discovery system with TLV message construction.

### Integration Points
- Pool cache provides real addresses
- Discovery queue handles unknown pools
- TLV builder creates proper messages
- Metrics track integration health

### Results
- ✅ No more placeholder addresses
- ✅ 99%+ cache hit rate after warmup
- ✅ Unknown pools queued for discovery
- ✅ Full persistence across restarts

### Testing
- [x] Integration tests pass
- [x] E2E test with real events successful
- [x] Metrics correctly tracked
- [x] Persistence verified
```

## ⚠️ Important Notes

1. **Never Block**: Always return None if pool unknown
2. **Queue Once**: Check if already queued before queueing
3. **Persist Cache**: Save on shutdown for fast restart
4. **Monitor Metrics**: Watch cache hit rate and queue depth
5. **Graceful Degradation**: System works even with cache misses

## 🤝 Coordination
- Integrates all previous POOL tasks
- Final piece before comprehensive testing (POOL-006)
- Critical for production deployment

---
*This completes the pool address fix - no more placeholders!*
