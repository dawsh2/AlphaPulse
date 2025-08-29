//! Service configuration and defaults
//!
//! This module contains default configuration values and constants
//! used across Torq services for consistency.

/// Dashboard service defaults
pub mod dashboard {
    /// Zero address constant for unknown/uninitialized values
    pub const ZERO_ADDRESS: &str = "0x0000000000000000000000000000000000000000";
    
    /// Default values for incomplete arbitrage opportunities
    pub const DEFAULT_PAIR: &str = "UNKNOWN-PAIR";
    // Note: DEX names should come from actual pool discovery or signal data
}

/// Adapter service defaults
pub mod adapters {
    /// Connection timeout (milliseconds)
    pub const CONNECTION_TIMEOUT_MS: u64 = 30_000;
    
    /// Reconnection backoff base (milliseconds)
    pub const RECONNECTION_BACKOFF_BASE_MS: u64 = 1_000;
    
    /// Maximum reconnection attempts
    pub const MAX_RECONNECTION_ATTEMPTS: usize = 10;
    
    /// Circuit breaker threshold (errors before opening)
    pub const CIRCUIT_BREAKER_THRESHOLD: usize = 5;
    
    /// Rate limit (requests per second)
    pub const DEFAULT_RATE_LIMIT_RPS: f64 = 10.0;
}

/// Strategy service defaults  
pub mod strategies {
    /// Flash arbitrage strategy ID
    pub const FLASH_ARBITRAGE_STRATEGY_ID: u16 = 21;
    
    /// Signal output queue size
    pub const SIGNAL_OUTPUT_QUEUE_SIZE: usize = 1000;
    
    // Note: Profit thresholds should be configurable per strategy instance
    // not hardcoded as global constants
}

/// Relay service defaults
pub mod relays {
    /// Unix socket buffer size
    pub const SOCKET_BUFFER_SIZE: usize = 65536;
    
    /// Message queue size
    pub const MESSAGE_QUEUE_SIZE: usize = 10000;
    
    /// Consumer heartbeat interval (milliseconds)
    pub const HEARTBEAT_INTERVAL_MS: u64 = 30_000;
}