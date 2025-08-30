/// Context information for send operations to aid in debugging
#[derive(Debug, Clone)]
pub struct SendContext {
    /// Size of the message payload in bytes
    pub message_size: usize,
    /// Correlation ID for tracing, if available
    pub correlation_id: Option<String>,
    /// Timestamp when send was attempted (nanoseconds since epoch)
    pub timestamp_ns: u64,
    /// Target service hint, if available
    pub target: Option<String>,
}

impl SendContext {
    pub fn new(message_size: usize, timestamp_ns: u64) -> Self {
        Self {
            message_size,
            correlation_id: None,
            timestamp_ns,
            target: None,
        }
    }

    pub fn with_correlation_id(mut self, id: String) -> Self {
        self.correlation_id = Some(id);
        self
    }

    pub fn with_target(mut self, target: String) -> Self {
        self.target = Some(target);
        self
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum SinkError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Connection lost: {0}")]
    ConnectionLost(String),

    #[error("Send failed: {error} (size: {size}B, id: {correlation_id:?}, target: {target:?})", 
            size = context.message_size,
            correlation_id = context.correlation_id,
            target = context.target)]
    SendFailed { error: String, context: SendContext },

    #[error("Buffer full, message dropped (size: {size}B, id: {correlation_id:?})",
            size = context.message_size,
            correlation_id = context.correlation_id)]
    BufferFull { context: SendContext },

    #[error("Message too large: {size}B exceeds limit of {limit}B")]
    MessageTooLarge { size: usize, limit: usize },

    #[error("Sink closed")]
    Closed,

    #[error("Timeout after {0} seconds")]
    Timeout(u64),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("IO error: {0}")]
    Io(String),

    #[error("Other error: {0}")]
    Other(String),
}

impl SinkError {
    /// Get correlation ID from error context
    fn correlation_id(&self) -> &str {
        match self {
            SinkError::SendFailed { context, .. } | SinkError::BufferFull { context } => {
                context.correlation_id.as_deref().unwrap_or("none")
            }
            _ => "n/a",
        }
    }

    /// Get target from error context
    fn target(&self) -> &str {
        match self {
            SinkError::SendFailed { context, .. } | SinkError::BufferFull { context } => {
                context.target.as_deref().unwrap_or("unknown")
            }
            _ => "n/a",
        }
    }

    /// Check if this is a recoverable error
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            SinkError::ConnectionLost(_) | SinkError::Timeout(_) | SinkError::BufferFull { .. }
        )
    }

    /// Check if this is a connection-related error
    pub fn is_connection_error(&self) -> bool {
        matches!(
            self,
            SinkError::ConnectionFailed(_) | SinkError::ConnectionLost(_)
        )
    }

    /// Create a connection failed error
    pub fn connection_failed(msg: impl Into<String>) -> Self {
        SinkError::ConnectionFailed(msg.into())
    }

    /// Create a send failed error with context
    pub fn send_failed_with_context(msg: impl Into<String>, context: SendContext) -> Self {
        SinkError::SendFailed {
            error: msg.into(),
            context,
        }
    }

    /// Create a send failed error (legacy method, creates minimal context)
    pub fn send_failed(msg: impl Into<String>) -> Self {
        let timestamp =
            network::safe_system_timestamp_ns_checked().unwrap_or_else(|e| {
                // Log error but continue - this is error handling code itself
                eprintln!("WARNING: Timestamp error in error handling: {}", e);
                0
            });
        SinkError::SendFailed {
            error: msg.into(),
            context: SendContext::new(0, timestamp),
        }
    }

    /// Create a buffer full error with context
    pub fn buffer_full_with_context(context: SendContext) -> Self {
        SinkError::BufferFull { context }
    }

    /// Create a message too large error
    pub fn message_too_large(size: usize, limit: usize) -> Self {
        SinkError::MessageTooLarge { size, limit }
    }

    /// Create an invalid config error
    pub fn invalid_config(msg: impl Into<String>) -> Self {
        SinkError::InvalidConfig(msg.into())
    }

    /// Create a timeout error
    pub fn timeout(seconds: u64) -> Self {
        SinkError::Timeout(seconds)
    }
}

impl From<std::io::Error> for SinkError {
    fn from(err: std::io::Error) -> Self {
        SinkError::Io(err.to_string())
    }
}
