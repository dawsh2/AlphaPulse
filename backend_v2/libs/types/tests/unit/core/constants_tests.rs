//! Protocol Constants Unit Tests
//!
//! Tests for protocol-level constants and their consistency across the system.

use protocol_v2::{MESSAGE_MAGIC, PROTOCOL_VERSION, RelayDomain, SourceType};

#[test]
fn test_message_magic_value() {
    // Critical: Magic number must be 0xDEADBEEF for Protocol V2
    assert_eq!(MESSAGE_MAGIC, 0xDEADBEEF);
}

#[test]
fn test_protocol_version() {
    // Version 1 is the current stable Protocol V2
    assert_eq!(PROTOCOL_VERSION, 1);
}

#[test]
fn test_relay_domain_enum_values() {
    // Test that relay domains have expected numeric values for wire format
    use std::mem::discriminant;
    
    let market_data = RelayDomain::MarketData;
    let signal = RelayDomain::Signal;
    let execution = RelayDomain::Execution;
    
    // Each domain should have a unique discriminant
    assert_ne!(discriminant(&market_data), discriminant(&signal));
    assert_ne!(discriminant(&signal), discriminant(&execution));
    assert_ne!(discriminant(&market_data), discriminant(&execution));
}

#[test]
fn test_source_type_enum_values() {
    // Test major source types exist
    let sources = [
        SourceType::BinanceCollector,
        SourceType::KrakenCollector,
        SourceType::PolygonCollector,
        SourceType::FlashArbitrageStrategy,
        SourceType::Dashboard,
    ];
    
    // Each source should have unique discriminant
    for (i, source_a) in sources.iter().enumerate() {
        for source_b in sources.iter().skip(i + 1) {
            assert_ne!(
                std::mem::discriminant(source_a),
                std::mem::discriminant(source_b),
                "Source types {:?} and {:?} have same discriminant",
                source_a, source_b
            );
        }
    }
}

#[test]
fn test_relay_domain_serialization_size() {
    // RelayDomain must be 1 byte for header layout
    assert_eq!(std::mem::size_of::<RelayDomain>(), 1);
}

#[test]
fn test_source_type_serialization_size() {
    // SourceType must be 2 bytes for header layout
    assert_eq!(std::mem::size_of::<SourceType>(), 2);
}

#[test]
fn test_constants_are_const() {
    // Verify constants are compile-time constants (not computed)
    const _MAGIC_TEST: u32 = MESSAGE_MAGIC;
    const _VERSION_TEST: u8 = PROTOCOL_VERSION;
    
    // If these compile, the constants are truly const
    assert_eq!(_MAGIC_TEST, 0xDEADBEEF);
    assert_eq!(_VERSION_TEST, 1);
}