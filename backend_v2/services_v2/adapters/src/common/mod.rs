//! # Common Adapter Infrastructure
//!
//! Shared trait definitions and utilities for all AlphaPulse adapter implementations.
//! Provides a unified interface for data collection, transformation, and output routing.

use crate::{AdapterError, Result};
use async_trait::async_trait;
use std::collections::HashMap;
use tokio::sync::mpsc;

pub mod auth;
pub mod metrics;

/// Core trait that all AlphaPulse adapters must implement
///
/// This trait defines the standard lifecycle and behavior for data collection
/// adapters, ensuring consistent interfaces across all exchange integrations.
#[async_trait]
pub trait Adapter: Send + Sync {
    /// Adapter configuration type
    type Config: Send + Sync + Clone;

    /// Start the adapter data collection process
    ///
    /// This method should:
    /// 1. Establish connections to external data sources
    /// 2. Begin continuous data collection
    /// 3. Handle automatic reconnection on failures
    /// 4. Transform raw data into Protocol V2 TLV messages
    /// 5. Route messages to appropriate relay domains
    async fn start(&self) -> Result<()>;

    /// Stop the adapter gracefully
    ///
    /// Should cleanly close all connections and stop background tasks
    async fn stop(&self) -> Result<()>;

    /// Get adapter health status
    ///
    /// Returns connection status, error counts, and performance metrics
    async fn health_check(&self) -> AdapterHealth;

    /// Get adapter configuration
    fn config(&self) -> &Self::Config;

    /// Get adapter identifier (unique name for this adapter instance)
    fn identifier(&self) -> &str;

    /// Get supported instrument types for this adapter
    fn supported_instruments(&self) -> Vec<InstrumentType>;

    /// Configure instruments to collect data for
    ///
    /// # Arguments
    /// * `instruments` - List of instruments to subscribe to
    async fn configure_instruments(&mut self, instruments: Vec<String>) -> Result<()>;

    /// Process a raw message from the external source
    ///
    /// This is the core transformation function that converts
    /// raw exchange data into Protocol V2 TLV messages
    async fn process_message(&self, raw_data: &[u8]) -> Result<Vec<Vec<u8>>>;
}

/// Health status information for an adapter
#[derive(Debug, Clone)]
pub struct AdapterHealth {
    /// Whether the adapter is connected and operating normally
    pub is_healthy: bool,

    /// Connection status to external data source
    pub connection_status: ConnectionStatus,

    /// Number of messages processed successfully
    pub messages_processed: u64,

    /// Number of errors encountered
    pub error_count: u64,

    /// Last error message if any
    pub last_error: Option<String>,

    /// Uptime since last restart
    pub uptime_seconds: u64,

    /// Current latency metrics
    pub latency_ms: Option<f64>,
}

/// Connection status for external data sources
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionStatus {
    /// Connected and receiving data
    Connected,

    /// Attempting to connect
    Connecting,

    /// Connection lost, attempting to reconnect
    Reconnecting,

    /// Disconnected (intentionally or due to unrecoverable error)
    Disconnected,
}

/// Types of financial instruments an adapter can handle
#[derive(Debug, Clone, PartialEq)]
pub enum InstrumentType {
    /// Cryptocurrency spot trading pairs
    CryptoSpot,

    /// Cryptocurrency futures contracts
    CryptoFutures,

    /// Decentralized exchange liquidity pools
    DexPools,

    /// Traditional stock equities
    Equities,

    /// Foreign exchange pairs
    Forex,

    /// Options contracts
    Options,
}

/// Standard adapter configuration parameters
#[derive(Debug, Clone)]
pub struct BaseAdapterConfig {
    /// Unique identifier for this adapter instance
    pub adapter_id: String,

    /// API credentials for external service access
    pub credentials: Option<HashMap<String, String>>,

    /// Connection timeout in milliseconds
    pub connection_timeout_ms: u64,

    /// Reconnection attempt interval in milliseconds
    pub reconnect_interval_ms: u64,

    /// Maximum number of reconnection attempts
    pub max_reconnect_attempts: u32,

