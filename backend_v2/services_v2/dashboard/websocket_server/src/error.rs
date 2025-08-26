//! Error types for dashboard WebSocket server

use thiserror::Error;

#[derive(Debug, Error)]
pub enum DashboardError {
    #[error("Protocol error: {0}")]
    Protocol(#[from] alphapulse_types::protocol::ProtocolError),

    #[error("WebSocket error: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] warp::Error),

    #[error("Configuration error: {message}")]
    Configuration { message: String },

    #[error("Client error: {message}")]
    Client { message: String },

    #[error("Relay connection error: {message}")]
    RelayConnection { message: String },
}

pub type Result<T> = std::result::Result<T, DashboardError>;
