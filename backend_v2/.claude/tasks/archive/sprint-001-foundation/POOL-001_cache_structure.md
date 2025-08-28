# Task POOL-001: Pool Cache Integration
*Agent Type: Integration Specialist*
*Branch: `fix/pool-cache-integration`*

## 📋 Your Mission
**INTEGRATE THE EXISTING** pool cache (`libs/state/market/src/pool_cache.rs`) into the Polygon collector to replace placeholder addresses with real pool/token information.

## 🎯 Context
**CRITICAL CLARIFICATION**: We ALREADY HAVE a complete, working `PoolCache` implementation in `libs/state/market/src/pool_cache.rs` with:
- DashMap for concurrent access
- RPC discovery functionality (`get_or_discover_pool()` method)
- TLV persistence to disk
- Full 20-byte addresses for all pools and tokens

**DO NOT CREATE NEW CACHE CODE** - Just integrate the existing one!

## 🔧 Git Setup Instructions

```bash
# Step 1: Start fresh from main
git checkout main
git pull origin main

# Step 2: Create your feature branch
git checkout -b fix/pool-cache-structure

# Step 3: Confirm you're on the right branch
git branch --show-current  # Should show: fix/pool-cache-structure
```

## 📝 Task Specification

### Existing Code to Use
- **Pool Cache**: `libs/state/market/src/pool_cache.rs` - Already has:
  - `PoolCache` with DashMap for concurrent access
  - `PoolInfo` struct with full 20-byte addresses
  - RPC discovery functionality
  - TLV persistence
  - Hit rate tracking

- **Pool State**: `libs/state/market/src/pool_state.rs` - Already has:
  - `PoolState` with full addresses
  - Token address getters
  - V2/V3 protocol support

### Files to Modify
1. `services_v2/adapters/src/polygon/polygon.rs` - Add cache integration
2. `services_v2/adapters/Cargo.toml` - Add dependency on `torq-state-market`

### Required Implementation

```rust
// In polygon.rs - Update imports
use torq_state_market::pool_cache::{PoolCache, PoolInfo};
use torq_state_market::pool_state::PoolStateManager;

// Update UnifiedPolygonCollector struct
pub struct UnifiedPolygonCollector {
    // ... existing fields ...

    // ADD THESE:
    pool_cache: Arc<PoolCache>,
    pool_state_manager: Arc<PoolStateManager>,
}

// In the constructor/new() method
impl UnifiedPolygonCollector {
    pub async fn new(config: Config) -> Result<Self> {
        // ... existing code ...

        // Initialize pool cache with persistence
        let cache_path = config.cache_dir.join("pool_cache.tlv");
        let pool_cache = Arc::new(
            PoolCache::load_or_create(
                cache_path,
                10_000, // max pools
                web3.clone(),
            ).await?
        );

        // Initialize pool state manager
        let pool_state_manager = Arc::new(
            PoolStateManager::new(Some(pool_cache.clone()))
        );

        Ok(Self {
            // ... existing fields ...
            pool_cache,
            pool_state_manager,
        })
    }
}

// In process_swap_event() - Replace placeholder logic
async fn process_swap_event(&self, log: &Log) -> Result<()> {
    // Extract REAL pool address
    let pool_address = log.address;

    // Try to get from cache first (fast path)
    let pool_info = match self.pool_cache.get(&pool_address).await {
        Some(info) => info,
        None => {
            // Queue for discovery (don't block!)
            self.pool_cache.queue_discovery(pool_address).await?;

            // Use temporary placeholder but log warning
            warn!("Unknown pool {} queued for discovery", pool_address);

            // Return early or use partial data
            return Ok(()); // Skip this event for now
        }
    };

    // Now we have REAL addresses!
    let pool_swap_tlv = PoolSwapTLV {
        timestamp,
        pool_address: pool_info.pool_address,  // REAL!
        token0: pool_info.token0,              // REAL!
        token1: pool_info.token1,              // REAL!
        token0_decimals: pool_info.token0_decimals,
        token1_decimals: pool_info.token1_decimals,
        // ... rest of fields
    };

    // Update pool state
    self.pool_state_manager.update_from_swap(&pool_swap_tlv).await?;

    // ... rest of processing
}
```

