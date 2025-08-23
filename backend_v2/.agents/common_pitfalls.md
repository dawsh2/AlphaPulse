# Common Pitfalls & Solutions

## Precision & Numbers

### ❌ DON'T: Use Floating Point for Prices
```rust
// WRONG - Precision loss!
let price: f64 = 0.12345678;
```

### ✅ DO: Use Appropriate Precision per Asset Type
```rust
// CORRECT - DEX pools: preserve native token precision
let weth_amount: i64 = 1_000_000_000_000_000_000; // 1 WETH (18 decimals)
let usdc_amount: i64 = 1_000_000;                 // 1 USDC (6 decimals)

// CORRECT - Traditional exchanges: 8-decimal fixed-point for USD prices
let btc_price: i64 = 4500000000000; // $45,000.00 (8 decimals: * 100_000_000)
```

## Timestamps

### ❌ DON'T: Truncate Timestamps
```python
# WRONG - Loses precision
timestamp_ms = timestamp_ns // 1_000_000
```

### ✅ DO: Preserve Nanoseconds
```python
# CORRECT - Full precision
timestamp_ns = int(time.time() * 1_000_000_000)
```

## TLV Protocol

### ❌ DON'T: Ignore TLV Bounds or Reuse Type Numbers
```rust
// WRONG - No bounds checking
let tlv_data = &payload[2..]; // Could overflow!

// WRONG - Reusing TLV type numbers
pub enum TLVType {
    Trade = 1,
    Quote = 1, // COLLISION!
}
```

### ✅ DO: Validate TLV Bounds and Maintain Type Registry
```rust
// CORRECT - Bounds checking
if offset + tlv_length > payload.len() {
    return Err(ParseError::TruncatedTLV);
}

// CORRECT - Unique type numbers with proper ranges
pub enum TLVType {
    Trade = 1,        // Market Data domain (1-19)
    Quote = 2,
    SignalIdentity = 20, // Signal domain (20-39)
}
```

## Configuration

### ❌ DON'T: Use Hardcoded Values
```rust
// WRONG - Hardcoded thresholds
if spread_percentage > 0.5 { // Hardcoded 0.5%
    execute_arbitrage();
}
const MIN_PROFIT: f64 = 100.0; // Hardcoded $100
```

### ✅ DO: Use Dynamic Configuration
```rust
// CORRECT - Configurable values
#[derive(Debug, Clone)]
pub struct ArbitrageConfig {
    pub min_spread_percentage: Decimal,
    pub min_profit_usd: Decimal,
    pub max_gas_cost_usd: Decimal,
}

if spread_percentage > config.min_spread_percentage {
    execute_arbitrage();
}
```

## Error Handling

### ❌ DON'T: Hide Failures or Break Message Structure
```rust
// WRONG - Deceptive behavior
match relay.send_tlv_message() {
    Ok(_) => {},
    Err(_) => { /* silently ignore - WRONG! */ }
}

// WRONG - Breaking TLV message structure
let broken_header = MessageHeader {
    magic: 0x12345678, // WRONG! Must be 0xDEADBEEF
    payload_size: 100,
    // ... but payload is actually 200 bytes
};
```

### ✅ DO: Be Transparent and Maintain Protocol Integrity
```rust
// CORRECT - Propagate TLV parsing failures
let message = parse_tlv_message(&bytes)
    .map_err(|e| {
        error!("TLV parsing failed: {}", e);
        e
    })?;

// CORRECT - Proper message construction
let mut builder = TLVMessageBuilder::new(relay_domain, source);
builder.add_tlv(TLVType::Trade, &trade_tlv);
let message = builder.build(); // Calculates correct sizes and checksum
```

## Documentation

### ❌ DON'T: Write Minimal Documentation
```rust
//! Brief module description
```

### ✅ DO: Write Comprehensive Structured Documentation
```rust
//! # ModuleName - System Component
//!
//! ## Purpose
//! Clear explanation of what this module does and why it exists
//!
//! ## Integration Points
//! - **Input**: What data/messages this receives
//! - **Output**: What this produces and where it goes
//! - **Dependencies**: What other components this relies on
//!
//! ## Architecture Role
//! ```text
//! [ASCII diagram showing data flow]
//! ```
//!
//! ## Performance Profile
//! - **Throughput**: Measured performance characteristics
//! - **Latency**: Timing requirements and targets
//! - **Memory**: Usage patterns and constraints
//!
//! ## Examples
//! [Complete usage examples with context]
```

## System Design

### ❌ DON'T: Create Duplicate Implementations
- No files with "enhanced", "fixed", "new", "v2" prefixes
- No multiple implementations of the same concept
- No "just in case" code

### ✅ DO: Maintain One Canonical Source
- Single implementation per concept
- Update existing files instead of creating duplicates
- Delete deprecated code immediately when replacing

### ❌ DON'T: Use Mock Data or Services
- **NEVER** use mock data, mock services, or mocked responses
- **NO** simulation modes that fake exchange responses
- **NO** stubbed WebSocket connections or API responses

### ✅ DO: Use Real Data Only
- **ALWAYS** use real exchange connections for testing
- **ALWAYS** test with actual market data and live price feeds
- **ALWAYS** write production-quality code from the start