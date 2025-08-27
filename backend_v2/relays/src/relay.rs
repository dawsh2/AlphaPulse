//! Generic relay implementation configured per domain

use crate::{
    create_transport_from_config, InfraTransportAdapter, MessageValidator, RelayConfig, RelayError,
    RelayResult, TopicRegistry, TransportAdapterConfig,
};
use alphapulse_codec::{parse_header, ProtocolError};
use alphapulse_types::protocol::{MessageHeader, RelayDomain};
use bytes::Bytes;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Generic relay implementation
pub struct Relay {
    /// Configuration for this relay
    config: RelayConfig,
    /// Topic registry for pub-sub routing
    topics: Arc<RwLock<TopicRegistry>>,
    /// Message validator based on domain
    validator: Box<dyn MessageValidator>,
    /// Transport layer (to be integrated with infra/transport)
    transport: Option<Box<dyn Transport>>,
    /// Performance metrics
    metrics: RelayMetrics,
}

/// Transport trait (placeholder until infra/transport integration)
#[async_trait::async_trait]
pub trait Transport: Send + Sync {
    async fn start(&mut self) -> RelayResult<()>;
    async fn stop(&mut self) -> RelayResult<()>;
    async fn receive(&mut self) -> RelayResult<Bytes>;
    async fn send(&mut self, data: &[u8], consumers: &[ConsumerId]) -> RelayResult<()>;
}

/// Consumer identifier
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct ConsumerId(pub String);

/// Relay performance metrics
#[derive(Debug, Default)]
pub struct RelayMetrics {
    pub messages_received: u64,
    pub messages_routed: u64,
    pub messages_dropped: u64,
    pub validation_failures: u64,
    pub routing_errors: u64,
}

impl Relay {
    /// Create a new relay from configuration
    pub async fn new(config: RelayConfig) -> RelayResult<Self> {
        info!(
            "Initializing relay: {} (domain {})",
            config.relay.name, config.relay.domain
        );

        // Create topic registry
        let topics = Arc::new(RwLock::new(TopicRegistry::new(&config.topics)?));

        // Create validator based on domain
        let validator = crate::validation::create_validator(&config.validation);

        Ok(Self {
            config,
            topics,
            validator,
            transport: None, // Will be set in init_transport()
            metrics: RelayMetrics::default(),
        })
    }

    /// Create relay from configuration file
    pub async fn from_config_file(path: &str) -> RelayResult<Self> {
        let config = RelayConfig::from_file(path)?;
        Self::new(config).await
    }

    /// Initialize transport (integrated with infra/transport)
    pub async fn init_transport(&mut self) -> RelayResult<()> {
        info!("Initializing transport: {}", self.config.transport.mode);

        // Create transport adapter configuration
        let adapter_config = create_transport_from_config(&self.config);

        // Create the transport adapter
        let adapter = InfraTransportAdapter::new(adapter_config).await?;
        self.transport = Some(Box::new(adapter));

        info!("Transport adapter initialized successfully");
        Ok(())
    }

    /// Start the relay
    pub async fn start(&mut self) -> RelayResult<()> {
        info!("Starting relay: {}", self.config.relay.name);

        // Initialize transport if not already done
        if self.transport.is_none() {
            self.init_transport().await?;
        }

        // Start transport
        if let Some(transport) = &mut self.transport {
            transport.start().await?;
        }

        // Start message processing loop
        self.run_message_loop().await
    }

    /// Main message processing loop
    async fn run_message_loop(&mut self) -> RelayResult<()> {
        info!("Relay {} entering message loop", self.config.relay.name);

        loop {
            // Receive message from transport
            let message_bytes = match self.receive_message().await {
                Ok(bytes) => bytes,
                Err(e) => {
                    error!("Failed to receive message: {}", e);
                    self.metrics.messages_dropped += 1;
                    continue;
                }
            };

            // Parse header
            let header = match parse_header(&message_bytes) {
                Ok(h) => h,
                Err(e) => {
                    debug!("Failed to parse header: {}", e);
                    self.metrics.messages_dropped += 1;
                    continue;
                }
            };

            // Check if message is for this relay's domain
            if !self.is_my_domain(header.relay_domain) {
                debug!("Ignoring message for domain {}", header.relay_domain);
                continue;
            }

            // Validate message based on policy
            if let Err(e) = self.validator.validate(header, &message_bytes) {
                warn!("Validation failed: {}", e);
                self.metrics.validation_failures += 1;
                if self.config.validation.strict {
                    continue; // Drop invalid messages in strict mode
                }
            }

            // Route message to topic subscribers
            match self.route_message(header, &message_bytes).await {
                Ok(count) => {
                    self.metrics.messages_routed += count as u64;
                    debug!("Routed message to {} consumers", count);
                }
                Err(e) => {
                    error!("Routing failed: {}", e);
                    self.metrics.routing_errors += 1;
                }
            }

            self.metrics.messages_received += 1;

            // Log metrics periodically
            if self.metrics.messages_received % 10000 == 0 {
                self.log_metrics();
            }
        }
    }

    /// Receive message from transport
    async fn receive_message(&mut self) -> RelayResult<Bytes> {
        if let Some(transport) = &mut self.transport {
            transport.receive().await
        } else {
            Err(RelayError::Transport(
                "Transport not initialized".to_string(),
            ))
        }
    }

    /// Check if message is for this relay's domain
    fn is_my_domain(&self, domain: u8) -> bool {
        domain == self.config.relay.domain
    }

    /// Route message to appropriate consumers
    async fn route_message(&mut self, header: &MessageHeader, data: &[u8]) -> RelayResult<usize> {
        // Extract topic from message
        let topics = self.topics.read().await;
        let topic =
            topics.extract_topic(header, Some(data), &self.config.topics.extraction_strategy)?;

        // Get subscribers for this topic
        let consumers = topics.get_subscribers(&topic);

        if consumers.is_empty() {
            debug!("No subscribers for topic: {}", topic);
            return Ok(0);
        }

        // Send to all subscribers
        if let Some(transport) = &mut self.transport {
            transport.send(data, &consumers).await?;
        }

        Ok(consumers.len())
    }

    /// Log performance metrics
    fn log_metrics(&self) {
        info!(
            "Relay {} metrics - Received: {}, Routed: {}, Dropped: {}, Validation Failures: {}, Routing Errors: {}",
            self.config.relay.name,
            self.metrics.messages_received,
            self.metrics.messages_routed,
            self.metrics.messages_dropped,
            self.metrics.validation_failures,
            self.metrics.routing_errors
        );
    }

    /// Stop the relay
    pub async fn stop(&mut self) -> RelayResult<()> {
        info!("Stopping relay: {}", self.config.relay.name);

        if let Some(transport) = &mut self.transport {
            transport.stop().await?;
        }

        self.log_metrics();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_relay_creation() {
        let config = RelayConfig::market_data_defaults();
        let relay = Relay::new(config).await.unwrap();
        assert_eq!(relay.config.relay.domain, 1);
    }
}
