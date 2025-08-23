# Common API Mistakes and Solutions

This guide shows the most common API confusion points and their solutions.

## InstrumentId Creation

### ❌ WRONG - These methods don't exist
```rust
// These will cause compilation errors:
InstrumentId::crypto("BTC", "USD")     // Method doesn't exist
InstrumentId::currency("USD")          // Method doesn't exist  
InstrumentId::forex("EUR", "USD")      // Method doesn't exist
InstrumentId::pair("BTC", "USD")       // Method doesn't exist
InstrumentId::symbol("AAPL")           // Method doesn't exist
```

### ✅ CORRECT - Available methods
```rust
use alphapulse_protocol_v2::InstrumentId;

// Cryptocurrency pairs - USE coin()
let btc_usd = InstrumentId::coin("BTC", "USD");
let eth_usdc = InstrumentId::coin("ETH", "USDC");

// Stocks and traditional assets - USE stock()
let apple = InstrumentId::stock("AAPL");
let tesla = InstrumentId::stock("TSLA");

// From raw numeric ID - USE from_u64()
let raw_id = InstrumentId::from_u64(12345);

// Convert back to raw ID
let numeric_id: u64 = btc_usd.into();
```

## TLV Message Construction

### ❌ WRONG - Incorrect message building
```rust
// These patterns are incorrect:
let trade = TradeTLV::new(/* args */);
let bytes = trade.to_bytes();          // Method doesn't exist
let message = TLVMessage::from(trade); // Constructor doesn't exist
```

### ✅ CORRECT - Proper TLV workflow
```rust
use alphapulse_protocol_v2::{TLVMessageBuilder, TLVType, TradeTLV, RelayDomain, SourceType};

// 1. Create the TLV struct
let trade = TradeTLV::new(/* proper args */);

// 2. Build message with routing
let message = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::BinanceCollector)
    .add_tlv(TLVType::Trade, &trade)
    .build();

// 3. Serialize for transport (zero-copy)
let bytes = message.as_bytes();

// 4. Parse received message
let header = alphapulse_protocol_v2::parse_header(&bytes)?;
let tlvs = alphapulse_protocol_v2::parse_tlv_extensions(&bytes[32..])?;
```

## Packed Struct Usage (Critical!)

### ❌ WRONG - Unsafe packed field access
```rust
let trade_tlv = TradeTLV::new(/* ... */);

// DANGEROUS - direct field access on packed structs
println!("Price: {}", trade_tlv.price);  // Undefined behavior!
let price_ref = &trade_tlv.price;        // Unaligned reference!
```

### ✅ CORRECT - Safe packed struct handling
```rust
use zerocopy::AsBytes;

let trade_tlv = TradeTLV::new(/* ... */);

// SAFE - copy fields to stack first
let price = trade_tlv.price;     // Copy to local variable
let volume = trade_tlv.volume;   // Now safe to use
println!("Trade: {} @ {}", volume, price);

// SAFE - serialization via zerocopy
let bytes = trade_tlv.as_bytes();
let recovered = TradeTLV::from_bytes(bytes)?;
```

## TLV Type Discovery

### ❌ WRONG - Hardcoded type assumptions
```rust
// Don't hardcode type numbers or assume types exist:
let trade_type = 1;  // Hardcoded - fragile
let unknown_type = TLVType::CustomTrade;  // Doesn't exist
```

### ✅ CORRECT - Use the developer API
```rust
use alphapulse_protocol_v2::{TLVType, RelayDomain};

// Discover available types
let info = TLVType::Trade.type_info();
println!("Type {}: {} - {}", info.type_number, info.name, info.description);

// Find types by domain
let market_types = TLVType::types_in_domain(RelayDomain::MarketData);
for tlv_type in market_types {
    println!("Available: {}", tlv_type.name());
}

// Check if type is implemented
if TLVType::Trade.is_implemented() {
    println!("Trade type is ready to use");
}
```

## Size and Bounds Validation

### ❌ WRONG - Ignoring size constraints
```rust
// Don't skip validation:
let raw_data = get_raw_bytes();
let trade = TradeTLV::from_bytes(&raw_data)?;  // Could fail!
```

