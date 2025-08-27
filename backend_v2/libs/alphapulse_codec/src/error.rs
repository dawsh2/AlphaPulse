//! Protocol-level errors for TLV message processing
//!
//! Provides comprehensive error handling for the AlphaPulse protocol codec,
//! including detailed context for debugging and monitoring. Each error variant
//! includes specific information about what went wrong and what was expected.

use thiserror::Error;

/// TLV parsing errors with comprehensive diagnostic context
///
/// Enhanced error reporting with actionable debugging information.
/// Each error variant includes specific context about what went wrong,
/// buffer state, and actionable troubleshooting guidance.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum ProtocolError {
    /// Message buffer is too small to contain expected data structure
    #[error("Message too small: need {need} bytes, got {got} (context: {context})")]
    MessageTooSmall { 
        need: usize, 
        got: usize,
        context: String,
    },

    /// Protocol magic number validation failed
    #[error("Invalid magic number: expected {expected:#010x}, got {actual:#010x} (offset: {offset}, indicates: {diagnosis})")]
    InvalidMagic { 
        expected: u32, 
        actual: u32,
        offset: usize,
        diagnosis: String,
    },

    /// Message checksum validation failed - indicates data corruption
    #[error("Checksum mismatch: expected {expected:#010x}, calculated {calculated:#010x} (message: {message_size} bytes, tlvs: {tlv_count}, cause: {likely_cause})")]
    ChecksumMismatch { 
        expected: u32, 
        calculated: u32,
        message_size: usize,
        tlv_count: usize,
        likely_cause: String,
    },

    /// TLV data is truncated - insufficient buffer for declared length
    #[error("Truncated TLV: need {required_bytes} bytes, buffer has {buffer_size} (TLV type {tlv_type} at offset {offset}, action: {suggested_action})")]
    TruncatedTLV { 
        buffer_size: usize,
        required_bytes: usize,
        tlv_type: u16,
        offset: usize,
        suggested_action: String,
    },

    /// TLV type number is not recognized in current protocol version
    #[error("Unknown TLV type {tlv_type}: valid ranges are 1-19 (MarketData), 20-39 (Signals), 40-79 (Execution), 80-99 (System)")]
    UnknownTLVType { tlv_type: u8 },

    /// Message source identifier is not recognized
    #[error("Unknown source type {source_type}: registered sources are available in relay configuration")]
    UnknownSource { source_type: u8 },

    /// Extended TLV format is malformed
    #[error("Invalid extended TLV format at offset {offset}: expected marker {expected_marker:#04x}, got {actual_marker:#04x} (check: {validation_hint})")]
    InvalidExtendedTLV {
        offset: usize,
        expected_marker: u16,
        actual_marker: u16,
        validation_hint: String,
    },

    /// TLV payload exceeds protocol limits
    #[error("TLV payload too large: {size} bytes exceeds limit {limit} (type {tlv_type}, consider: {recommendation})")]
    PayloadTooLarge { 
        size: usize,
        limit: usize,
        tlv_type: u8,
        recommendation: String,
    },

    /// Complete message exceeds protocol limits
    #[error("Message too large: {size} bytes exceeds maximum {max} (payload: {payload_size}, tlvs: {tlv_count})")]
    MessageTooLarge { 
        size: usize, 
        max: usize,
        payload_size: usize,
        tlv_count: usize,
    },

    /// TLV payload size doesn't match expected size for type
    #[error("TLV payload size mismatch for type {tlv_type}: expected {expected} bytes, got {got} (struct: {struct_name})")]
    PayloadSizeMismatch { 
        tlv_type: u8, 
        expected: usize, 
        got: usize,
        struct_name: String,
    },

    /// TLV payload contains invalid data for the declared type
    #[error("Invalid TLV payload for type {tlv_type} at offset {offset}: {validation_error} (buffer: {buffer_size} bytes)")]
    InvalidPayload {
        tlv_type: u8,
        offset: usize,
        validation_error: String,
        buffer_size: usize,
    },

    /// Protocol version is not supported by this parser
    #[error("Unsupported TLV version {version}: supported versions are {supported_versions}")]
    UnsupportedVersion { 
        version: u8,
        supported_versions: String,
    },

    /// Message was sent to wrong relay domain
    #[error("Relay domain mismatch: expected {expected} ({expected_name}), got {got} ({got_name}) - route to correct relay")]
    RelayDomainMismatch { 
        expected: u8, 
        got: u8,
        expected_name: String,
        got_name: String,
    },

    /// General parsing error with contextual information
    #[error("Parse error at byte {offset}: {description} (buffer: {buffer_size} bytes, context: {context})")]
    ParseError {
        offset: usize,
        description: String,
        buffer_size: usize,
        context: String,
    },

    /// Invalid instrument identifier
    #[error("Invalid instrument: {0}")]
    InvalidInstrument(String),

    /// Recovery operation failed
    #[error("Recovery failed: {0}")]
    Recovery(String),
}