    /// Circuit breaker configuration
    pub circuit_breaker_enabled: bool,

    /// Rate limiting configuration
    pub rate_limit_requests_per_second: Option<u32>,

    /// Enable detailed metrics collection
    pub metrics_enabled: bool,

    /// Output channel capacity for message buffering
    pub output_channel_capacity: usize,
}

impl Default for BaseAdapterConfig {
    fn default() -> Self {
        Self {
            adapter_id: "default_adapter".to_string(),
            credentials: None,
            connection_timeout_ms: 30000,
            reconnect_interval_ms: 5000,
            max_reconnect_attempts: 10,
            circuit_breaker_enabled: true,
            rate_limit_requests_per_second: None,
            metrics_enabled: true,
            output_channel_capacity: 10000,
        }
    }
}

/// Factory trait for creating adapter instances
pub trait AdapterFactory<A: Adapter> {
    /// Create a new adapter instance with the given configuration
    fn create_adapter(config: A::Config) -> Result<A>;

    /// Validate adapter configuration before creation
    fn validate_config(config: &A::Config) -> Result<()>;
}

/// Trait for adapters that support live configuration updates
#[async_trait]
pub trait ConfigurableAdapter: Adapter {
    /// Update adapter configuration without restarting
    async fn update_config(&mut self, config: Self::Config) -> Result<()>;

    /// Add instruments to existing subscription list
    async fn add_instruments(&mut self, instruments: Vec<String>) -> Result<()>;

    /// Remove instruments from subscription list
    async fn remove_instruments(&mut self, instruments: Vec<String>) -> Result<()>;
}

/// Standard output interface for sending processed messages
#[async_trait]
pub trait AdapterOutput: Send + Sync {
    /// Send a single Protocol V2 message
    async fn send_message(&self, message: Vec<u8>) -> Result<()>;

    /// Send multiple messages in batch for efficiency
    async fn send_batch(&self, messages: Vec<Vec<u8>>) -> Result<()>;

    /// Check if output channel is ready to accept messages
    fn is_ready(&self) -> bool;

    /// Get the current queue depth for backpressure monitoring
    fn queue_depth(&self) -> usize;
}

/// Default implementation of AdapterOutput using mpsc channel
pub struct ChannelOutput {
    sender: mpsc::Sender<Vec<u8>>,
}

impl ChannelOutput {
    /// Create new channel output with the given sender
    pub fn new(sender: mpsc::Sender<Vec<u8>>) -> Self {
        Self { sender }
    }
}

#[async_trait]
impl AdapterOutput for ChannelOutput {
    async fn send_message(&self, message: Vec<u8>) -> Result<()> {
        self.sender
            .send(message)
            .await
            .map_err(|e| AdapterError::ConfigError(format!("Failed to send message: {}", e)))?;
        Ok(())
    }

    async fn send_batch(&self, messages: Vec<Vec<u8>>) -> Result<()> {
        for message in messages {
            self.send_message(message).await?;
        }
        Ok(())
    }

    fn is_ready(&self) -> bool {
        !self.sender.is_closed()
    }

    fn queue_depth(&self) -> usize {
        // Note: mpsc::Sender doesn't expose queue depth directly
        // This would require a custom implementation or metrics collection
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base_adapter_config_default() {
        let config = BaseAdapterConfig::default();

        assert_eq!(config.adapter_id, "default_adapter");
        assert_eq!(config.connection_timeout_ms, 30000);
        assert_eq!(config.max_reconnect_attempts, 10);
        assert!(config.circuit_breaker_enabled);
        assert!(config.metrics_enabled);
    }

    #[test]
    fn test_connection_status_types() {
        let statuses = vec![
            ConnectionStatus::Connected,
            ConnectionStatus::Connecting,
            ConnectionStatus::Reconnecting,
            ConnectionStatus::Disconnected,
        ];

        for status in statuses {
            // Ensure all status types can be constructed and compared
            assert!(matches!(
                status,
                ConnectionStatus::Connected
                    | ConnectionStatus::Connecting
                    | ConnectionStatus::Reconnecting
                    | ConnectionStatus::Disconnected
            ));
        }
    }
}
