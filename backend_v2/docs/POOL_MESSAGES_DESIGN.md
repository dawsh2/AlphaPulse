# Pool Message Types Design

## Problem
We have overlapping and confusing message types for pool events. A swap can appear as:
- PoolSwapTLV (just amounts)
- PoolUpdateTLV with swap data
- Part of state tracking

## Solution: Clear Separation by Purpose

### 1. Raw Event Messages (What Happened)
These are direct translations of blockchain events:

```rust
/// V2 Sync Event - Full state after any change
PoolSyncTLV {
    pool_id: PoolInstrumentId,
    reserve0: i64,          // Complete reserve0 (8 decimals)
    reserve1: i64,          // Complete reserve1 (8 decimals)
    timestamp_ns: u64,
    block_number: u64,
}

/// Swap Event - Trade execution details
PoolSwapTLV {
    pool_id: PoolInstrumentId,
    token_in: u64,
    token_out: u64,
    amount_in: i64,
    amount_out: i64,
    // V3 specific (0 for V2):
    sqrt_price_x96_after: u64,  // New price after swap
    tick_after: i32,             // New tick after swap
    liquidity_after: i64,        // Active liquidity after swap
    timestamp_ns: u64,
    block_number: u64,
}

/// Mint Event - Liquidity addition
PoolMintTLV {
    pool_id: PoolInstrumentId,
    provider: u64,
    amount0: i64,
    amount1: i64,
    liquidity_delta: i64,
    tick_lower: i32,    // V3 only (i32::MIN for V2)
    tick_upper: i32,    // V3 only (i32::MAX for V2)
    timestamp_ns: u64,
    block_number: u64,
}

/// Burn Event - Liquidity removal
PoolBurnTLV {
    pool_id: PoolInstrumentId,
    provider: u64,
    amount0: i64,
    amount1: i64,
    liquidity_delta: i64,
    tick_lower: i32,    // V3 only
    tick_upper: i32,    // V3 only
    timestamp_ns: u64,
    block_number: u64,
}
```

### 2. State Messages (Current State)
These represent the current state of a pool:

```rust
/// Complete pool state snapshot
PoolStateTLV {
    pool_id: PoolInstrumentId,
    // V2 state:
    reserve0: i64,
    reserve1: i64,
    // V3 state:
    sqrt_price_x96: u64,
    tick: i32,
    liquidity: i64,
    // Common:
    fee_rate: u32,
    last_update_block: u64,
    last_update_ns: u64,
}
```

### 3. Processing Flow

```
Blockchain Events → Collector → Event TLVs → Relay → Pool State Service
                                                    ↓
                                            Maintain State
                                                    ↓
                                            Strategy Services
```

## Key Principles

1. **Event messages are immutable facts** - They record what happened
2. **State messages are derived** - Built from event stream
3. **No duplication** - Each fact appears in exactly one message type
4. **Protocol agnostic at event level** - V2/V3 differences handled by state service

## Message Flow Examples

### V2 Pool Discovery
1. First swap → `PoolSwapTLV` (amounts only)
2. Followed by → `PoolSyncTLV` (full reserves)
3. State service creates initial `PoolStateTLV`

### V3 Pool Discovery  
1. First swap → `PoolSwapTLV` (includes sqrt_price, tick, liquidity)
2. State service creates initial `PoolStateTLV` from swap data
3. No sync event needed - swap has everything

### Ongoing Updates
- Every V2 swap → `PoolSwapTLV` + `PoolSyncTLV`
- Every V3 swap → `PoolSwapTLV` (with state)
- Mints/Burns → `PoolMintTLV`/`PoolBurnTLV`
- State service maintains current state

## Benefits

1. **Clear semantics** - Each message has one purpose
2. **No redundancy** - Swap data isn't duplicated
3. **Event sourcing** - Can rebuild state from events
4. **Protocol flexibility** - New protocols just need new event types
5. **Efficient** - V2 gets full state from Sync, V3 from Swap

## Implementation Plan

1. Update PoolSwapTLV to include V3 state fields
2. Create PoolSyncTLV for V2 sync events
3. Remove PoolUpdateTLV (replaced by specific events)
4. Pool state service consumes all event types
5. Maintains state using appropriate logic per protocol