# Adapter Validation Reference Manual

## Overview
This document provides the technical procedures for the mandatory four-step validation pipeline that EVERY data type from EVERY adapter must complete before production use.

**CRITICAL**: This validation is performed during development only. Production code does not run these validation procedures.

**Architecture Note**: Validation tests the stateless data transformation performed by adapters. Adapters are pure functions: Raw Data → TLV Messages. State management, invalidation logic, and subscription tracking are handled by relays and consumers, not adapters.

## Validation Requirements
- **Mandatory for all data types**: No exceptions
- **Development-time only**: Not run in production
- **Zero tolerance for data loss**: Perfect roundtrip required
- **Real data only**: No mocks or synthetic data permitted

## System Quality Philosophy During Validation

**CRITICAL: Never bypass deeper architectural issues to complete validation tasks.**

When compilation errors or test failures occur during validation:
1. **Address the root cause** - Fix underlying system problems, don't work around them
2. **No workarounds or bypasses** - Don't create isolated tests or mock implementations to avoid compilation errors
3. **System-level thinking** - Consider impact on entire codebase, not just immediate validation task
4. **Quality over speed** - Take time to fix foundational issues properly

**The global goal is always producing high-quality, system-level code. Validation task completion must never blind us to this objective.**

If validation cannot run due to broader codebase issues, fix the compilation errors system-wide first.

## Four-Step Validation Process

Every step in the data pipeline MUST be validated to ensure correctness. Each step catches different failure modes:

| Step | Purpose | Common Failures Caught |
|------|---------|----------------------|
| 1. Raw Parsing | External → Struct | Missing fields, wrong types, precision loss |
| 2. Serialization | Struct → Binary | Overflow, wrong format, encoding errors |
| 3. Deserialization | Binary → Struct | Corruption, alignment issues, truncation |
| 4. Deep Equality | Round-trip check | Any data loss through pipeline |

### Step 1: Validate Raw Data Parsing
Verify that exchange/blockchain data is parsed correctly without data loss.

```rust
pub fn validate_raw_parsing(raw_json: &Value, parsed: &ExchangeEvent) -> Result<()> {
    // 1. All required fields extracted
    assert!(!parsed.price.is_empty(), "Price field missing");
    assert!(!parsed.symbol.is_empty(), "Symbol field missing");
    
    // 2. Semantic correctness - parsed matches original
    if let Some(original_price) = raw_json["price"].as_str() {
        assert_eq!(parsed.price, original_price, "Price parsing changed value");
    }
    
    // 3. Field values are reasonable (NOT business logic)
    let price = Decimal::from_str(&parsed.price)?;
    assert!(price > Decimal::ZERO, "Price must be positive");
    
    // 4. No precision loss during parsing
    // Example: "123.456789" should stay as string, not become f64
    assert!(parsed.preserves_decimal_precision(), "Precision lost");
    
    Ok(())
}
```

**Key Points:**
- Check structural integrity, not business rules
- Preserve original precision (use strings/Decimal, not f64)
- Validate parsed data matches source exactly

### Step 2: Validate TLV Serialization
Verify that parsed data correctly converts to Protocol V2 binary format.

```rust
pub fn validate_tlv_serialization(parsed: &ExchangeEvent) -> Result<Vec<u8>> {
    // 1. Convert to TLV struct
    let tlv = TradeTLV::try_from(parsed.clone())?;
    
    // 2. Validate TLV fields before serialization
    assert_eq!(tlv.venue().unwrap(), VenueId::Coinbase, "Wrong venue");
    assert!(tlv.price > 0, "Price lost in conversion");
    assert!(tlv.timestamp_ns > 0, "Invalid timestamp");
    
    // 3. Serialize to bytes (use as_bytes() not to_bytes())
    let bytes = tlv.as_bytes().to_vec();
    
    // 4. Validate serialized format
    assert!(!bytes.is_empty(), "Empty serialization");
    assert_eq!(bytes.len(), std::mem::size_of::<TradeTLV>(), "Wrong size");
    
    // 5. Check TLV header is correct
    let header = &bytes[0..4];
    // Verify header contains correct type and length
    
    Ok(bytes)
}
```

**Common Issues:**
- Using wrong API: `to_bytes()` doesn't exist, use `as_bytes()`
- Decimal overflow when converting to fixed-point
- Wrong InstrumentId constructor (use `coin()` not `crypto()`)

### Step 3: Validate TLV Deserialization  
Verify that binary format correctly deserializes back to TLV struct.

