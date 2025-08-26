---
task_id: TEST-001
status: TODO
priority: CRITICAL
estimated_hours: 4
assigned_branch: test/unit-test-framework
assignee: TBD
created: 2025-01-27
completed: null
---

# TEST-001: Unit Test Framework for Protocol V2

## 🔴 CRITICAL INSTRUCTIONS
```bash
# BEFORE STARTING - VERIFY YOU'RE NOT ON MAIN:
git branch --show-current

# If you see "main", IMMEDIATELY run:
git checkout -b test/unit-test-framework

# NEVER commit directly to main!
```

## Status
**Status**: TODO
**Priority**: CRITICAL
**Branch**: `test/unit-test-framework`
**Estimated**: 4 hours

## Problem Statement
The protocol_v2 crate lacks comprehensive unit tests for TLV message serialization/deserialization. This is the foundation of our entire message system and must be bulletproof.

## Acceptance Criteria
- [ ] Every TLV message type has encode/decode tests
- [ ] Round-trip testing (encode → decode → encode) verified
- [ ] Edge cases tested (empty payloads, max sizes, invalid data)
- [ ] Test coverage >90% for protocol_v2/src/tlv/
- [ ] Tests run in <1 second
- [ ] No hardcoded test data - use builders/factories

## Technical Approach - Rust Testing Convention

### 🏗️ The Idiomatic Approach: Both Unit AND Integration Tests

Following Rust ecosystem standards, we implement **both** testing approaches:

#### 1. Unit Tests (Inside src/ modules)
**Location**: `#[cfg(test)] mod tests {}` blocks at the bottom of each `.rs` file  
**Purpose**: White-box testing of private functions and internal logic  
**Access**: Can test both private and public functions

#### 2. Integration Tests (In tests/ directory) 
**Location**: `protocol_v2/tests/` directory  
**Purpose**: Black-box testing of public API from user's perspective  
**Access**: Only public functions (what external users would call)

### Files to Create/Modify

#### Unit Tests (Inside src/)
- `protocol_v2/src/tlv/market_data.rs` - Add `#[cfg(test)] mod tests {}`
- `protocol_v2/src/tlv/signals.rs` - Add `#[cfg(test)] mod tests {}`  
- `protocol_v2/src/tlv/execution.rs` - Add `#[cfg(test)] mod tests {}`
- `protocol_v2/src/identifiers/mod.rs` - Add `#[cfg(test)] mod tests {}`

#### Integration Tests (In tests/)
- `protocol_v2/tests/tlv_integration.rs` - Full workflow testing
- `protocol_v2/tests/precision_validation.rs` - End-to-end precision tests

### Implementation Steps

#### Step 1: Unit Tests (White-Box Testing)
```rust
// In protocol_v2/src/tlv/market_data.rs

fn internal_validation(trade: &TradeTLV) -> bool {
    // Private function - only unit tests can access this
    trade.price > 0 && trade.quantity > 0
}

pub fn create_trade_tlv(price: i64, quantity: i64) -> Result<TradeTLV, Error> {
    let trade = TradeTLV { price, quantity, ..Default::default() };
    if internal_validation(&trade) {
        Ok(trade)
    } else {
        Err(Error::InvalidTrade)
    }
}

#[cfg(test)]
mod tests {
    use super::*; // Import everything from parent module
    
    #[test]
    fn test_internal_validation() {
        // We CAN test private functions in unit tests!
        let valid_trade = TradeTLV { price: 100, quantity: 50, ..Default::default() };
        assert!(internal_validation(&valid_trade));
        
        let invalid_trade = TradeTLV { price: -100, quantity: 50, ..Default::default() };
        assert!(!internal_validation(&invalid_trade));
    }
    
    #[test]
    fn test_trade_serialization_roundtrip() {
        // Given: Valid trade data
        let original = TradeTLV {
            instrument_id: InstrumentId::from_bytes(&[1, 2, 3, 4]),
            price: 4500000000000, // $45,000.00 (8 decimal precision)
            quantity: 100000000,   // 1.0 token (18 decimal precision)
            timestamp: 1234567890123456789,
            is_buy: true,
        };
        
        // When: Serialize and deserialize
        let bytes = original.as_bytes();
        let decoded = TradeTLV::from_bytes(&bytes).unwrap();
        
        // Then: Perfect round-trip
        assert_eq!(decoded, original);
        assert_eq!(decoded.price, 4500000000000); // Exact precision
    }
    
    #[test]
    fn test_edge_cases() {
        // Test zero values
        let zero_trade = TradeTLV { price: 0, quantity: 0, ..Default::default() };
        let result = create_trade_tlv(0, 0);
        assert!(result.is_err()); // Should fail validation
        
        // Test maximum values
        let max_trade = TradeTLV { 
            price: i64::MAX, 
            quantity: i64::MAX, 
            ..Default::default() 
        };
        let bytes = max_trade.as_bytes();
        let decoded = TradeTLV::from_bytes(&bytes).unwrap();
        assert_eq!(decoded.price, i64::MAX);
    }
}
```

