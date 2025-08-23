//! Hybrid Transport Module
//!
//! Provides hybrid transport capabilities that can route messages through
//! either direct network transport or message queues based on configuration
//! and channel requirements.

pub mod config;
pub mod router;
pub mod bridge;

// Re-export main types
pub use config::{TransportConfig, TransportMode, ChannelConfig, RouteConfig};
pub use router::{TransportRouter, RouteDecision};
pub use bridge::{TransportBridge, BridgeConfig};

use crate::{TransportError, Result, Transport, Priority, TransportStatistics};
use async_trait::async_trait;
use std::collections::HashMap;
use tokio::sync::RwLock;

/// Hybrid transport that combines direct and message queue transport
pub struct HybridTransport {
    config: TransportConfig,
    router: TransportRouter,
    bridge: Option<TransportBridge>,
    direct_transport: Option<Box<dyn Transport>>,
    #[cfg(feature = "message-queues")]
    mq_transport: Option<Box<dyn crate::mq::MessageQueueTransport>>,
    statistics: RwLock<TransportStatistics>,
}

impl HybridTransport {
    /// Create new hybrid transport
    pub async fn new(config: TransportConfig) -> Result<Self> {
        config.validate()?;

        let router = TransportRouter::new(config.clone());
        let bridge = if config.enable_bridge {
            // Convert from config::BridgeConfig to bridge::BridgeConfig
            let bridge_config = bridge::BridgeConfig {
                max_queue_size: config.bridge.buffer_size,
                enable_deduplication: true,
                message_ttl_seconds: 60,
                retry_attempts: 3,
                retry_delay_ms: 100,
            };
            Some(TransportBridge::new(bridge_config).await?)
        } else {
            None
        };

        Ok(Self {
            config,
            router,
            bridge,
            direct_transport: None,
            #[cfg(feature = "message-queues")]
            mq_transport: None,
            statistics: RwLock::new(TransportStatistics::default()),
        })
    }

    /// Initialize direct network transport
    pub async fn init_direct_transport(&mut self, transport: Box<dyn Transport>) -> Result<()> {
        self.direct_transport = Some(transport);
        Ok(())
    }

    /// Initialize message queue transport
    #[cfg(feature = "message-queues")]
    pub async fn init_mq_transport(&mut self, transport: Box<dyn crate::mq::MessageQueueTransport>) -> Result<()> {
        self.mq_transport = Some(transport);
        Ok(())
    }

    /// Route message through appropriate transport
    async fn route_message(
        &self,
        target_node: &str,
        target_actor: &str,
        message: &[u8],
        priority: Priority,
    ) -> Result<()> {
        // Get routing decision
        let decision = self.router.route_decision(target_node, target_actor, priority)?;

        match decision {
            RouteDecision::Direct => {
                if let Some(ref transport) = self.direct_transport {
                    transport.send_with_priority(target_node, target_actor, message, priority).await?;
                } else {
                    return Err(TransportError::configuration(
                        "Direct transport not initialized",
                        Some("direct_transport")
                    ));
                }
            }
            
            #[cfg(feature = "message-queues")]
            RouteDecision::MessageQueue { queue_name } => {
                if let Some(ref transport) = self.mq_transport {
                    transport.publish(&queue_name, message, priority).await?;
                } else {
                    return Err(TransportError::configuration(
                        "Message queue transport not initialized",
                        Some("mq_transport")
                    ));
                }
            }
            
            RouteDecision::Bridge { .. } => {
                if let Some(ref bridge) = self.bridge {
                    bridge.forward_message(target_node, target_actor, message, priority).await?;
                } else {
                    return Err(TransportError::configuration(
                        "Transport bridge not initialized",
                        Some("bridge")
                    ));
                }
            }
        }

        // Update statistics
        {
            let mut stats = self.statistics.write().await;
            stats.messages_sent += 1;
            stats.bytes_sent += message.len() as u64;
        }

        Ok(())
    }

    /// Get configuration
    pub fn config(&self) -> &TransportConfig {
        &self.config
    }

    /// Get router
    pub fn router(&self) -> &TransportRouter {
        &self.router
    }

    /// Update routing configuration
    pub async fn update_config(&mut self, config: TransportConfig) -> Result<()> {
        config.validate()?;
        self.router.update_config(config.clone()).await?;
        self.config = config;
        Ok(())
    }

    /// Get transport health status
    pub fn health_status(&self) -> HybridTransportHealth {
        HybridTransportHealth {
            direct_healthy: self.direct_transport.as_ref()
                .map(|t| t.is_healthy())
                .unwrap_or(false),
            
            #[cfg(feature = "message-queues")]
            mq_healthy: self.mq_transport.as_ref()
                .map(|t| t.is_healthy())
                .unwrap_or(false),
            
            #[cfg(not(feature = "message-queues"))]
            mq_healthy: false,
            
            bridge_healthy: self.bridge.as_ref()
                .map(|b| b.is_healthy())
                .unwrap_or(true), // If no bridge, consider healthy
            
            router_healthy: self.router.is_healthy(),
        }
    }
}