```rust
pub fn validate_tlv_deserialization(bytes: &[u8]) -> Result<TradeTLV> {
    // 1. Deserialize from bytes (use from_bytes() not read_from())
    let recovered = TradeTLV::from_bytes(bytes)?;
    
    // 2. CRITICAL: Copy packed fields before validation!
    // See PACKED_STRUCTS.md for why this is essential
    let price = recovered.price;      // Copy to avoid unaligned access
    let volume = recovered.volume;    // Copy to avoid unaligned access
    let timestamp = recovered.timestamp_ns;  // Copy to avoid unaligned access
    
    // 3. Structural validation - all fields present and valid
    assert_eq!(recovered.venue().unwrap(), VenueId::Coinbase, "Venue corrupted");
    assert!(price > 0, "Price corrupted: {}", price);
    assert!(volume > 0, "Volume corrupted: {}", volume);
    assert!(timestamp > 0, "Timestamp corrupted");
    
    // 4. No buffer overrun or underrun
    assert_eq!(bytes.len(), std::mem::size_of::<TradeTLV>(), "Size mismatch");
    
    Ok(recovered)
}
```

**Platform Considerations:**
- Test on ARM (M1/M2 Macs) for alignment issues
- Packed struct access can crash on some architectures
- Always copy fields before use!

### Step 4: Validate Deep Equality - Zero Data Loss
Verify perfect roundtrip with absolutely no data loss through the entire pipeline.

```rust
pub fn validate_deep_equality(original: &TradeTLV, recovered: &TradeTLV) -> Result<()> {
    // 1. Field-by-field equality (MUST copy packed fields!)
    let orig_price = original.price;
    let recv_price = recovered.price;
    assert_eq!(orig_price, recv_price, "Price precision lost: {} vs {}", orig_price, recv_price);
    
    let orig_volume = original.volume;
    let recv_volume = recovered.volume;  
    assert_eq!(orig_volume, recv_volume, "Volume precision lost");
    
    let orig_ts = original.timestamp_ns;
    let recv_ts = recovered.timestamp_ns;
    assert_eq!(orig_ts, recv_ts, "Timestamp precision lost");
    
    // 2. Structural equality - entire struct matches
    assert_eq!(original, recovered, "Structural equality failed");
    
    // 3. Binary equality - re-serialization test
    let original_bytes = original.as_bytes().to_vec();
    let recovered_bytes = recovered.as_bytes().to_vec();
    assert_eq!(original_bytes, recovered_bytes, "Binary representation differs");
    
    // 4. Hash verification for paranoid validation
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut h1 = DefaultHasher::new();
    original_bytes.hash(&mut h1);
    
    let mut h2 = DefaultHasher::new();
    recovered_bytes.hash(&mut h2);
    
    assert_eq!(h1.finish(), h2.finish(), "Hash mismatch - data corrupted!");
    
    println!("✅ Perfect round-trip: {} bytes, zero data loss", original_bytes.len());
    Ok(())
}
```

**What This Validates:**
- Every bit of precision is preserved
- No floating-point rounding errors
- No truncation or overflow
- Serialization is perfectly reversible
- Platform-independent consistency

### Complete Validation Pipeline Implementation
```rust
/// MANDATORY validation pipeline for all data types
/// This is called during development only, never in production
pub fn complete_validation_pipeline(raw_data: &[u8]) -> Result<PoolSwapTLV> {
    // Parse raw data using your adapter's parser
    let parsed = YourAdapter::parse(raw_data)?;
    
    // STEP 1: Validate raw data parsing
    validate_raw_parsing(raw_data, &parsed)?;
    
    // Transform to TLV format
    let original_tlv = PoolSwapTLV::from(parsed);
    
    // STEP 2: Validate TLV serialization
    let bytes = validate_tlv_serialization(&original_tlv)?;
    
    // STEP 3: Validate TLV deserialization  
    let recovered_tlv = validate_tlv_deserialization(&bytes)?;
    
    // STEP 4: Validate semantic & deep equality
    validate_equality(&original_tlv, &recovered_tlv)?;
    
    // Optional: Cross-validation with alternative data source
    if let Some(alt_source) = get_alternative_source() {
        cross_validate(&recovered_tlv, &alt_source)?;
    }
    
    Ok(recovered_tlv)
}

/// Integration with actual test framework
#[test]
fn test_[venue]_[data_type]_validation_pipeline() {
    let real_samples = load_fixtures("tests/fixtures/[venue]/[data_type]_real.json");
    
    for sample in real_samples {
        let result = complete_validation_pipeline(&sample);
        assert!(result.is_ok(), "Validation failed for sample: {:?}", sample);
    }
}
```

