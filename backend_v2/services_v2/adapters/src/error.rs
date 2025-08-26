//! Error types for the adapters module

use alphapulse_types::VenueId;
use thiserror::Error;

/// Result type alias for adapter operations
pub type Result<T> = std::result::Result<T, AdapterError>;

/// Main error type for adapter operations
#[derive(Debug, Error)]
pub enum AdapterError {
    /// Connection-related errors
    #[error("Connection failed for venue {venue}: {reason}")]
    ConnectionFailed {
        /// The venue that failed to connect
        venue: VenueId,
        /// Reason for the failure
        reason: String,
    },

    /// Connection timeout during establish or receive operations
    #[error("Connection timeout for venue {venue} after {timeout_ms}ms")]
    ConnectionTimeout {
        /// The venue that timed out
        venue: VenueId,
        /// Timeout duration in milliseconds
        timeout_ms: u64,
    },

    /// Authentication failure with exchange credentials
    #[error("Authentication failed for venue {venue}")]
    AuthenticationFailed {
        /// The venue where auth failed
        venue: VenueId,
    },

    /// Rate limit exceeded on exchange API
    #[error("Rate limit exceeded for venue {venue}")]
    RateLimitExceeded {
        /// The venue that rate limited us
        venue: VenueId,
    },

    /// Message processing errors
    #[error("Invalid message format: {0}")]
    InvalidMessage(String),

    /// JSON parsing error from exchange response
    #[error("Failed to parse JSON: {0}")]
    JsonParse(#[from] serde_json::Error),

    /// Parse error with context
    #[error("Parse error for venue {venue}: {message} - {error}")]
    ParseError {
        /// The venue where parsing failed
        venue: VenueId,
        /// The message that failed to parse
        message: String,
        /// The specific error
        error: String,
    },

    /// Required field missing from exchange message
    #[error("Missing required field: {field}")]
    MissingField {
        /// The field that was missing
        field: String,
    },

    /// Invalid numeric value in exchange data
    #[error("Invalid numeric value: {value}")]
    InvalidNumeric {
        /// The value that couldn't be parsed
        value: String,
    },

    /// Protocol errors
    #[error("Failed to build TLV message: {0}")]
    TLVBuildFailed(String),

    /// Failed to send TLV message via relay output
    #[error("Failed to send TLV message: {0}")]
    TLVSendFailed(String),

    /// Invalid instrument identifier or symbol
    #[error("Invalid instrument: {0}")]
    InvalidInstrument(String),

    /// System errors
    #[error("WebSocket error: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),

    /// I/O error during network operations
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Configuration error in adapter settings
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Circuit breaker triggered due to repeated failures
    #[error("Circuit breaker open for venue {venue}")]
    CircuitBreakerOpen {
        /// The venue whose circuit breaker is open
        venue: VenueId,
    },

    /// Recovery errors
    #[error("Maximum reconnection attempts ({max_attempts}) exceeded for venue {venue}")]
    MaxReconnectAttemptsExceeded {
        /// The venue that failed to reconnect
        venue: VenueId,
        /// Maximum attempts that were tried
        max_attempts: u32,
    },

    /// Venue permanently failed, requires manual intervention
    #[error("Venue {venue} is in failed state, manual intervention required")]
    VenueFailed {
        /// The venue in failed state
        venue: VenueId,
    },

    /// Internal errors
    #[error("Internal error: {0}")]
    Internal(String),

    /// Not supported operation
    #[error("Operation not supported: {0}")]
    NotSupported(String),

    /// Validation error (for production safety checks)
    #[error("Validation failed: {0}")]
    ValidationFailed(String),

    /// Connection error (simplified)
    #[error("Connection error: {0}")]
    ConnectionError(String),

    /// Provider error
    #[error("Provider error: {0}")]
    ProviderError(String),

    /// Config error
    #[error("Config error: {0}")]
    ConfigError(String),

    /// Connection closed error
    #[error("Connection closed for venue {venue}: {reason:?}")]
    ConnectionClosed {
        /// The venue whose connection was closed
        venue: VenueId,
        /// Optional reason for closure
        reason: Option<String>,
    },

    /// Generic errors
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl AdapterError {
    /// Check if this error is recoverable through retry
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            AdapterError::ConnectionFailed { .. }
                | AdapterError::ConnectionTimeout { .. }
                | AdapterError::InvalidMessage(_)
                | AdapterError::JsonParse(_)
                | AdapterError::InvalidNumeric { .. }
                | AdapterError::TLVSendFailed(_)
                | AdapterError::WebSocket(_)
                | AdapterError::Io(_)
        )
    }

    /// Check if this error should trigger state invalidation
    pub fn should_invalidate_state(&self) -> bool {
        matches!(
            self,
            AdapterError::ConnectionFailed { .. }
                | AdapterError::ConnectionTimeout { .. }
                | AdapterError::AuthenticationFailed { .. }
                | AdapterError::WebSocket(_)
                | AdapterError::VenueFailed { .. }
        )
    }

    /// Check if this error indicates a permanent failure
    pub fn is_permanent(&self) -> bool {
        matches!(
            self,
            AdapterError::AuthenticationFailed { .. }
                | AdapterError::Configuration(_)
                | AdapterError::MaxReconnectAttemptsExceeded { .. }
                | AdapterError::VenueFailed { .. }
        )
    }
}
