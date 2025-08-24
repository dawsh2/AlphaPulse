//! Transport Bridge
//!
//! Bridges between different transport types, allowing messages to flow
//! between direct network connections and message queues.

use crate::{Priority, Result, TransportError};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, info, warn};

/// Configuration for transport bridge
#[derive(Debug, Clone, Default)]
pub struct BridgeConfig {
    /// Maximum queue size for buffered messages
    pub max_queue_size: usize,
    /// Enable message deduplication
    pub enable_deduplication: bool,
    /// Message TTL in seconds
    pub message_ttl_seconds: u64,
    /// Retry configuration
    pub retry_attempts: u32,
    pub retry_delay_ms: u64,
}

/// Transport bridge for hybrid transport
pub struct TransportBridge {
    config: BridgeConfig,
    /// Message buffer for bridging
    message_buffer: Arc<RwLock<MessageBuffer>>,
    /// Bridge worker handle
    worker_handle: Option<tokio::task::JoinHandle<()>>,
    /// Shutdown signal
    shutdown_tx: Option<mpsc::Sender<()>>,
}

/// Internal message buffer
struct MessageBuffer {
    messages: Vec<BridgedMessage>,
    dedup_cache: HashMap<u64, std::time::Instant>,
}

/// Message being bridged
#[derive(Debug, Clone)]
struct BridgedMessage {
    target_node: String,
    target_actor: String,
    message: Vec<u8>,
    priority: Priority,
    timestamp: std::time::Instant,
    retry_count: u32,
}

impl TransportBridge {
    /// Create new transport bridge
    pub async fn new(config: BridgeConfig) -> Result<Self> {
        let message_buffer = Arc::new(RwLock::new(MessageBuffer {
            messages: Vec::with_capacity(config.max_queue_size),
            dedup_cache: HashMap::new(),
        }));

        Ok(Self {
            config,
            message_buffer,
            worker_handle: None,
            shutdown_tx: None,
        })
    }

    /// Start the bridge worker
    pub async fn start(&mut self) -> Result<()> {
        if self.worker_handle.is_some() {
            return Ok(()); // Already started
        }

        info!("Starting transport bridge");

        let (shutdown_tx, mut shutdown_rx) = mpsc::channel(1);
        self.shutdown_tx = Some(shutdown_tx);

        let buffer = self.message_buffer.clone();
        let config = self.config.clone();

        // Start bridge worker task
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(100));

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        // Process buffered messages
                        Self::process_messages(&buffer, &config).await;
                    }
                    _ = shutdown_rx.recv() => {
                        info!("Transport bridge shutting down");
                        break;
                    }
                }
            }
        });

        self.worker_handle = Some(handle);
        info!("Transport bridge started successfully");
        Ok(())
    }

    /// Stop the bridge worker
    pub async fn stop(&mut self) -> Result<()> {
        info!("Stopping transport bridge");

        // Send shutdown signal
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(()).await;
        }

        // Wait for worker to finish
        if let Some(handle) = self.worker_handle.take() {
            let _ = handle.await;
        }

        // Process any remaining messages
        Self::flush_messages(&self.message_buffer, &self.config).await;

        info!("Transport bridge stopped successfully");
        Ok(())
    }

    /// Forward a message through the bridge
    pub async fn forward_message(
        &self,
        target_node: &str,
        target_actor: &str,
        message: &[u8],
        priority: Priority,
    ) -> Result<()> {
        // Check if bridge is running
        if self.worker_handle.is_none() {
            return Err(TransportError::transport(
                "Bridge not started",
                Some("forward_message"),
            ));
        }

        // Check queue size
        {
            let buffer = self.message_buffer.read().await;
            if buffer.messages.len() >= self.config.max_queue_size {
                return Err(TransportError::transport(
                    "Bridge buffer full",
                    Some("forward_message"),
                ));
            }
        }

        // Add message to buffer
        let bridged_message = BridgedMessage {
            target_node: target_node.to_string(),
            target_actor: target_actor.to_string(),
            message: message.to_vec(),
            priority,
            timestamp: std::time::Instant::now(),
            retry_count: 0,
        };

        {
            let mut buffer = self.message_buffer.write().await;

            // Check for duplicates if enabled
            if self.config.enable_deduplication {
                let msg_hash = Self::calculate_hash(message);

                // Check if we've seen this message recently
                if let Some(last_seen) = buffer.dedup_cache.get(&msg_hash) {
                    if last_seen.elapsed().as_secs() < self.config.message_ttl_seconds {
                        debug!("Dropping duplicate message");
                        return Ok(());
                    }
                }

                // Update dedup cache
                buffer
                    .dedup_cache
                    .insert(msg_hash, std::time::Instant::now());

                // Clean old entries
                buffer
                    .dedup_cache
                    .retain(|_, time| time.elapsed().as_secs() < self.config.message_ttl_seconds);
            }

            buffer.messages.push(bridged_message);
        }

        Ok(())
    }

    /// Process buffered messages
    async fn process_messages(buffer: &Arc<RwLock<MessageBuffer>>, config: &BridgeConfig) {
        let messages_to_process = {
            let mut buffer = buffer.write().await;

            // Remove expired messages
            let ttl = std::time::Duration::from_secs(config.message_ttl_seconds);
            buffer.messages.retain(|msg| msg.timestamp.elapsed() < ttl);

            // Sort by priority (higher priority first)
            buffer.messages.sort_by(|a, b| b.priority.cmp(&a.priority));

            // Take messages to process
            let count = std::cmp::min(buffer.messages.len(), 100);
            buffer.messages.drain(..count).collect::<Vec<_>>()
        };

        // Process each message
        for mut msg in messages_to_process {
            // In a real implementation, this would forward to the actual transport
            // For now, we just simulate processing
            debug!(
                "Processing bridged message to {}:{} (priority: {:?})",
                msg.target_node, msg.target_actor, msg.priority
            );

            // Simulate processing with possible failure
            let success = true; // In real implementation, try actual forwarding

            if !success && msg.retry_count < config.retry_attempts {
                // Retry failed message
                msg.retry_count += 1;

                // Add back to buffer with delay
                tokio::time::sleep(std::time::Duration::from_millis(config.retry_delay_ms)).await;

                let mut buffer = buffer.write().await;
                buffer.messages.push(msg);
            }
        }
    }

    /// Flush all remaining messages
    async fn flush_messages(buffer: &Arc<RwLock<MessageBuffer>>, config: &BridgeConfig) {
        let remaining_count = {
            let buffer = buffer.read().await;
            buffer.messages.len()
        };

        if remaining_count > 0 {
            warn!(
                "Flushing {} remaining messages from bridge",
                remaining_count
            );
            Self::process_messages(buffer, config).await;
        }
    }

    /// Calculate hash for deduplication
    fn calculate_hash(message: &[u8]) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        message.hash(&mut hasher);
        hasher.finish()
    }

    /// Check if bridge is healthy
    pub fn is_healthy(&self) -> bool {
        self.worker_handle
            .as_ref()
            .map(|h| !h.is_finished())
            .unwrap_or(false)
    }

    /// Get buffer statistics
    pub async fn statistics(&self) -> BridgeStatistics {
        let buffer = self.message_buffer.read().await;

        BridgeStatistics {
            buffered_messages: buffer.messages.len(),
            dedup_cache_size: buffer.dedup_cache.len(),
            is_running: self.is_healthy(),
        }
    }
}

