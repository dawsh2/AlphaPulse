# Task POOL-002: Swap Event Address Extraction
*Agent Type: Event Parser Specialist*
*Branch: `fix/swap-event-extraction`*

## 📋 Your Mission
Fix the swap event processing to extract REAL pool and token addresses from Polygon events instead of using placeholders.

## 🎯 Context
Currently in `polygon.rs:765-766`, we're using `[0u8; 20]` placeholders. The ACTUAL addresses are available in the event data - we just need to extract them properly!

## 🔧 Git Setup Instructions

```bash
# Step 1: Start fresh from main
git checkout main
git pull origin main

# Step 2: Create your feature branch
git checkout -b fix/swap-event-extraction

# Step 3: Confirm branch
git branch --show-current  # Should show: fix/swap-event-extraction
```

## 📝 Task Specification

### Files to Modify
1. `services_v2/adapters/src/polygon/polygon.rs` (main fix)
2. `services_v2/adapters/src/polygon/event_decoder.rs` (create if needed)

### Current Broken Code (polygon.rs:765-766)
```rust
// CURRENT - BROKEN!
let pool_swap_tlv = PoolSwapTLV {
    timestamp,
    pool_address: [0u8; 20],  // PLACEHOLDER - FIX THIS!
    token0: [0u8; 20],        // PLACEHOLDER - FIX THIS!
    token1: [0u8; 20],        // PLACEHOLDER - FIX THIS!
    // ... rest of fields
};
```

### Required Implementation

```rust
// In process_swap_event() - around line 765

// Step 1: Extract pool address from log
let pool_address = log.address; // This is the ACTUAL pool address!

// Step 2: Detect protocol version from event signature
let is_v3_swap = log.topics[0] == *V3_SWAP_SIGNATURE;
let is_v2_swap = log.topics[0] == *V2_SWAP_SIGNATURE;

// Step 3: Extract token addresses based on protocol
let (token0, token1) = if is_v3_swap {
    // V3: Tokens might be in topics or need RPC call
    extract_v3_tokens(&log)?
} else if is_v2_swap {
    // V2: Always need RPC call or cache lookup
    extract_v2_tokens(&log, pool_address).await?
} else {
    // Unknown protocol - log warning
    warn!("Unknown swap event signature: {:?}", log.topics[0]);
    ([0u8; 20], [0u8; 20]) // Fallback only for unknown
};

// Step 4: Create TLV with REAL addresses
let pool_swap_tlv = PoolSwapTLV {
    timestamp,
    pool_address: pool_address.0,  // Use REAL address!
    token0,                         // Use REAL token!
    token1,                         // Use REAL token!
    // ... rest of fields with proper extraction
};
```

### Helper Functions to Implement

```rust
// event_decoder.rs - New file or add to polygon.rs

use web3::types::{H256, H160, Log};

// Keccak256 hashes of event signatures
lazy_static! {
    // Swap(address,uint256,uint256,uint256,uint256,address)
    static ref V2_SWAP_SIGNATURE: H256 = H256::from_slice(
        &hex!("d78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822")
    );

    // Swap(address,address,int256,int256,uint160,uint128,int24)
    static ref V3_SWAP_SIGNATURE: H256 = H256::from_slice(
        &hex!("c42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67")
    );
}

fn extract_v3_tokens(log: &Log) -> Result<([u8; 20], [u8; 20]), ParseError> {
    // V3 specific: Parse from topics or data
    // Topics[1] = sender, Topics[2] = recipient
    // Need to determine token addresses from pool

    // For now, return placeholder - will need cache/RPC
    Ok(([0u8; 20], [0u8; 20]))
}

async fn extract_v2_tokens(
    log: &Log,
    pool_address: H160
) -> Result<([u8; 20], [u8; 20]), ParseError> {
    // V2: Tokens not in event, need from cache or RPC

    // This is where POOL-001's cache will be used!
    // For now, implement temporary logic

    // Temporary: Just note that we need the cache
    warn!("Pool {} needs token discovery", pool_address);
    Ok(([0u8; 20], [0u8; 20]))
}

fn extract_swap_amounts(log: &Log, is_v3: bool) -> SwapAmounts {
    if is_v3 {
        // V3: amount0 and amount1 are in data as int256
        parse_v3_amounts(&log.data)
    } else {
        // V2: amounts in data as uint256
        parse_v2_amounts(&log.data)
    }
}
```

