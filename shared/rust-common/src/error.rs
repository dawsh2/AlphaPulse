// Error types for AlphaPulse Rust services
use thiserror::Error;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

#[derive(Error, Debug)]
pub enum AlphaPulseError {
    #[error("WebSocket connection error: {0}")]
    WebSocketError(#[from] tokio_tungstenite::tungstenite::Error),
    
    #[error("Redis error: {0}")]
    RedisError(#[from] redis::RedisError),
    
    #[error("JSON serialization error: {0}")]
    JsonError(#[from] serde_json::Error),
    
    #[error("HTTP client error: {0}")]
    HttpError(#[from] reqwest::Error),
    
    #[error("URL parse error: {0}")]
    UrlParseError(#[from] url::ParseError),
    
    #[error("Parse error: {0}")]
    ParseError(String),
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
    
    #[error("Network error: {0}")]
    NetworkError(String),
    
    #[error("Exchange API error: {exchange} - {message}")]
    ExchangeError {
        exchange: String,
        message: String,
    },
    
    #[error("Buffer overflow: {0}")]
    BufferOverflow(String),
    
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, AlphaPulseError>;

impl IntoResponse for AlphaPulseError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AlphaPulseError::WebSocketError(_) => (StatusCode::BAD_GATEWAY, "WebSocket connection error"),
            AlphaPulseError::RedisError(_) => (StatusCode::SERVICE_UNAVAILABLE, "Database error"),
            AlphaPulseError::JsonError(_) => (StatusCode::BAD_REQUEST, "Invalid JSON"),
            AlphaPulseError::HttpError(_) => (StatusCode::BAD_GATEWAY, "External service error"),
            AlphaPulseError::UrlParseError(_) => (StatusCode::BAD_REQUEST, "Invalid URL"),
            AlphaPulseError::ParseError(_) => (StatusCode::BAD_REQUEST, "Parse error"),
            AlphaPulseError::ConfigError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Configuration error"),
            AlphaPulseError::NetworkError(_) => (StatusCode::SERVICE_UNAVAILABLE, "Network error"),
            AlphaPulseError::ExchangeError { .. } => (StatusCode::BAD_GATEWAY, "Exchange API error"),
            AlphaPulseError::BufferOverflow(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Buffer overflow"),
            AlphaPulseError::DatabaseError(_) => (StatusCode::SERVICE_UNAVAILABLE, "Database error"),
            AlphaPulseError::IoError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "IO error"),
        };

        let body = axum::Json(serde_json::json!({
            "error": error_message,
            "details": self.to_string()
        }));

        (status, body).into_response()
    }
}