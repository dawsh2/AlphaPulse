# Binance Adapter

## Official Data Format Documentation
- **WebSocket API**: [Binance WebSocket Stream Documentation](https://binance-docs.github.io/apidocs/spot/en/#websocket-market-streams)
- **REST API**: [Binance REST API Documentation](https://binance-docs.github.io/apidocs/spot/en/)
- **Message Format**: JSON format with string-encoded decimals for precision
- **Rate Limits**: [Binance API Rate Limits](https://binance-docs.github.io/apidocs/spot/en/#limits)

## Validation Checklist
- [ ] Raw data parsing validation implemented
- [ ] TLV serialization validation implemented  
- [ ] TLV deserialization validation implemented
- [ ] Semantic & deep equality validation implemented
- [ ] Performance targets met (<10ms per validation)
- [ ] Real data fixtures created (no mocks)

## Test Coverage
- **Unit Tests**: `tests/validation/binance.rs` 
- **Integration Tests**: `tests/integration/binance.rs`
- **Real Data Fixtures**: `tests/fixtures/binance/`
- **Performance Tests**: Included in validation tests

## Performance Characteristics
- Validation Speed: ~2ms per event (target: <10ms)
- Throughput: ~5,000 events/second
- Memory Usage: ~5MB baseline

## Data Format Specifics

### Trade Stream Format
```json
{
  "e": "trade",          // Event type
  "E": 123456789,        // Event time (ms)
  "s": "BNBBTC",         // Symbol
  "t": 12345,            // Trade ID
  "p": "0.001",          // Price (string for precision)
  "q": "100",            // Quantity (string for precision)
  "b": 88,               // Buyer order ID
  "a": 50,               // Seller order ID
  "T": 123456785,        // Trade time (ms)
  "m": true,             // Is buyer the market maker?
  "M": true              // Ignore field
}
```

### Precision Handling
- **Price/Quantity**: Provided as strings to prevent floating-point precision loss
- **Timestamps**: Milliseconds since Unix epoch, converted to nanoseconds
- **Decimal Conversion**: Uses `rust_decimal::Decimal` for exact arithmetic
- **Fixed-Point Storage**: 8 decimal places for USD prices (`* 100_000_000`)

### Symbol Normalization
- **Input**: `BTCUSDT` (concatenated format)
- **Output**: `BTC/USDT` (slash-separated format)
- **Special Cases**: Handle stablecoins (USDT, USDC, BUSD) and ETH variants

## Semantic Validation Rules

### Trade Direction
- `m: true` = Buyer is market maker (sell order filled)
- `m: false` = Seller is market maker (buy order filled)
- Validates trade direction consistency

### Price Validation
- Price must be positive: `assert!(price > Decimal::ZERO)`
- Price within reasonable bounds: `assert!(price < Decimal::from(10_000_000))`
- No scientific notation in price strings

### Volume Validation
- Quantity must be positive: `assert!(quantity > Decimal::ZERO)`
- Volume validation against market caps
- Min/max size constraints per symbol

### Timestamp Validation
- Event time must be recent: `assert!(event_time > now - 1_hour)`
- Trade time must be before or equal to event time
- Nanosecond precision preservation

## Four-Step Validation Process

### Step 1: Raw Data Parsing
```rust
pub fn validate_binance_raw_parsing(raw_json: &[u8], parsed: &BinanceTradeEvent) -> Result<()> {
    // Validate JSON structure
    assert!(parsed.event_type == "trade", "Invalid event type");
    assert!(!parsed.symbol.is_empty(), "Symbol cannot be empty");
    assert!(!parsed.price.is_empty(), "Price cannot be empty");
    
    // Validate decimal parsing
    let price = Decimal::from_str(&parsed.price)?;
    assert!(price > Decimal::ZERO, "Price must be positive");
    
    Ok(())
}
```

### Step 2: TLV Serialization
```rust
pub fn validate_binance_tlv_serialization(tlv: &TradeTLV) -> Result<Vec<u8>> {
    // Semantic validation
    assert_eq!(tlv.venue, VenueId::Binance, "Venue must be Binance");
    assert!(tlv.price > 0, "Price must be positive in TLV");
    assert!(tlv.quantity > 0, "Quantity must be positive in TLV");
    
    // Serialize and validate
    let bytes = tlv.to_bytes();
    assert!(!bytes.is_empty(), "Serialization cannot be empty");
    
    Ok(bytes)
}
```

### Step 3: TLV Deserialization
```rust
pub fn validate_binance_tlv_deserialization(bytes: &[u8]) -> Result<TradeTLV> {
    let recovered = TradeTLV::from_bytes(bytes)?;
    
    // Structural validation
    assert_eq!(recovered.venue, VenueId::Binance, "Venue corruption detected");
    assert!(recovered.price > 0, "Price corruption detected");
    assert!(recovered.timestamp_ns > 0, "Timestamp corruption detected");
    
    Ok(recovered)
}
```

### Step 4: Deep Equality
```rust
pub fn validate_binance_deep_equality(original: &TradeTLV, recovered: &TradeTLV) -> Result<()> {
    // Exact field equality
    assert_eq!(original.venue, recovered.venue, "Venue mismatch");
    assert_eq!(original.instrument_id, recovered.instrument_id, "Instrument mismatch");
    assert_eq!(original.price, recovered.price, "Price precision loss");
    assert_eq!(original.quantity, recovered.quantity, "Quantity precision loss");
    assert_eq!(original.timestamp_ns, recovered.timestamp_ns, "Timestamp precision loss");
    
    // Structural equality
    assert_eq!(original, recovered, "Deep equality failed");
    
    Ok(())
}
```

## Error Handling Patterns

### Connection Recovery
- Exponential backoff: 1s, 2s, 4s, 8s, max 30s
- Automatic subscription restoration after reconnection
- Circuit breaker after 5 consecutive failures

### Data Validation Errors
- Invalid JSON: Log and skip message, maintain connection
- Missing fields: Log warning, attempt graceful degradation
- Precision loss: **FATAL** - terminate and alert

### Rate Limiting
- Respect Binance rate limits: 5 requests/second
- Implement connection-level throttling
- Queue overflow protection

## Performance Benchmarks

### Throughput Tests
```bash
cargo bench --bench binance_throughput
```
**Target**: >5,000 trades/second processing

### Latency Tests
```bash
cargo test --test binance_latency_validation
```
**Target**: <2ms parse-to-TLV latency

### Memory Tests
```bash
cargo test --test binance_memory_validation
```
**Target**: <10MB resident memory

## Real Data Test Fixtures

### Location
`tests/fixtures/binance/`
- `btc_usdt_trades.json` - High-volume BTC/USDT trades
- `small_cap_trades.json` - Low-volume altcoin trades  
- `edge_cases.json` - Boundary values and edge cases

### Fixture Validation
All fixtures must be real data from Binance WebSocket streams:
```bash
# Validate fixtures are real (not synthetic)
cargo test --test binance_fixture_authenticity
```

## Known Limitations

### Precision Constraints
- Fixed-point storage limits USD prices to ~$92 million maximum
- Timestamp precision: nanoseconds (Binance provides milliseconds)
- Volume precision: Limited by Decimal type (38 digits)

### Symbol Coverage
- Only major spot pairs supported initially
- Futures/options require separate implementation
- Margin trading data not captured

### Market Hours
- 24/7 operation, no market close handling needed
- Maintenance windows: Typically 30-60 minutes daily

## Troubleshooting

### Common Issues
1. **WebSocket disconnections**: Check network stability and rate limits
2. **Precision loss warnings**: Verify decimal string parsing
3. **Symbol normalization failures**: Check symbol mapping table
4. **Validation failures**: Enable debug logging to trace pipeline

### Debug Commands
```bash
# Enable trace logging
RUST_LOG=alphapulse_adapters::binance=trace cargo run

# Run validation with detailed output
cargo test --test binance_validation -- --nocapture

# Performance profiling
cargo flamegraph --test binance_performance
```