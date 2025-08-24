//! Transport Error Types
//!
//! Comprehensive error handling for network transport, message queues,
//! and topology integration failures.

use std::net::SocketAddr;
use thiserror::Error;

/// Main transport error type
#[derive(Error, Debug)]
pub enum TransportError {
    /// Network connectivity errors
    #[error("Network error: {message}")]
    Network {
        message: String,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Connection management errors  
    #[error("Connection error: {message} (remote: {remote_addr:?})")]
    Connection {
        message: String,
        remote_addr: Option<SocketAddr>,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Protocol and serialization errors
    #[error("Protocol error: {message}")]
    Protocol {
        message: String,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Configuration errors
    #[error("Configuration error: {message}")]
    Configuration {
        message: String,
        field: Option<String>,
    },

    /// Message queue specific errors
    #[error("Message queue error: {backend}: {message}")]
    MessageQueue {
        backend: String,
        message: String,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Security and encryption errors
    #[error("Security error: {message}")]
    Security {
        message: String,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Compression/decompression errors
    #[error("Compression error: {codec}: {message}")]
    Compression { codec: String, message: String },

    /// Transport timeout errors
    #[error("Timeout error: {operation} exceeded {timeout_ms}ms")]
    Timeout { operation: String, timeout_ms: u64 },

    /// Resource exhaustion errors
    #[error("Resource exhausted: {resource}: {message}")]
    ResourceExhausted { resource: String, message: String },

    /// Topology integration errors
    #[error("Topology error: {message}")]
    Topology {
        message: String,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Monitoring and metrics errors
    #[error("Monitoring error: {message}")]
    Monitoring {
        message: String,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Circuit breaker and health check errors
    #[error("Health check failed: {check_type}: {message}")]
    HealthCheck { check_type: String, message: String },

    /// Generic I/O errors
    #[error("I/O error: {message}")]
    Io {
        message: String,
        source: std::io::Error,
    },
}

/// Result type alias for transport operations
pub type Result<T> = std::result::Result<T, TransportError>;

impl TransportError {
    /// Create a network error
    pub fn network(message: impl Into<String>) -> Self {
        Self::Network {
            message: message.into(),
            source: None,
        }
    }

    /// Create a network error with source
    pub fn network_with_source(
        message: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::Network {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a connection error
    pub fn connection(message: impl Into<String>, remote_addr: Option<SocketAddr>) -> Self {
        Self::Connection {
            message: message.into(),
            remote_addr,
            source: None,
        }
    }

    /// Create a connection error with source
    pub fn connection_with_source(
        message: impl Into<String>,
        remote_addr: Option<SocketAddr>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::Connection {
            message: message.into(),
            remote_addr,
            source: Some(Box::new(source)),
        }
    }

    /// Create a protocol error
    pub fn protocol(message: impl Into<String>) -> Self {
        Self::Protocol {
            message: message.into(),
            source: None,
        }
    }

    /// Create a protocol error with source
    pub fn protocol_with_source(
        message: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::Protocol {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a configuration error
    pub fn configuration(message: impl Into<String>, field: Option<&str>) -> Self {
        Self::Configuration {
            message: message.into(),
            field: field.map(|s| s.to_string()),
        }
    }

    /// Create a message queue error
    pub fn message_queue(backend: impl Into<String>, message: impl Into<String>) -> Self {
        Self::MessageQueue {
            backend: backend.into(),
            message: message.into(),
            source: None,
        }
    }

    /// Create a message queue error with source
    pub fn message_queue_with_source(
        backend: impl Into<String>,
        message: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::MessageQueue {
            backend: backend.into(),
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a security error
    pub fn security(message: impl Into<String>) -> Self {
        Self::Security {
            message: message.into(),
            source: None,
        }
    }

    /// Create a compression error
    pub fn compression(codec: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Compression {
            codec: codec.into(),
            message: message.into(),
        }
    }

    /// Create a timeout error
    pub fn timeout(operation: impl Into<String>, timeout_ms: u64) -> Self {
        Self::Timeout {
            operation: operation.into(),
            timeout_ms,
        }
    }

    /// Create a resource exhausted error
    pub fn resource_exhausted(resource: impl Into<String>, message: impl Into<String>) -> Self {
        Self::ResourceExhausted {
            resource: resource.into(),
            message: message.into(),
        }
    }

    /// Create a topology error
    pub fn topology(message: impl Into<String>) -> Self {
        Self::Topology {
            message: message.into(),
            source: None,
        }
    }

    /// Create a topology error with source
    pub fn topology_with_source(
        message: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::Topology {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a health check error
    pub fn health_check(check_type: impl Into<String>, message: impl Into<String>) -> Self {
        Self::HealthCheck {
            check_type: check_type.into(),
            message: message.into(),
        }
    }

    /// Create a generic transport error (alias for network)
    pub fn transport(message: impl Into<String>, context: Option<&str>) -> Self {
        let msg = if let Some(ctx) = context {
            format!("{}: {}", ctx, message.into())
        } else {
            message.into()
        };
        Self::network(msg)
    }

    /// Create a resolution error (alias for topology)
    pub fn resolution(message: impl Into<String>, actor: Option<&str>) -> Self {
        let msg = if let Some(a) = actor {
            format!("Failed to resolve actor {}: {}", a, message.into())
        } else {
            message.into()
        };
        Self::topology(msg)
    }

    /// Check if this is a retryable error
    pub fn is_retryable(&self) -> bool {
        match self {
            TransportError::Network { .. } => true,
            TransportError::Connection { .. } => true,
            TransportError::Timeout { .. } => true,
            TransportError::ResourceExhausted { .. } => true,
            TransportError::Protocol { .. } => false,
            TransportError::Configuration { .. } => false,
            TransportError::Security { .. } => false,
            TransportError::Compression { .. } => false,
            TransportError::MessageQueue { .. } => true, // May be temporary
            TransportError::Topology { .. } => false,
            TransportError::Monitoring { .. } => true,
            TransportError::HealthCheck { .. } => true,
            TransportError::Io { .. } => true,
        }
    }

    /// Check if this is a transient error
    pub fn is_transient(&self) -> bool {
        match self {
            TransportError::Network { .. } => true,
            TransportError::Connection { .. } => true,
            TransportError::Timeout { .. } => true,
            TransportError::ResourceExhausted { .. } => true,
            _ => false,
        }
    }

    /// Get error category for metrics
    pub fn category(&self) -> &'static str {
        match self {
            TransportError::Network { .. } => "network",
            TransportError::Connection { .. } => "connection",
            TransportError::Protocol { .. } => "protocol",
            TransportError::Configuration { .. } => "configuration",
            TransportError::MessageQueue { .. } => "message_queue",
            TransportError::Security { .. } => "security",
            TransportError::Compression { .. } => "compression",
            TransportError::Timeout { .. } => "timeout",
            TransportError::ResourceExhausted { .. } => "resource_exhausted",
            TransportError::Topology { .. } => "topology",
            TransportError::Monitoring { .. } => "monitoring",
            TransportError::HealthCheck { .. } => "health_check",
            TransportError::Io { .. } => "io",
        }
    }
}

/// Convert standard I/O errors to transport errors
impl From<std::io::Error> for TransportError {
    fn from(error: std::io::Error) -> Self {
        TransportError::Io {
            message: error.to_string(),
            source: error,
        }
    }
}

/// Convert topology errors to transport errors
impl From<alphapulse_topology::error::TopologyError> for TransportError {
    fn from(error: alphapulse_topology::error::TopologyError) -> Self {
        TransportError::topology_with_source("Topology integration failed", error)
    }
}

/// Convert serde YAML errors to transport errors
impl From<serde_yaml::Error> for TransportError {
    fn from(error: serde_yaml::Error) -> Self {
        TransportError::configuration(format!("YAML configuration error: {}", error), None)
    }
}

/// Convert bincode errors to transport errors
impl From<bincode::Error> for TransportError {
    fn from(error: bincode::Error) -> Self {
        TransportError::protocol_with_source("Binary serialization failed", error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn test_error_construction() {
        let err = TransportError::network("Connection refused");
        assert_eq!(err.category(), "network");
        assert!(err.is_retryable());
        assert!(err.is_transient());
    }

    #[test]
    fn test_connection_error() {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)), 8080);
        let err = TransportError::connection("Handshake failed", Some(addr));

        match err {
            TransportError::Connection { remote_addr, .. } => {
                assert_eq!(remote_addr, Some(addr));
            }
            _ => panic!("Expected Connection error"),
        }
    }

    #[test]
    fn test_error_categorization() {
        assert_eq!(TransportError::protocol("test").category(), "protocol");
        assert_eq!(
            TransportError::timeout("connect", 5000).category(),
            "timeout"
        );
        assert_eq!(
            TransportError::compression("lz4", "test").category(),
            "compression"
        );
    }

    #[test]
    fn test_retryable_errors() {
        assert!(TransportError::network("test").is_retryable());
        assert!(TransportError::timeout("test", 1000).is_retryable());
        assert!(!TransportError::configuration("test", None).is_retryable());
        assert!(!TransportError::security("test").is_retryable());
    }

    #[test]
    fn test_transient_errors() {
        assert!(TransportError::network("test").is_transient());
        assert!(TransportError::connection("test", None).is_transient());
        assert!(!TransportError::protocol("test").is_transient());
        assert!(!TransportError::configuration("test", None).is_transient());
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "test");
        let transport_err = TransportError::from(io_err);

        match transport_err {
            TransportError::Io { message, .. } => {
                assert!(message.contains("test"));
            }
            _ => panic!("Expected Io error"),
        }
    }
}