## Required Test Implementation

### Test Structure
```
tests/
├── fixtures/
│   └── [venue]/
│       └── [data_type]_real.json    # Real captured data samples
├── validation/
│   ├── token_address_validator.rs   # 🔥 CRITICAL: Token address validation
│   └── [venue]_[data_type].rs       # Validation pipeline tests
└── integration/
    └── [venue]_integration.rs       # Full system integration tests
```

**Critical Files:**
- `token_address_validator.rs` - **MANDATORY** for DeFi adapters. Validates token addresses, pool addresses, and decimals against actual blockchain state via RPC calls.

### Mandatory Test Pattern
```rust
#[test]
fn test_[venue]_[data_type]_validation_pipeline() {
    // Load multiple real data samples - NEVER use mocks
    let real_samples = load_fixtures("fixtures/[venue]/[data_type]_real.json");
    assert!(!real_samples.is_empty(), "Must have real data samples");
    
    for (i, sample) in real_samples.iter().enumerate() {
        // Run complete validation pipeline
        let result = complete_validation_pipeline(sample);
        assert!(result.is_ok(), 
            "Validation pipeline failed for sample {}: {:?}", i, result.err());
        
        let validated_tlv = result.unwrap();
        
        // Additional semantic checks specific to this data type
        validate_semantic_correctness(&validated_tlv)?;
    }
}

#[test] 
fn test_[venue]_[data_type]_edge_cases() {
    // Test boundary conditions and edge cases
    let edge_cases = load_fixtures("fixtures/[venue]/[data_type]_edge_cases.json");
    
    for edge_case in edge_cases {
        let result = complete_validation_pipeline(&edge_case);
        // May pass or fail depending on whether edge case is valid
        // Document expected behavior in test
    }
}

#[test]
fn test_[venue]_[data_type]_error_handling() {
    // Test malformed data handling
    let malformed_data = load_fixtures("fixtures/[venue]/[data_type]_malformed.json");
    
    for malformed in malformed_data {
        let result = complete_validation_pipeline(&malformed);
        assert!(result.is_err(), "Malformed data should be rejected");
    }
}
```

## Semantic Validation

### Purpose
Ensure that data fields are mapped to their correct semantic meaning, preventing issues like "fees stored in profit field".

### Common Semantic Errors

#### 1. Direction Confusion
```rust
// WRONG: Confusing input/output
let swap = PoolSwapTLV {
    amount_in: event.amount_out,  // Swapped!
    amount_out: event.amount_in,  // Swapped!
    // ...
};

// CORRECT: Validate semantics
fn validate_swap_direction(event: &SwapEvent) -> Result<(u128, u128, bool)> {
    match (event.amount0, event.amount1) {
        (pos, neg) if pos > 0 && neg < 0 => {
            Ok((pos as u128, (-neg) as u128, true))  // token0 in
        }
        (neg, pos) if neg < 0 && pos > 0 => {
            Ok((pos as u128, (-neg) as u128, false)) // token1 in
        }
        _ => Err("Invalid swap amounts")
    }
}
```

#### 2. Venue Confusion
```rust
// WRONG: Using protocol as venue
let swap = PoolSwapTLV {
    venue: VenueId::UniswapV3,  // Wrong! This is protocol, not venue
    // ...
};

// CORRECT: Use blockchain as venue
let swap = PoolSwapTLV {
    venue: VenueId::Polygon,    // Correct: blockchain is the venue
    // ...
};
```

#### 3. Precision Confusion
```rust
// WRONG: Mixing decimal places
let trade = TradeTLV {
    price: 45000,  // Is this $45,000 or $0.45?
    // ...
};

// CORRECT: Document and validate precision
let trade = TradeTLV {
    price: 4500000000000,  // $45,000.00 with 8 decimals
    // ...
};
assert!(trade.price > 100000000, "Price seems too low - check decimals");
```

### Semantic Test Matrix

| Field Type | Validation Rules | Example Test |
|------------|-----------------|--------------|
| Amounts | Must be positive after conversion | `assert!(amount > 0)` |
| Prices | Must be within reasonable range | `assert!(price > 100 && price < 10^15)` |
| Timestamps | Must be recent and not future | `assert!(ts > NOW - 1_DAY && ts <= NOW)` |
| Addresses | Must be valid format | `assert!(addr != [0u8; 20])` |
| Decimals | Must match token specification | `assert!(weth_decimals == 18)` |

