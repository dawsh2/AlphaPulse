//! # Validation Configuration Module
//!
//! Provides configurable validation parameters to avoid hardcoded values
//! and enable deployment-specific tuning.

use std::time::Duration;
use serde::{Deserialize, Serialize};

/// Validation configuration for different deployment environments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConfig {
    /// Maximum message sizes per domain (in bytes)
    pub max_message_sizes: DomainMessageLimits,
    
    /// Timestamp validation parameters
    pub timestamp: TimestampConfig,
    
    /// Sequence validation parameters
    pub sequence: SequenceConfig,
    
    /// Pool discovery configuration
    pub pool_discovery: PoolDiscoveryConfig,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            max_message_sizes: DomainMessageLimits::default(),
            timestamp: TimestampConfig::default(),
            sequence: SequenceConfig::default(),
            pool_discovery: PoolDiscoveryConfig::default(),
        }
    }
}

/// Message size limits per domain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainMessageLimits {
    /// Market data domain message size limit
    pub market_data: usize,
    /// Signal domain message size limit
    pub signal: usize,
    /// Execution domain message size limit
    pub execution: usize,
    /// System domain message size limit
    pub system: usize,
}

impl Default for DomainMessageLimits {
    fn default() -> Self {
        Self {
            market_data: 4096,    // 4KB for high-frequency market data
            signal: 8192,         // 8KB for signal messages
            execution: 16384,     // 16KB for execution messages
            system: 32768,        // 32KB for system messages
        }
    }
}

/// Timestamp validation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimestampConfig {
    /// Maximum allowed timestamp drift into the future (prevents timestamp manipulation)
    pub max_future_drift: Duration,
    
    /// Maximum allowed timestamp age (prevents replay of old messages)
    pub max_age: Duration,
    
    /// Whether to enforce strict timestamp validation
    pub enforce_validation: bool,
}

impl Default for TimestampConfig {
    fn default() -> Self {
        Self {
            max_future_drift: Duration::from_secs(5),  // 5 seconds future tolerance
            max_age: Duration::from_secs(60),          // Messages older than 60s rejected
            enforce_validation: true,
        }
    }
}

/// Sequence number validation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequenceConfig {
    /// Maximum allowed gap in sequence numbers before considering it an error
    pub max_sequence_gap: u64,
    
    /// Whether to enforce monotonic sequence numbers
    pub enforce_monotonic: bool,
    
    /// Time window for duplicate detection (in seconds)
    pub duplicate_window: Duration,
    
    /// Maximum number of sequence numbers to track per source
    pub max_tracked_sequences: usize,
}

impl Default for SequenceConfig {
    fn default() -> Self {
        Self {
            max_sequence_gap: 100,  // Allow up to 100 message gap
            enforce_monotonic: true,
            duplicate_window: Duration::from_secs(300), // 5 minute window
            max_tracked_sequences: 10000,
        }
    }
}

/// Pool discovery queue configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolDiscoveryConfig {
    /// Maximum number of pending pool discovery requests
    pub max_queue_size: usize,
    
    /// Timeout for pool discovery RPC calls
    pub rpc_timeout: Duration,
    
    /// Maximum concurrent RPC calls
    pub max_concurrent_rpcs: usize,
    
    /// Cache TTL for discovered pools
    pub cache_ttl: Duration,
    
    /// Enable pool discovery queue
    pub enabled: bool,
}

impl Default for PoolDiscoveryConfig {
    fn default() -> Self {
        Self {
            max_queue_size: 1000,
            rpc_timeout: Duration::from_secs(5),
            max_concurrent_rpcs: 10,
            cache_ttl: Duration::from_secs(3600), // 1 hour cache
            enabled: true,
        }
    }
}

impl ValidationConfig {
    /// Load configuration from environment variables with fallback to defaults
    pub fn from_env() -> Self {
        let mut config = Self::default();
        
        // Override from environment if set
        if let Ok(val) = std::env::var("TORQ_MAX_MESSAGE_SIZE_MARKET") {
            if let Ok(size) = val.parse() {
                config.max_message_sizes.market_data = size;
            }
        }
        
        if let Ok(val) = std::env::var("TORQ_MAX_MESSAGE_SIZE_SIGNAL") {
            if let Ok(size) = val.parse() {
                config.max_message_sizes.signal = size;
            }
        }
        
        if let Ok(val) = std::env::var("TORQ_MAX_MESSAGE_SIZE_EXECUTION") {
            if let Ok(size) = val.parse() {
                config.max_message_sizes.execution = size;
            }
        }
        
        if let Ok(val) = std::env::var("TORQ_TIMESTAMP_MAX_DRIFT") {
            if let Ok(secs) = val.parse() {
                config.timestamp.max_future_drift = Duration::from_secs(secs);
            }
        }
        
        if let Ok(val) = std::env::var("TORQ_SEQUENCE_MAX_GAP") {
            if let Ok(gap) = val.parse() {
                config.sequence.max_sequence_gap = gap;
            }
        }
        
        config
    }
    
    /// Create a production configuration with stricter limits
    pub fn production() -> Self {
        Self {
            max_message_sizes: DomainMessageLimits {
                market_data: 2048,   // Smaller for performance
                signal: 4096,
                execution: 8192,
                system: 16384,
            },
            timestamp: TimestampConfig {
                max_future_drift: Duration::from_secs(2),  // Tighter tolerance
                max_age: Duration::from_secs(30),          // Stricter age limit
                enforce_validation: true,
            },
            sequence: SequenceConfig {
                max_sequence_gap: 50,  // Tighter gap tolerance
                enforce_monotonic: true,
                duplicate_window: Duration::from_secs(600), // 10 minutes
                max_tracked_sequences: 50000,
            },
            pool_discovery: PoolDiscoveryConfig {
                max_queue_size: 5000,
                rpc_timeout: Duration::from_secs(3),
                max_concurrent_rpcs: 20,
                cache_ttl: Duration::from_secs(7200), // 2 hour cache
                enabled: true,
            },
        }
    }
    
    /// Create a development configuration with relaxed limits
    pub fn development() -> Self {
        Self {
            max_message_sizes: DomainMessageLimits {
                market_data: 8192,
                signal: 16384,
                execution: 32768,
                system: 65536,
            },
            timestamp: TimestampConfig {
                max_future_drift: Duration::from_secs(30),
                max_age: Duration::from_secs(300),
                enforce_validation: false,  // Relaxed for development
            },
            sequence: SequenceConfig {
                max_sequence_gap: 1000,
                enforce_monotonic: false,  // Relaxed for development
                duplicate_window: Duration::from_secs(60),
                max_tracked_sequences: 1000,
            },
            pool_discovery: PoolDiscoveryConfig {
                max_queue_size: 100,
                rpc_timeout: Duration::from_secs(10),
                max_concurrent_rpcs: 5,
                cache_ttl: Duration::from_secs(600),
                enabled: true,
            },
        }
    }
}