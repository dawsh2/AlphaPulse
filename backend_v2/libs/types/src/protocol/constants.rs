//! Protocol constants and basic types
//!
//! These are fundamental data types that should remain in types crate
//! to avoid circular dependencies. Protocol logic remains in codec.

use thiserror::Error;
use std::convert::TryFrom;
use serde::{Deserialize, Serialize};

/// Protocol magic number for message validation
pub const MESSAGE_MAGIC: u32 = 0xDEADBEEF;

/// Current protocol version
pub const PROTOCOL_VERSION: u8 = 1;

/// Unix socket paths for relay connections
pub const MARKET_DATA_RELAY_PATH: &str = "/tmp/torq_market_data_relay.sock";
pub const SIGNAL_RELAY_PATH: &str = "/tmp/torq_signal_relay.sock";
pub const EXECUTION_RELAY_PATH: &str = "/tmp/torq_execution_relay.sock";

/// Relay domain enumeration for message routing
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RelayDomain {
    MarketData = 0,
    Signal = 1,
    Execution = 2,
    System = 3,
}

impl TryFrom<u8> for RelayDomain {
    type Error = ProtocolError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(RelayDomain::MarketData),
            1 => Ok(RelayDomain::Signal),
            2 => Ok(RelayDomain::Execution),
            3 => Ok(RelayDomain::System),
            _ => Err(ProtocolError::InvalidRelayDomain(value)),
        }
    }
}

impl From<RelayDomain> for u8 {
    fn from(domain: RelayDomain) -> Self {
        domain as u8
    }
}

/// Source type enumeration for message attribution
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SourceType {
    Unknown = 0,
    BinanceCollector = 1,
    CoinbaseCollector = 2,
    KrakenCollector = 3,
    PolygonCollector = 4,
    GeminiCollector = 5,
    ArbitrageStrategy = 10,
    ExecutionEngine = 20,
    Dashboard = 30,
    TestClient = 99,
}

impl TryFrom<u8> for SourceType {
    type Error = ProtocolError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(SourceType::Unknown),
            1 => Ok(SourceType::BinanceCollector),
            2 => Ok(SourceType::CoinbaseCollector),
            3 => Ok(SourceType::KrakenCollector),
            4 => Ok(SourceType::PolygonCollector),
            5 => Ok(SourceType::GeminiCollector),
            10 => Ok(SourceType::ArbitrageStrategy),
            20 => Ok(SourceType::ExecutionEngine),
            30 => Ok(SourceType::Dashboard),
            99 => Ok(SourceType::TestClient),
            _ => Err(ProtocolError::InvalidSourceType(value)),
        }
    }
}

impl From<SourceType> for u8 {
    fn from(source: SourceType) -> Self {
        source as u8
    }
}

/// Protocol errors
#[derive(Error, Debug)]
pub enum ProtocolError {
    #[error("Invalid magic number: expected {expected:#x}, got {actual:#x}")]
    InvalidMagic { expected: u32, actual: u32 },
    
    #[error("Message too small: need {need} bytes, got {got} bytes")]
    MessageTooSmall { need: usize, got: usize },
    
    #[error("Checksum mismatch")]
    ChecksumMismatch,
    
    #[error("Invalid TLV type: {0}")]
    InvalidTLVType(u8),
    
    #[error("TLV payload too large: {size} bytes exceeds maximum")]
    PayloadTooLarge { size: usize },
    
    #[error("Parse error: {0}")]
    ParseError(String),
    
    #[error("Invalid instrument: {0}")]
    InvalidInstrument(String),
    
    #[error("Invalid relay domain: {0}")]
    InvalidRelayDomain(u8),
    
    #[error("Invalid source type: {0}")]
    InvalidSourceType(u8),
    
    #[error("Parse error")]
    Parse,
}

impl ProtocolError {
    pub fn message_too_small(need: usize, got: usize, context: &str) -> Self {
        Self::ParseError(format!(
            "Message too small for {}: need {} bytes, got {} bytes",
            context, need, got
        ))
    }
}