/// Bridge statistics
#[derive(Debug, Clone)]
pub struct BridgeStatistics {
    /// Number of messages currently buffered
    pub buffered_messages: usize,
    /// Size of deduplication cache
    pub dedup_cache_size: usize,
    /// Whether bridge is running
    pub is_running: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_bridge_creation() {
        let config = BridgeConfig {
            max_queue_size: 1000,
            enable_deduplication: true,
            message_ttl_seconds: 60,
            retry_attempts: 3,
            retry_delay_ms: 100,
        };

        let bridge = TransportBridge::new(config).await.unwrap();
        assert!(!bridge.is_healthy()); // Not started yet
    }

    #[tokio::test]
    async fn test_bridge_start_stop() {
        let config = BridgeConfig {
            max_queue_size: 1000,
            enable_deduplication: false,
            message_ttl_seconds: 60,
            retry_attempts: 3,
            retry_delay_ms: 100,
        };

        let mut bridge = TransportBridge::new(config).await.unwrap();

        // Start bridge
        bridge.start().await.unwrap();
        assert!(bridge.is_healthy());

        // Stop bridge
        bridge.stop().await.unwrap();
        assert!(!bridge.is_healthy());
    }

    #[tokio::test]
    async fn test_message_forwarding() {
        let config = BridgeConfig {
            max_queue_size: 1000,
            enable_deduplication: false,
            message_ttl_seconds: 60,
            retry_attempts: 3,
            retry_delay_ms: 100,
        };

        let mut bridge = TransportBridge::new(config).await.unwrap();
        bridge.start().await.unwrap();

        // Forward a message
        let result = bridge
            .forward_message("node1", "actor1", b"test message", Priority::Normal)
            .await;

        assert!(result.is_ok());

        // Check statistics
        let stats = bridge.statistics().await;
        assert!(stats.is_running);

        bridge.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_deduplication() {
        let config = BridgeConfig {
            max_queue_size: 1000,
            enable_deduplication: true,
            message_ttl_seconds: 60,
            retry_attempts: 3,
            retry_delay_ms: 100,
        };

        let mut bridge = TransportBridge::new(config).await.unwrap();
        bridge.start().await.unwrap();

        // Send same message twice
        let message = b"duplicate message";

        bridge
            .forward_message("node1", "actor1", message, Priority::Normal)
            .await
            .unwrap();
        bridge
            .forward_message("node1", "actor1", message, Priority::Normal)
            .await
            .unwrap();

        // Second message should be deduplicated
        let stats = bridge.statistics().await;
        assert_eq!(stats.dedup_cache_size, 1);

        bridge.stop().await.unwrap();
    }
}