#### Step 2: Integration Tests (Black-Box Testing)
```rust
// In protocol_v2/tests/tlv_integration.rs

use protocol_v2::{create_trade_tlv, TLVMessageBuilder}; // Only public API

#[test]
fn test_full_tlv_message_workflow() {
    // This simulates how external users would interact with our crate
    
    // We CANNOT call internal_validation() here - it's private!
    // We can only test the public API
    
    let trade = create_trade_tlv(4500000000000, 100000000).unwrap();
    
    let mut builder = TLVMessageBuilder::new();
    builder.add_tlv(TLVType::Trade, &trade);
    let message = builder.build();
    
    assert!(message.is_ok());
    assert_eq!(message.unwrap().payload_size, trade.size());
}

#[test]
fn test_multi_component_integration() {
    // Test that different public components work together
    let trade = create_trade_tlv(1000000000, 50000000).unwrap();
    let quote = create_quote_tlv(999999999, 1000000001).unwrap();
    
    let mut builder = TLVMessageBuilder::new();
    builder.add_tlv(TLVType::Trade, &trade);
    builder.add_tlv(TLVType::Quote, &quote);
    
    let message = builder.build().unwrap();
    assert!(message.payload_size > trade.size() + quote.size());
}
```

#### Step 3: Test Utilities (Shared builders)
```rust
// protocol_v2/src/test_utils.rs
pub mod builders {
    use super::*;
    
    pub fn valid_trade_tlv() -> TradeTLV {
        TradeTLV {
            instrument_id: InstrumentId::from_venue_and_symbol("uniswap_v3", "WETH/USDC"),
            price: 4500000000000, // $45,000.00 (8 decimal precision)
            quantity: 100000000,   // 1.0 WETH (18 decimal precision)
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as i64,
            is_buy: true,
        }
    }
    
    pub fn max_size_trade_tlv() -> TradeTLV {
        TradeTLV {
            price: i64::MAX,
            quantity: i64::MAX,
            ..valid_trade_tlv()
        }
    }
}
```

### Testing Hierarchy Summary

| Test Type | Location | Access | Purpose | Example |
|-----------|----------|--------|---------|---------|
| **Unit Tests** | `src/tlv/market_data.rs` | Private + Public | Test algorithms, edge cases, internal state | `assert!(internal_validation(&trade))` |
| **Integration Tests** | `tests/tlv_integration.rs` | Public only | Test workflows, component interaction | `create_trade_tlv().unwrap()` |
| **E2E Tests** | `tests/e2e/` | Full system | Test complete message flows | Relay → Consumer communication |

## Testing Instructions
```bash
# Run unit tests for protocol_v2
cargo test --package protocol_v2 --lib

# Check coverage
cargo tarpaulin --packages protocol_v2 --lib --out Html

# Run with verbose output
cargo test --package protocol_v2 --lib -- --nocapture

# Benchmark test performance
cargo test --package protocol_v2 --lib --release -- --test-threads=1
```

## Git Workflow
```bash
# 1. Start on your branch
git checkout -b test/unit-test-framework

# 2. Make changes and commit
git add protocol_v2/src/
git commit -m "test: add comprehensive unit tests for protocol_v2 TLV messages"

# 3. Push to your branch
git push origin test/unit-test-framework

# 4. Create PR
gh pr create --title "TEST-001: Unit test framework for protocol_v2" \
             --body "Implements comprehensive unit testing for TLV serialization"
```

## Completion Checklist
- [ ] Working on correct branch (not main)
- [ ] All TLV types have unit tests
- [ ] Round-trip tests passing
- [ ] Edge cases covered
- [ ] Coverage >90%
- [ ] Tests run in <1 second
- [ ] PR created
- [ ] Updated task status to COMPLETE

## Notes
Focus on testing the critical path: message serialization/deserialization. This is where precision bugs hide!