#[async_trait]
impl Transport for HybridTransport {
    async fn start(&mut self) -> Result<()> {
        tracing::info!("Starting hybrid transport");

        // Start direct transport if available
        if let Some(ref mut transport) = self.direct_transport {
            transport.start().await?;
        }

        // Start message queue transport if available
        #[cfg(feature = "message-queues")]
        if let Some(ref mut transport) = self.mq_transport {
            transport.start().await?;
        }

        // Start bridge if available
        if let Some(ref mut bridge) = self.bridge {
            bridge.start().await?;
        }

        tracing::info!("Hybrid transport started successfully");
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        tracing::info!("Stopping hybrid transport");

        // Stop bridge first
        if let Some(ref mut bridge) = self.bridge {
            bridge.stop().await?;
        }

        // Stop message queue transport
        #[cfg(feature = "message-queues")]
        if let Some(ref mut transport) = self.mq_transport {
            transport.stop().await?;
        }

        // Stop direct transport
        if let Some(ref mut transport) = self.direct_transport {
            transport.stop().await?;
        }

        tracing::info!("Hybrid transport stopped successfully");
        Ok(())
    }

    async fn send_to_actor(
        &self,
        target_node: &str,
        target_actor: &str,
        message: &[u8],
    ) -> Result<()> {
        self.route_message(target_node, target_actor, message, Priority::Normal).await
    }

    async fn send_with_priority(
        &self,
        target_node: &str,
        target_actor: &str,
        message: &[u8],
        priority: Priority,
    ) -> Result<()> {
        self.route_message(target_node, target_actor, message, priority).await
    }

    fn is_healthy(&self) -> bool {
        let health = self.health_status();
        health.router_healthy && 
        (health.direct_healthy || health.mq_healthy) &&
        health.bridge_healthy
    }

    fn statistics(&self) -> TransportStatistics {
        // This is a blocking call, but statistics() trait method is not async
        // In a real implementation, we might use a different approach
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let stats = self.statistics.read().await;
                
                // Combine with underlying transport statistics
                let mut combined = stats.clone();
                
                if let Some(ref transport) = self.direct_transport {
                    let direct_stats = transport.statistics();
                    combined.messages_sent += direct_stats.messages_sent;
                    combined.bytes_sent += direct_stats.bytes_sent;
                    combined.active_connections += direct_stats.active_connections;
                }
                
                combined
            })
        })
    }
}

/// Health status for hybrid transport
#[derive(Debug, Clone)]
pub struct HybridTransportHealth {
    /// Direct transport health
    pub direct_healthy: bool,
    /// Message queue transport health
    pub mq_healthy: bool,
    /// Bridge health
    pub bridge_healthy: bool,
    /// Router health
    pub router_healthy: bool,
}

impl HybridTransportHealth {
    /// Check if any transport is healthy
    pub fn any_healthy(&self) -> bool {
        self.direct_healthy || self.mq_healthy
    }

    /// Check if all transports are healthy
    pub fn all_healthy(&self) -> bool {
        self.direct_healthy && self.mq_healthy && self.bridge_healthy && self.router_healthy
    }

    /// Get health score (0.0 - 1.0)
    pub fn health_score(&self) -> f64 {
        let mut score = 0.0;
        let mut total = 0.0;

        if self.direct_healthy { score += 0.4; }
        total += 0.4;

        if self.mq_healthy { score += 0.3; }
        total += 0.3;

        if self.bridge_healthy { score += 0.2; }
        total += 0.2;

        if self.router_healthy { score += 0.1; }
        total += 0.1;

        if total > 0.0 { score / total } else { 0.0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Criticality, Reliability};

    #[test]
    fn test_transport_config_validation() {
        let config = TransportConfig {
            default_mode: TransportMode::Auto,
            channels: HashMap::new(),
            routes: Vec::new(),
            enable_bridge: false,
            bridge: BridgeConfig::default(),
        };

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_health_status() {
        let health = HybridTransportHealth {
            direct_healthy: true,
            mq_healthy: false,
            bridge_healthy: true,
            router_healthy: true,
        };

        assert!(health.any_healthy());
        assert!(!health.all_healthy());
        assert!(health.health_score() > 0.5);
        assert!(health.health_score() < 1.0);
    }

    #[test]
    fn test_all_healthy_transport() {
        let health = HybridTransportHealth {
            direct_healthy: true,
            mq_healthy: true,
            bridge_healthy: true,
            router_healthy: true,
        };

        assert!(health.all_healthy());
        assert_eq!(health.health_score(), 1.0);
    }

    #[test]
    fn test_no_healthy_transport() {
        let health = HybridTransportHealth {
            direct_healthy: false,
            mq_healthy: false,
            bridge_healthy: false,
            router_healthy: false,
        };

        assert!(!health.any_healthy());
        assert!(!health.all_healthy());
        assert_eq!(health.health_score(), 0.0);
    }
}