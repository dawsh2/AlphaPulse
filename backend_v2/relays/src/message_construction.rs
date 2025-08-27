//! # Message Construction Module - Protocol V2 TLV Message Building
//!
//! ## Purpose
//! Provides relay-specific message construction utilities leveraging codec's
//! TLVMessageBuilder for proper Protocol V2 compliance. Ensures all messages constructed
//! by relays follow correct format with proper headers, checksums, and TLV encoding.

use crate::{RelayError, RelayResult};
use codec::{InstrumentId, TLVMessageBuilder, TLVType};
use alphapulse_types::protocol::{MessageHeader, RelayDomain, SourceType};
use bytes::Bytes;
use tracing::debug;
use zerocopy::{AsBytes, FromBytes};

/// Relay message builder wrapper for Protocol V2 compliance
pub struct RelayMessageBuilder {
    builder: TLVMessageBuilder,
    relay_domain: RelayDomain,
}

impl RelayMessageBuilder {
    /// Create a new message builder for a specific relay domain
    pub fn new(relay_domain: RelayDomain, source: SourceType) -> Self {
        let builder = TLVMessageBuilder::new(relay_domain as u8, source as u8);
        Self {
            builder,
            relay_domain,
        }
    }

    /// Add a TLV extension to the message
    pub fn add_tlv(&mut self, tlv_type: TLVType, data: &[u8]) -> RelayResult<()> {
        // Validate TLV type is appropriate for this relay domain
        if !Self::validate_tlv_for_domain(self.relay_domain, tlv_type) {
            return Err(RelayError::Validation(format!(
                "TLV type {:?} not valid for domain {:?}",
                tlv_type, self.relay_domain
            )));
        }

        self.builder.add_tlv(tlv_type, data);
        Ok(())
    }

    /// Add a trade TLV with instrument ID
    pub fn add_trade_tlv(
        &mut self,
        instrument_id: &InstrumentId,
        price: i64,
        quantity: i64,
        timestamp: u64,
    ) -> RelayResult<()> {
        // Ensure we're in the market data domain
        if self.relay_domain != RelayDomain::MarketData {
            return Err(RelayError::Validation(
                "Trade TLVs only valid in MarketData domain".to_string(),
            ));
        }

        // Construct trade TLV data (simplified - real implementation would use TradeTLV struct)
        let mut data = Vec::with_capacity(48);
        data.extend_from_slice(instrument_id.as_bytes());
        data.extend_from_slice(&price.to_le_bytes());
        data.extend_from_slice(&quantity.to_le_bytes());
        data.extend_from_slice(&timestamp.to_le_bytes());

        self.add_tlv(TLVType::Trade, &data)
    }

    /// Add a signal TLV
    pub fn add_signal_tlv(
        &mut self,
        signal_id: u64,
        signal_type: u8,
        confidence: f32,
        data: &[u8],
    ) -> RelayResult<()> {
        // Ensure we're in the signal domain
        if self.relay_domain != RelayDomain::Signal {
            return Err(RelayError::Validation(
                "Signal TLVs only valid in Signal domain".to_string(),
            ));
        }

        // Construct signal TLV data
        let mut tlv_data = Vec::with_capacity(13 + data.len());
        tlv_data.extend_from_slice(&signal_id.to_le_bytes());
        tlv_data.push(signal_type);
        tlv_data.extend_from_slice(&confidence.to_le_bytes());
        tlv_data.extend_from_slice(data);

        self.add_tlv(TLVType::SignalIdentity, &tlv_data)
    }

    /// Add an order status TLV
    pub fn add_order_status_tlv(
        &mut self,
        order_id: u64,
        status: u8,
        filled_quantity: i64,
        remaining_quantity: i64,
    ) -> RelayResult<()> {
        // Ensure we're in the execution domain
        if self.relay_domain != RelayDomain::Execution {
            return Err(RelayError::Validation(
                "OrderStatus TLVs only valid in Execution domain".to_string(),
            ));
        }

        // Construct order status TLV data
        let mut data = Vec::with_capacity(25);
        data.extend_from_slice(&order_id.to_le_bytes());
        data.push(status);
        data.extend_from_slice(&filled_quantity.to_le_bytes());
        data.extend_from_slice(&remaining_quantity.to_le_bytes());

        self.add_tlv(TLVType::OrderStatus, &data)
    }

