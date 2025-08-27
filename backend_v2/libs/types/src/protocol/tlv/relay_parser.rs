//! Optimized TLV Parser for Relay Processing
//!
//! Provides performance-optimized TLV parsing for relay servers that:
//! - Skips strict payload size validation (for performance)
//! - Focuses on TLV type range validation (domain routing)
//! - Minimizes allocation and validation overhead

use super::{ParseError, ParseResult};

/// Fast TLV parsing for relay processing - skips strict size validation
pub fn parse_tlv_extensions_for_relay(tlv_data: &[u8]) -> ParseResult<Vec<RelayTLV>> {
    let mut tlvs = Vec::new();
    let mut offset = 0;

    while offset < tlv_data.len() {
        if offset + 2 > tlv_data.len() {
            return Err(ParseError::TruncatedTLV { offset });
        }

        let tlv_type = tlv_data[offset];
        let tlv_length = tlv_data[offset + 1] as usize;

        if offset + 2 + tlv_length > tlv_data.len() {
            return Err(ParseError::TruncatedTLV { offset });
        }

        // Parse standard TLV without strict size validation
        let tlv = RelayTLV {
            tlv_type,
            tlv_length: tlv_length as u8,
            payload_offset: offset + 2,
        };

        tlvs.push(tlv);
        offset += 2 + tlv_length;
    }

    Ok(tlvs)
}

/// Lightweight TLV representation for relay processing
/// Only stores offsets instead of copying payload data
#[derive(Debug, Clone)]
pub struct RelayTLV {
    pub tlv_type: u8,
    pub tlv_length: u8,
    pub payload_offset: usize,
}

impl RelayTLV {
    /// Check if this TLV type is valid for the given domain
    pub fn is_valid_for_domain(&self, domain: crate::RelayDomain) -> bool {
        match domain {
            crate::RelayDomain::MarketData => (1..=19).contains(&self.tlv_type),
            crate::RelayDomain::Signal => {
                (20..=39).contains(&self.tlv_type) || (60..=79).contains(&self.tlv_type)
            }
            crate::RelayDomain::Execution => (40..=59).contains(&self.tlv_type),
            crate::RelayDomain::System => (80..=119).contains(&self.tlv_type),
        }
    }

    /// Get the relay domain this TLV should route to
    pub fn get_target_domain(&self) -> crate::RelayDomain {
        match self.tlv_type {
            1..=19 => crate::RelayDomain::MarketData, // Market data events
            20..=39 => crate::RelayDomain::Signal,    // Strategy signals
            40..=59 => crate::RelayDomain::Execution, // Order execution
            60..=79 => crate::RelayDomain::Signal,    // Portfolio/Risk → Signal (analytics)
            80..=119 => crate::RelayDomain::System,   // Compliance/System
            _ => crate::RelayDomain::MarketData,      // Default fallback
        }
    }

    /// Extract payload from the original TLV data buffer
    pub fn get_payload<'a>(&self, tlv_data: &'a [u8]) -> Option<&'a [u8]> {
        let start = self.payload_offset;
        let end = start + self.tlv_length as usize;

        if end <= tlv_data.len() {
            Some(&tlv_data[start..end])
        } else {
            None
        }
    }
}

/// Ultra-fast domain validation for market data relay
/// Only checks TLV type ranges without parsing full structure
pub fn validate_market_data_domain_fast(tlv_data: &[u8]) -> bool {
    let mut offset = 0;

    while offset + 2 <= tlv_data.len() {
        let tlv_type = tlv_data[offset];

        // Market data must be in range 1-19
        if !(1..=19).contains(&tlv_type) {
            return false;
        }

        let tlv_length = tlv_data[offset + 1] as usize;
        offset += 2 + tlv_length;

        // Bounds check
        if offset > tlv_data.len() {
            return false;
        }
    }

    true
}

/// Ultra-fast domain validation for signal relay
pub fn validate_signal_domain_fast(tlv_data: &[u8]) -> bool {
    let mut offset = 0;

    while offset + 2 <= tlv_data.len() {
        let tlv_type = tlv_data[offset];

        // Signal data must be in range 20-39
        if !(20..=39).contains(&tlv_type) {
            return false;
        }

        let tlv_length = tlv_data[offset + 1] as usize;
        offset += 2 + tlv_length;

        if offset > tlv_data.len() {
            return false;
        }
    }

    true
}

/// Ultra-fast domain validation for execution relay
pub fn validate_execution_domain_fast(tlv_data: &[u8]) -> bool {
    let mut offset = 0;

    while offset + 2 <= tlv_data.len() {
        let tlv_type = tlv_data[offset];

        // Execution data must be in range 40-59
        if !(40..=59).contains(&tlv_type) {
            return false;
        }

        let tlv_length = tlv_data[offset + 1] as usize;
        offset += 2 + tlv_length;

        if offset > tlv_data.len() {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    // TLVMessageBuilder moved to codec to avoid circular dependency
    use crate::{RelayDomain, SourceType, TLVType};

    #[test]
    fn test_relay_tlv_parsing() {
        // Create a TLV payload manually (without codec dependency)
        let mut tlv_payload = Vec::new();

        // TLV header: Type=1 (Trade), Length=100
        tlv_payload.push(1u8); // TLV Type: Trade
        tlv_payload.push(100u8); // TLV Length: 100 bytes

        // Add 100 bytes of payload
        tlv_payload.extend_from_slice(&vec![0u8; 100]);

        // This should work with relay parser (no strict size validation)
        let tlvs = parse_tlv_extensions_for_relay(&tlv_payload).unwrap();
        assert_eq!(tlvs.len(), 1);
        assert_eq!(tlvs[0].tlv_type, TLVType::Trade as u8);
        assert_eq!(tlvs[0].tlv_length, 100);
    }

    #[test]
    fn test_domain_validation() {
        // Test market data validation
        let market_data = vec![1, 4, 0x01, 0x02, 0x03, 0x04]; // Type 1, Length 4
        assert!(validate_market_data_domain_fast(&market_data));

        // Test signal data should fail market data validation
        let signal_data = vec![20, 4, 0x01, 0x02, 0x03, 0x04]; // Type 20, Length 4
        assert!(!validate_market_data_domain_fast(&signal_data));
        assert!(validate_signal_domain_fast(&signal_data));

        // Test execution data
        let execution_data = vec![40, 4, 0x01, 0x02, 0x03, 0x04]; // Type 40, Length 4
        assert!(validate_execution_domain_fast(&execution_data));
    }

    #[test]
    fn test_relay_tlv_domain_routing() {
        let tlv_data = vec![1, 4, 0x01, 0x02, 0x03, 0x04]; // Market data TLV
        let tlvs = parse_tlv_extensions_for_relay(&tlv_data).unwrap();

        assert_eq!(tlvs[0].get_target_domain(), crate::RelayDomain::MarketData);
        assert!(tlvs[0].is_valid_for_domain(crate::RelayDomain::MarketData));
        assert!(!tlvs[0].is_valid_for_domain(crate::RelayDomain::Signal));
    }
}
