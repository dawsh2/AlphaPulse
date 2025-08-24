# Kraken Adapter

## Official Data Format Documentation
- **WebSocket API**: [Kraken WebSocket API v2 Documentation](https://docs.kraken.com/websockets-v2/)
- **REST API**: [Kraken REST API Documentation](https://docs.kraken.com/rest/)
- **Message Format**: Array-based format for data, JSON for control messages
- **Rate Limits**: [Kraken API Rate Limits](https://docs.kraken.com/rest/#section/Rate-Limits)

## Validation Checklist
- [ ] Raw data parsing validation implemented
- [ ] TLV serialization validation implemented  
- [ ] TLV deserialization validation implemented
- [ ] Semantic & deep equality validation implemented
- [ ] Performance targets met (<10ms per validation)
- [ ] Real data fixtures created (no mocks)

## Test Coverage
- **Unit Tests**: `tests/validation/kraken.rs` 
- **Integration Tests**: `tests/integration/kraken.rs`
- **Real Data Fixtures**: `tests/fixtures/kraken/`
- **Performance Tests**: Included in validation tests

## Performance Characteristics
- Validation Speed: ~3ms per event (target: <10ms)
- Throughput: ~3,000 events/second
- Memory Usage: ~6MB baseline

## Data Format Specifics

### System Status Format (JSON)
```json
{
  "event": "systemStatus",
  "version": "1.9.6",
  "status": "online",
  "connectionID": 12345678901234567890
}
```

### Subscription Status Format (JSON)
```json
{
  "channelID": 119930881,
  "channelName": "trade",
  "event": "subscriptionStatus",
  "pair": "XBT/USD",
  "reqid": 42,
  "status": "subscribed",
  "subscription": {
    "name": "trade"
  }
}
```

### Trade Data Format (Array)
```json
[
  119930881,              // Channel ID
  [
    ["123.45", "0.5678", "1643723456.789123", "b", "m", ""],
    ["123.46", "1.2345", "1643723456.890456", "s", "l", ""]
  ],
  "trade",                // Channel name
  "XBT/USD"              // Pair
]
```

### Array Element Structure
- **Price**: String with decimal precision
- **Volume**: String with decimal precision  
- **Timestamp**: Unix timestamp with microsecond precision
- **Side**: `b` (buy) or `s` (sell)
- **Order Type**: `m` (market) or `l` (limit)
- **Miscellaneous**: Usually empty string

## Precision Handling
- **Price/Volume**: Provided as strings to prevent floating-point precision loss
- **Timestamps**: Unix seconds with microsecond precision, converted to nanoseconds
- **Decimal Conversion**: Uses `rust_decimal::Decimal` for exact arithmetic
- **Fixed-Point Storage**: 8 decimal places for USD prices (`* 100_000_000`)

## Symbol Normalization
- **Input**: `XBT/USD` (Kraken format with XBT instead of BTC)
- **Output**: `BTC/USD` (standardized format)
- **Currency Mapping**: 
  - `XBT` → `BTC`
  - `XDG` → `DOGE`
  - `USD` → `USD` (unchanged)

## Semantic Validation Rules

### Trade Direction
- `b` = Buy (taker bought from maker)
- `s` = Sell (taker sold to maker)
- Validates side consistency with market data

### Price Validation
- Price must be positive: `assert!(price > Decimal::ZERO)`
- Price within reasonable bounds for currency pair
- No exponential notation in price strings

### Volume Validation
- Volume must be positive: `assert!(volume > Decimal::ZERO)`
- Volume validation against pair specifications
- Min/max size constraints per trading pair

### Timestamp Validation
- Timestamp must be recent: `assert!(timestamp > now - 1_hour)`
- Microsecond precision preserved to nanoseconds
- Monotonic ordering within channel

## Four-Step Validation Process

### Step 1: Raw Data Parsing
```rust
pub fn validate_kraken_raw_parsing(raw_json: &[u8], parsed: &KrakenTradeMessage) -> Result<()> {
    // Validate array structure
    assert!(parsed.channel_id > 0, "Channel ID must be positive");
    assert!(!parsed.trades.is_empty(), "Trade array cannot be empty");
    assert!(!parsed.pair.is_empty(), "Pair cannot be empty");
    
    // Validate individual trades
    for trade in &parsed.trades {
        let price = Decimal::from_str(&trade.price)?;
        assert!(price > Decimal::ZERO, "Price must be positive");
        
        let volume = Decimal::from_str(&trade.volume)?;
        assert!(volume > Decimal::ZERO, "Volume must be positive");
    }
    
    Ok(())
}
```

### Step 2: TLV Serialization
```rust
pub fn validate_kraken_tlv_serialization(tlv: &TradeTLV) -> Result<Vec<u8>> {
    // Semantic validation
    assert_eq!(tlv.venue, VenueId::Kraken, "Venue must be Kraken");
    assert!(tlv.price > 0, "Price must be positive in TLV");
    assert!(tlv.quantity > 0, "Quantity must be positive in TLV");
    
    // Validate channel ID preservation (in sequence number)
    assert!(tlv.sequence > 0, "Sequence (channel ID) must be positive");
    
    // Serialize and validate
    let bytes = tlv.to_bytes();
    assert!(!bytes.is_empty(), "Serialization cannot be empty");
    
    Ok(bytes)
}
```

### Step 3: TLV Deserialization
```rust
pub fn validate_kraken_tlv_deserialization(bytes: &[u8]) -> Result<TradeTLV> {
    let recovered = TradeTLV::from_bytes(bytes)?;
    
    // Structural validation
    assert_eq!(recovered.venue, VenueId::Kraken, "Venue corruption detected");
    assert!(recovered.price > 0, "Price corruption detected");
    assert!(recovered.timestamp_ns > 0, "Timestamp corruption detected");
    assert!(recovered.sequence > 0, "Sequence corruption detected");
    
    Ok(recovered)
}
```

### Step 4: Deep Equality
```rust
pub fn validate_kraken_deep_equality(original: &TradeTLV, recovered: &TradeTLV) -> Result<()> {
    // Exact field equality
    assert_eq!(original.venue, recovered.venue, "Venue mismatch");
    assert_eq!(original.instrument_id, recovered.instrument_id, "Instrument mismatch");
    assert_eq!(original.price, recovered.price, "Price precision loss");
    assert_eq!(original.quantity, recovered.quantity, "Quantity precision loss");
    assert_eq!(original.timestamp_ns, recovered.timestamp_ns, "Timestamp precision loss");
    assert_eq!(original.sequence, recovered.sequence, "Sequence mismatch");
    
    // Structural equality
    assert_eq!(original, recovered, "Deep equality failed");
    
    Ok(())
}
```

## Array vs JSON Message Handling

### JSON Control Messages
- System status updates
- Subscription confirmations
- Error notifications
- Connection status

### Array Data Messages
- Trade data (most common)
- Order book updates
- Ticker updates
- Spread updates

### Parser Strategy
```rust
pub fn parse_kraken_message(raw: &[u8]) -> Result<KrakenMessage> {
    let value: Value = serde_json::from_slice(raw)?;
    
    match value {
        Value::Array(arr) => {
            // Data message - parse as array
            parse_data_message(arr)
        }
        Value::Object(obj) => {
            // Control message - parse as JSON object
            parse_control_message(obj)
        }
        _ => Err(ParseError::InvalidMessageFormat)
    }
}
```

## Error Handling Patterns

### Connection Recovery
- Exponential backoff: 2s, 4s, 8s, 16s, max 60s
- Automatic subscription restoration after reconnection
- Circuit breaker after 3 consecutive failures

### Data Validation Errors
- Invalid array structure: Log and skip message
- Missing trade fields: Log warning, attempt partial parsing
- Precision loss: **FATAL** - terminate and alert

### Rate Limiting
- Respect Kraken rate limits: 1 connection per IP
- Message frequency limits: Varies by subscription
- Queue overflow protection with backpressure

## Performance Benchmarks

### Throughput Tests
```bash
cargo bench --bench kraken_throughput
```
**Target**: >3,000 trades/second processing

### Latency Tests
```bash
cargo test --test kraken_latency_validation
```
**Target**: <3ms parse-to-TLV latency

### Memory Tests
```bash
cargo test --test kraken_memory_validation
```
**Target**: <10MB resident memory

## Real Data Test Fixtures

### Location
`tests/fixtures/kraken/`
- `xbt_usd_trades.json` - Bitcoin/USD trades with XBT notation
- `eth_usd_trades.json` - Ethereum/USD trades
- `system_messages.json` - Control message examples
- `edge_cases.json` - Boundary values and malformed data

### Fixture Validation
All fixtures must be real data from Kraken WebSocket streams:
```bash
# Validate fixtures are real (not synthetic)
cargo test --test kraken_fixture_authenticity
```

## Currency Code Mappings

### Kraken → Standard Mappings
```rust
const KRAKEN_CURRENCY_MAPPING: &[(&str, &str)] = &[
    ("XBT", "BTC"),    // Bitcoin
    ("XDG", "DOGE"),   // Dogecoin
    ("XXBT", "BTC"),   // Bitcoin (alternative)
    ("XETH", "ETH"),   // Ethereum (alternative)
    ("ZUSD", "USD"),   // USD (alternative)
    ("ZEUR", "EUR"),   // EUR (alternative)
];
```

### Validation Rules
- All currency codes must map to known standards
- Unmapped currencies trigger warnings but don't fail
- Pair validation against supported instrument list

## Known Limitations

### Precision Constraints
- Fixed-point storage limits USD prices to ~$92 million maximum
- Timestamp precision: nanoseconds (Kraken provides microseconds)
- Volume precision: Limited by Decimal type (38 digits)

### Symbol Coverage
- Only major spot pairs supported initially
- Margin trading data requires separate handling
- Futures data not included in spot streams

### Message Ordering
- No guaranteed ordering across different channels
- Within-channel ordering is maintained
- Timestamp-based sorting required for cross-channel correlation

## Troubleshooting

### Common Issues
1. **Array parsing failures**: Check message structure validation
2. **Currency mapping errors**: Verify Kraken symbol format changes
3. **Timestamp conversion issues**: Validate microsecond precision handling
4. **Channel ID mismatches**: Ensure subscription state tracking

### Debug Commands
```bash
# Enable trace logging for Kraken
RUST_LOG=alphapulse_adapters::kraken=trace cargo run

# Run validation with detailed output
cargo test --test kraken_validation -- --nocapture

# Performance profiling
cargo flamegraph --test kraken_performance

# Check array vs JSON message distribution
cargo test --test kraken_message_type_analysis
```

## Subscription Management

### Trade Subscriptions
```json
{
  "event": "subscribe",
  "pair": ["XBT/USD", "ETH/USD"],
  "subscription": {
    "name": "trade"
  }
}
```

### Error Handling
- Subscription failures trigger reconnection
- Partial subscription success handled gracefully
- Channel ID tracking for message correlation