    /// Build the complete message with proper header and checksum
    pub fn build(self) -> Bytes {
        let message = self.builder.build();
        debug!(
            "Built Protocol V2 message for domain {:?}: {} bytes",
            self.relay_domain,
            message.len()
        );
        Bytes::from(message)
    }

    /// Validate that a TLV type is appropriate for a given relay domain
    fn validate_tlv_for_domain(domain: RelayDomain, tlv_type: TLVType) -> bool {
        let type_num = tlv_type as u8;
        match domain {
            RelayDomain::MarketData => (1..=19).contains(&type_num),
            RelayDomain::Signal => (20..=39).contains(&type_num),
            RelayDomain::Execution => (40..=79).contains(&type_num),
            RelayDomain::System => (100..=119).contains(&type_num),
            _ => false, // Unknown domains
        }
    }
}

/// Factory functions for common message types
pub mod factory {
    use super::*;

    /// Create a heartbeat message for relay health monitoring
    pub fn create_heartbeat(domain: RelayDomain, source: SourceType) -> Bytes {
        let mut builder = RelayMessageBuilder::new(domain, source);

        // Add heartbeat TLV
        let heartbeat_data = b"HB";
        builder
            .add_tlv(TLVType::Heartbeat, heartbeat_data)
            .expect("Failed to add heartbeat TLV");

        builder.build()
    }

    /// Create a relay status message
    pub fn create_status_message(
        domain: RelayDomain,
        source: SourceType,
        connected_clients: u32,
        messages_processed: u64,
        uptime_seconds: u64,
    ) -> Bytes {
        let mut builder = RelayMessageBuilder::new(domain, source);

        // Create status data
        let mut status_data = Vec::with_capacity(16);
        status_data.extend_from_slice(&connected_clients.to_le_bytes());
        status_data.extend_from_slice(&messages_processed.to_le_bytes());
        status_data.extend_from_slice(&uptime_seconds.to_le_bytes());

        builder
            .add_tlv(TLVType::SystemHealth, &status_data)
            .expect("Failed to add status TLV");

        builder.build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use codec::parse_header;

    #[test]
    fn test_relay_message_builder() {
        let mut builder = RelayMessageBuilder::new(RelayDomain::MarketData, SourceType::Kraken);

        // Create a simple instrument ID
        let instrument_id =
            InstrumentId::coin(codec::VenueId::Kraken, "BTC-USD").unwrap();

        // Add a trade TLV
        builder
            .add_trade_tlv(
                &instrument_id,
                4500000000000, // $45,000.00
                100000000,     // 1 BTC
                1234567890000000000,
            )
            .unwrap();

        let message = builder.build();

        // Verify message can be parsed
        let header = parse_header(&message).unwrap();
        assert_eq!(header.relay_domain, RelayDomain::MarketData as u8);
        assert_eq!(header.source, SourceType::Kraken as u8);
    }

    #[test]
    fn test_domain_validation() {
        let mut builder = RelayMessageBuilder::new(RelayDomain::MarketData, SourceType::Kraken);

        // Try to add a signal TLV to market data domain - should fail
        let result = builder.add_signal_tlv(123, 1, 0.95, b"test");

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Signal TLVs only valid in Signal domain"));
    }

    #[test]
    fn test_heartbeat_creation() {
        let heartbeat = factory::create_heartbeat(RelayDomain::MarketData, SourceType::Kraken);

        // Parse and verify
        let header = parse_header(&heartbeat).unwrap();
        assert_eq!(header.relay_domain, RelayDomain::MarketData as u8);
        assert!(heartbeat.len() > 32); // Header + TLV data
    }
}
