# Pool Cache and Full Address Implementation

## Executive Summary

This document describes the critical architectural change to enable trade execution in the AlphaPulse system by storing and transmitting full 20-byte Ethereum addresses instead of truncated 8-byte identifiers.

## The Problem

### Current State (Broken)
The system receives DEX swap events from Polygon but **cannot execute trades** because:
1. Token addresses are truncated from 20 bytes to 8 bytes for "efficiency"
2. Smart contracts require full 20-byte addresses to execute trades
3. The truncated addresses cannot be reversed to recover the full address

**Example:**
```
Full USDC address:     0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48 (20 bytes)
Truncated in system:   0xa0b86991c6218b36 (8 bytes)
Lost forever:          c1d19d4a2e9eb0ce3606eb48 (12 bytes)
```

### Impact
- ❌ Cannot execute arbitrage trades (missing address information)
- ❌ Cannot interact with DEX smart contracts
- ❌ System can only detect opportunities, not act on them

## The Solution

### Enhanced Pool Cache Architecture

Replace truncated IDs with full addresses throughout the system:

```
BEFORE: Polygon Event → Truncated IDs → Cannot Execute
AFTER:  Polygon Event → Full Addresses → Can Execute Trades
```

### Key Components

#### 1. Enhanced Pool Cache (`EnhancedPoolCache`)
Responsible for discovering and caching complete pool information:

```rust
pub struct PoolInfo {
    pub token0: H160,              // Full 20-byte address
    pub token1: H160,              // Full 20-byte address
    pub token0_decimals: u8,       // e.g., 18 for WETH, 6 for USDC
    pub token1_decimals: u8,       // Critical for amount calculations
    pub pool_type: PoolType,       // V2, V3, etc.
    pub discovered_at: u64,        // Timestamp for cache invalidation
}
```

#### 2. TLV Message Updates
All pool-related TLV structures now include full addresses:

```rust
pub struct PoolSwapTLV {
    pub venue: VenueId,
    pub pool_address: [u8; 20],    // Full pool contract address
    pub token_in_addr: [u8; 20],   // Full token address for execution
    pub token_out_addr: [u8; 20],  // Full token address for execution
    pub amount_in: i64,
    pub amount_out: i64,
    // ... other fields
}
```

#### 3. RPC Discovery Process
When a new pool is encountered:

```
1. Swap event received from blockchain
2. Check cache for pool information
3. If not cached:
   a. Query pool contract for token0 address (RPC call)
   b. Query pool contract for token1 address (RPC call)
   c. Query token contracts for decimals (RPC calls)
   d. Cache complete information
4. Build TLV with full addresses
5. Send to downstream consumers
```

## Implementation Details

### Pool Discovery Flow

```rust
async fn process_swap_log(&mut self, log: &Log) -> Result<TLVMessage> {
    // Block until we have complete pool information
    let pool_info = self.pool_cache
        .get_or_discover_pool(log.address)
        .await?;
    
    // Parse swap data from log
    let (amount_in, amount_out) = parse_swap_amounts(&log)?;
    
    // Build complete TLV message
    let swap_tlv = PoolSwapTLV {
        pool_address: log.address.to_fixed_bytes(),
        token_in_addr: pool_info.token0.to_fixed_bytes(),
        token_out_addr: pool_info.token1.to_fixed_bytes(),
        amount_in,
        amount_out,
        // ... decimals and other fields
    };
    
    Ok(swap_tlv.to_tlv_message())
}
```

### Resilient RPC Handling

The system implements robust RPC communication:

1. **Retry Logic**: Exponential backoff for failed calls
2. **Failover**: Primary and backup RPC endpoints
3. **Rate Limiting**: Respect provider limits (e.g., 1500 req/sec for Ankr)
4. **Caching**: Never query the same pool twice

```rust
async fn resilient_rpc_call<T>(&self, operation: impl Fn() -> Future<T>) -> Result<T> {
    let mut delay = Duration::from_millis(100);
    
    for attempt in 0..=3 {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) if attempt < 3 => {
                sleep(delay).await;
                delay *= 2; // Exponential backoff
            }
            Err(e) => return Err(e),
        }
    }
}
```

### Performance Optimizations

