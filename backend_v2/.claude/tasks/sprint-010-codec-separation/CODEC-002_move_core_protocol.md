---
task_id: CODEC-002
status: TODO
priority: CRITICAL
estimated_hours: 6
assigned_branch: refactor/core-protocol-logic
assignee: TBD
created: 2025-08-26
completed: null
---

# CODEC-002: Move Core Protocol Logic to Codec

## 🔴 CRITICAL INSTRUCTIONS
```bash
# BEFORE STARTING - VERIFY YOU'RE NOT ON MAIN:
git branch --show-current

# If you see "main", IMMEDIATELY run:
git checkout -b refactor/core-protocol-logic

# NEVER commit directly to main!
```

## Status
**Status**: TODO
**Priority**: CRITICAL
**Branch**: `refactor/core-protocol-logic`
**Estimated**: 6 hours

## Problem Statement
Move the core protocol logic from protocol_v2 to libs/alphapulse_codec. This includes TLVMessageBuilder, parsing functions, and protocol validation - the "rules" that define how the AlphaPulse protocol works.

## Acceptance Criteria
- [ ] TLVMessageBuilder moved to codec crate
- [ ] Core parsing functions (parse_header, parse_tlv_extensions) moved
- [ ] ProtocolError enum and validation logic moved
- [ ] Message construction and validation preserved exactly
- [ ] Performance maintained (>1M msg/s construction, >1.6M parsing)
- [ ] All protocol_v2 tests pass with new locations
- [ ] Clean separation: no network logic in codec

## Technical Approach - The "Rules" Layer

### What Gets Moved to Codec
```rust
// The "Grammar" of AlphaPulse Protocol:

// Message Construction
- TLVMessageBuilder
- Message validation logic
- Header construction

// Parsing Logic  
- parse_header()
- parse_tlv_extensions()
- TLV validation and bounds checking

// Protocol Rules
- ProtocolError enum
- Message size limits
- TLV format validation
- Checksum verification
```

### Files to Create/Modify

#### libs/alphapulse_codec/src/message_builder.rs
```rust
// COPY from protocol_v2/src/message_builder.rs

use alphapulse_types::{TradeTLV, QuoteTLV}; // Import data types
use crate::{TLVType, MESSAGE_MAGIC};

/// Builds valid TLV messages according to AlphaPulse protocol rules
pub struct TLVMessageBuilder {
    // COPY exact implementation from protocol_v2
    relay_domain: u8,
    source: u16,
    sequence: u64,
    tlvs: Vec<TLVExtension>,
}

impl TLVMessageBuilder {
    // COPY all methods exactly:
    // - new()
    // - add_tlv()
    // - build()
    // - calculate_checksum()
    // etc.
    
    /// Constructs a complete TLV message with proper header
    pub fn build(self) -> Result<Vec<u8>, ProtocolError> {
        // COPY exact implementation - this is protocol logic, not transport
        let header = MessageHeader {
            magic: MESSAGE_MAGIC,
            version: 1,
            payload_size: self.calculate_payload_size(),
            relay_domain: self.relay_domain,
            source: self.source,
            sequence: self.sequence,
            timestamp: current_timestamp_nanos(),
            checksum: 0, // Will be calculated
        };
        
        // COPY rest of build logic exactly
    }
}
```