### ✅ CORRECT - Validate before parsing
```rust
let raw_data = get_raw_bytes();

// Check size constraints first
match TLVType::Trade.size_constraint() {
    TLVSizeConstraint::Fixed(expected) => {
        if raw_data.len() != expected {
            return Err(ProtocolError::InvalidSize);
        }
    }
    TLVSizeConstraint::Bounded { min, max } => {
        if raw_data.len() < min || raw_data.len() > max {
            return Err(ProtocolError::InvalidSize);
        }
    }
    TLVSizeConstraint::Variable => {
        // Variable size - just check minimum
    }
}

let trade = TradeTLV::from_bytes(&raw_data)?;
```

## Relay Domain Routing

### ❌ WRONG - Manual routing logic
```rust
// Don't hardcode routing rules:
let message_type = 1;  // Trade
let relay = if message_type <= 19 {
    "market_data"
} else if message_type <= 39 {
    "signals"  
} else {
    "execution"
};
```

### ✅ CORRECT - Use automatic routing
```rust
// Routing is automatic based on TLV type:
let domain = TLVType::Trade.relay_domain();
println!("Routes to: {:?}", domain);  // RelayDomain::MarketData

// Get all types for a domain
let execution_types = TLVType::types_in_domain(RelayDomain::Execution);
```

## Error Handling

### ❌ WRONG - Ignoring specific errors
```rust
// Don't use generic error handling:
let result = parse_header(&data);
if result.is_err() {
    println!("Something went wrong");  // Unhelpful
    return;
}
```

### ✅ CORRECT - Handle specific error types
```rust
use alphapulse_protocol_v2::{ProtocolError, ParseError};

match parse_header(&data) {
    Ok(header) => {
        // Process header
    }
    Err(ProtocolError::Parse(ParseError::TruncatedHeader)) => {
        eprintln!("Message too short for header");
    }
    Err(ProtocolError::ChecksumFailed) => {
        eprintln!("Message integrity check failed");
    }
    Err(e) => {
        eprintln!("Protocol error: {}", e);
    }
}
```

## Method Discovery Tips

### Use IDE Features
1. **Type and press `.`** - See available methods
2. **Use `cargo doc --open`** - Browse full API documentation  
3. **Check examples/** - Runnable code samples
4. **Use the help module** - `use alphapulse_protocol_v2::help::*;`

### Available InstrumentId Methods (Complete List)
```rust
// These are ALL the available methods:
InstrumentId::coin(base, quote)     // Crypto pairs
InstrumentId::stock(symbol)         // Stocks  
InstrumentId::from_u64(id)          // Raw numeric ID

// Convert back to u64
let numeric: u64 = instrument_id.into();

// Methods that DON'T exist:
// ❌ crypto(), currency(), forex(), pair(), symbol(), new()
```

### Available TLVType Methods (Complete List)
```rust
// Information methods:
TLVType::Trade.name()                    // "Trade"
TLVType::Trade.description()             // Human description
TLVType::Trade.type_info()               // Complete metadata
TLVType::Trade.size_constraint()         // Size validation info
TLVType::Trade.relay_domain()            // Routing domain
TLVType::Trade.is_implemented()          // Implementation status

// Query methods:
TLVType::types_in_domain(domain)         // Filter by domain
TLVType::all_implemented()               // All available types
TLVType::generate_markdown_table()       // Auto-generate docs
```

## Quick Reference

When you see a compilation error like "method not found", check this guide first. The most common issues are:

1. **`InstrumentId::crypto()`** → Use `InstrumentId::coin()`
2. **`trade.to_bytes()`** → Use `trade.as_bytes()` (zerocopy trait)
3. **Direct packed field access** → Copy to local variable first
4. **Missing TLV methods** → Check the complete method list above

## Getting Help

1. **Run examples**: `cargo run --example instrument_id_creation`
2. **Browse docs**: `cargo doc --open --document-private-items`
3. **Use inline help**: `alphapulse_protocol_v2::help::show_instrument_id_methods()`
4. **Check type info**: `TLVType::Trade.type_info()`