#### Batch Discovery
Process multiple new pools in parallel:
```rust
stream::iter(new_pool_addresses)
    .map(|addr| self.discover_single_pool(addr))
    .buffer_unordered(10)  // Max 10 concurrent RPC calls
    .collect()
    .await
```

#### Event Queuing
Events are queued while pool discovery is in progress:
```rust
pub struct EnhancedPoolCache {
    pending_events: VecDeque<Log>,         // Queue while discovering
    discovery_in_progress: HashSet<H160>,  // Prevent duplicate RPC calls
}
```

## Migration Path

### Phase 1: Protocol Updates ✅
- Updated TLV structures to include full addresses
- Modified serialization/deserialization methods
- Removed truncated PoolInstrumentId system

### Phase 2: Collector Integration (In Progress)
- Implement EnhancedPoolCache with RPC discovery
- Update process_swap_log to fetch full addresses
- Add resilient RPC handling

### Phase 3: Downstream Updates
- Update strategies to use full addresses for execution
- Modify caching to use pool addresses as keys
- Update tests for new message formats

## Configuration

### RPC Endpoints
```toml
[rpc]
primary_url = "https://polygon-rpc.com"
backup_urls = ["https://rpc-mainnet.matic.network"]
rate_limit_per_second = 1000
timeout_ms = 5000
max_retries = 3
```

### Known Pools Snapshot (Optional)
To avoid RPC calls on startup, load known pools from snapshot:
```json
{
  "0x45dda9cb7c25131df268515131f647d726f50608": {
    "token0": "0x2791bca1f2de4661ed88a30c99a7a9449aa84174",
    "token1": "0x7ceb23fd6bc0add59e62ac25578270cff1b9f619",
    "token0_decimals": 6,
    "token1_decimals": 18
  }
}
```

## Benefits

### Immediate
- ✅ **Enables Trade Execution**: Full addresses allow smart contract interaction
- ✅ **Self-Contained Messages**: Each TLV has all information needed
- ✅ **No External Dependencies**: No need for separate address registries

### Long-term
- ✅ **Simpler Architecture**: Removed complex ID system
- ✅ **Better Maintainability**: Direct use of addresses
- ✅ **Production Ready**: Resilient RPC handling and caching

## Common Issues and Solutions

### Issue: RPC Rate Limits
**Solution**: Implement token bucket rate limiting and use multiple endpoints

### Issue: Pool Discovery Latency
**Solution**: Pre-warm cache with known pools on startup

### Issue: RPC Failures
**Solution**: Exponential backoff with failover to backup endpoints

### Issue: Memory Usage
**Solution**: LRU cache with configurable size limits

## Testing

### Unit Tests
- Pool discovery with mocked RPC
- Cache hit/miss scenarios
- Resilient RPC retry logic

### Integration Tests
- End-to-end: Event → Discovery → TLV
- Parallel discovery performance
- Failover scenarios

### Load Tests
- 1000+ new pools discovered in parallel
- Sustained event processing at 10,000 events/sec
- Memory usage under high cache pressure

## Monitoring

Key metrics to track:
- RPC call success rate
- Pool discovery latency (p50, p95, p99)
- Cache hit ratio
- Queue depth for pending events
- Memory usage of cache

## Conclusion

This implementation solves the critical execution problem by ensuring every message contains the complete information needed to interact with blockchain smart contracts. The architecture is resilient, performant, and production-ready.

## Documentation Updates Required

After testing is complete, the following documentation must be updated:

### 1. `protocol.md`
- Update TLV structure definitions to show full address fields
- Update message size calculations
- Document the removal of PoolInstrumentId

### 2. `message-types.md`
- Update all pool-related TLV type specifications
- Show new field layouts with [u8; 20] addresses
- Update size constraints for each message type

### 3. `CLAUDE.md`
- Add section on full address requirement for execution
- Update examples to show new TLV structures
- Document the pool discovery process

### 4. `MAINTENANCE.md`
- Add pool cache maintenance procedures
- Document RPC endpoint rotation
- Add monitoring requirements for pool discovery

### 5. `README.md`
- Update architecture diagram if addresses are shown
- Note the execution capability enhancement
- Update quick start examples if they show TLV creation

## References

- [Protocol V2 Specification](./protocol.md)
- [TLV Message Format](./message-types.md)
- [Polygon DEX Integration](../services_v2/adapters/README.md)