### Cargo.toml Addition
```toml
[dependencies]
torq-state-market = { path = "../../../libs/state/market" }
```

## ✅ Acceptance Criteria

1. **Performance Requirements**
   - [ ] Cache lookups complete in <1μs
   - [ ] Insert operations complete in <10μs
   - [ ] Thread-safe for concurrent access
   - [ ] Memory usage <50MB for 10,000 pools

2. **Functionality Requirements**
   - [ ] LRU eviction when cache full
   - [ ] Hit/miss rate tracking
   - [ ] Atomic persistence to disk
   - [ ] Graceful recovery from corrupted cache file

3. **Code Quality**
   - [ ] Comprehensive unit tests
   - [ ] Documentation for all public methods
   - [ ] No unsafe code unless justified
   - [ ] Follow Torq conventions (no unwrap() in production)

## 🧪 Testing Instructions

```bash
# Run your specific tests
cargo test --package services_v2 pool_cache

# Check performance
cargo bench --package services_v2 pool_cache

# Verify no regression in hot path
cargo test --package protocol_v2 --test performance_regression
```

## 🔄 Rollback Instructions

If this integration causes issues in production:

```bash
# Immediate rollback
git revert HEAD
git push origin main

# Or rollback to specific commit before this change
git checkout main
git reset --hard <commit-before-pool-001>
git push --force-with-lease origin main

# Clear any corrupted cache files
rm -f /var/lib/torq/pool_cache.tlv
rm -f /var/lib/torq/pool_cache.tlv.journal

# Restart affected services
systemctl restart torq-polygon-collector
systemctl restart torq-market-data-relay
```

## 📤 Commit & Push Instructions

```bash
# Stage your changes
git add services_v2/adapters/src/polygon/pool_cache.rs
git add services_v2/adapters/src/polygon/mod.rs
git add services_v2/adapters/src/polygon/types.rs

# Commit with descriptive message
git commit -m "feat(pool): implement high-performance pool metadata cache

- Add PoolCache with LRU eviction and hit rate tracking
- Support atomic persistence for crash recovery
- Maintain <1μs lookup performance with RwLock
- Add comprehensive unit tests for cache operations"

# Push to remote (first time)
git push -u origin fix/pool-cache-structure

# Subsequent pushes
git push
```

## 🔄 Pull Request Template

```markdown
## Task POOL-001: Pool Cache Structure

### Summary
Implemented high-performance cache for pool metadata with persistence support.

### Changes
- Created `pool_cache.rs` with thread-safe HashMap implementation
- Added LRU eviction for memory management
- Implemented atomic file persistence for crash recovery
- Added hit rate tracking for monitoring

### Performance
- Cache lookup: 0.8μs (✅ <1μs target)
- Insert operation: 7μs (✅ <10μs target)
- Memory usage: 32MB for 10,000 pools (✅ <50MB target)

### Testing
- [x] Unit tests passing
- [x] Benchmark results attached
- [x] No hot path regression

### Checklist
- [x] Code follows Torq conventions
- [x] No unwrap() in production code
- [x] Documentation complete
- [x] Ready for review
```

## ⚠️ Important Notes

1. **DO NOT** use standard Mutex - use tokio::sync::RwLock for async
2. **DO NOT** block on I/O operations in cache methods
3. **ENSURE** thread safety - this will be accessed from multiple tokio tasks
4. **CONSIDER** pre-populating with known high-volume pools
5. **IMPLEMENT** metrics for monitoring (hit rate, size, evictions)

## 🤝 Coordination
- No dependencies on other tasks
- POOL-003 and POOL-004 will use your cache interface
- Keep interface stable once defined

---
*Questions? Check MASTER_COORDINATION.md or ask in the main terminal.*
