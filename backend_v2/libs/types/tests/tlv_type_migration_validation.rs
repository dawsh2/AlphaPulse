//! TLV Type Migration Validation Tests
//!
//! Validates that the TLV type consolidation maintains backward compatibility
//! by ensuring all previously used type numbers still map to the same semantic types.

use codec::protocol::tlv::types::TLVType;
use codec::protocol::RelayDomain;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_core_market_data_types_preserved() {
        // Verify core market data types (1-19) maintain their mappings
        assert_eq!(TLVType::Trade as u8, 1);
        assert_eq!(TLVType::Quote as u8, 2);
        assert_eq!(TLVType::OrderBook as u8, 3);
        assert_eq!(TLVType::GasPrice as u8, 18);

        // Verify they route to correct domain
        assert_eq!(TLVType::Trade.relay_domain(), RelayDomain::MarketData);
        assert_eq!(TLVType::Quote.relay_domain(), RelayDomain::MarketData);
        assert_eq!(TLVType::OrderBook.relay_domain(), RelayDomain::MarketData);
        assert_eq!(TLVType::GasPrice.relay_domain(), RelayDomain::MarketData);
    }

    #[test]
    fn test_core_signal_types_preserved() {
        // Verify core signal types (20-39) maintain their mappings
        assert_eq!(TLVType::SignalIdentity as u8, 20);
        assert_eq!(TLVType::Economics as u8, 22);
        assert_eq!(TLVType::ArbitrageSignal as u8, 32);

        // Verify they route to correct domain
        assert_eq!(TLVType::SignalIdentity.relay_domain(), RelayDomain::Signal);
        assert_eq!(TLVType::Economics.relay_domain(), RelayDomain::Signal);
        assert_eq!(TLVType::ArbitrageSignal.relay_domain(), RelayDomain::Signal);
    }

    #[test]
    fn test_core_execution_types_preserved() {
        // Verify core execution types (40-59) maintain their mappings
        assert_eq!(TLVType::OrderRequest as u8, 40);
        assert_eq!(TLVType::Fill as u8, 42);
        assert_eq!(TLVType::ExecutionReport as u8, 45);

        // Verify they route to correct domain
        assert_eq!(TLVType::OrderRequest.relay_domain(), RelayDomain::Execution);
        assert_eq!(TLVType::Fill.relay_domain(), RelayDomain::Execution);
        assert_eq!(TLVType::ExecutionReport.relay_domain(), RelayDomain::Execution);
    }

    #[test]
    fn test_system_types_preserved() {
        // Verify core system types (100-119) maintain their mappings
        assert_eq!(TLVType::Heartbeat as u8, 100);

        // Verify they route to correct domain
        assert_eq!(TLVType::Heartbeat.relay_domain(), RelayDomain::System);
    }

    #[test]
    fn test_expected_payload_sizes_preserved() {
        // Verify that expected payload sizes match Protocol V2 specifications
        
        // Fixed-size types - these must be exact for backward compatibility
        assert_eq!(TLVType::Trade.expected_payload_size(), Some(40));
        assert_eq!(TLVType::Quote.expected_payload_size(), Some(52));
        assert_eq!(TLVType::SignalIdentity.expected_payload_size(), Some(16));
        assert_eq!(TLVType::Economics.expected_payload_size(), Some(32));
        assert_eq!(TLVType::GasPrice.expected_payload_size(), Some(32));
        assert_eq!(TLVType::Heartbeat.expected_payload_size(), Some(16));

        // Variable size types should return None
        assert_eq!(TLVType::OrderBook.expected_payload_size(), None);
    }

    #[test]
    fn test_domain_ranges_intact() {
        // Test that domain ranges are preserved exactly as documented
        
        // Market Data domain: 1-19
        for type_num in 1..=19_u8 {
            if let Ok(tlv_type) = TLVType::try_from(type_num) {
                assert_eq!(
                    tlv_type.relay_domain(),
                    RelayDomain::MarketData,
                    "Type {} should be in MarketData domain",
                    type_num
                );
            }
        }

        // Signal domain: 20-39
        for type_num in 20..=39_u8 {
            if let Ok(tlv_type) = TLVType::try_from(type_num) {
                assert_eq!(
                    tlv_type.relay_domain(),
                    RelayDomain::Signal,
                    "Type {} should be in Signal domain",
                    type_num
                );
            }
        }

        // Execution domain: 40-59
        for type_num in 40..=59_u8 {
            if let Ok(tlv_type) = TLVType::try_from(type_num) {
                assert_eq!(
                    tlv_type.relay_domain(),
                    RelayDomain::Execution,
                    "Type {} should be in Execution domain",
                    type_num
                );
            }
        }

        // System domain: 100-119
        for type_num in 100..=119_u8 {
            if let Ok(tlv_type) = TLVType::try_from(type_num) {
                assert_eq!(
                    tlv_type.relay_domain(),
                    RelayDomain::System,
                    "Type {} should be in System domain",
                    type_num
                );
            }
        }
    }

    #[test]
    fn test_no_duplicate_type_numbers() {
        // Collect all implemented TLV types and their numbers
        let all_types = TLVType::all_implemented();
        let mut type_numbers = std::collections::HashSet::new();

        for tlv_type in all_types {
            let type_num = tlv_type as u8;
            assert!(
                type_numbers.insert(type_num),
                "Duplicate TLV type number {} found for {:?}",
                type_num,
                tlv_type
            );
        }
    }

    #[test]
    fn test_extended_tlv_marker() {
        // Verify ExtendedTLV marker is preserved at 255
        assert_eq!(TLVType::ExtendedTLV as u8, 255);
    }

    #[test]
    fn test_tlv_type_introspection_available() {
        // Verify that introspection methods are available from consolidated type
        let trade_info = TLVType::Trade.type_info();
        assert_eq!(trade_info.type_number, 1);
        assert_eq!(trade_info.name, "Trade");
        assert!(trade_info.description.len() > 0);
        assert_eq!(trade_info.relay_domain, RelayDomain::MarketData);
    }

    /// Regression test: Verify specific types that were previously in message_sink
    /// These types may not be in the canonical enum but should be handled gracefully
    #[test]
    fn test_legacy_message_sink_type_compatibility() {
        // These were the types defined in the old message_sink TLV enum
        // Some may not exist in canonical enum - that's expected
        
        // Types that should exist
        assert!(TLVType::try_from(1_u8).is_ok()); // Trade
        assert!(TLVType::try_from(2_u8).is_ok()); // Quote  
        assert!(TLVType::try_from(3_u8).is_ok()); // OrderBook
        assert!(TLVType::try_from(20_u8).is_ok()); // SignalIdentity

        // Types that were in message_sink but might not be in canonical enum
        // We just verify they fail gracefully, not crash
        let _result = TLVType::try_from(4_u8); // OHLC - may or may not exist
        let _result = TLVType::try_from(5_u8); // Volume - may or may not exist
    }
}