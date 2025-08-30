//! # Torq Relay Core Infrastructure
//!
//! Shared infrastructure components for all relay domains including transport
//! adapters, validation policies, topic routing, and message construction.

pub mod common;
pub mod config;
pub mod message_construction;
pub mod topics;
pub mod transport;
pub mod types;
pub mod validation;

// Re-export commonly used types
pub use common::*;
pub use config::*;
pub use message_construction::*;
// Message sink moved to services_v2/messaging/patterns/message_sink
pub use topics::*;
pub use transport::*;
pub use types::*;
pub use validation::*;

use codec::protocol::ProtocolError;

/// Relay-specific errors
#[derive(Debug, thiserror::Error)]
pub enum RelayError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Transport error: {0}")]
    Transport(String),

    #[error("Topic not found: {0}")]
    TopicNotFound(String),

    #[error("Validation failed: {0}")]
    Validation(String),

    #[error("Protocol error: {0}")]
    Protocol(#[from] ProtocolError),

    #[error("Codec error: {0}")]
    Codec(#[from] codec::ProtocolError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result type for relay operations
pub type RelayResult<T> = std::result::Result<T, RelayError>;