# Dependency Import Patterns

**Purpose**: Standardized patterns for importing from `alphapulse_types` and `codec` to maintain clean architecture.

## Import Pattern Guidelines

### Core Principle: Import Based on Functionality Needed

```rust
// Types only (data structures and constants)
use alphapulse_types::{QuoteTLV, InstrumentId, VenueId, InvalidationReason, TLVType};

// Codec only (message processing)
use codec::{TLVMessageBuilder, parse_header, parse_tlv_extensions, ParseError};

// Both (full message pipeline)
use alphapulse_types::{QuoteTLV, StateInvalidationTLV, RelayDomain, SourceType, TLVType};
use codec::{TLVMessageBuilder, parse_header, parse_tlv_extensions, ParseError, TLVExtensionEnum};
```

## Service-Level Import Patterns

### Dashboard WebSocket Server
**Purpose**: Parse incoming TLV messages and convert to JSON

```rust
// services_v2/dashboard/websocket_server/src/relay_consumer.rs
use codec::{
    parse_header, parse_tlv_extensions, ParseError, TLVExtensionEnum,
};
use alphapulse_types::{
    QuoteTLV, InvalidationReason, StateInvalidationTLV, PoolSwapTLV,
    InstrumentId, VenueId, SystemHealthTLV, TraceEvent
};
```

**Rationale**: Needs codec parsing functions and type definitions for deserialization.

### Flash Arbitrage Strategy  
**Purpose**: Consume market data, generate signals, output TLV messages

```rust
// services_v2/strategies/flash_arbitrage/src/signal_output.rs
use codec::TLVMessageBuilder;
use alphapulse_types::{
    ArbitrageSignalTLV, RelayDomain, SourceType, TLVType,
    InstrumentId, VenueId
};
```

**Rationale**: Needs message construction (codec) and signal types (types).

### Adapter Services
**Purpose**: Collect market data, output TLV messages, manage state

```rust
// services_v2/adapters/src/input/state_manager.rs
use alphapulse_types::{
    tlv, StateInvalidationTLV, InstrumentId, InvalidationReason, 
    RelayDomain, SourceType, TLVType, VenueId
};
```

**Rationale**: Needs type definitions and built-in TLV message construction.

### Relay Services
**Purpose**: Route messages between domains, validate TLV formats

```rust
// relays/src/topics.rs
use codec::{parse_tlv_extensions, TLVExtensionEnum};
use alphapulse_types::{TLVType, RelayDomain};
```

**Rationale**: Needs parsing functions for routing and type definitions for validation.

## Anti-Patterns to Avoid

### ❌ Don't Import Everything
```rust
// WRONG: Pulls in unnecessary dependencies
use alphapulse_types::*;
use codec::*;
```

### ❌ Don't Mix Construction Patterns
```rust
// WRONG: Inconsistent message building approaches
use alphapulse_types::tlv::build_message_direct;  // Built-in approach
use codec::TLVMessageBuilder;          // Builder approach
```

**Preferred**: Choose one approach consistently within a service.

### ❌ Don't Reintroduce Circular Dependencies
```rust
// WRONG: This would recreate the circular dependency
// In libs/types/src/lib.rs:
use codec::TLVMessageBuilder;  // DON'T DO THIS!
```

## Recommended Patterns by Use Case

### 1. Pure Data Processing (Types Only)
```rust
use alphapulse_types::{QuoteTLV, InstrumentId, VenueId};

fn process_quote(quote: &QuoteTLV) -> f64 {
    quote.bid_price as f64 / 100_000_000.0  // Convert to USD
}
```

### 2. Message Parsing (Codec + Types)
```rust
use codec::{parse_header, parse_tlv_extensions, ParseError};
use alphapulse_types::{QuoteTLV, StateInvalidationTLV};

fn parse_message(data: &[u8]) -> Result<(), ParseError> {
    let header = parse_header(data)?;
    let payload = &data[32..];
    let tlvs = parse_tlv_extensions(payload)?;
    
    for tlv in tlvs {
        match tlv {
            TLVExtensionEnum::Standard(std_tlv) => {
                // Process standard TLV
            }
            TLVExtensionEnum::Extended(ext_tlv) => {
                // Process extended TLV
            }
        }
    }
    Ok(())
}
```

