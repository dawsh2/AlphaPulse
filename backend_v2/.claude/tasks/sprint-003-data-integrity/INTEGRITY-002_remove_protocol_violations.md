# INTEGRITY-002: Remove Protocol-Violating DemoDeFiArbitrageTLV

## Task Overview
**Sprint**: 003-data-integrity
**Priority**: CRITICAL
**Estimate**: 3 hours
**Status**: TODO
**Blocker**: Type 255 is reserved for ExtendedTLV, not signals!

## Problem
DemoDeFiArbitrageTLV violates Protocol V2 by using type 255 (ExtendedTLV marker) as a signal type. Signals MUST use types 20-39 per domain boundaries.

## Files to Modify
- `protocol_v2/src/tlv/types.rs` - Remove DemoDeFiArbitrageTLV
- `protocol_v2/src/tlv/mod.rs` - Remove module export
- `services_v2/dashboard/websocket_server/src/message_converter.rs` - Remove type 255 handling
- `services_v2/strategies/flash_arbitrage/src/signal_output.rs` - Use ArbitrageSignalTLV

## Protocol Violations

### Current (WRONG)
```rust
// types.rs - VIOLATION!
pub enum TLVType {
    DemoDeFiArbitrage = 255, // This is ExtendedTLV marker!
}

// This breaks the entire protocol!
// Type 255 means "look for extended type in next bytes"
```

### Required (CORRECT)
```rust
// Use existing ArbitrageSignalTLV (type 21)
pub enum TLVType {
    ArbitrageSignal = 21, // Proper Signal domain (20-39)
}
```

## Implementation Steps

### Step 1: Remove DemoDeFiArbitrageTLV
```bash
# Remove all traces
grep -r "DemoDeFiArbitrageTLV" --include="*.rs"
# Delete every occurrence
```

### Step 2: Update Dashboard Converter
```rust
// BEFORE: Special handling for type 255
match tlv_type {
    255 => { // WRONG! This is ExtendedTLV!
        process_demo_arbitrage(data)
    }
}

// AFTER: Proper signal handling
match tlv_type {
    21 => { // ArbitrageSignalTLV
        process_arbitrage_signal(data)
    }
}
```

### Step 3: Update Signal Output
```rust
// BEFORE: Protocol violation
let tlv = DemoDeFiArbitrageTLV { ... }; // Type 255

// AFTER: Proper signal
let tlv = ArbitrageSignalTLV { ... }; // Type 21
builder.add_tlv(TLVType::ArbitrageSignal, &tlv);
```

### Step 4: Update Frontend Expectations
```typescript
// Frontend should expect type 21, not 255
interface ArbitrageSignal {
    tlvType: 21; // Not 255!
    // ... rest of fields
}
```

## TDD Test Cases

```rust
#[test]
fn test_no_type_255_signals() {
    // Type 255 must NEVER be used for signals
    let message = create_test_signal_message();
    let tlvs = parse_tlv_extensions(&message);

    for tlv in tlvs {
        assert_ne!(tlv.tlv_type, 255, "Type 255 is ExtendedTLV only!");
        if is_signal_type(tlv.tlv_type) {
            assert!(tlv.tlv_type >= 20 && tlv.tlv_type <= 39);
        }
    }
}

#[test]
fn test_signal_domain_boundaries() {
    // Signals MUST be in range 20-39
    assert_eq!(TLVType::ArbitrageSignal as u8, 21);
    assert!(21 >= 20 && 21 <= 39);
}

#[test]
fn test_extended_tlv_reserved() {
    // Type 255 is reserved for protocol extension
    assert_eq!(TLVType::ExtendedTLV as u8, 255);
    // This should NEVER be a data-carrying type
}
```

## Domain Boundary Reference
Per Protocol V2 specification:
- **Market Data**: Types 1-19
- **Signals**: Types 20-39 ← Arbitrage signals go HERE
- **Execution**: Types 40-79
- **System**: Types 80-99
- **Reserved**: Types 100-254
- **ExtendedTLV**: Type 255 (protocol extension marker)

## Validation Checklist
- [ ] DemoDeFiArbitrageTLV completely removed
- [ ] No references to type 255 for signals
- [ ] ArbitrageSignalTLV (type 21) used instead
- [ ] Dashboard handles type 21 correctly
- [ ] Tests verify domain boundaries
- [ ] Protocol compliance validated

## Why This Matters
Using type 255 for data violates the protocol extension mechanism. When we need types beyond 255, the protocol uses type 255 as a marker to read extended type info. By using it for data, we've broken forward compatibility.

## Definition of Done
- Type 255 no longer used for signals
- ArbitrageSignalTLV (type 21) handles all arbitrage data
- Dashboard correctly processes type 21
- Protocol V2 compliance restored
- Tests enforce domain boundaries
