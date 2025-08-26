//! # Relay Error Types
//!
//! Comprehensive error handling for the generic relay engine.
//! Provides clear error categories and proper error propagation.

use thiserror::Error;

/// Relay engine operation errors
#[derive(Error, Debug)]
pub enum RelayEngineError {
    /// Setup and initialization errors
    #[error("Setup error: {0}")]
    Setup(String),

    /// Transport layer errors (Unix sockets, network)
    #[error("Transport error: {0}")]
    Transport(String),

    /// Message validation and parsing errors
    #[error("Message validation error: {0}")]
    Validation(String),

    /// Client connection management errors
    #[error("Client management error: {0}")]
    Client(String),

    /// IO errors from tokio operations
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Generic error for unexpected conditions
    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<alphapulse_types::protocol::ProtocolError> for RelayEngineError {
    fn from(err: alphapulse_types::protocol::ProtocolError) -> Self {
        RelayEngineError::Validation(format!("Protocol error: {}", err))
    }
}

/// Result type alias for relay engine operations
pub type Result<T> = std::result::Result<T, RelayEngineError>;