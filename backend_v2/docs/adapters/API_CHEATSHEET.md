# API Cheatsheet - Common Methods & Gotchas

Quick reference for actual API methods vs common incorrect assumptions when building adapters.

## 🚨 Most Common API Mistakes

### InstrumentId Creation

| ❌ WRONG (Doesn't Exist) | ✅ CORRECT | Usage |
|--------------------------|-----------|-------|
| `InstrumentId::crypto("BTC", "USD")` | `InstrumentId::coin("BTC", "USD")` | Cryptocurrency pairs |
| `InstrumentId::currency("USD")` | `InstrumentId::fiat("USD")` | Fiat currencies |
| `InstrumentId::forex("EUR/USD")` | `InstrumentId::fx("EUR", "USD")` | Foreign exchange |
| `InstrumentId::new(...)` | Use specific constructors | No generic new() |

### StateManager (Don't Use in Adapters!)

| ❌ WRONG | ✅ CORRECT | Note |
|----------|-----------|------|
| `StateManager::new()` | `StateManager::with_venue_and_metrics(venue, metrics)` | But DON'T use in adapters |
| In adapter struct | Not in adapters at all | Adapters are stateless |

### TLV Message Methods

| ❌ WRONG | ✅ CORRECT | Purpose |
|----------|-----------|---------|
| `TradeTLV::read_from(bytes)` | `TradeTLV::from_bytes(bytes)` | Deserialize from bytes |
| `tlv.to_bytes()` | `tlv.as_bytes()` | Serialize to bytes |
| `tlv.write_to(writer)` | Use `as_bytes()` then write | No direct writer method |
| `TradeTLV::parse(bytes)` | `TradeTLV::from_bytes(bytes)` | Parsing is from_bytes |

### Connection Management

| ❌ WRONG | ✅ CORRECT | Context |
|----------|-----------|---------|
| `ConnectionManager::new()` | `ConnectionManager::new(venue, config, metrics)` | Requires all 3 params |
| `Url::parse(url)` then connect | Direct string in `connect_async(url_str)` | No Url type needed |

## 📋 Complete API Reference

### InstrumentId Constructors
```rust
// Actual available methods (from protocol_v2):
InstrumentId::coin(base: &str, quote: &str) -> InstrumentId  // Crypto: BTC/USD
InstrumentId::stock(symbol: &str) -> InstrumentId             // Stocks: AAPL
InstrumentId::option(underlying: &str, expiry: u64, strike: i64, is_call: bool) -> InstrumentId
InstrumentId::future(symbol: &str, expiry: u64) -> InstrumentId
InstrumentId::fx(base: &str, quote: &str) -> InstrumentId     // Forex: EUR/USD
InstrumentId::fiat(currency: &str) -> InstrumentId            // Fiat: USD
InstrumentId::ethereum_token(address: &str) -> Result<InstrumentId>  // 0x...
InstrumentId::pool(venue: VenueId, base: &str, quote: &str) -> InstrumentId  // DEX pools
```

### TradeTLV Methods
```rust
// Construction
TradeTLV::new(venue: VenueId, instrument: InstrumentId, price: i64, 
              volume: i64, side: u8, timestamp_ns: u64) -> Self

// Serialization (zerocopy trait)
tlv.as_bytes() -> &[u8]                    // Get bytes (zero-copy)
TradeTLV::from_bytes(bytes: &[u8]) -> Result<Self>  // Parse from bytes

// Message conversion
tlv.to_tlv_message() -> TLVMessage         // Convert to full message

// Field access (MUST copy packed fields!)
let price = tlv.price;     // ✅ Copy first
let volume = tlv.volume;   // ✅ Copy first
// DON'T: &tlv.price or pass by reference
```

### QuoteTLV Methods
```rust
// Similar to TradeTLV
QuoteTLV::new(venue: VenueId, instrument: InstrumentId, 
              bid_price: i64, bid_size: i64,
              ask_price: i64, ask_size: i64, 
              timestamp_ns: u64) -> Self

quote.as_bytes() -> &[u8]
QuoteTLV::from_bytes(bytes: &[u8]) -> Result<Self>
quote.to_tlv_message() -> TLVMessage
```

### ConnectionManager
```rust
// Construction (requires all 3 parameters)
ConnectionManager::new(
    venue: VenueId,
    config: ConnectionConfig, 
    metrics: Arc<AdapterMetrics>
) -> Self

// Connection lifecycle
connection.connect() -> Result<Stream>     // Establishes connection
connection.disconnect() -> Result<()>      // Clean disconnect
connection.state() -> ConnectionState      // Current state
connection.handle_disconnect() -> Result<()>  // Trigger reconnection

// ConnectionConfig fields
ConnectionConfig {
    url: String,                          // WebSocket URL as String
    connect_timeout: Duration,
    message_timeout: Duration,
    base_backoff_ms: u64,                // Milliseconds
    max_backoff_ms: u64,                 // Milliseconds
    max_reconnect_attempts: u32,
    health_check_interval: Duration,
}
```

### WebSocket Connection
```rust
// tokio-tungstenite methods
// Use string directly, not Url type
tokio_tungstenite::connect_async("wss://...") -> Result<(Stream, Response)>

// Message types
Message::Text(String)
Message::Binary(Vec<u8>)
Message::Ping(Vec<u8>)
Message::Pong(Vec<u8>)
Message::Close(Option<CloseFrame>)
Message::Frame(Frame)  // Raw frame

// Stream operations
stream.split() -> (SplitSink, SplitStream)
write.send(Message) -> Result<()>
read.next() -> Option<Result<Message>>
```

### Decimal Conversion (rust_decimal)
```rust
use rust_decimal::{Decimal, prelude::*};

// Parse from string
let decimal = Decimal::from_str("123.456789")?;

// Convert to fixed-point (8 decimals for CEX)
let multiplier = Decimal::from(100_000_000i64);
let fixed_point = (decimal * multiplier).to_i64()
    .ok_or_else(|| AdapterError::Overflow)?;

// Important constants
Decimal::ZERO
Decimal::ONE
```

### Metrics (AdapterMetrics)
```rust
// Available counters (atomic)
metrics.messages_processed.fetch_add(1, Ordering::Relaxed);
metrics.messages_failed.fetch_add(1, Ordering::Relaxed);
metrics.bytes_received.fetch_add(size, Ordering::Relaxed);
metrics.bytes_sent.fetch_add(size, Ordering::Relaxed);

// Gauge for current values
metrics.connected_streams.store(count, Ordering::Relaxed);

// Timing (record in microseconds)
let start = std::time::Instant::now();
// ... processing ...
let elapsed_us = start.elapsed().as_micros() as u64;
metrics.processing_time_us.fetch_add(elapsed_us, Ordering::Relaxed);
```

## 🎯 Quick Patterns

### Pattern: String to Fixed-Point
```rust
fn parse_price_to_fixed_point(price_str: &str) -> Result<i64> {
    let decimal = Decimal::from_str(price_str)?;
    let fixed = (decimal * Decimal::from(100_000_000)).to_i64()
        .ok_or_else(|| AdapterError::Overflow)?;
    Ok(fixed)
}
```

### Pattern: Normalize Exchange Symbol
```rust
// Coinbase: BTC-USD → BTC/USD
normalized = symbol.replace('-', "/");

// Binance: BTCUSDT → BTC/USDT
if symbol.ends_with("USDT") {
    base = &symbol[..symbol.len()-4];
    quote = "USDT";
}
```

### Pattern: Safe Packed Field Access
```rust
// With TradeTLV or other packed structs
let trade_tlv = TradeTLV::from_bytes(bytes)?;

// ALWAYS copy fields before use
let price = trade_tlv.price;      // Copy to stack
let volume = trade_tlv.volume;    // Copy to stack

// Now safe to use
println!("Price: {}, Volume: {}", price, volume);

// NEVER do this:
// println!("{}", trade_tlv.price);  // ❌ Unaligned reference!
```

### Pattern: Error Propagation
```rust
// Convert exchange errors to AdapterError
serde_json::from_str(&msg)
    .map_err(|e| AdapterError::ParseError {
        venue: VenueId::YourExchange,
        message: "JSON parse".to_string(),
        error: e.to_string(),
    })?;
```

## 🔍 How to Find Available Methods

### Using rustdoc
```bash
# Generate and open API docs
cargo doc --package alphapulse_protocol_v2 --open
cargo doc --package alphapulse_adapters --open

# Search for specific types
cargo doc --package alphapulse_protocol_v2 --open
# Then search for "InstrumentId" in browser
```

### Using rust-analyzer in VSCode
1. Hover over type for documentation
2. `Ctrl+Space` for autocomplete shows available methods
3. `F12` to go to definition
4. `Shift+F12` to find all references

### Grep for Examples
```bash
# Find all InstrumentId usage
rg "InstrumentId::" --type rust

# Find TradeTLV conversions
rg "TradeTLV::from" --type rust

# Find working adapter examples
rg "impl.*Adapter" */src/adapter.rs
```

## ⚠️ Deprecated/Removed APIs

These were in old versions but no longer exist:

- `Symbol` type → Replaced by `InstrumentId`
- `normalize_symbol()` free function → Use exchange-specific normalization
- `StateManager` in adapters → Moved to relay/consumer layer
- `validate_arbitrage()` → Moved to strategy layer
- Generic `new()` constructors → Use specific named constructors

## 📚 See Also

- `IMPLEMENTATION_GUIDE.md` - Step-by-step adapter creation
- `coinbase/README.md` - Reference implementation details
- `protocol_v2/README.md` - Binary protocol specification
- Individual adapter READMEs for exchange-specific patterns