impl ProtocolError {
    /// Create enhanced MessageTooSmall error with diagnostic context
    pub fn message_too_small(need: usize, got: usize, context: impl Into<String>) -> Self {
        Self::MessageTooSmall {
            need,
            got,
            context: context.into(),
        }
    }

    /// Create enhanced InvalidMagic error with diagnostic context
    pub fn invalid_magic(expected: u32, actual: u32, offset: usize) -> Self {
        let diagnosis = match actual {
            0x00000000 => "uninitialized buffer",
            0xFFFFFFFF => "corrupted buffer or wrong endianness",
            _ if actual.swap_bytes() == expected => "byte order (endianness) mismatch",
            _ => "data corruption or wrong protocol version",
        };
        
        Self::InvalidMagic {
            expected,
            actual,
            offset,
            diagnosis: diagnosis.to_string(),
        }
    }

    /// Create enhanced ChecksumMismatch error with diagnostic context
    pub fn checksum_mismatch(
        expected: u32, 
        calculated: u32, 
        message_size: usize, 
        tlv_count: usize
    ) -> Self {
        let likely_cause = if expected == 0 {
            "message created without checksum calculation"
        } else if calculated == 0 {
            "checksum calculation failed or disabled"
        } else {
            "data corruption during transmission"
        };
        
        Self::ChecksumMismatch {
            expected,
            calculated,
            message_size,
            tlv_count,
            likely_cause: likely_cause.to_string(),
        }
    }

    /// Create enhanced TruncatedTLV error with diagnostic context
    pub fn truncated_tlv(
        buffer_size: usize,
        required_bytes: usize,
        tlv_type: u16,
        offset: usize,
    ) -> Self {
        let suggested_action = if buffer_size == 0 {
            "check message framing and socket reads"
        } else if required_bytes > buffer_size * 2 {
            "likely corrupted TLV length field"
        } else {
            "incomplete message transmission - retry or increase buffer"
        };
        
        Self::TruncatedTLV {
            buffer_size,
            required_bytes,
            tlv_type,
            offset,
            suggested_action: suggested_action.to_string(),
        }
    }

    /// Create enhanced InvalidExtendedTLV error
    pub fn invalid_extended_tlv(offset: usize, expected: u16, actual: u16) -> Self {
        let hint = "extended TLV must start with 0xFF00 marker";
        
        Self::InvalidExtendedTLV {
            offset,
            expected_marker: expected,
            actual_marker: actual,
            validation_hint: hint.to_string(),
        }
    }

    /// Create enhanced PayloadTooLarge error
    pub fn payload_too_large(size: usize, limit: usize, tlv_type: u8) -> Self {
        let recommendation = if size > limit * 10 {
            "likely corrupted length field - validate TLV structure"
        } else {
            "consider message fragmentation or protocol upgrade"
        };
        
        Self::PayloadTooLarge {
            size,
            limit,
            tlv_type,
            recommendation: recommendation.to_string(),
        }
    }

    /// Create enhanced InvalidPayload error
    pub fn invalid_payload(
        tlv_type: u8,
        offset: usize,
        validation_error: impl Into<String>,
        buffer_size: usize,
    ) -> Self {
        Self::InvalidPayload {
            tlv_type,
            offset,
            validation_error: validation_error.into(),
            buffer_size,
        }
    }

    /// Create enhanced ParseError with contextual information
    pub fn parse_error(
        offset: usize,
        description: impl Into<String>,
        buffer_size: usize,
        context: impl Into<String>,
    ) -> Self {
        Self::ParseError {
            offset,
            description: description.into(),
            buffer_size,
            context: context.into(),
        }
    }
}

/// Legacy alias for ParseError - maintains compatibility with existing code
pub type ParseError = ProtocolError;

/// Result type for protocol operations
pub type ProtocolResult<T> = std::result::Result<T, ProtocolError>;

/// Legacy alias for ParseResult - maintains compatibility with existing code
pub type ParseResult<T> = ProtocolResult<T>;