### 3. Message Construction (Codec + Types)
```rust
use codec::TLVMessageBuilder;
use alphapulse_types::{
    ArbitrageSignalTLV, RelayDomain, SourceType, TLVType,
    InstrumentId, VenueId
};

fn create_signal_message(signal: ArbitrageSignalTLV) -> Vec<u8> {
    let mut builder = TLVMessageBuilder::new(
        RelayDomain::Signals,
        SourceType::FlashArbitrage,
    );
    builder.add_tlv(TLVType::ArbitrageSignal, &signal);
    builder.build()
}
```

### 4. State Management (Built-in TLV Functions)
```rust
use alphapulse_types::{
    tlv, StateInvalidationTLV, InvalidationReason,
    RelayDomain, SourceType, TLVType, VenueId
};

fn create_invalidation_message(venue: VenueId, instruments: &[InstrumentId]) -> Result<Vec<u8>, Error> {
    let invalidation = StateInvalidationTLV::new(
        venue, 1, instruments, InvalidationReason::Disconnection,
        timestamp_ns()
    )?;
    
    // Use built-in TLV message construction
    tlv::build_message_direct(
        RelayDomain::MarketData,
        SourceType::StateManager,
        TLVType::StateInvalidation,
        &invalidation,
    )
}
```

## Testing Import Patterns

### Unit Tests (Types Focus)
```rust
// tests/unit/my_module.rs
use crate::{QuoteTLV, InstrumentId, VenueId};  // Internal imports
use alphapulse_transport::time::safe_system_timestamp_ns;

#[test]
fn test_quote_construction() {
    let quote = QuoteTLV::new(
        InstrumentId::from_venue_and_symbol(VenueId::Binance, "BTCUSDT"),
        4500000000000,
        4500100000000,
        1000000000,
        500000000,
        safe_system_timestamp_ns(),
    );
    assert_eq!(quote.bid_price, 4500000000000);
}
```

### Integration Tests (Codec + Types)
```rust
// tests/integration/message_flow.rs
use codec::{TLVMessageBuilder, parse_header};
use alphapulse_types::{QuoteTLV, RelayDomain, SourceType, TLVType};

#[test]
fn test_message_round_trip() {
    // Construction
    let mut builder = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::Adapter);
    builder.add_tlv(TLVType::Quote, &quote);
    let message = builder.build();
    
    // Parsing
    let header = parse_header(&message).unwrap();
    assert_eq!(header.relay_domain, RelayDomain::MarketData as u8);
}
```

## Future-Proofing Guidelines

### When Adding New Services
1. **Identify your service's role**: Does it create, consume, or route messages?
2. **Import minimally**: Only import what you actually need
3. **Follow existing patterns**: Use the same import style as similar services
4. **Document special cases**: If you need both packages, explain why

### When Adding New TLV Types
1. **Add to types package**: All TLV struct definitions go in `alphapulse-types`
2. **Add parsing support**: Parsing utilities go in `codec`
3. **Update service imports**: Services using the new type import from `alphapulse-types`
4. **Update builders**: Message construction helpers stay in `codec`

### Version Compatibility
```toml
# In service Cargo.toml files:
[dependencies]
alphapulse-types = { path = "../../libs/types" }
codec = { path = "../../libs/codec" }

# Keep versions in sync - both should be updated together
# when making breaking changes to the protocol
```

## Troubleshooting Import Issues

### Common Error: "Cannot find QuoteTLV in scope"
**Solution**: Add types import
```rust
use alphapulse_types::QuoteTLV;
```

### Common Error: "Cannot find TLVMessageBuilder in scope"  
**Solution**: Add codec import
```rust
use codec::TLVMessageBuilder;
```

### Common Error: "Circular dependency detected"
**Check**: Are you importing codec from types package?
- Remove any `codec` imports from `libs/types/src/**/*.rs`
- Move message construction code to service level

### Common Error: "Missing ParseError type"
**Solution**: Import from codec package
```rust
use codec::ParseError;
```

## Performance Considerations

### Binary Size Optimization
Services that only need type definitions should not import codec:
```rust
// Minimal imports = smaller binary
use alphapulse_types::{QuoteTLV, InstrumentId};

// Instead of:
// use codec::*;  // Pulls in unnecessary parsing machinery
```

### Compilation Time
Services with minimal imports compile faster:
```rust
// Fast compilation
use alphapulse_types::QuoteTLV;

// vs slower compilation with many imports
use alphapulse_types::{
    QuoteTLV, StateInvalidationTLV, PoolSwapTLV, SystemHealthTLV, 
    TraceEvent, /* ... many more types ... */
};
```

## References

- **ADR-001**: Architecture decision for codec/types separation
- **AlphaPulse Practices**: `.claude/docs/core/practices.md`
- **Development Guide**: `.claude/docs/core/development.md`