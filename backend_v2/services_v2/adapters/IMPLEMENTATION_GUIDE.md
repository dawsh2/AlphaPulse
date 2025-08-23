# Adapter Implementation Guide

Complete step-by-step guide for implementing new exchange adapters in the AlphaPulse system.

## Table of Contents
1. [Before You Start](#before-you-start)
2. [Architecture Overview](#architecture-overview)
3. [Step-by-Step Implementation](#step-by-step-implementation)
4. [Common Patterns](#common-patterns)
5. [Testing Requirements](#testing-requirements)
6. [Common Pitfalls](#common-pitfalls)
7. [Checklist](#checklist)

## Before You Start

### Essential References
- **📋 API_CHEATSHEET.md** - Check this FIRST to avoid common API mistakes!
- **coinbase/** - Reference implementation for CEX adapters
- **polygon_dex/** - Reference implementation for DEX adapters

### Choose Your Template
- **CEX WebSocket**: Use `src/input/collectors/coinbase/` as template
- **DEX Events**: Use `src/input/collectors/polygon_dex/` as template

### Required Knowledge
- Rust async/await and tokio
- Protocol V2 TLV message format
- Exchange's API documentation
- No floating-point arithmetic (use fixed-point integers)

### Key Principles
1. **Stateless**: Adapters are pure data transformers
2. **Zero Trust**: Validate all external data
3. **Precision First**: Never lose decimal precision
4. **Fast Path**: <1ms latency target

## Architecture Overview

```
┌─────────────────┐
│ Exchange API    │ (WebSocket/RPC)
└────────┬────────┘
         ↓
┌─────────────────┐
│ Your Adapter    │ ← You implement this
├─────────────────┤
│ - Parse JSON    │
│ - Validate      │
│ - Convert→TLV   │
│ - Forward       │
└────────┬────────┘
         ↓
┌─────────────────┐
│ TLV Messages    │ (Binary Protocol V2)
└────────┬────────┘
         ↓
┌─────────────────┐
│ Relay/Consumer  │ (Downstream systems)
└─────────────────┘
```

### What Adapters DO and DON'T Do

| ✅ DO | ❌ DON'T |
|-------|---------|
| Connect to exchange | Manage trading state |
| Parse messages | Store historical data |
| Convert to TLV | Make trading decisions |
| Handle reconnection | Implement complex logic |
| Forward data | Validate business rules |

## Step-by-Step Implementation

### Step 1: Create Directory Structure

```bash
src/input/collectors/your_exchange/
├── mod.rs          # Main implementation
└── README.md       # Exchange-specific docs
```

### Step 2: Define Message Types

```rust
// Match exchange's JSON structure EXACTLY
#[derive(Debug, Clone, Deserialize)]
pub struct YourExchangeTrade {
    #[serde(rename = "p")]  // If exchange uses short names
    pub price: String,       // ALWAYS String for precision
    
    #[serde(rename = "q")]
    pub quantity: String,    // ALWAYS String for precision
    
    pub symbol: String,
    pub timestamp: u64,
    pub side: String,        // "buy" or "sell"
}
```

### Step 3: Implement Validation

```rust
impl YourExchangeTrade {
    pub fn validate(&self) -> Result<()> {
        // Check required fields
        if self.price.is_empty() {
            return Err(AdapterError::Validation("Empty price".into()));
        }
        
        // Validate side
        if self.side != "buy" && self.side != "sell" {
            return Err(AdapterError::Validation("Invalid side".into()));
        }
        
        // Parse decimals to check validity
        let _price = Decimal::from_str(&self.price)?;
        let _qty = Decimal::from_str(&self.quantity)?;
        
        Ok(())
    }
}
```

### Step 4: Implement TLV Conversion

```rust
impl TryFrom<YourExchangeTrade> for TradeTLV {
    type Error = AdapterError;
    
    fn try_from(trade: YourExchangeTrade) -> Result<Self> {
        // Validate first
        trade.validate()?;
        
        // Convert strings to fixed-point (8 decimals for CEX)
        let price = parse_decimal_to_fixed_point(&trade.price, 8)?;
        let volume = parse_decimal_to_fixed_point(&trade.quantity, 8)?;
        
        // Map side
        let side = match trade.side.as_str() {
            "buy" => 0,
            "sell" => 1,
            _ => return Err(AdapterError::Validation("Invalid side".into())),
        };
        
        // Create InstrumentId (IMPORTANT: Use correct method!)
        let instrument = InstrumentId::coin(
            &extract_base(&trade.symbol),
            &extract_quote(&trade.symbol)
        );
        
        Ok(TradeTLV::new(
            VenueId::YourExchange,
            instrument,
            price,
            volume,
            side,
            trade.timestamp * 1_000_000, // Convert to nanos
        ))
    }
}

fn parse_decimal_to_fixed_point(s: &str, decimals: u32) -> Result<i64> {
    let decimal = Decimal::from_str(s)?;
    let multiplier = 10_i64.pow(decimals);
    let fixed = (decimal * Decimal::from(multiplier)).to_i64()
        .ok_or_else(|| AdapterError::Overflow)?;
    Ok(fixed)
}
```

### Step 5: Implement Collector

```rust
pub struct YourExchangeCollector {
    // NO StateManager! Adapters are stateless
    connection: Arc<ConnectionManager>,
    output_tx: Sender<TLVMessage>,
    metrics: Arc<AdapterMetrics>,
    running: Arc<AtomicBool>,
}

impl YourExchangeCollector {
    pub fn new(
        config: ConnectionConfig,
        output_tx: Sender<TLVMessage>,
        metrics: Arc<AdapterMetrics>,
    ) -> Self {
        Self {
            // Use ConnectionManager for robust reconnection
            connection: Arc::new(ConnectionManager::new(
                VenueId::YourExchange,
                config,
                metrics.clone(),
            )),
            output_tx,
            metrics,
            running: Arc::new(AtomicBool::new(false)),
        }
    }
}
```

### Step 6: Implement WebSocket Handler

```rust
impl YourExchangeCollector {
    async fn handle_message(&self, msg: Message) -> Result<()> {
        match msg {
            Message::Text(text) => {
                // Parse JSON
                let trade: YourExchangeTrade = serde_json::from_str(&text)?;
                
                // Convert to TLV
                let tlv = TradeTLV::try_from(trade)?;
                
                // Forward to output
                self.output_tx.send(tlv.to_tlv_message()).await?;
                
                // Update metrics
                self.metrics.messages_processed.fetch_add(1, Ordering::Relaxed);
            }
            Message::Close(_) => {
                // Handle disconnection
                self.connection.handle_disconnect().await;
            }
            _ => {} // Ignore ping/pong/binary
        }
        
        Ok(())
    }
}
```

## Common Patterns

### Pattern: Packed Field Access
⚠️ **CRITICAL**: See `PACKED_STRUCTS.md` for full safety information!

```rust
// WRONG - Creates unaligned reference (undefined behavior!)
println!("{}", tlv.price);  // ❌ Can crash on ARM/M1/M2!

// CORRECT - Always copy packed fields first
let price = tlv.price;      // ✅ Copy to stack
println!("{}", price);      // ✅ Now safe to use
```

### Pattern: InstrumentId Creation
```rust
// WRONG - Methods that don't exist
InstrumentId::crypto("BTC", "USD")   // ❌
InstrumentId::currency("USD")        // ❌

// CORRECT - Actual methods
InstrumentId::coin("BTC", "USD")     // ✅ For crypto
InstrumentId::stock("AAPL")          // ✅ For stocks
```

### Pattern: Decimal Handling
```rust
// WRONG - Float loses precision
let price: f64 = 123.456789;  // ❌

// CORRECT - String → Decimal → Fixed-point
let price_str = "123.456789";
let decimal = Decimal::from_str(price_str)?;
let fixed_point = (decimal * Decimal::from(100_000_000)).to_i64()?;  // ✅
```

### Pattern: Error Handling
```rust
// WRONG - Panic on error
let trade: Trade = serde_json::from_str(&msg).unwrap();  // ❌

// CORRECT - Propagate errors
let trade: Trade = serde_json::from_str(&msg)
    .map_err(|e| AdapterError::ParseError(e.to_string()))?;  // ✅
```

## Testing Requirements

### 1. Unit Tests (Required)
```rust
#[test]
fn test_message_parsing() {
    let json = r#"{"price": "100.50", "quantity": "1.5", ...}"#;
    let trade: YourExchangeTrade = serde_json::from_str(json).unwrap();
    assert_eq!(trade.price, "100.50");
}

#[test]
fn test_tlv_conversion() {
    let trade = YourExchangeTrade { ... };
    let tlv = TradeTLV::try_from(trade).unwrap();
    assert_eq!(tlv.price, 10050000000); // 100.50 * 1e8
}
```

### 2. Roundtrip Validation (Required)
```rust
#[test]
fn test_roundtrip_validation() {
    // Parse → TLV → Binary → TLV → Verify
    let json = load_real_sample();
    let parsed = parse_message(json);
    let tlv = TradeTLV::try_from(parsed)?;
    let bytes = tlv.as_bytes();
    let recovered = TradeTLV::from_bytes(bytes)?;
    assert_eq!(tlv, recovered);  // Must be identical
}
```

### 3. Real Data Tests (Required)
```rust
#[test]
fn test_with_real_exchange_data() {
    // Use captured real data, NOT mocks
    let samples = load_captured_samples("fixtures/exchange_trades.json");
    for sample in samples {
        validate_complete_pipeline(sample)?;
    }
}
```

## Common Pitfalls

### Pitfall 1: StateManager in Adapters
```rust
// WRONG - Adapters should NOT have state
pub struct BadAdapter {
    state: Arc<StateManager>,  // ❌
}

// CORRECT - Stateless
pub struct GoodAdapter {
    connection: Arc<ConnectionManager>,  // ✅
    output_tx: Sender<TLVMessage>,      // ✅
}
```

### Pitfall 2: Wrong API Methods
```rust
// These don't exist but seem logical:
StateManager::new()           // ❌ Use with_venue_and_metrics()
InstrumentId::crypto()        // ❌ Use coin()
TradeTLV::read_from()        // ❌ Use from_bytes()
tlv.to_bytes()               // ❌ Use as_bytes()
```

### Pitfall 3: Floating Point Math
```rust
// WRONG
let total = price * quantity;  // ❌ Float multiplication

// CORRECT
let price_fixed = parse_to_fixed_point(price_str)?;
let qty_fixed = parse_to_fixed_point(qty_str)?;
let total = price_fixed.checked_mul(qty_fixed)?;  // ✅
```

### Pitfall 4: Trusting Exchange Data
```rust
// WRONG - No validation
let tlv = TradeTLV::try_from(untrusted_data)?;  // ❌

// CORRECT - Validate first
untrusted_data.validate()?;  // ✅
let tlv = TradeTLV::try_from(untrusted_data)?;
```

## Checklist

Before considering your adapter complete:

### Implementation
- [ ] Created directory with mod.rs and README.md
- [ ] Defined message structs matching exchange format
- [ ] Implemented validation for all message types
- [ ] Implemented TryFrom for TLV conversion
- [ ] Used ConnectionManager for WebSocket handling
- [ ] NO StateManager in the adapter
- [ ] All decimal math uses fixed-point integers
- [ ] Packed field accesses use copy pattern

### Testing
- [ ] Unit tests for parsing
- [ ] Unit tests for TLV conversion
- [ ] Roundtrip validation test
- [ ] Tests use real captured data (no mocks)
- [ ] Four-step validation implemented
- [ ] Performance meets <1ms target

### Documentation
- [ ] README explains exchange specifics
- [ ] Code has explanatory comments
- [ ] Common pitfalls documented
- [ ] API endpoints documented

### Code Quality
- [ ] No compiler warnings
- [ ] cargo clippy passes
- [ ] cargo fmt applied
- [ ] No hardcoded values
- [ ] Proper error handling

## Getting Help

1. **Use Coinbase adapter as reference** - It's the canonical example
2. **Check Protocol V2 docs** - For TLV message format
3. **Run validation tests** - They catch most issues
4. **Review this guide** - Most problems are covered here

## Final Notes

Remember: Adapters are simple data transformers. If you find yourself implementing complex logic, state management, or business rules, you're probably doing too much. Keep it simple:

1. Connect to exchange
2. Parse messages
3. Convert to TLV
4. Forward downstream

That's it! Everything else belongs in other parts of the system.