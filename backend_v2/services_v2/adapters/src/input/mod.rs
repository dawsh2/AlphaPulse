//! Input adapters for collecting market data from external sources

pub mod collectors;
pub mod components;
pub mod connection;
pub mod state_manager;

use crate::Result;
use async_trait::async_trait;
use alphapulse_types::{InstrumentId, VenueId};

pub use connection::{ConnectionManager, ConnectionState};
pub use state_manager::StateManager;

/// Core trait for all input adapters
#[async_trait]
pub trait InputAdapter: Send + Sync {
    /// Get the venue this adapter connects to
    fn venue(&self) -> VenueId;

    /// Start the adapter and begin collecting data
    async fn start(&mut self) -> Result<()>;

    /// Stop the adapter gracefully
    async fn stop(&mut self) -> Result<()>;

    /// Check if adapter is currently connected
    fn is_connected(&self) -> bool;

    /// Get list of instruments being tracked
    fn tracked_instruments(&self) -> Vec<InstrumentId>;

    /// Subscribe to specific instruments (if supported)
    async fn subscribe(&mut self, instruments: Vec<InstrumentId>) -> Result<()>;

    /// Unsubscribe from instruments
    async fn unsubscribe(&mut self, instruments: Vec<InstrumentId>) -> Result<()>;

    /// Force reconnection
    async fn reconnect(&mut self) -> Result<()>;

    /// Get adapter health status
    async fn health_check(&self) -> HealthStatus;
}

/// Health status for an input adapter
#[derive(Debug, Clone)]
pub struct HealthStatus {
    /// Overall health level
    pub level: HealthLevel,
    /// Connection state
    pub connection: ConnectionState,
    /// Messages received in last minute
    pub messages_per_minute: u64,
    /// Last message timestamp (nanoseconds since epoch)
    pub last_message_time: Option<u64>,
    /// Number of tracked instruments
    pub instrument_count: usize,
    /// Recent error count
    pub error_count: u64,
    /// Additional venue-specific details
    pub details: Option<String>,
}

/// Health level enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthLevel {
    /// Everything working normally
    Healthy,
    /// Some issues but still functional
    Degraded,
    /// Major issues, not functional
    Unhealthy,
}

impl HealthStatus {
    /// Create a healthy status
    pub fn healthy(connection: ConnectionState, messages_per_minute: u64) -> Self {
        Self {
            level: HealthLevel::Healthy,
            connection,
            messages_per_minute,
            last_message_time: Some(current_nanos()),
            instrument_count: 0,
            error_count: 0,
            details: None,
        }
    }

    /// Create an unhealthy status
    pub fn unhealthy(connection: ConnectionState, reason: String) -> Self {
        Self {
            level: HealthLevel::Unhealthy,
            connection,
            messages_per_minute: 0,
            last_message_time: None,
            instrument_count: 0,
            error_count: 0,
            details: Some(reason),
        }
    }

    /// Check if status indicates problems
    pub fn has_issues(&self) -> bool {
        self.level != HealthLevel::Healthy
    }
}

/// Get current time in nanoseconds since epoch (protocol-consistent)
fn current_nanos() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64
}
