---
task_id: CODEC-001
status: COMPLETE
priority: CRITICAL
estimated_hours: 4
assigned_branch: refactor/codec-foundation
assignee: TBD
created: 2025-08-26
completed: null
---

# CODEC-001: Create libs/alphapulse_codec Foundation

## 🔴 CRITICAL INSTRUCTIONS
```bash
# BEFORE STARTING - VERIFY YOU'RE NOT ON MAIN:
git branch --show-current

# If you see "main", IMMEDIATELY run:
git worktree add -b refactor/codec-foundation

# NEVER commit directly to main!
```

## Status
**Status**: TODO
**Priority**: CRITICAL
**Branch**: `refactor/codec-foundation`
**Estimated**: 4 hours

## Problem Statement
Create the foundation for `libs/alphapulse_codec` - the new home for protocol rules and logic. This will contain the "grammar" of the AlphaPulse system: encoding/decoding rules, message builders, and protocol constants.

## Acceptance Criteria
- [ ] New `libs/alphapulse_codec` crate created with proper Cargo.toml
- [ ] Bijective InstrumentId system moved from protocol_v2
- [ ] TLVType registry and constants moved from protocol_v2
- [ ] Core protocol constants (MESSAGE_MAGIC, etc.) moved
- [ ] All moved code works identically to original
- [ ] Clean dependency on `libs/types` only
- [ ] No network or transport logic included

## Technical Approach

### New Crate Structure
```
libs/alphapulse_codec/
├── Cargo.toml           # New crate configuration
├── src/
│   ├── lib.rs          # Main exports and module organization
│   ├── instrument_id.rs # Bijective InstrumentId system (COPY from protocol_v2)
│   ├── tlv_types.rs    # TLVType registry and constants (COPY from protocol_v2)
│   └── constants.rs    # Protocol constants (COPY from protocol_v2)
└── tests/
    └── codec_tests.rs  # Basic codec functionality tests
```

### Files to Create

#### libs/alphapulse_codec/Cargo.toml
```toml
[package]
name = "alphapulse_codec"
version = "0.1.0"
edition = "2021"
description = "AlphaPulse protocol codec - encoding/decoding rules and message construction"

[dependencies]
# Only depend on pure data types - no network dependencies
alphapulse_types = { path = "../types" }
thiserror = "1.0"
serde = { version = "1.0", features = ["derive"] }
zerocopy = { version = "0.7", features = ["derive"] }

[dev-dependencies]
criterion = "0.5"
```

#### libs/alphapulse_codec/src/lib.rs
```rust
//! AlphaPulse Protocol Codec
//!
//! This crate contains the "Rules" layer of the AlphaPulse system:
//! - Protocol encoding/decoding logic
//! - Message construction and validation
//! - Bijective identifier systems
//! - TLV type registry and constants
//!
//! ## What This Crate Contains
//! - TLVMessageBuilder for constructing valid messages
//! - InstrumentId bijective identifier system
//! - Protocol parsing functions
//! - TLVType registry and validation
//! - Protocol constants and error types
//!
//! ## What This Crate Does NOT Contain
//! - Network transport logic (belongs in network/)
//! - Raw data structure definitions (belongs in libs/types)
//! - Socket management or connection handling

pub mod instrument_id;
pub mod tlv_types;
pub mod constants;

// Re-export key types for convenience
pub use instrument_id::{InstrumentId, VenueId};
pub use tlv_types::{TLVType, TlvTypeRegistry};
pub use constants::*;
```

### Implementation Steps

#### Step 1: Create Crate Structure (1 hour)
1. **Create directory structure** for new codec crate
2. **Set up Cargo.toml** with minimal dependencies (only libs/types)
3. **Create basic lib.rs** with module organization
4. **Add to workspace** in root Cargo.toml

#### Step 2: Move Bijective InstrumentId System (1.5 hours)
```rust
// COPY from protocol_v2/src/identifiers/ to libs/alphapulse_codec/src/instrument_id.rs

/// Bijective instrument identifier system
/// Self-describing IDs that can be reversed to extract venue and asset info
pub struct InstrumentId {
    // COPY exact implementation from protocol_v2
}

impl InstrumentId {
    // COPY all methods exactly as-is:
    // - from_venue_and_symbol()
    // - to_fast_hash()
    // - extract_venue()
    // - extract_symbol()
    // etc.
}

// COPY all tests from protocol_v2 bijection tests
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_bijective_roundtrip() {
        // COPY existing bijection tests exactly
    }
}
```