#### libs/alphapulse_codec/src/parser.rs
```rust
// COPY from protocol_v2/src/parser.rs

use alphapulse_types::*; // Import all data structures
use crate::{ProtocolError, MESSAGE_MAGIC};

/// Parses 32-byte message header according to protocol rules
pub fn parse_header(bytes: &[u8]) -> Result<MessageHeader, ProtocolError> {
    // COPY exact implementation from protocol_v2
    if bytes.len() < 32 {
        return Err(ProtocolError::TruncatedHeader);
    }
    
    let magic = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    if magic != MESSAGE_MAGIC {
        return Err(ProtocolError::InvalidMagic(magic));
    }
    
    // COPY rest of header parsing exactly
}

/// Parses TLV extensions from payload according to protocol format
pub fn parse_tlv_extensions(payload: &[u8]) -> Result<Vec<TLVExtension>, ProtocolError> {
    // COPY exact implementation from protocol_v2
    let mut tlvs = Vec::new();
    let mut offset = 0;
    
    while offset < payload.len() {
        // COPY TLV parsing logic exactly - this defines protocol format
        if offset + 4 > payload.len() {
            return Err(ProtocolError::TruncatedTLV);
        }
        
        let tlv_type = u16::from_le_bytes([payload[offset], payload[offset + 1]]);
        let tlv_length = u16::from_le_bytes([payload[offset + 2], payload[offset + 3]]);
        
        // COPY validation and parsing logic exactly
    }
    
    Ok(tlvs)
}

/// Validates TLV message format and contents
pub fn validate_message(header: &MessageHeader, payload: &[u8]) -> Result<(), ProtocolError> {
    // COPY exact validation logic from protocol_v2
    // This is pure protocol rules - no network concerns
}
```

#### libs/alphapulse_codec/src/error.rs
```rust
// COPY from protocol_v2/src/error.rs

use thiserror::Error;

/// Protocol-level errors for TLV message processing
#[derive(Error, Debug, Clone, PartialEq)]
pub enum ProtocolError {
    #[error("Invalid magic number: expected {expected:#x}, got {got:#x}")]
    InvalidMagic { expected: u32, got: u32 },
    
    #[error("Truncated header: expected 32 bytes, got {got}")]
    TruncatedHeader { got: usize },
    
    #[error("Truncated TLV: not enough bytes for header")]
    TruncatedTLV,
    
    #[error("Invalid TLV type: {0}")]
    InvalidTLVType(u16),
    
    #[error("TLV payload size mismatch: expected {expected}, got {got}")]
    PayloadSizeMismatch { expected: usize, got: usize },
    
    #[error("Message too large: {size} bytes exceeds maximum {max}")]
    MessageTooLarge { size: usize, max: usize },
    
    #[error("Checksum mismatch: expected {expected:#x}, got {got:#x}")]
    ChecksumMismatch { expected: u32, got: u32 },
    
    // COPY all other error variants exactly
}
```

### Implementation Steps

#### Step 1: Move Message Builder (2 hours)
1. **Copy TLVMessageBuilder** to libs/alphapulse_codec/src/message_builder.rs
2. **Update imports** to use alphapulse_types for data structures
3. **Preserve exact functionality** - no behavior changes
4. **Test construction performance** - maintain >1M msg/s

#### Step 2: Move Parsing Logic (2 hours)  
1. **Copy parsing functions** to libs/alphapulse_codec/src/parser.rs
2. **Preserve parsing performance** - maintain >1.6M msg/s
3. **Keep validation logic** exactly as-is
4. **Test with existing protocol_v2 test cases**

#### Step 3: Move Protocol Errors (1 hour)
1. **Copy ProtocolError enum** to libs/alphapulse_codec/src/error.rs
2. **Update all error handling** to use new location
3. **Preserve error semantics** exactly

#### Step 4: Update Exports and Integration (1 hour)
```rust
// libs/alphapulse_codec/src/lib.rs - UPDATE
pub mod message_builder;
pub mod parser;
pub mod error;

// Re-export main interfaces
pub use message_builder::TLVMessageBuilder;
pub use parser::{parse_header, parse_tlv_extensions, validate_message};
pub use error::ProtocolError;

// Performance benchmarks - must maintain targets
const TARGET_CONSTRUCTION_RATE: u64 = 1_000_000; // msg/s
const TARGET_PARSING_RATE: u64 = 1_600_000;      // msg/s
```

### Testing Strategy

