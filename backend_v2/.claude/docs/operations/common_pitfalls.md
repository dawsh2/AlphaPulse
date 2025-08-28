# Common Pitfalls in Torq Development

This document captures common mistakes and anti-patterns encountered in the Torq codebase, along with the correct approaches to avoid them.

## 1. Zero-Copy Violations in TLV Message Construction

### ❌ WRONG: Converting to bytes defeats zero-copy optimization
```rust
// This pattern adds unnecessary allocations and copies!
let trade_tlv = TradeTLV::new(...);
let bytes = trade_tlv.to_bytes();  // Unnecessary conversion - adds ~50-100ns

// This won't even compile - Vec<u8> doesn't implement AsBytes trait
let message = build_message_direct(
    RelayDomain::MarketData,
    SourceType::PolygonCollector,
    TLVType::Trade,
    &bytes,  // ❌ ERROR: Vec<u8> doesn't implement AsBytes!
);
```

### ✅ CORRECT: Pass struct directly for true zero-copy
```rust
// Direct struct reference enables zero-copy serialization
let trade_tlv = TradeTLV::new(...);
let message = build_message_direct(
    RelayDomain::MarketData,
    SourceType::PolygonCollector,
    TLVType::Trade,
    &trade_tlv,  // ✅ Direct reference - true zero-copy!
)?;
```

### Performance Impact
- Each violation adds ~100ns overhead (allocation + copy)
- At 1M msg/s, this is 10% performance degradation
- Cumulative effect can reduce throughput from 1M to 900K msg/s

### How to Verify
```bash
# Run zero-copy validation tests
cargo test --package protocol_v2 --test zerocopy_validation

# Check performance benchmarks
cargo run --bin test_protocol --release
```

## 2. Precision Loss with Floating Point

### ❌ WRONG: Using f64 for financial calculations
```rust
// Precision loss in financial calculations!
let price: f64 = 0.12345678;  // Limited to ~15 decimal digits
let total = price * quantity;  // Accumulates rounding errors
```

### ✅ CORRECT: Use appropriate fixed-point precision
```rust
// DEX pools: preserve native token precision
let weth_amount: i128 = 1_000_000_000_000_000_000; // 1 WETH (18 decimals)
let usdc_amount: i128 = 1_000_000;                 // 1 USDC (6 decimals)

// Traditional exchanges: 8-decimal fixed-point for USD prices
let btc_price: i64 = 4500000000000; // $45,000.00 (8 decimals)
```

## 3. Timestamp Truncation

### ❌ WRONG: Truncating nanosecond timestamps
```rust
// Loses microsecond precision!
let timestamp_ms = timestamp_ns / 1_000_000;
```

### ✅ CORRECT: Preserve full nanosecond precision
```rust
// Keep full precision for accurate event ordering
let timestamp_ns = protocol_v2::current_timestamp_ns();
```

## 4. TLV Type Number Collisions

### ❌ WRONG: Reusing TLV type numbers
```rust
pub enum TLVType {
    Trade = 1,
    Quote = 1,  // ❌ COLLISION! Will cause parsing errors
}
```

### ✅ CORRECT: Unique type numbers with domain ranges
```rust
pub enum TLVType {
    // Market Data domain (1-19)
    Trade = 1,
    Quote = 2,
    
    // Signal domain (20-39)
    SignalIdentity = 20,
    
    // Execution domain (40-79)
    ExecutionRequest = 40,
}
```

## 5. Missing TLV Bounds Checking

### ❌ WRONG: No bounds validation
```rust
// Buffer overflow risk!
let tlv_data = &payload[offset..];  // Could read beyond buffer
```

### ✅ CORRECT: Always validate bounds
```rust
// Safe parsing with bounds check
if offset + tlv_length > payload.len() {
    return Err(ParseError::TruncatedTLV);
}
let tlv_data = &payload[offset..offset + tlv_length];
```

## 6. Hardcoded Configuration Values

### ❌ WRONG: Hardcoded thresholds and parameters
```rust
// Inflexible and hard to tune
if spread_percentage > 0.5 {  // Hardcoded 0.5%
    execute_arbitrage();
}
const MIN_PROFIT: f64 = 100.0;  // Hardcoded $100
```

### ✅ CORRECT: Dynamic configuration
```rust
// Configurable for different market conditions
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

## 7. Silent Failure Handling

### ❌ WRONG: Hiding failures
```rust
// Deceptive - hides real issues!
match relay.send_tlv_message() {
    Ok(_) => {},
    Err(_) => { /* silently ignore */ }
}
```

### ✅ CORRECT: Transparent error handling
```rust
// Log and propagate errors appropriately
let message = parse_tlv_message(&bytes)
    .map_err(|e| {
        error!("TLV parsing failed: {}", e);
        metrics.parsing_failures.inc();
        e
    })?;
```

## 8. Breaking Message Structure Invariants

### ❌ WRONG: Invalid message headers
```rust
// Breaks protocol!
let header = MessageHeader {
    magic: 0x12345678,     // WRONG! Must be 0xDEADBEEF
    payload_size: 100,     // But actual payload is 200 bytes
    // ...
};
```

### ✅ CORRECT: Maintain protocol integrity
```rust
// Use builder to ensure correctness
let message = build_message_direct(
    RelayDomain::MarketData,
    SourceType::PolygonCollector,
    TLVType::Trade,
    &trade_tlv,
)?;  // Builder calculates correct sizes and checksum
```

## 9. Mock Data in Production Code

### ❌ WRONG: Using mock or dummy data
```rust
// NEVER use fake data!
let mock_pool = PoolInfo {
    address: [0x00; 20],  // Dummy address
    liquidity: 1000000,   // Fake liquidity
    // ...
};
```

### ✅ CORRECT: Always use real data
```rust
// Query actual on-chain data
let pool = pool_cache
    .get_or_fetch(&pool_address)
    .await?;  // Real pool data from RPC
```

## 10. Blocking in Hot Path

### ❌ WRONG: Synchronous operations in event handlers
```rust
// Blocks WebSocket processing!
async fn handle_swap_event(&self, event: SwapEvent) {
    let pool_info = self.fetch_pool_from_rpc(&event.pool).await?;  // BLOCKS!
    // ...
}
```

### ✅ CORRECT: Queue for background processing
```rust
// Non-blocking with background queue
async fn handle_swap_event(&self, event: SwapEvent) {
    if let Some(pool_info) = self.pool_cache.get(&event.pool) {
        // Use cached data
    } else {
        // Queue for background fetch
        self.fetch_queue.send(event.pool).await?;
    }
}
```

## Prevention Strategies

1. **Run validation tests regularly**:
   ```bash
   cargo test --package protocol_v2
   ```

2. **Use performance benchmarks**:
   ```bash
   cargo bench --baseline main
   ```

3. **Enable debug assertions in development**:
   ```rust
   debug_assert!(tlv_size == std::mem::size_of::<TradeTLV>());
   ```

4. **Review PR checklist**:
   - [ ] No floating point for prices
   - [ ] No timestamp truncation
   - [ ] No hardcoded values
   - [ ] No mock data
   - [ ] No silent failures
   - [ ] Bounds checking on all buffer operations
   - [ ] Zero-copy patterns for TLV messages

## References

- [Protocol V2 Specification](../protocol.md)
- [Performance Requirements](../performance.md)
- [Testing Guide](./testing.md)
- [Development Workflow](./development.md)