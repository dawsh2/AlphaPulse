# Coinbase Adapter

**📚 Reference Implementation** - Use this as a template for new CEX adapters

## Official Data Format Documentation
- **WebSocket API**: [Coinbase WebSocket Stream Documentation](https://docs.cdp.coinbase.com/exchange/docs/websocket-overview)
- **REST API**: [Coinbase Exchange REST API Documentation](https://docs.cdp.coinbase.com/exchange/docs/welcome)
- **Message Format**: JSON format with string-encoded decimals for precision
- **Rate Limits**: [Coinbase API Rate Limits](https://docs.cdp.coinbase.com/exchange/docs/websocket-rate-limits)

## Validation Checklist
- [x] Raw data parsing validation implemented (with semantic correctness checks)
- [x] TLV serialization validation implemented  
- [x] TLV deserialization validation implemented
- [x] Semantic & deep equality validation implemented
- [ ] Performance targets met (<10ms per validation)
- [x] Real data fixtures created (447 real trade samples)

## Test Coverage
- **Unit Tests**: `tests/validation/coinbase_validation.rs` 
- **Integration Tests**: `tests/integration/coinbase.rs` (pending)
- **Real Data Fixtures**: `tests/fixtures/coinbase/trades_raw.json`
- **Performance Tests**: Included in validation tests

## Performance Characteristics
- Validation Speed: TBD (target: <10ms)
- Throughput: TBD events/second
- Memory Usage: TBD MB baseline

## Data Format Specifics

### Trade Stream Format (Matches Channel)
```json
{
  "type": "match",                    // Event type ("match" or "last_match")
  "trade_id": 865127782,              // Unique trade ID
  "maker_order_id": "5f4bb11b-f065-4025-ad53-2091b10ad2cf",  // Maker order UUID
  "taker_order_id": "66715b57-0167-4ae9-8b2b-75a064a923f4",  // Taker order UUID  
  "side": "buy",                      // Taker side: "buy" or "sell"
  "size": "0.00004147",               // Trade size (string for precision)
  "price": "116827.85",               // Trade price (string for precision)
  "product_id": "BTC-USD",            // Symbol format (dash-separated)
  "sequence": 110614077300,           // Message sequence number
  "time": "2025-08-22T20:11:30.012637Z"  // ISO 8601 timestamp
}
```

### Precision Handling
- **Price/Size**: Provided as strings to prevent floating-point precision loss
- **Timestamps**: ISO 8601 format with microsecond precision, converted to nanoseconds
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

2. **Adapt Message Types**: Define your exchange's message structs (like `CoinbaseMatchEvent`)
   ```rust
   #[derive(Debug, Clone, Deserialize)]
   pub struct YourExchangeTrade {
       // Match your exchange's JSON structure exactly
   }
   ```

3. **Implement Conversions**: Create `TryFrom` implementations for TLV conversion
   ```rust
   impl TryFrom<YourExchangeTrade> for TradeTLV {
       // Convert exchange format → Protocol V2 TLV
   }
   ```

4. **Handle WebSocket Connection**: Use `ConnectionManager` for robust reconnection
   ```rust
   let connection = Arc::new(ConnectionManager::new(
       VenueId::YourExchange,
       config,
       metrics.clone()
   ));
   ```

5. **Implement Validation**: Follow the four-step validation pattern (see tests)

## Code Structure

### Key Components

| File/Module | Purpose | Key Patterns |
|------------|---------|--------------|
| `mod.rs` | Main adapter logic | Stateless transformation |
| `CoinbaseMatchEvent` | Exchange message struct | JSON deserialization with `serde` |
| `CoinbaseCollector` | WebSocket connection handler | NO StateManager - stateless only |
| `impl TryFrom` | Format conversion | Exchange JSON → TradeTLV |
| `validate()` | Input validation | Semantic correctness checks |
| `tests` | Unit tests | Roundtrip validation required |

### Data Flow

```
Coinbase WebSocket → JSON Parse → CoinbaseMatchEvent → Validation → TradeTLV → Binary Output
                     (serde)      (struct)             (semantic)   (Protocol V2)
```

## Common Pitfalls & Solutions

### ❌ Wrong API Usage
```rust
// WRONG - These methods don't exist:
InstrumentId::crypto("BTC", "USD")  // ❌
StateManager::new()                 // ❌
TradeTLV::read_from(bytes)         // ❌

// CORRECT - Use these instead:
InstrumentId::coin("BTC", "USD")   // ✅
StateManager::with_venue_and_metrics(venue, metrics) // ✅ (but don't use in adapters!)
TradeTLV::from_bytes(bytes)        // ✅
```

### ❌ StateManager in Adapters
```rust
// WRONG - Adapters should NOT manage state:
pub struct WrongAdapter {
    state: Arc<StateManager>,  // ❌ No state in adapters!
}

// CORRECT - Adapters are stateless transformers:
pub struct CorrectAdapter {
    connection: Arc<ConnectionManager>,  // ✅ Connection management only
    output_tx: Sender<TLVMessage>,      // ✅ Output channel
    // NO state management
}
```

### ❌ Packed Field Access
```rust
// WRONG - Direct access to packed fields causes undefined behavior:
println!("{}", tlv.price);  // ❌ Unaligned reference!

// CORRECT - Always copy packed fields first:
let price = tlv.price;      // ✅ Copy to stack
println!("{}", price);      // ✅ Now safe to use
```

### ❌ Floating Point for Money
```rust
// WRONG - Loses precision:
let price: f64 = 123.45;  // ❌

// CORRECT - Use fixed-point integers:
let price: i64 = 12345000000;  // ✅ $123.45 as 8-decimal fixed-point
```

### ❌ Missing Validation
```rust
// WRONG - Trusting exchange data blindly:
let tlv = TradeTLV::try_from(event)?;  // What if event has invalid data?

// CORRECT - Validate before conversion:
event.validate()?;  // ✅ Check semantic correctness
let tlv = TradeTLV::try_from(event)?;
```

## Template Checklist

When using this as a template for new adapters:

- [ ] Replace "Coinbase" with your exchange name throughout
- [ ] Update WebSocket URL and subscription messages
- [ ] Define your exchange's message structures
- [ ] Implement `TryFrom` conversions to TLV types
- [ ] Add validation for your exchange's data format
- [ ] Create unit tests with real captured data
- [ ] Implement the four-step validation test
- [ ] Update this README with exchange-specific details
- [ ] Remove StateManager if you copied from old examples
- [ ] Ensure all packed field accesses use the copy pattern

### Symbol Normalization
- **Input**: `BTC-USD` (dash-separated format)
- **Output**: `BTC/USD` (slash-separated format)
- **Special Cases**: Handle various product types (spot, stablecoins)

## Semantic Validation Rules

### Trade Direction
- `side: "buy"` = Taker bought (market buy order)
- `side: "sell"` = Taker sold (market sell order)
- Validates trade direction consistency with market conventions

### Price Validation
- Price must be positive: `assert!(price > Decimal::ZERO)`
- **No artificial bounds**: Collectors forward ALL data received from providers
- No scientific notation in price strings
- Preserves exact string precision from Coinbase

### Volume Validation
- Size must be positive: `assert!(size > Decimal::ZERO)`
- **No market cap constraints**: All provider data is forwarded
- **No min/max size constraints**: Validation is for corruption detection only

### Timestamp Validation
- ISO 8601 format: `2025-08-22T20:11:30.012637Z`
- Nanosecond precision preservation (despite microsecond source)
- **No recency constraints**: Forward all timestamps as received

## Four-Step Validation Process

### Step 1: Raw Data Parsing + Semantic Correctness
```rust
pub fn validate_coinbase_raw_parsing(raw_json: &Value, parsed: &CoinbaseMatchEvent) -> Result<()> {
    // Validate JSON structure completeness
    assert!(!parsed.price.is_empty(), "Price cannot be empty");
    assert!(!parsed.size.is_empty(), "Size cannot be empty");
    
    // SEMANTIC CORRECTNESS: Compare parsed vs original JSON 
    if let Some(original_price) = raw_json["price"].as_str() {
        assert_eq!(parsed.price, original_price, "Price semantic corruption");
    }
    
    // Precision preservation validation
    let price = Decimal::from_str(&parsed.price)?;
    assert!(price > Decimal::ZERO, "Price must be positive");
    
    Ok(())
}
```

### Step 2: TLV Serialization
```rust
pub fn validate_coinbase_tlv_serialization(tlv: &TradeTLV) -> Result<Vec<u8>> {
    // Semantic validation only - NO artificial bounds
    assert_eq!(tlv.venue().unwrap(), VenueId::Coinbase, "Venue must be Coinbase");
    assert!(tlv.price > 0, "Price must be positive in TLV");
    assert!(tlv.volume > 0, "Volume must be positive in TLV");
    
    // Serialize and validate structure
    let bytes = tlv.as_bytes().to_vec();
    assert!(!bytes.is_empty(), "Serialization cannot be empty");
    
    Ok(bytes)
}
```

### Step 3: TLV Deserialization
```rust
pub fn validate_coinbase_tlv_deserialization(bytes: &[u8]) -> Result<TradeTLV> {
    let recovered = TradeTLV::from_bytes(bytes)?;
    
    // Structural integrity validation (corruption detection only)
    assert_eq!(recovered.venue().unwrap(), VenueId::Coinbase, "Venue corruption");
    assert!(recovered.price > 0, "Price corruption detected");
    assert!(recovered.timestamp_ns > 0, "Timestamp corruption detected");
    
    // NO artificial bounds - collectors forward all provider data
    
    Ok(recovered)
}
```

### Step 4: Deep Equality
```rust
pub fn validate_coinbase_deep_equality(original: &TradeTLV, recovered: &TradeTLV) -> Result<()> {
    // Exact field equality - zero data loss
    assert_eq!(original.venue().unwrap(), recovered.venue().unwrap(), "Venue mismatch");
    assert_eq!(original.price, recovered.price, "Price precision loss");
    assert_eq!(original.volume, recovered.volume, "Volume precision loss");
    assert_eq!(original.timestamp_ns, recovered.timestamp_ns, "Timestamp precision loss");
    
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
- Exponential backoff: 1s, 2s, 4s, 8s, max 30s
- Automatic subscription restoration after reconnection
- Circuit breaker after 5 consecutive failures

### Data Validation Errors
- Invalid JSON: Log and skip message, maintain connection
- Missing fields: Log warning, attempt graceful degradation  
- **Precision loss: FATAL** - terminate and alert
- **Semantic corruption: FATAL** - terminate and alert

### Rate Limiting
- Respect Coinbase rate limits per documentation
- Implement connection-level throttling
- Queue overflow protection

## Performance Benchmarks

### Throughput Tests
```bash
cargo bench --bench coinbase_throughput
```
**Target**: >5,000 trades/second processing

### Latency Tests  
```bash
cargo test --test coinbase_validation -- --nocapture
```
**Target**: <10ms parse-to-TLV latency

### Memory Tests
```bash
cargo test --test coinbase_memory_validation
```
**Target**: <10MB resident memory

## Real Data Test Fixtures

### Location
`tests/fixtures/coinbase/`
- `trades_raw.json` - 447 real trade samples from WebSocket capture

### Fixture Validation
All fixtures are real data captured from Coinbase WebSocket:
- Captured using temporary script: `scripts/temp/capture_coinbase_trades.py`
- Contains diverse samples: BTC-USD, ETH-USD, various sizes and prices
- No synthetic or mocked data permitted

### Sample Validation
```bash
# Run complete validation pipeline with real data
cargo test coinbase_validation -- --nocapture
```

## Known Limitations

### Precision Constraints
- Fixed-point storage: 8 decimal places for USD prices
- Timestamp precision: nanoseconds (Coinbase provides microseconds)
- Volume precision: Limited by i64 type capacity

### Symbol Coverage  
- Major spot pairs supported initially
- Product expansion requires InstrumentId mapping updates
- Only "matches" channel implemented initially

### Market Hours
- 24/7 operation, no market close handling needed
- Maintenance windows: Check Coinbase status page

## Troubleshooting

### Common Issues
1. **WebSocket disconnections**: Check network stability and rate limits
2. **Precision loss warnings**: Verify decimal string parsing 
3. **Symbol normalization failures**: Check product_id mapping
4. **Validation failures**: Enable debug logging to trace pipeline

### Debug Commands
```bash
# Enable trace logging
RUST_LOG=alphapulse_adapters::coinbase=trace cargo run

# Run validation with detailed output  
cargo test coinbase_validation -- --nocapture

# Performance profiling
cargo flamegraph --test coinbase_performance
```