//! Relay domains for message routing in Torq system

use num_enum::TryFromPrimitive;

/// Relay domains for message routing
/// 
/// Messages are routed to domain-specific relays based on their TLV type
/// and processing requirements. Each domain has its own performance
/// characteristics and validation policies.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TryFromPrimitive)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum RelayDomain {
    /// Market data domain (TLV types 1-19)
    /// High-frequency price updates and order book data
    MarketData = 1,
    
    /// Signal domain (TLV types 20-39, 60-79)
    /// Trading signals and analytics messages
    Signal = 2,
    
    /// Execution domain (TLV types 40-59)
    /// Order execution and trade confirmations
    Execution = 3,
    
    /// System domain (TLV types 80-99)
    /// Infrastructure and monitoring messages
    System = 4,
}

impl RelayDomain {
    /// Get the Unix domain socket path for this relay
    pub fn socket_path(&self) -> &'static str {
        match self {
            RelayDomain::MarketData => crate::MARKET_DATA_RELAY_PATH,
            RelayDomain::Signal => crate::SIGNAL_RELAY_PATH,
            RelayDomain::Execution => crate::EXECUTION_RELAY_PATH,
            RelayDomain::System => "/tmp/torq/system.sock", // TODO: Add to constants
        }
    }
    
    /// Determine relay domain from TLV type number
    pub fn from_tlv_type(tlv_type: u8) -> Option<Self> {
        match tlv_type {
            1..=19 => Some(RelayDomain::MarketData),
            20..=39 | 60..=79 => Some(RelayDomain::Signal),
            40..=59 => Some(RelayDomain::Execution),
            80..=99 => Some(RelayDomain::System),
            _ => None,
        }
    }
}