## ✅ Acceptance Criteria

1. **Address Extraction**
   - [ ] Pool address extracted from `log.address`
   - [ ] V2/V3 protocol detection working
   - [ ] Token extraction logic implemented
   - [ ] No more hardcoded `[0u8; 20]` for known pools

2. **Event Parsing**
   - [ ] Correctly identify V2 vs V3 swaps
   - [ ] Parse amounts based on protocol version
   - [ ] Handle unknown event signatures gracefully
   - [ ] Log warnings for unrecognized pools

3. **Code Quality**
   - [ ] Proper error handling (no panics)
   - [ ] Informative log messages
   - [ ] Unit tests for event parsing
   - [ ] Comments explaining V2/V3 differences

## 🧪 Testing Instructions

```bash
# Test with real Polygon events
cargo test --package services_v2 event_extraction

# Run against live data (if available)
RUST_LOG=debug cargo run --bin polygon_collector_test

# Verify TLV construction
cargo test --package protocol_v2 pool_swap_tlv
```

## 📤 Commit & Push Instructions

```bash
# Stage your changes
git add services_v2/adapters/src/polygon/polygon.rs
git add services_v2/adapters/src/polygon/event_decoder.rs  # if created

# Commit with clear message
git commit -m "fix(polygon): extract real addresses from swap events

- Extract pool address from log.address instead of placeholder
- Detect V2 vs V3 protocol from event signature
- Add helper functions for token extraction
- Prepare integration points for pool cache"

# Push to remote
git push -u origin fix/swap-event-extraction
```

## 🔄 Pull Request Template

```markdown
## Task POOL-002: Swap Event Address Extraction

### Summary
Fixed critical issue where swap events used placeholder addresses instead of real ones.

### Changes
- Extract pool address from `log.address`
- Implement V2/V3 protocol detection via event signatures
- Add token extraction logic (pending cache integration)
- Remove hardcoded `[0u8; 20]` placeholders

### Key Improvements
- Pool addresses now correctly extracted from events
- Protocol version detection enables proper parsing
- Foundation laid for token discovery system

### Testing
- [x] Event parsing tests pass
- [x] V2/V3 detection validated
- [x] No performance regression

### Dependencies
- Prepared for POOL-001 cache integration
- Token extraction will use cache when available

### Ready for Review
- [x] All acceptance criteria met
- [x] Tests passing
- [x] Documentation complete
```

## ⚠️ Important Notes

1. **Event Signatures**: V2 and V3 have DIFFERENT Swap event signatures!
2. **Token Order**: Ensure token0 < token1 (address ordering) for consistency
3. **Amount Signs**: V3 uses signed integers, V2 uses unsigned
4. **Topics vs Data**: Indexed parameters in topics, rest in data
5. **Decimal Handling**: Don't normalize - preserve native precision!

## 🤝 Coordination
- Your extraction logic will be used by POOL-005 for TLV integration
- POOL-003/004 will provide token addresses you can't extract directly
- Keep log messages informative for debugging

## 🔍 Reference: Event Structures

```solidity
// Uniswap V2 Swap Event
event Swap(
    address indexed sender,
    uint amount0In,
    uint amount1In,
    uint amount0Out,
    uint amount1Out,
    address indexed to
);

// Uniswap V3 Swap Event
event Swap(
    address indexed sender,
    address indexed recipient,
    int256 amount0,
    int256 amount1,
    uint160 sqrtPriceX96,
    uint128 liquidity,
    int24 tick
);
```

---
*Remember: The pool address is RIGHT THERE in log.address - we just weren't using it!*