## Precision Validation

### Numeric Precision Rules

#### For CEX Data (TradeTLV, QuoteTLV)
- Use `i64` with 8 decimal places
- Maximum value: ~$92 million
- Validation: `assert!(price < i64::MAX / 100000000)`

#### For DEX Data (PoolSwapTLV, etc.)
- Use `u128` for amounts (no scaling)
- Use native token precision (18 for ETH, 6 for USDC)
- Store decimals separately
- Validation: No truncation allowed

### Precision Test
```rust
#[test]
fn test_precision_preservation() {
    // Test with maximum blockchain values
    let amount = u128::from_str("193906370624164215157").unwrap();
    
    let swap = PoolSwapTLV {
        amount_in: amount,
        amount_in_decimals: 18,
        // ...
    };
    
    let bytes = swap.to_bytes();
    let recovered = PoolSwapTLV::from_bytes(&bytes).unwrap();
    
    // Must preserve exact value
    assert_eq!(swap.amount_in, recovered.amount_in);
    assert_eq!(swap.amount_in_decimals, recovered.amount_in_decimals);
    
    // Convert to human readable and back
    let human = amount as f64 / 10f64.powi(18);
    let back = (human * 10f64.powi(18)) as u128;
    
    // This will likely fail due to float precision!
    // This is why we store native amounts
    assert_ne!(amount, back, "Float conversion loses precision");
}
```

## Common Validation Failures & Solutions

### Failure: "Price precision lost"
**Symptom**: Step 4 fails with mismatched prices
**Cause**: Using f64 instead of Decimal or fixed-point
```rust
// WRONG
let price: f64 = 123.456789;  // Precision loss!

// CORRECT  
let price_str = "123.456789";
let price_decimal = Decimal::from_str(price_str)?;
let price_fixed = (price_decimal * Decimal::from(100_000_000)).to_i64()?;
```

### Failure: "Timestamp mismatch"
**Symptom**: Timestamps don't match in Step 4
**Cause**: Converting nanoseconds to milliseconds
```rust
// WRONG
let timestamp_ms = timestamp_ns / 1_000_000;  // Lost precision!

// CORRECT
let timestamp_ns = timestamp_ns;  // Keep full nanosecond precision
```

### Failure: Segmentation Fault in Tests
**Symptom**: Tests crash on M1/M2 Macs or ARM
**Cause**: Direct access to packed struct fields
```rust
// WRONG
assert_eq!(tlv.price, expected);  // Unaligned access!

// CORRECT
let price = tlv.price;  // Copy first
assert_eq!(price, expected);
```

### Failure: "InstrumentId mismatch"
**Symptom**: Different InstrumentIds for same symbol
**Cause**: Inconsistent symbol normalization
```rust
// Problem: "BTC-USD" vs "BTC/USD" vs "BTCUSD"
// Solution: Always normalize to slash format
let normalized = symbol.replace('-', "/").replace("_", "/");
```

## Cross-Source Validation

### Purpose
Validate data consistency by comparing with alternative sources.

### Token Address Validation (Critical Implementation)

**Location**: `tests/validation/token_address_validator.rs`

This is the **most critical validation** for DeFi data - ensuring token addresses, pool addresses, and decimals are correctly parsed by cross-checking against actual blockchain state.

```rust
use alphapulse_tests::validation::TokenAddressValidator;

#[tokio::test]
async fn test_token_address_validation() {
    let validator = TokenAddressValidator::new(
        "https://polygon-rpc.com/",
        "cache/",
        137 // Polygon chain ID
    ).await.unwrap();
    
    // Load real swap event from fixtures
    let swap_log = load_real_swap_log("fixtures/polygon/v3_swap_real.json");
    
    // Validate token addresses against blockchain
    let validated = validator
        .validate_token_addresses(&swap_log, DEXProtocol::UniswapV3)
        .await.unwrap();
    
    // Verify token decimals match on-chain reality
    assert_eq!(validated.pool_info.token0_decimals, 18); // WMATIC
    assert_eq!(validated.pool_info.token1_decimals, 6);  // USDC
    
    // Verify pool address exists in factory
    assert!(validated.pool_info.pool_address != Address::zero());
    
    // Verify parsed amounts match event data
    assert_eq!(validated.validated_data.amount_in, swap_log.topics[1]);
}
```

