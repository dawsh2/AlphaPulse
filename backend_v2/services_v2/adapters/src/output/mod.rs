//! Output adapters for sending TLV messages to various destinations
//!
//! Output adapters handle the delivery of processed market data, signals, and execution
//! messages to downstream consumers such as:
//! - Strategy engines
//! - Risk management systems  
//! - External APIs
//! - Database storage
//! - Message queues

pub mod relay_output;

pub use relay_output::{RelayOutput, RelayOutputStats};

use crate::{AdapterError, Result};
use async_trait::async_trait;
use protocol_v2::{InstrumentId, VenueId};

/// Output adapter trait for sending TLV messages
#[async_trait]
pub trait OutputAdapter: Send + Sync {
    /// Get the output adapter type
    fn adapter_type(&self) -> &str;

    /// Start the output adapter
    async fn start(&mut self) -> Result<()>;

    /// Stop the output adapter
    async fn stop(&mut self) -> Result<()>;

    /// Send a single TLV message
    async fn send(&self, message: Vec<u8>) -> Result<()>;

    /// Send multiple TLV messages
    async fn send_batch(&self, messages: Vec<Vec<u8>>) -> Result<()> {
        for message in messages {
            self.send(message).await?;
        }
        Ok(())
    }

    /// Check if the adapter is ready to send messages
    fn is_ready(&self) -> bool;

    /// Get current buffer size (if applicable)
    fn buffer_size(&self) -> usize {
        0
    }
}

/// Output adapter manager for coordinating multiple outputs
pub struct OutputManager {
    adapters: Vec<Box<dyn OutputAdapter>>,
}

impl OutputManager {
    /// Create new output manager
    pub fn new() -> Self {
        Self {
            adapters: Vec::new(),
        }
    }

    /// Add an output adapter
    pub fn add_adapter(&mut self, adapter: Box<dyn OutputAdapter>) {
        self.adapters.push(adapter);
    }

    /// Start all adapters
    pub async fn start_all(&mut self) -> Result<()> {
        for adapter in &mut self.adapters {
            adapter.start().await?;
        }
        Ok(())
    }

    /// Stop all adapters
    pub async fn stop_all(&mut self) -> Result<()> {
        for adapter in &mut self.adapters {
            adapter.stop().await?;
        }
        Ok(())
    }

    /// Send message to all ready adapters
    pub async fn broadcast(&self, message: Vec<u8>) -> Result<()> {
        for adapter in &self.adapters {
            if adapter.is_ready() {
                if let Err(e) = adapter.send(message.clone()).await {
                    tracing::warn!("Failed to send to {}: {}", adapter.adapter_type(), e);
                }
            }
        }
        Ok(())
    }
}

impl Default for OutputManager {
    fn default() -> Self {
        Self::new()
    }
}
