# Gemini Adapter

**🔷 Production Implementation** - CEX adapter for Gemini Exchange market data collection

## Official Data Format Documentation
- **WebSocket API**: [Gemini WebSocket API Documentation](https://docs.gemini.com/websocket-api/)
- **REST API**: [Gemini REST API Documentation](https://docs.gemini.com/rest-api/)
- **Message Format**: JSON format with string-encoded decimals for precision
- **Rate Limits**: [Gemini API Rate Limits](https://docs.gemini.com/websocket-api/#rate-limits)

## Validation Checklist
- [x] Raw data parsing validation implemented (with semantic correctness checks)
- [x] TLV serialization validation implemented  
- [x] TLV deserialization validation implemented
- [x] Semantic & deep equality validation implemented
- [ ] Performance targets met (<10ms per validation)
- [x] Real data fixtures created (real-world format tests)

## Test Coverage
- **Unit Tests**: `tests/gemini_roundtrip_test.rs` 
- **Integration Tests**: `tests/integration/gemini.rs` (pending)
- **Real Data Fixtures**: Included in roundtrip tests with actual Gemini API formats
- **Performance Tests**: Included in validation tests

## Performance Characteristics
- Validation Speed: TBD (target: <10ms)
- Throughput: TBD events/second
- Memory Usage: TBD MB baseline

## Data Format Specifics

### Trade Stream Format (Market Data Updates)
```json
{
  "type": "update",                     // Event type ("update" or "heartbeat")
  "eventId": 36902233362,               // Unique event ID
  "socket_sequence": 661,               // WebSocket sequence number
  "events": [
    {
      "type": "trade",                  // Event type ("trade")
      "tid": 36902233362,               // Trade ID
      "price": "23570.44",              // Trade price (string for precision)
      "amount": "0.0009",               // Trade amount (string for precision)
      "makerSide": "ask",               // Maker side: "bid" or "ask"
      "timestampms": 1629464726493      // Unix timestamp in milliseconds
    }
  ]
}
```

### Heartbeat Format
```json
{
  "type": "heartbeat",
  "socket_sequence": 1
}
```

### Precision Handling
- **Price/Amount**: Provided as strings to prevent floating-point precision loss
- **Timestamps**: Unix milliseconds, converted to nanoseconds for protocol consistency
- **Decimal Conversion**: Uses `rust_decimal::Decimal` for exact arithmetic
- **Fixed-Point Storage**: 8 decimal places for USD prices (`* 100_000_000`)

## Implementation Guidance

### Using This as a Template

When creating a new CEX adapter based on this implementation:

1. **Copy Structure**: Start by copying this directory structure
   ```
   src/input/collectors/your_exchange/
   ├── mod.rs          # Main collector implementation
   └── README.md       # Exchange-specific documentation
   ```

2. **Adapt Message Types**: Define your exchange's message structs (like `GeminiTradeEvent`)
   ```rust
   #[derive(Debug, Clone, Deserialize)]
   pub struct YourExchangeTrade {
       // Match your exchange's JSON structure exactly
   }
   ```

3. **Implement Conversions**: Create `TryFrom` implementations for TLV conversion
   ```rust
   impl TryFrom<(&YourExchangeTrade, &str)> for TradeTLV {
       // Convert exchange format → Protocol V2 TLV
   }
   ```

4. **Handle WebSocket Connection**: Use `ConnectionManager` for robust reconnection
   ```rust
   let connection = Arc::new(ConnectionManager::new(config));
   ```

5. **Implement Validation**: Follow the four-step validation pattern (see tests)

## Code Structure

### Key Components

| File/Module | Purpose | Key Patterns |
|------------|---------|--------------|
| `gemini.rs` | Main adapter logic | Stateless transformation |
| `GeminiTradeEvent` | Exchange message struct | JSON deserialization with `serde` |
| `GeminiCollector` | WebSocket connection handler | NO StateManager - stateless only |
| `impl TryFrom` | Format conversion | Exchange JSON → TradeTLV |
| `validate()` | Input validation | Semantic correctness checks |
| `tests` | Unit tests | Roundtrip validation required |

### Data Flow

```
Gemini WebSocket → JSON Parse → GeminiTradeEvent → Validation → TradeTLV → Binary Output
                   (serde)      (struct)           (semantic)   (Protocol V2)
```

## Gemini-Specific Implementation Details

### WebSocket Connection Strategy
- **Per-Symbol Connections**: Gemini uses separate WebSocket per symbol
- **URL Format**: `wss://api.gemini.com/v1/marketdata/{symbol}`
- **Symbol Format**: Lowercase concatenated (e.g., `btcusd`, `ethusd`)
- **Multiple Symbols**: Currently implements single connection, TODO: concurrent connections

### Symbol Normalization
```rust
// Input: "btcusd" (Gemini format)
// Output: "BTC/USD" (Protocol V2 standard)
pub fn normalized_symbol(&self, symbol: &str) -> String {
    match symbol.to_lowercase().as_str() {
        "btcusd" => "BTC/USD".to_string(),
        "ethusd" => "ETH/USD".to_string(),
        "maticusd" => "MATIC/USD".to_string(),
        // Fallback pattern for new symbols
        _ => format!("{}/{}", base.to_uppercase(), quote.to_uppercase())
    }
}
```

### Trade Side Mapping
```rust
// Gemini provides maker side, we need market perspective (taker)
pub fn trade_side(&self) -> u8 {
    match self.maker_side.as_str() {
        "bid" => 1,    // Maker was bidding, taker sold (market sell)
        "ask" => 0,    // Maker was asking, taker bought (market buy)
        _ => 0,        // Default to buy
    }
}
```

## Common Pitfalls & Solutions

### ❌ Wrong Symbol Format
```rust
// WRONG - Using Coinbase format:
let url = format!("wss://api.gemini.com/v1/marketdata/{}", "BTC-USD");  // ❌

// CORRECT - Use Gemini format:
let url = format!("wss://api.gemini.com/v1/marketdata/{}", "btcusd");   // ✅
```

### ❌ Incorrect Trade Side Logic
```rust
// WRONG - Using side directly:
let side = if event.maker_side == "bid" { 0 } else { 1 };  // ❌ Backwards!

// CORRECT - Think from taker perspective:
let side = match event.maker_side.as_str() {
    "bid" => 1,    // ✅ Maker bidding = taker sold (market sell)
    "ask" => 0,    // ✅ Maker asking = taker bought (market buy)
    _ => 0,
};
```

### ❌ Missing Symbol Context in TLV Conversion
```rust
// WRONG - No symbol context:
let tlv = TradeTLV::try_from(event)?;  // ❌ Can't normalize symbol

// CORRECT - Provide symbol context:
let tlv = TradeTLV::try_from((&event, "btcusd"))?;  // ✅ Symbol available for normalization
```

### ❌ Packed Field Access
```rust
// WRONG - Direct access to packed fields causes undefined behavior:
println!("{}", tlv.price);  // ❌ Unaligned reference!

// CORRECT - Always copy packed fields first:
let price = tlv.price;      // ✅ Copy to stack
println!("{}", price);      // ✅ Now safe to use
```

### ❌ Multiple Connection Management
```rust
// CURRENT LIMITATION - Only single symbol supported:
let collector = GeminiCollector::new(
    vec!["btcusd".to_string(), "ethusd".to_string()],  // Only first symbol used
    tx
);

// TODO - Implement concurrent connections:
// Spawn separate WebSocket for each symbol
```

## Template Checklist

When using this as a template for new adapters:

- [ ] Replace "Gemini" with your exchange name throughout
- [ ] Update WebSocket URL and subscription messages  
- [ ] Define your exchange's message structures
- [ ] Implement `TryFrom` conversions to TLV types
- [ ] Add validation for your exchange's data format
- [ ] Create unit tests with real captured data
- [ ] Implement the four-step validation test
- [ ] Update this README with exchange-specific details
- [ ] Remove StateManager if you copied from old examples
- [ ] Ensure all packed field accesses use the copy pattern
- [ ] Handle your exchange's specific symbol format
- [ ] Map trade sides correctly for your exchange's semantics

## Semantic Validation Rules

### Trade Direction (Gemini-Specific)
- `makerSide: "bid"` = Maker was bidding, taker sold (market sell) → side = 1
- `makerSide: "ask"` = Maker was asking, taker bought (market buy) → side = 0
- Validates trade direction consistency with Gemini's maker-centric reporting

### Price Validation
- Price must be positive: `assert!(price > Decimal::ZERO)`
- **No artificial bounds**: Collectors forward ALL data received from providers
- No scientific notation in price strings
- Preserves exact string precision from Gemini

### Volume Validation
- Amount must be positive: `assert!(amount > Decimal::ZERO)`
- **No market cap constraints**: All provider data is forwarded
- **No min/max size constraints**: Validation is for corruption detection only

### Timestamp Validation
- Unix milliseconds format: `1629464726493`
- Nanosecond precision preservation (multiply by 1,000,000)
- **No recency constraints**: Forward all timestamps as received

## Four-Step Validation Process

### Step 1: Raw Data Parsing + Semantic Correctness
```rust
pub fn validate_gemini_raw_parsing(raw_json: &Value, parsed: &GeminiMarketDataEvent) -> Result<()> {
    // Validate JSON structure completeness
    assert_eq!(parsed.event_type, "update", "Event type must be update");
    assert!(parsed.events.is_some(), "Events array cannot be missing");
    
    let trade_event = &parsed.events.as_ref().unwrap()[0];
    assert!(!trade_event.price.is_empty(), "Price cannot be empty");
    assert!(!trade_event.amount.is_empty(), "Amount cannot be empty");
    
    // SEMANTIC CORRECTNESS: Compare parsed vs original JSON 
    if let Some(events) = raw_json["events"].as_array() {
        let original_trade = &events[0];
        assert_eq!(trade_event.price, original_trade["price"].as_str().unwrap(), 
                   "Price semantic corruption");
        assert_eq!(trade_event.amount, original_trade["amount"].as_str().unwrap(), 
                   "Amount semantic corruption");
    }
    
    // Precision preservation validation
    let price = Decimal::from_str(&trade_event.price)?;
    assert!(price > Decimal::ZERO, "Price must be positive");
    
    Ok(())
}
```

### Step 2: TLV Serialization
```rust
pub fn validate_gemini_tlv_serialization(tlv: &TradeTLV) -> Result<Vec<u8>> {
    // Semantic validation only - NO artificial bounds
    assert_eq!(tlv.venue().unwrap(), VenueId::Gemini, "Venue must be Gemini");
    assert!(tlv.price > 0, "Price must be positive in TLV");
    assert!(tlv.volume > 0, "Volume must be positive in TLV");
    
    // Verify fixed-point conversion correctness
    // Example: "23570.44" → 2357044000000 (23570.44 * 100_000_000)
    
    // Serialize and validate structure
    let bytes = tlv.as_bytes().to_vec();
    assert!(!bytes.is_empty(), "Serialization cannot be empty");
    assert_eq!(bytes.len(), std::mem::size_of::<TradeTLV>(), "TLV size mismatch");
    
    Ok(bytes)
}
```

### Step 3: TLV Deserialization
```rust
pub fn validate_gemini_tlv_deserialization(bytes: &[u8]) -> Result<TradeTLV> {
    let recovered = TradeTLV::from_bytes(bytes)?;
    
    // Structural integrity validation (corruption detection only)
    assert_eq!(recovered.venue().unwrap(), VenueId::Gemini, "Venue corruption");
    assert!(recovered.price > 0, "Price corruption detected");
    assert!(recovered.volume > 0, "Volume corruption detected");
    assert!(recovered.timestamp_ns > 0, "Timestamp corruption detected");
    
    // NO artificial bounds - collectors forward all provider data
    
    Ok(recovered)
}
```

### Step 4: Deep Equality
```rust
pub fn validate_gemini_deep_equality(original: &TradeTLV, recovered: &TradeTLV) -> Result<()> {
    // Exact field equality - zero data loss
    // Copy packed fields to avoid ARM/M1 alignment issues
    let orig_price = original.price;
    let rec_price = recovered.price;
    assert_eq!(orig_price, rec_price, "Price precision loss");
    
    let orig_volume = original.volume;
    let rec_volume = recovered.volume;
    assert_eq!(orig_volume, rec_volume, "Volume precision loss");
    
    let orig_timestamp = original.timestamp_ns;
    let rec_timestamp = recovered.timestamp_ns;
    assert_eq!(orig_timestamp, rec_timestamp, "Timestamp precision loss");
    
    // Structural equality
    assert_eq!(original, recovered, "Deep equality failed");
    
    // Hash comparison for extra verification
    let original_bytes = original.as_bytes().to_vec();
    let recovered_bytes = recovered.as_bytes().to_vec();
    assert_eq!(original_bytes, recovered_bytes, "Re-serialization produces different bytes");
    
    Ok(())
}
```

## Critical Validation Principles

### Data Forwarding Philosophy
- **Forward ALL data received**: No artificial constraints on price/volume ranges
- **Corruption detection only**: Validation detects parsing errors, not business logic violations
- **Preserve provider semantics**: Maintain exact meaning from source data
- **Zero data loss**: Perfect roundtrip through serialization pipeline required

### What Validation Does NOT Do
- ❌ Enforce "reasonable" price bounds (e.g., max $1M BTC price)
- ❌ Apply volume constraints based on market caps
- ❌ Filter trades by recency or time windows  
- ❌ Normalize or modify provider data formats
- ❌ Apply business logic or trading constraints

### What Validation DOES Do
- ✅ Detect JSON parsing corruption
- ✅ Verify semantic correctness during parsing (parsed field == original field)
- ✅ Ensure precision preservation through Decimal types
- ✅ Validate perfect serialization roundtrip (zero data loss)
- ✅ Check for structural integrity after deserialization
- ✅ Verify TLV format compliance

## Error Handling Patterns

### Connection Recovery
- Exponential backoff: 5s base, max 60s
- Automatic reconnection on WebSocket errors
- Max 10 reconnection attempts before giving up

### Data Validation Errors
- Invalid JSON: Log and skip message, maintain connection
- Missing fields: Log warning, attempt graceful degradation  
- **Precision loss: FATAL** - terminate and alert
- **Semantic corruption: FATAL** - terminate and alert

### Rate Limiting
- Default: 1000 requests per minute (configurable)
- Per-venue rate limiting using `governor` crate
- Connection-level throttling for WebSocket messages

## Performance Benchmarks

### Throughput Tests
```bash
cargo bench --bench gemini_throughput
```
**Target**: >5,000 trades/second processing

### Latency Tests  
```bash
cargo test --test gemini_roundtrip_test -- --nocapture
```
**Target**: <10ms parse-to-TLV latency

### Memory Tests
```bash
cargo test --test gemini_memory_validation
```
**Target**: <10MB resident memory

## Real Data Test Fixtures

### Location
`tests/gemini_roundtrip_test.rs`
- Real-world JSON format validation
- Multiple test scenarios with actual Gemini API responses
- Edge case testing (small values, large values, precision boundaries)

### Fixture Validation
All test data uses real formats from Gemini WebSocket API:
- Based on official Gemini API documentation examples
- Contains diverse samples: BTC/USD, ETH/USD, various sizes and prices  
- No synthetic or mocked data permitted
- Tests actual market data scenarios

### Sample Validation
```bash
# Run complete validation pipeline with real data
cargo test gemini_roundtrip_test -- --nocapture
```

## Gemini-Specific Features

### Symbol Format Handling
- **Input**: `btcusd` (lowercase concatenated)
- **Output**: `BTC/USD` (standard slash format)
- **Mapping**: Comprehensive symbol dictionary with fallback pattern
- **Extension**: Easy to add new symbols via pattern matching

### WebSocket Architecture
- **Endpoint Pattern**: `wss://api.gemini.com/v1/marketdata/{symbol}`
- **Public Data**: No authentication required for market data
- **Heartbeat Handling**: Processes heartbeat messages for connection health
- **Message Types**: Handles `update` and `heartbeat` message types

### Multiple Symbol Support
```rust
// Current: Single symbol connection
let collector = GeminiCollector::new(
    vec!["btcusd".to_string(), "ethusd".to_string()],  // Only first symbol used
    tx
);

// TODO: Implement concurrent connections for multiple symbols
// Each symbol requires separate WebSocket connection to Gemini
```

## Known Limitations

### Connection Constraints
- **Single Symbol**: Currently supports one symbol per collector instance
- **WebSocket Architecture**: Gemini requires separate connection per symbol
- **Scaling**: Multiple symbols need multiple collector instances

### Precision Constraints
- Fixed-point storage: 8 decimal places for USD prices
- Timestamp precision: nanoseconds (Gemini provides milliseconds)
- Volume precision: Limited by i64 type capacity

### Symbol Coverage  
- Major crypto pairs supported initially
- Product expansion requires symbol mapping updates
- Only trade data implemented (no order book initially)

### Market Hours
- 24/7 operation for crypto markets
- Maintenance windows: Check Gemini status page

## Troubleshooting

### Common Issues
1. **WebSocket disconnections**: Check network stability and rate limits
2. **Symbol format errors**: Verify lowercase concatenated format (`btcusd`)
3. **Trade side confusion**: Remember Gemini reports maker side, not taker side
4. **Multiple symbol limitations**: Each symbol needs separate connection
5. **Precision loss warnings**: Verify decimal string parsing

### Debug Commands
```bash
# Enable trace logging
RUST_LOG=alphapulse_adapters::gemini=trace cargo run

# Run validation with detailed output  
cargo test gemini_roundtrip_test -- --nocapture

# Test specific symbol normalization
cargo test test_gemini_symbol_normalization -- --nocapture

# Test trade side conversion
cargo test test_gemini_trade_side_conversion -- --nocapture

# Performance profiling
cargo flamegraph --test gemini_performance
```

### Connection Issues
```bash
# Test WebSocket connectivity manually
websocat wss://api.gemini.com/v1/marketdata/btcusd

# Check rate limiting
curl -I https://api.gemini.com/v1/symbols

# Verify symbol availability
curl https://api.gemini.com/v1/symbols
```

## Development Notes

### Relationship to Other Adapters
- **Pattern**: Follows `CoinbaseCollector` stateless transformer pattern
- **Differences**: Per-symbol WebSocket vs. multi-symbol single connection
- **Commonalities**: Same TLV output format, validation approach, error handling

### Future Enhancements
1. **Multi-Symbol Support**: Implement concurrent WebSocket connections
2. **Order Book Data**: Add support for L2 order book updates
3. **Reconnection Optimization**: Symbol-specific backoff strategies
4. **Rate Limit Optimization**: Per-symbol rate limiting refinement

### Integration Points
- **Input**: Gemini WebSocket streams per symbol
- **Output**: TradeTLV messages via MarketDataRelay
- **Dependencies**: ConnectionManager, AdapterMetrics, RateLimiter
- **Protocol**: Protocol V2 TLV with 8-decimal fixed-point precision

## References

### Gemini API Documentation
- [WebSocket API Overview](https://docs.gemini.com/websocket-api/)
- [Market Data WebSocket](https://docs.gemini.com/websocket-api/#market-data)
- [Symbol Details](https://docs.gemini.com/rest-api/#symbol-details)
- [Rate Limits](https://docs.gemini.com/websocket-api/#rate-limits)

### AlphaPulse Integration
- **Protocol V2**: `backend_v2/docs/protocol.md`
- **Adapter Framework**: `services_v2/adapters/README.md`
- **Testing Standards**: `tests/validation/README.md`
- **Performance Targets**: >1M msg/s TLV construction, <35μs conversion latency