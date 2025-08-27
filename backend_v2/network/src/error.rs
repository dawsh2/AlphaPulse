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

    /// Feature not implemented yet
    #[error("Feature '{feature}' not implemented: {reason}")]
    NotImplemented { feature: String, reason: String },

    /// Precision and financial calculation errors
    #[error("Precision error: {message}")]
    Precision { message: String },

    /// System-level errors
    #[error("System error: {message}")]
    System { message: String },
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

    /// Create a not implemented error
    pub fn not_implemented(feature: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::NotImplemented {
            feature: feature.into(),
            reason: reason.into(),
        }
    }

    /// Create a precision error
    pub fn precision(message: impl Into<String>) -> Self {
        Self::Precision {
            message: message.into(),
        }
    }

    /// Create a system error
    pub fn system(message: impl Into<String>) -> Self {
        Self::System {
            message: message.into(),
        }
    }

    /// Create a protocol error with detailed context preservation
    pub fn protocol_with_field(message: impl Into<String>, field: Option<&str>) -> Self {
        let context_message = if let Some(f) = field {
            format!("{} (field: {})", message.into(), f)
        } else {
            message.into()
        };
        
        Self::Protocol {
            message: context_message,
            source: None,
        }
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
            TransportError::NotImplemented { .. } => false,
            TransportError::Precision { .. } => false,
            TransportError::System { .. } => true,
        }
    }

    /// Check if this is a transient error
    pub fn is_transient(&self) -> bool {
        match self {
            TransportError::Network { .. } => true,
            TransportError::Connection { .. } => true,
            TransportError::Timeout { .. } => true,
            TransportError::ResourceExhausted { .. } => true,
            TransportError::System { .. } => true,
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
            TransportError::NotImplemented { .. } => "not_implemented",
            TransportError::Precision { .. } => "precision",
            TransportError::System { .. } => "system",
        }
    }
}

// Custom Clone implementation since Box<dyn Error> doesn't implement Clone
impl Clone for TransportError {
    fn clone(&self) -> Self {
        match self {
            TransportError::Network { message, .. } => {
                TransportError::Network {
                    message: message.clone(),
                    source: None, // Source errors are not cloneable, so we omit them
                }
            }
            TransportError::Connection { message, remote_addr, .. } => {
                TransportError::Connection {
                    message: message.clone(),
                    remote_addr: *remote_addr,
                    source: None,
                }
            }
            TransportError::Protocol { message, .. } => {
                TransportError::Protocol {
                    message: message.clone(),
                    source: None,
                }
            }
            TransportError::Configuration { message, field } => {
                TransportError::Configuration {
                    message: message.clone(),
                    field: field.clone(),
                }
            }
            TransportError::MessageQueue { backend, message, .. } => {
                TransportError::MessageQueue {
                    backend: backend.clone(),
                    message: message.clone(),
                    source: None,
                }
            }
            TransportError::Security { message, .. } => {
                TransportError::Security {
                    message: message.clone(),
                    source: None,
                }
            }
            TransportError::Compression { codec, message } => {
                TransportError::Compression {
                    codec: codec.clone(),
                    message: message.clone(),
                }
            }
            TransportError::Timeout { operation, timeout_ms } => {
                TransportError::Timeout {
                    operation: operation.clone(),
                    timeout_ms: *timeout_ms,
                }
            }
            TransportError::ResourceExhausted { resource, message } => {
                TransportError::ResourceExhausted {
                    resource: resource.clone(),
                    message: message.clone(),
                }
            }
            TransportError::Topology { message, .. } => {
                TransportError::Topology {
                    message: message.clone(),
                    source: None,
                }
            }
            TransportError::Monitoring { message, .. } => {
                TransportError::Monitoring {
                    message: message.clone(),
                    source: None,
                }
            }
            TransportError::HealthCheck { check_type, message } => {
                TransportError::HealthCheck {
                    check_type: check_type.clone(),
                    message: message.clone(),
                }
            }
            TransportError::Io { message, source } => {
                TransportError::Io {
                    message: message.clone(),
                    source: std::io::Error::new(source.kind(), message.as_str()),
                }
            }
            TransportError::NotImplemented { feature, reason } => {
                TransportError::NotImplemented {
                    feature: feature.clone(),
                    reason: reason.clone(),
                }
            }
            TransportError::Precision { message } => {
                TransportError::Precision {
                    message: message.clone(),
                }
            }
            TransportError::System { message } => {
                TransportError::System {
                    message: message.clone(),
                }
            }
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
impl From<crate::topology::error::TopologyError> for TransportError {
    fn from(error: crate::topology::error::TopologyError) -> Self {
        // Preserve context from topology error for better debugging
        let context_message = match &error {
            crate::topology::error::TopologyError::ActorNotFound { actor } => {
                format!("Topology integration failed: Actor '{}' not found", actor)
            },
            crate::topology::error::TopologyError::NodeNotFound { node } => {
                format!("Topology integration failed: Node '{}' not found", node)
            },
            crate::topology::error::TopologyError::Config { message } => {
                format!("Topology integration failed: Configuration error: {}", message)
            },
            crate::topology::error::TopologyError::TransportResolution { reason } => {
                format!("Topology integration failed: Transport resolution: {}", reason)
            },
            crate::topology::error::TopologyError::NotImplemented { feature, planned_phase } => {
                format!("Topology integration failed: {} not implemented (planned for {})", feature, planned_phase)
            },
            _ => format!("Topology integration failed: {}", error),
        };
        
        TransportError::topology_with_source(context_message, error)
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
