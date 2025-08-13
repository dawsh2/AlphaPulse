// Error types for AlphaPulse Rust services
use thiserror::Error;

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