### Implementation Pattern
```rust
async fn cross_validate_swap(our_swap: &PoolSwapTLV) -> Result<()> {
    // Method 1: Direct RPC query
    let rpc_event = web3
        .eth()
        .get_logs(Filter {
            address: Some(our_swap.pool_address.into()),
            block: Some(our_swap.block_number.into()),
            // ...
        })
        .await?;
    
    // Method 2: Alternative data provider
    let etherscan_tx = query_etherscan(tx_hash).await?;
    
    // Method 3: The Graph protocol
    let graph_swap = query_the_graph(pool_id, block).await?;
    
    // All should match
    assert_eq!(our_swap.amount_in, rpc_event.amount_in);
    assert_eq!(our_swap.amount_in, etherscan_tx.amount_in);
    assert_eq!(our_swap.amount_in, graph_swap.amount_in);
    
    Ok(())
}
```

## Validation Test Suite

### Required Test Coverage

#### 1. Unit Tests (Parser Level)
```rust
#[test]
fn test_parse_normal_case() { /* ... */ }

#[test]
fn test_parse_edge_cases() { /* ... */ }

#[test]
fn test_parse_malformed_data() { /* ... */ }
```

#### 2. Semantic Tests
```rust
#[test]
fn test_semantic_field_mapping() { /* ... */ }

#[test]
fn test_direction_detection() { /* ... */ }

#[test]
fn test_venue_assignment() { /* ... */ }
```

#### 3. Deep Equality Tests
```rust
#[test]
fn test_deep_equality_normal() { /* ... */ }

#[test]
fn test_deep_equality_extremes() { /* ... */ }

#[test]
fn test_deep_equality_all_types() { /* ... */ }
```

#### 4. Integration Tests
```rust
#[tokio::test]
async fn test_e2e_with_relay() { /* ... */ }

#[tokio::test]
async fn test_concurrent_processing() { /* ... */ }

#[tokio::test]
async fn test_error_recovery() { /* ... */ }
```

#### 5. Performance Tests
```rust
#[bench]
fn bench_parse_throughput(b: &mut Bencher) { /* ... */ }

#[bench]
fn bench_serialization(b: &mut Bencher) { /* ... */ }
```

## Pre-Production Validation Checklist

### Mandatory Requirements (No Exceptions)
- [ ] **CRITICAL**: Complete validation pipeline passes for ALL data types
- [ ] **CRITICAL**: Token address validation passes (DeFi adapters only)
- [ ] 1000+ real messages processed without error per data type
- [ ] Deep equality tests pass for all message types
- [ ] Semantic validation catches known error patterns
- [ ] Cross-validation with alternative sources when available
- [ ] No precision loss detected in any sample
- [ ] Error handling validated with malformed data
- [ ] Edge cases properly handled
- [ ] Real data fixtures captured and validated
- [ ] Token decimals verified against on-chain contracts (DeFi)

### Post-Validation Development
- [ ] Production code optimized (validation pipeline removed)
- [ ] 24-hour stability test in production configuration
- [ ] Performance profiling completed
- [ ] Memory leak testing completed

**CRITICAL**: NO data type can be used in production without completing the validation pipeline.

## Common Validation Failures

### Failure: "Deep equality check failed"
**Cause**: Data loss during serialization
**Fix**: Check field sizes, ensure using correct types (u128 vs u64)

### Failure: "Semantic validation failed: invalid amounts"
**Cause**: Sign handling error in DEX events
**Fix**: Properly handle two's complement negative values

### Failure: "Cross-validation mismatch"
**Cause**: Different data sources have different precision
**Fix**: Understand each source's precision model

### Failure: "Precision loss detected"
**Cause**: Float conversion or truncation
**Fix**: Use integer types, preserve native precision

## Validation Tools

### Token Address Validator (Critical for DeFi)
```bash
cargo test --test token_address_validator -- --nocapture
cargo run --bin validate_token_addresses -- --rpc https://polygon-rpc.com/
```

### Deep Equality Validator
```bash
cargo run --bin validate_deep_equality -- --adapter polygon_dex
```

### Semantic Validator
```bash
cargo run --bin validate_semantics -- --check-all
```

### Cross-Source Validator
```bash
cargo run --bin cross_validate -- --source1 websocket --source2 rpc
```

## References
- `SOP.md` - Standard operating procedure for adapter development
- `STRUCTURE.md` - Adapter module organization
- Test fixtures - `tests/fixtures/`
- Protocol V2 spec - `protocol_v2/README.md`