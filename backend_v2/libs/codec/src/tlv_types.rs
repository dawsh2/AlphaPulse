//! # TLV Type System - Protocol V2 Message Type Registry
//!
//! ## Purpose
//!
//! Comprehensive type registry and introspection system for Protocol V2 TLV messages.
//! Provides domain-based organization (1-19 MarketData, 20-39 Signal, 40-59 Execution, 100-119 System)
//! with automatic routing, size validation, and rich developer API for discovery and documentation
//! generation. The type system enforces protocol integrity while enabling rapid development
//! through runtime introspection and comprehensive metadata.
//!
//! ## Integration Points
//!
//! - **Message Construction**: TLVMessageBuilder uses type metadata for format selection
//! - **Parsing Validation**: Parser validates payload sizes against type constraints
//! - **Relay Routing**: Automatic domain-based routing to appropriate relay services
//! - **Documentation**: Auto-generation of API references and message type tables
//! - **Development Tools**: IDE integration through rich type introspection API
//! - **Service Discovery**: Runtime enumeration of available message types
//!
//! ## Architecture Role
//!
//! ```text
//! Developer Tools → [TLV Type Registry] → Protocol Implementation
//!       ↑                ↓                        ↓
//!   IDE Help        Type Metadata           Message Routing
//!   Code Gen        Size Validation         Service Discovery
//!   Docs Gen        Domain Mapping          Format Selection
//! ```
//!
//! The type registry serves as the central source of truth for all Protocol V2 message
//! types, enabling both compile-time safety and runtime discoverability.

use num_enum::TryFromPrimitive;

// Import TLVType and related types from the canonical location in libs/types
use torq_types::protocol::tlv::types::{TLVSizeConstraint as TypeSizeConstraint, TLVTypeInfo, TLVImplementationStatus};
use torq_types::RelayDomain;

// Re-export for backward compatibility  
pub use torq_types::protocol::tlv::types::TLVType;

// TLVSizeConstraint is now imported from libs/types
// Re-export for backward compatibility
pub use torq_types::protocol::tlv::types::TLVSizeConstraint;

// TLVType enum is now imported from libs/types above
// The complete Protocol V2 type registry is maintained in libs/types/src/protocol/tlv/types.rs

// TLVType methods are already implemented in the canonical type from libs/types
// All methods like name(), size_constraint(), is_implemented(), expected_payload_size()
// and all_implemented() are available from the imported type

/// Registry for TLV type metadata and introspection
pub struct TlvTypeRegistry;

impl TlvTypeRegistry {
    /// Get all available TLV types
    pub fn all_types() -> Vec<TLVType> {
        TLVType::all_implemented()
    }

    /// Validate payload size for given TLV type
    pub fn validate_size(tlv_type: TLVType, payload_size: usize) -> bool {
        match tlv_type.size_constraint() {
            TLVSizeConstraint::Fixed(expected) => payload_size == expected,
            TLVSizeConstraint::Bounded { min, max } => payload_size >= min && payload_size <= max,
            TLVSizeConstraint::Variable => true, // Accept any size
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tlv_type_basic_functionality() {
        let trade_type = TLVType::Trade;
        assert_eq!(trade_type.name(), "Trade");
        assert_eq!(trade_type as u8, 1);
        assert!(trade_type.is_implemented());

        match trade_type.size_constraint() {
            TLVSizeConstraint::Fixed(40) => (), // Expected
            _ => panic!("Trade should be fixed 40 bytes"),
        }
    }

    #[test]
    fn test_size_validation() {
        // Fixed size validation
        assert!(TlvTypeRegistry::validate_size(TLVType::Trade, 40));
        assert!(!TlvTypeRegistry::validate_size(TLVType::Trade, 39));
        assert!(!TlvTypeRegistry::validate_size(TLVType::Trade, 41));

        // Variable size validation (always passes)
        assert!(TlvTypeRegistry::validate_size(TLVType::OrderBook, 100));
        assert!(TlvTypeRegistry::validate_size(TLVType::OrderBook, 1000));
        assert!(TlvTypeRegistry::validate_size(TLVType::OrderBook, 10));
    }

    #[test]
    fn test_try_from_primitive() {
        // Test conversion from u8 to TLVType
        assert_eq!(TLVType::try_from(1u8).unwrap(), TLVType::Trade);
        assert_eq!(TLVType::try_from(2u8).unwrap(), TLVType::Quote);
        assert_eq!(TLVType::try_from(100u8).unwrap(), TLVType::Heartbeat);

        // Test invalid type number
        assert!(TLVType::try_from(99u8).is_err());
    }
}
