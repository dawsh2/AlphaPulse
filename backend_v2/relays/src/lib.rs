//! AlphaPulse Relay Infrastructure
//!
//! Application-aware message routing layer between transport and services.

pub mod config;
pub mod relay;
pub mod signal_relay;
pub mod topics;
pub mod transport_adapter;
pub mod types;
pub mod validation;

pub use config::*;
pub use relay::*;
pub use signal_relay::*;
pub use topics::*;
pub use transport_adapter::*;
pub use types::*;
pub use validation::*;

use protocol_v2::{ProtocolError, Result};

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

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result type for relay operations
pub type RelayResult<T> = std::result::Result<T, RelayError>;
