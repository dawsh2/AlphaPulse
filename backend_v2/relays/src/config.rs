//! Relay configuration management

use crate::RelayError;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Main relay configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RelayConfig {
    pub relay: RelaySettings,
    pub transport: TransportConfig,
    pub validation: ValidationPolicy,
    pub topics: TopicConfig,
    pub performance: PerformanceConfig,
}

/// Core relay settings
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RelaySettings {
    /// Relay domain (1=market_data, 2=signal, 3=execution)
    pub domain: u8,
    /// Human-readable name
    pub name: String,
    /// Optional description
    pub description: Option<String>,
}

/// Transport configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TransportConfig {
    /// Transport mode (unix_socket, tcp, udp, message_queue)
    pub mode: String,
    /// Path for unix socket
    pub path: Option<String>,
    /// Address for network transports
    pub address: Option<String>,
    /// Port for network transports
    pub port: Option<u16>,
    /// Use topology integration
    pub use_topology: bool,
}

/// Validation policies per domain
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ValidationPolicy {
    /// Enable checksum validation
    pub checksum: bool,
    /// Enable audit logging
    pub audit: bool,
    /// Enable strict mode (fail on any validation error)
    pub strict: bool,
    /// Maximum message size in bytes
    pub max_message_size: Option<usize>,
}

/// Topic routing configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TopicConfig {
    /// Default topic for unspecified messages
    pub default: String,
    /// Available topics for subscription
    pub available: Vec<String>,
    /// Enable automatic topic discovery
    pub auto_discover: bool,
    /// Topic extraction strategy
    pub extraction_strategy: TopicExtractionStrategy,
}

/// How to extract topic from messages
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TopicExtractionStrategy {
    /// Use source type from header
    SourceType,
    /// Use instrument venue
    InstrumentVenue,
    /// Use custom TLV field
    CustomField(u8),
    /// Fixed topic for all messages
    Fixed(String),
}

/// Performance tuning parameters
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PerformanceConfig {
    /// Target throughput (messages/second)
    pub target_throughput: Option<u64>,
    /// Buffer size for message queues
    pub buffer_size: usize,
    /// Maximum concurrent connections
    pub max_connections: usize,
    /// Batch size for processing
    pub batch_size: usize,
    /// Enable performance monitoring
    pub monitoring: bool,
}

impl RelayConfig {
    /// Load configuration from TOML file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, RelayError> {
        let contents = std::fs::read_to_string(path)
            .map_err(|e| RelayError::Config(format!("Failed to read config file: {}", e)))?;

