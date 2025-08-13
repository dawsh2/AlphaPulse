// Error types for AlphaPulse Rust services
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AlphaPulseError {
    #[error("Redis error: {0}")]
    RedisError(#[from] redis::RedisError),
    
    #[error("WebSocket error: {0}")]
    WebSocketError(#[from] tokio_tungstenite::tungstenite::Error),
    
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),
    
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    
    #[error("Parse error: {0}")]
    ParseError(String),
    
    #[error("Config error: {0}")]
    ConfigError(String),
    
    #[error("Channel send error")]
    ChannelSendError,
    
    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),
    
    #[error("Invalid data: {0}")]
    InvalidData(String),
    
    #[error("Unknown error: {0}")]
    Unknown(String),
    
    #[error("URL parse error: {0}")]
    UrlParseError(#[from] url::ParseError),
    
    #[error("Buffer overflow: index {index} >= capacity {capacity}")]
    BufferOverflow { index: usize, capacity: usize },
    
    #[error("System time error: {0}")]
    SystemTimeError(#[from] std::time::SystemTimeError),
    
    #[error("Memory mapping error: {0}")]
    MemoryMappingError(String),
    
    #[error("Invalid memory layout: expected size {expected}, got {actual}")]
    InvalidMemoryLayout { expected: usize, actual: usize },
    
    #[error("Shared memory corruption detected")]
    MemoryCorruption,
}

pub type Result<T> = std::result::Result<T, AlphaPulseError>;