#### Step 3: Move TLV Type Registry (1 hour)
```rust
// COPY from protocol_v2/src/tlv/types.rs to libs/alphapulse_codec/src/tlv_types.rs

/// Official TLV type registry for AlphaPulse protocol
#[repr(u16)]
pub enum TLVType {
    // Market Data domain (1-19)
    Trade = 1,
    Quote = 2,
    // ... COPY all existing types exactly
    
    // Signal domain (20-39)
    SignalIdentity = 20,
    // ... COPY all existing types exactly
    
    // Execution domain (40-79)  
    ExecutionOrder = 40,
    // ... COPY all existing types exactly
}

pub struct TlvTypeRegistry {
    // COPY registry implementation exactly
}

// COPY all validation logic exactly
```

#### Step 4: Move Protocol Constants (0.5 hours)
```rust
// COPY from protocol_v2/src/constants.rs to libs/alphapulse_codec/src/constants.rs

/// Protocol magic number for message headers
pub const MESSAGE_MAGIC: u32 = 0xDEADBEEF;

/// Maximum message size in bytes
pub const MAX_MESSAGE_SIZE: usize = 1024 * 1024; // 1MB

// COPY all other protocol constants exactly
```

### Testing Requirements

#### Unit Tests (White-Box Testing)
```rust
// libs/alphapulse_codec/src/instrument_id.rs
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_instrument_id_bijection() {
        // Test that InstrumentId can round-trip perfectly
        let venue = "uniswap_v3";
        let symbol = "WETH/USDC";
        
        let id = InstrumentId::from_venue_and_symbol(venue, symbol);
        assert_eq!(id.extract_venue(), venue);
        assert_eq!(id.extract_symbol(), symbol);
    }
    
    #[test]
    fn test_fast_hash_performance() {
        // Ensure O(1) hash performance
        let id = InstrumentId::from_venue_and_symbol("polygon", "MATIC/USDC");
        let hash = id.to_fast_hash();
        assert!(hash != 0);
    }
}
```

#### Integration Tests (Black-Box Testing)
```rust
// libs/alphapulse_codec/tests/codec_tests.rs
use alphapulse_codec::{InstrumentId, TLVType};

#[test]
fn test_codec_public_api() {
    // Test the public API that external crates will use
    let id = InstrumentId::from_venue_and_symbol("kraken", "BTC/USD");
    assert!(!id.extract_venue().is_empty());
    
    let tlv_type = TLVType::Trade;
    assert_eq!(tlv_type as u16, 1);
}
```

### Testing Instructions
```bash
# Test new codec crate
cargo test --package alphapulse_codec

# Test integration with types
cargo test --package alphapulse_codec --test codec_tests

# Benchmark performance (ensure no regression)
cargo bench --package alphapulse_codec

# Verify workspace builds
cargo build --workspace
```

## Git Workflow
```bash
# 1. Start on your branch
git worktree add -b refactor/codec-foundation

# 2. Create crate structure first
git add libs/alphapulse_codec/Cargo.toml libs/alphapulse_codec/src/lib.rs
git commit -m "feat: create libs/alphapulse_codec foundation crate"

# 3. Move InstrumentId system
git add libs/alphapulse_codec/src/instrument_id.rs
git commit -m "refactor: move bijective InstrumentId system to codec crate"

# 4. Move TLV types and constants
git add libs/alphapulse_codec/src/tlv_types.rs libs/alphapulse_codec/src/constants.rs
git commit -m "refactor: move TLV type registry and constants to codec crate"

# 5. Push and create PR
git push origin refactor/codec-foundation
gh pr create --title "CODEC-001: Create libs/alphapulse_codec foundation" \
             --body "Creates new codec crate with InstrumentId, TLV types, and constants"
```

## Completion Checklist
- [ ] Working on correct branch (not main)
- [ ] New libs/alphapulse_codec crate created
- [ ] InstrumentId system moved and working identically
- [ ] TLV type registry moved and working identically
- [ ] Protocol constants moved and accessible
- [ ] All tests passing in new crate
- [ ] Integration tests verify identical behavior
- [ ] Workspace builds successfully
- [ ] PR created
- [ ] **🚨 CRITICAL: Updated task status to COMPLETE** ← AGENTS MUST DO THIS!

## Notes
This task creates the foundation for the codec layer. Subsequent tasks will move the message builder and parsing logic. The goal is to establish a clean home for protocol rules separate from data structures (types) and transport (network).