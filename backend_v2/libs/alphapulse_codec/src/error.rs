//! Protocol-level errors for TLV message processing
//!
//! Provides comprehensive error handling for the AlphaPulse protocol codec,
//! including detailed context for debugging and monitoring. Each error variant
//! includes specific information about what went wrong and what was expected.

use thiserror::Error;

/// TLV parsing errors with detailed context
///
/// Provides comprehensive error information for debugging and monitoring.
/// Each error variant includes specific context about what went wrong and
/// what was expected, enabling precise error handling and diagnostics.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum ProtocolError {
    #[error("Message too small: need {need} bytes, got {got}")]
    MessageTooSmall { need: usize, got: usize },

    #[error("Invalid magic number: expected {expected:#x}, got {actual:#x}")]
    InvalidMagic { expected: u32, actual: u32 },

    #[error("Checksum mismatch: expected {expected:#x}, calculated {calculated:#x}")]
    ChecksumMismatch { expected: u32, calculated: u32 },

    #[error("Truncated TLV at offset {offset}")]
    TruncatedTLV { offset: usize },

    #[error("Unknown TLV type: {0}")]
    UnknownTLVType(u8),

    #[error("Unknown source type: {0}")]
    UnknownSource(u8),

    #[error("Invalid extended TLV format")]
    InvalidExtendedTLV,

    #[error("TLV payload too large: {size} bytes")]
    PayloadTooLarge { size: usize },

    #[error("Message too large: {size} bytes exceeds maximum {max}")]
    MessageTooLarge { size: usize, max: usize },

    #[error("TLV payload size mismatch: expected {expected}, got {got}")]
    PayloadSizeMismatch { expected: usize, got: usize },

    #[error("Invalid TLV payload")]
    InvalidPayload,

    #[error("Unsupported TLV version: {version}")]
    UnsupportedVersion { version: u8 },

    #[error("Relay domain mismatch: expected {expected}, got {got}")]
    RelayDomainMismatch { expected: u8, got: u8 },
}

/// Legacy alias for ParseError - maintains compatibility with existing code
pub type ParseError = ProtocolError;

/// Result type for protocol operations
pub type ProtocolResult<T> = std::result::Result<T, ProtocolError>;

/// Legacy alias for ParseResult - maintains compatibility with existing code
pub type ParseResult<T> = ProtocolResult<T>;