#### Unit Tests (Protocol Rules)
```rust
// libs/alphapulse_codec/src/message_builder.rs
#[cfg(test)]
mod tests {
    use super::*;
    use alphapulse_types::TradeTLV;
    
    #[test]
    fn test_message_builder_construction() {
        // Test that builder creates valid messages
        let trade = TradeTLV {
            price: 4500000000000, // $45,000.00
            quantity: 100000000,  // 1.0 tokens
            // ... rest of trade data
        };
        
        let mut builder = TLVMessageBuilder::new(1, 100); // domain=1, source=100
        builder.add_tlv(TLVType::Trade, &trade).unwrap();
        let message = builder.build().unwrap();
        
        // Verify proper message structure
        assert_eq!(&message[0..4], &MESSAGE_MAGIC.to_le_bytes());
        assert!(message.len() >= 32); // Header + payload
    }
    
    #[test]
    fn test_message_builder_performance() {
        // Ensure >1M msg/s construction rate
        let start = std::time::Instant::now();
        let iterations = 100_000;
        
        for _ in 0..iterations {
            let mut builder = TLVMessageBuilder::new(1, 1);
            // Add minimal TLV
            builder.build().unwrap();
        }
        
        let elapsed = start.elapsed();
        let rate = iterations as f64 / elapsed.as_secs_f64();
        assert!(rate > 1_000_000.0, "Construction rate too slow: {} msg/s", rate);
    }
}
```

#### Integration Tests (Parser + Builder)
```rust
// libs/alphapulse_codec/tests/codec_integration.rs
use alphapulse_codec::{TLVMessageBuilder, parse_header, parse_tlv_extensions};
use alphapulse_types::TradeTLV;

#[test]
fn test_round_trip_message_processing() {
    // Build message
    let original_trade = TradeTLV { /* ... */ };
    let mut builder = TLVMessageBuilder::new(1, 100);
    builder.add_tlv(TLVType::Trade, &original_trade).unwrap();
    let message_bytes = builder.build().unwrap();
    
    // Parse message back
    let header = parse_header(&message_bytes[0..32]).unwrap();
    let payload = &message_bytes[32..32 + header.payload_size as usize];
    let tlvs = parse_tlv_extensions(payload).unwrap();
    
    // Verify perfect round-trip
    assert_eq!(tlvs.len(), 1);
    assert_eq!(tlvs[0].header.tlv_type, TLVType::Trade as u16);
    
    // Decode trade and verify exact match
    let decoded_trade = TradeTLV::from_bytes(&tlvs[0].data).unwrap();
    assert_eq!(decoded_trade, original_trade);
}
```

### Testing Instructions
```bash
# Test codec crate independently  
cargo test --package alphapulse_codec

# Test performance benchmarks
cargo bench --package alphapulse_codec

# Test integration with types
cargo test --package alphapulse_codec --test codec_integration

# Verify no regressions in protocol_v2
cargo test --package protocol_v2
```

## Git Workflow
```bash
# 1. Start on your branch (ensure CODEC-001 is complete first)
git checkout -b refactor/core-protocol-logic

# 2. Move message builder
git add libs/alphapulse_codec/src/message_builder.rs
git commit -m "refactor: move TLVMessageBuilder to codec crate"

# 3. Move parsing logic
git add libs/alphapulse_codec/src/parser.rs  
git commit -m "refactor: move core parsing functions to codec crate"

# 4. Move protocol errors
git add libs/alphapulse_codec/src/error.rs
git commit -m "refactor: move ProtocolError to codec crate"

# 5. Update integration and exports
git add libs/alphapulse_codec/src/lib.rs
git commit -m "refactor: update codec crate exports and integration"

# 6. Push and create PR
git push origin refactor/core-protocol-logic
gh pr create --title "CODEC-002: Move core protocol logic to codec crate" \
             --body "Moves TLVMessageBuilder, parsing, and validation to dedicated codec layer"
```

## Completion Checklist
- [ ] Working on correct branch (not main)
- [ ] TLVMessageBuilder moved and working identically
- [ ] Parsing functions moved and working identically  
- [ ] Protocol errors moved and handled correctly
- [ ] Performance maintained (>1M construction, >1.6M parsing)
- [ ] All protocol_v2 tests updated and passing
- [ ] Integration tests verify round-trip functionality
- [ ] Codec crate exports clean public API
- [ ] PR created
- [ ] **🚨 CRITICAL: Updated task status to COMPLETE** ← AGENTS MUST DO THIS!

## Notes
This task moves the "brain" of the protocol - the logic that defines how messages are constructed and parsed. After this task, libs/alphapulse_codec will contain all the rules that define the AlphaPulse protocol format, separate from both data definitions (types) and transport concerns (network).