        toml::from_str(&contents)
            .map_err(|e| RelayError::Config(format!("Failed to parse config: {}", e)))
    }

    /// Create default config for a domain
    pub fn default_for_domain(domain: u8) -> Self {
        match domain {
            1 => Self::market_data_defaults(),
            2 => Self::signal_defaults(),
            3 => Self::execution_defaults(),
            _ => Self::market_data_defaults(),
        }
    }

    /// Default configuration for market data relay
    pub fn market_data_defaults() -> Self {
        Self {
            relay: RelaySettings {
                domain: 1,
                name: "market_data".to_string(),
                description: Some("High-throughput market data relay".to_string()),
            },
            transport: TransportConfig {
                mode: "unix_socket".to_string(),
                path: Some("/tmp/alphapulse/market_data.sock".to_string()),
                address: None,
                port: None,
                use_topology: false,
            },
            validation: ValidationPolicy {
                checksum: false, // Skip for performance
                audit: false,
                strict: false,
                max_message_size: Some(65536),
            },
            topics: TopicConfig {
                default: "market_data_all".to_string(),
                available: vec![
                    "market_data_polygon".to_string(),
                    "market_data_ethereum".to_string(),
                    "market_data_kraken".to_string(),
                    "market_data_binance".to_string(),
                ],
                auto_discover: true,
                extraction_strategy: TopicExtractionStrategy::SourceType,
            },
            performance: PerformanceConfig {
                target_throughput: Some(1_000_000), // >1M msg/s
                buffer_size: 65536,
                max_connections: 1000,
                batch_size: 100,
                monitoring: true,
            },
        }
    }

    /// Default configuration for signal relay
    pub fn signal_defaults() -> Self {
        Self {
            relay: RelaySettings {
                domain: 2,
                name: "signal".to_string(),
                description: Some("Reliable signal relay with validation".to_string()),
            },
            transport: TransportConfig {
                mode: "unix_socket".to_string(),
                path: Some("/tmp/alphapulse/signals.sock".to_string()),
                address: None,
                port: None,
                use_topology: false,
            },
            validation: ValidationPolicy {
                checksum: true, // Enable for reliability
                audit: false,
                strict: true,
                max_message_size: Some(32768),
            },
            topics: TopicConfig {
                default: "signals_all".to_string(),
                available: vec![
                    "arbitrage_signals".to_string(),
                    "trend_signals".to_string(),
                    "risk_signals".to_string(),
                ],
                auto_discover: false,
                extraction_strategy: TopicExtractionStrategy::SourceType,
            },
            performance: PerformanceConfig {
                target_throughput: Some(100_000), // >100K msg/s
                buffer_size: 32768,
                max_connections: 100,
                batch_size: 50,
                monitoring: true,
            },
        }
    }

    /// Default configuration for execution relay
    pub fn execution_defaults() -> Self {
        Self {
            relay: RelaySettings {
                domain: 3,
                name: "execution".to_string(),
                description: Some("Secure execution relay with full audit".to_string()),
            },
            transport: TransportConfig {
                mode: "unix_socket".to_string(),
                path: Some("/tmp/alphapulse/execution.sock".to_string()),
                address: None,
                port: None,
                use_topology: false,
            },
            validation: ValidationPolicy {
                checksum: true, // Full validation
                audit: true,    // Audit logging
                strict: true,   // Fail on any error
                max_message_size: Some(16384),
            },
            topics: TopicConfig {
                default: "execution_all".to_string(),
                available: vec![
                    "orders".to_string(),
                    "fills".to_string(),
                    "cancellations".to_string(),
                ],
                auto_discover: false,
                extraction_strategy: TopicExtractionStrategy::Fixed("execution".to_string()),
            },
            performance: PerformanceConfig {
                target_throughput: Some(50_000), // >50K msg/s
                buffer_size: 16384,
                max_connections: 50,
                batch_size: 10,
                monitoring: true,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_configs() {
        let market = RelayConfig::market_data_defaults();
        assert_eq!(market.relay.domain, 1);
        assert!(!market.validation.checksum);

        let signal = RelayConfig::signal_defaults();
        assert_eq!(signal.relay.domain, 2);
        assert!(signal.validation.checksum);

        let execution = RelayConfig::execution_defaults();
        assert_eq!(execution.relay.domain, 3);
        assert!(execution.validation.checksum);
        assert!(execution.validation.audit);
    }
}

/// Signal relay specific configuration
///
/// Specialized configuration for the signal distribution relay with
/// performance tuning and connection management parameters.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SignalRelayConfig {
    /// Maximum number of concurrent consumer connections
    pub max_consumers: usize,

    /// Channel buffer size for signal broadcasting
    pub channel_buffer_size: usize,

    /// Cleanup interval for stale connections (milliseconds)
    pub cleanup_interval_ms: u64,

    /// Connection timeout for detecting dead connections (seconds)
    pub connection_timeout_seconds: u64,

    /// Enable detailed metrics collection
    pub enable_metrics: bool,

    /// Metrics reporting interval (seconds)
    pub metrics_interval_seconds: u64,
}

impl Default for SignalRelayConfig {
    fn default() -> Self {
        Self {
            max_consumers: 1000,
            channel_buffer_size: 1000,
            cleanup_interval_ms: 5000,
            connection_timeout_seconds: 30,
            enable_metrics: true,
            metrics_interval_seconds: 60,
        }
    }
}

impl SignalRelayConfig {
    /// Create configuration optimized for high throughput
    pub fn high_throughput() -> Self {
        Self {
            max_consumers: 5000,
            channel_buffer_size: 10000,
            cleanup_interval_ms: 2000,
            connection_timeout_seconds: 15,
            enable_metrics: true,
            metrics_interval_seconds: 30,
        }
    }

    /// Create configuration optimized for low latency
    pub fn low_latency() -> Self {
        Self {
            max_consumers: 500,
            channel_buffer_size: 100,
            cleanup_interval_ms: 1000,
            connection_timeout_seconds: 10,
            enable_metrics: false, // Disable for minimal overhead
            metrics_interval_seconds: 120,
        }
    }

    /// Validate configuration parameters
    pub fn validate(&self) -> Result<(), RelayError> {
        if self.max_consumers == 0 {
            return Err(RelayError::Config("max_consumers must be > 0".to_string()));
        }

        if self.channel_buffer_size == 0 {
            return Err(RelayError::Config(
                "channel_buffer_size must be > 0".to_string(),
            ));
        }

        if self.cleanup_interval_ms < 100 {
            return Err(RelayError::Config(
                "cleanup_interval_ms must be >= 100".to_string(),
            ));
        }

        if self.connection_timeout_seconds < 5 {
            return Err(RelayError::Config(
                "connection_timeout_seconds must be >= 5".to_string(),
            ));
        }

        Ok(())
    }
}
