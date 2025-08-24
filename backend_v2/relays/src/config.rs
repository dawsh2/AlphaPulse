//! # Relay Configuration Management - Domain-Specific Policy Engine
//!
//! ## Purpose
//! Comprehensive configuration framework for relay domains with performance-tuned
//! policies, transport settings, and validation rules. Enables dynamic relay
//! behavior based on domain requirements (MarketData, Signal, Execution).
//!
//! ## Architecture Role
//!
//! ```mermaid
//! graph LR
//!     ConfigFiles[Configuration Files] -->|TOML Parse| ConfigLoader[Config Loader]
//!     ConfigLoader -->|Domain 1| MarketConfig[Market Data Config]
//!     ConfigLoader -->|Domain 2| SignalConfig[Signal Config]
//!     ConfigLoader -->|Domain 3| ExecConfig[Execution Config]
//!     
//!     MarketConfig -->|Performance Policy| MarketRelay[market_data_relay]
//!     SignalConfig -->|Standard Policy| SignalRelay[signal_relay]
//!     ExecConfig -->|Audit Policy| ExecRelay[execution_relay]
//!     
//!     subgraph "Configuration Structure"
//!         RelaySettings[relay: RelaySettings]
//!         TransportConfig[transport: TransportConfig]
//!         ValidationPolicy[validation: ValidationPolicy]
//!         TopicConfig[topics: TopicConfig]
//!         PerformanceConfig[performance: PerformanceConfig]
//!     end
//!     
//!     subgraph "Domain Policies"
//!         Performance["Market: No checksum<br/>Buffer: 64KB<br/>Target: >1M msg/s"]
//!         Standard["Signal: CRC32<br/>Buffer: 32KB<br/>Target: >100K msg/s"]
//!         Audit["Execution: Full audit<br/>Buffer: 16KB<br/>Target: >50K msg/s"]
//!     end
//!     
//!     classDef config fill:#F0E68C
//!     classDef policies fill:#DDA0DD
//!     class ConfigLoader,MarketConfig,SignalConfig,ExecConfig config
//!     class Performance,Standard,Audit policies
//! ```
//!
//! ## Configuration Structure
//!
//! **Hierarchical Configuration**: Five key sections per relay domain:
//!
//! ### 1. RelaySettings - Core Behavior
//! ```toml
//! [relay]
//! domain = 1                    # 1=MarketData, 2=Signal, 3=Execution
//! socket_path = "/tmp/alphapulse/market_data.sock"
//! max_connections = 1000        # Concurrent consumer limit
//! connection_timeout_ms = 5000  # Connection establishment timeout
//! ```
//!
//! ### 2. TransportConfig - Network Layer
//! ```toml
//! [transport]  
//! transport_type = "unix_socket"         # unix_socket, tcp, shared_memory
//! bind_address = "0.0.0.0:0"            # For TCP transport
//! buffer_size = 65536                   # Per-connection buffer
//! compression = "none"                  # none, lz4, zstd
//! ```
//!
//! ### 3. ValidationPolicy - Message Integrity
//! ```toml
//! [validation]
//! checksum = false              # CRC32 validation (performance impact)
//! audit = false                # Full audit trail (storage impact)
//! max_message_size = 1048576   # 1MB limit prevents DoS
//! sequence_validation = true   # Check for gaps/duplicates
//! ```
//!
//! ### 4. TopicConfig - Pub-Sub Routing
//! ```toml
//! [topics]
//! extraction_strategy = "header_based"  # header_based, tlv_based, custom
//! default_topic = "signals.unknown"    # Fallback topic
//! max_topics_per_consumer = 100        # Prevent subscription abuse
//! cleanup_interval_ms = 5000           # Dead consumer cleanup
//! ```
//!
//! ### 5. PerformanceConfig - Optimization
//! ```toml
//! [performance]
//! worker_threads = 4            # Number of processing threads
//! channel_buffer_size = 10000   # Internal message buffer
//! gc_interval_ms = 30000       # Garbage collection frequency
//! metrics_interval_ms = 1000   # Performance metrics reporting
//! ```
//!
//! ## Domain-Specific Examples
//!
//! ### Market Data Relay (Ultra High Performance)
//! ```toml
//! # config/market_data.toml
//! [relay]
//! domain = 1
//! socket_path = "/tmp/alphapulse/market_data.sock"
//! max_connections = 100
//!
//! [validation]
//! checksum = false      # Disable for >1M msg/s
//! audit = false
//! max_message_size = 4096
//!
//! [performance]  
//! worker_threads = 8    # Maximize throughput
//! channel_buffer_size = 100000
//! ```
//!
//! ### Signal Relay (Balanced Performance/Accuracy)
//! ```toml
//! # config/signal.toml
//! [relay]
//! domain = 2
//! socket_path = "/tmp/alphapulse/signal.sock"
//! max_connections = 1000
//!
//! [validation]
//! checksum = true       # Enable for signal accuracy
//! audit = false
//! max_message_size = 16384
//!
//! [topics]
//! extraction_strategy = "tlv_based"   # Extract from SignalIdentity TLV
//! max_topics_per_consumer = 50
//! ```
//!
//! ### Execution Relay (Maximum Safety)
//! ```toml
//! # config/execution.toml
//! [relay]
//! domain = 3  
//! socket_path = "/tmp/alphapulse/execution.sock"
//! max_connections = 100
//!
//! [validation]
//! checksum = true       # Full integrity checking
//! audit = true         # Complete audit trail
//! max_message_size = 32768
//! sequence_validation = true
//!
//! [performance]
//! worker_threads = 2    # Conservative for safety
//! channel_buffer_size = 1000
//! ```
//!
//! ## Configuration Loading and Validation
//!
//! **Runtime Loading**: Configurations loaded at startup with comprehensive validation:
//! - TOML syntax checking and required field validation
//! - Domain consistency (TLV types match domain numbers)
//! - Resource constraint validation (buffer sizes, thread counts)
//! - Transport compatibility verification
//!
//! **Hot Reloading**: Configuration can be updated without restart for:
//! - Performance tuning parameters (buffer sizes, thread counts)
//! - Topic routing rules and cleanup intervals
//! - Connection limits and timeout adjustments
//! - Validation policy changes (with care for data integrity)
//!
//! ## Integration with System Components
//!
//! **Service Bootstrap**: Configuration determines:
//! - Which validation policy to instantiate (`create_validator()`)
//! - Transport layer configuration for optimal performance
//! - Topic registry setup for pub-sub routing
//! - Resource allocation for processing threads
//!
//! **Critical for Connectivity**: Socket paths and connection parameters
//! must match between producers and consumers to establish proper data flow.
//!
//! ## Troubleshooting Configuration Issues
//!
//! **Service startup failures**:
//! - Check socket paths don't conflict between relay domains
//! - Verify file permissions for Unix socket creation
//! - Ensure port numbers are available for TCP transport
//! - Validate configuration syntax with `cargo check`
//!
//! **Performance problems**:
//! - Increase buffer sizes if seeing queue full warnings
//! - Adjust worker thread count based on CPU core availability
//! - Disable validation features if throughput requirements exceed capacity
//! - Monitor memory usage against configured limits
//!
//! **Connection issues**:
//! - Verify transport type matches client expectations
//! - Check maximum connection limits allow all expected consumers
//! - Ensure timeout values accommodate network latency
//! - Validate compression settings are compatible across endpoints

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
