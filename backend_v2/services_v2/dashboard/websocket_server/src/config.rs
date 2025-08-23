//! Dashboard server configuration

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardConfig {
    /// WebSocket server bind address
    pub bind_address: String,

    /// WebSocket server port
    pub port: u16,

    /// Market data relay path
    pub market_data_relay_path: String,

    /// Signal relay path
    pub signal_relay_path: String,

    /// Execution relay path
    pub execution_relay_path: String,

    /// Maximum number of concurrent WebSocket connections
    pub max_connections: usize,

    /// Message buffer size per client
    pub client_buffer_size: usize,

    /// Enable CORS for web browsers
    pub enable_cors: bool,

    /// Heartbeat interval in seconds
    pub heartbeat_interval_secs: u64,
}

impl Default for DashboardConfig {
    fn default() -> Self {
        Self {
            bind_address: "127.0.0.1".to_string(),
            port: 8080,
            market_data_relay_path: "/tmp/alphapulse/market_data.sock".to_string(),
            signal_relay_path: "/tmp/alphapulse/signals.sock".to_string(),
            execution_relay_path: "/tmp/alphapulse/execution.sock".to_string(),
            max_connections: 1000,
            client_buffer_size: 1000,
            enable_cors: true,
            heartbeat_interval_secs: 30,
        }
    }
}
