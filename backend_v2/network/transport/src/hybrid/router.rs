//! Transport Router
//!
//! Routes messages to appropriate transports based on configuration,
//! actor requirements, and current network conditions.

use super::config::{ChannelConfig, TransportConfig, TransportMode};
use crate::{Priority, Result};
use std::collections::HashMap;
use tracing::debug;

/// Transport router for hybrid transport
#[derive(Debug, Clone)]
pub struct TransportRouter {
    config: TransportConfig,
    channel_cache: HashMap<String, ChannelConfig>,
}

/// Routing decision for a message
#[derive(Debug, Clone)]
pub enum RouteDecision {
    /// Send via direct network transport
    Direct,
    /// Send via message queue
    #[cfg(feature = "message-queues")]
    MessageQueue { queue_name: String },
    /// Send via transport bridge
    Bridge { target_node: String },
}

impl TransportRouter {
    /// Create new transport router
    pub fn new(config: TransportConfig) -> Self {
        let channel_cache = config.channels.clone();

        Self {
            config,
            channel_cache,
        }
    }

    /// Make routing decision for a message
    pub fn route_decision(
        &self,
        target_node: &str,
        target_actor: &str,
        priority: Priority,
    ) -> Result<RouteDecision> {
        // Check for specific channel configuration
        let channel_key = format!("{}:{}", target_node, target_actor);

        if let Some(channel_config) = self.channel_cache.get(&channel_key) {
            return self.route_with_channel_config(channel_config, priority);
        }

        // Check for actor-level configuration
        if let Some(channel_config) = self.channel_cache.get(target_actor) {
            return self.route_with_channel_config(channel_config, priority);
        }

        // Fall back to default routing
        self.route_with_default_mode(target_node, priority)
    }

    /// Route using specific channel configuration
    fn route_with_channel_config(
        &self,
        channel_config: &ChannelConfig,
        priority: Priority,
    ) -> Result<RouteDecision> {
        match &channel_config.mode {
            TransportMode::Direct => Ok(RouteDecision::Direct),

            TransportMode::MessageQueue => {
                #[cfg(feature = "message-queues")]
                {
                    let queue_name = channel_config
                        .queue_name
                        .clone()
                        .unwrap_or_else(|| "default".to_string());
                    Ok(RouteDecision::MessageQueue { queue_name })
                }
                #[cfg(not(feature = "message-queues"))]
                Ok(RouteDecision::Direct)
            }

            TransportMode::DirectWithMqFallback => {
                // Try direct first, MQ is fallback
                Ok(RouteDecision::Direct)
            }

            TransportMode::MqWithDirectFallback => {
                #[cfg(feature = "message-queues")]
                {
                    let queue_name = channel_config
                        .queue_name
                        .clone()
                        .unwrap_or_else(|| "default".to_string());
                    Ok(RouteDecision::MessageQueue { queue_name })
                }
                #[cfg(not(feature = "message-queues"))]
                Ok(RouteDecision::Direct)
            }

            TransportMode::Auto => {
                // Auto mode: choose based on priority and reliability requirements
                match priority {
                    Priority::Critical => Ok(RouteDecision::Direct),
                    Priority::High => {
                        if channel_config.reliability.requires_guaranteed_delivery() {
                            #[cfg(feature = "message-queues")]
                            {
                                let queue_name = channel_config
                                    .queue_name
                                    .clone()
                                    .unwrap_or_else(|| "high_priority".to_string());
                                Ok(RouteDecision::MessageQueue { queue_name })
                            }
                            #[cfg(not(feature = "message-queues"))]
                            Ok(RouteDecision::Direct)
                        } else {
                            Ok(RouteDecision::Direct)
                        }
                    }
                    Priority::Normal | Priority::Background => {
                        #[cfg(feature = "message-queues")]
                        {
                            let queue_name = channel_config
                                .queue_name
                                .clone()
                                .unwrap_or_else(|| "normal".to_string());
                            Ok(RouteDecision::MessageQueue { queue_name })
                        }
                        #[cfg(not(feature = "message-queues"))]
                        Ok(RouteDecision::Direct)
                    }
                }
            }
        }
    }

    /// Route using default mode
    fn route_with_default_mode(
        &self,
        target_node: &str,
        priority: Priority,
    ) -> Result<RouteDecision> {
        match self.config.default_mode {
            TransportMode::Direct => Ok(RouteDecision::Direct),

            TransportMode::MessageQueue => {
                #[cfg(feature = "message-queues")]
                {
                    let queue_name = format!("node_{}", target_node);
                    Ok(RouteDecision::MessageQueue { queue_name })
                }
                #[cfg(not(feature = "message-queues"))]
                Ok(RouteDecision::Direct)
            }

            TransportMode::DirectWithMqFallback => {
                // Default to direct, MQ is fallback
                Ok(RouteDecision::Direct)
            }

            TransportMode::MqWithDirectFallback => {
                #[cfg(feature = "message-queues")]
                {
                    let queue_name = format!("node_{}", target_node);
                    Ok(RouteDecision::MessageQueue { queue_name })
                }
                #[cfg(not(feature = "message-queues"))]
                Ok(RouteDecision::Direct)
            }

            TransportMode::Auto => {
                // Auto mode with default rules
                match priority {
                    Priority::Critical => Ok(RouteDecision::Direct),
                    _ => {
                        #[cfg(feature = "message-queues")]
                        {
                            let queue_name = format!("node_{}", target_node);
                            Ok(RouteDecision::MessageQueue { queue_name })
                        }
                        #[cfg(not(feature = "message-queues"))]
                        Ok(RouteDecision::Direct)
                    }
                }
            }
        }
    }

    /// Update router configuration
    pub async fn update_config(&mut self, config: TransportConfig) -> Result<()> {
        config.validate()?;
        self.channel_cache = config.channels.clone();
        self.config = config;
        debug!("Transport router configuration updated");
        Ok(())
    }

    /// Check if router is healthy
    pub fn is_healthy(&self) -> bool {
        // Router is healthy if configuration is valid
        self.config.validate().is_ok()
    }

    /// Get current configuration
    pub fn config(&self) -> &TransportConfig {
        &self.config
    }

    /// Get channel configuration for a specific target
    pub fn get_channel_config(
        &self,
        target_node: &str,
        target_actor: &str,
    ) -> Option<&ChannelConfig> {
        let channel_key = format!("{}:{}", target_node, target_actor);
        self.channel_cache
            .get(&channel_key)
            .or_else(|| self.channel_cache.get(target_actor))
    }

    /// Add or update channel configuration
    pub fn set_channel_config(&mut self, key: String, config: ChannelConfig) {
        self.channel_cache.insert(key, config);
    }

    /// Remove channel configuration
    pub fn remove_channel_config(&mut self, key: &str) -> Option<ChannelConfig> {
        self.channel_cache.remove(key)
    }

    /// Get all configured channels
    pub fn list_channels(&self) -> &HashMap<String, ChannelConfig> {
        &self.channel_cache
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Criticality, Reliability};
    use std::collections::HashMap;

    fn create_test_config() -> TransportConfig {
        let mut channels = HashMap::new();

        // Critical channel
        channels.insert(
            "critical_actor".to_string(),
            ChannelConfig {
                mode: TransportMode::Direct,
                criticality: Criticality::Critical,
                reliability: Reliability::BestEffort,
                queue_name: None,
            },
        );

        // Normal channel with message queue
        #[cfg(feature = "message-queues")]
        channels.insert(
            "normal_actor".to_string(),
            ChannelConfig {
                mode: TransportMode::MessageQueue,
                criticality: Criticality::Normal,
                reliability: Reliability::Guaranteed,
                queue_name: Some("normal_queue".to_string()),
            },
        );

        super::config::TransportConfig {
            default_mode: TransportMode::Auto,
            channels,
            routes: Vec::new(),
            enable_bridge: false,
            bridge: super::config::BridgeConfig::default(),
        }
    }

    #[test]
    fn test_router_creation() {
        let config = create_test_config();
        let router = TransportRouter::new(config);
        assert!(router.is_healthy());
    }

    #[test]
    fn test_critical_actor_routing() {
        let config = create_test_config();
        let router = TransportRouter::new(config);

        let decision = router
            .route_decision("node1", "critical_actor", Priority::Critical)
            .unwrap();

        match decision {
            RouteDecision::Direct => {} // Expected
            _ => panic!("Expected direct routing for critical actor"),
        }
    }

    #[cfg(feature = "message-queues")]
    #[test]
    fn test_normal_actor_routing() {
        let config = create_test_config();
        let router = TransportRouter::new(config);

        let decision = router
            .route_decision("node1", "normal_actor", Priority::Normal)
            .unwrap();

        match decision {
            RouteDecision::MessageQueue { queue_name } => {
                assert_eq!(queue_name, "normal_queue");
            }
            _ => panic!("Expected message queue routing for normal actor"),
        }
    }

    #[test]
    fn test_auto_mode_critical_priority() {
        let config = create_test_config();
        let router = TransportRouter::new(config);

        let decision = router
            .route_decision("node1", "unknown_actor", Priority::Critical)
            .unwrap();

        match decision {
            RouteDecision::Direct => {} // Expected for critical priority
            _ => panic!("Expected direct routing for critical priority in auto mode"),
        }
    }

    #[test]
    fn test_channel_config_management() {
        let config = create_test_config();
        let mut router = TransportRouter::new(config);

        // Test getting existing config
        let critical_config = router.get_channel_config("node1", "critical_actor");
        assert!(critical_config.is_some());

        // Test adding new config
        let new_config = ChannelConfig {
            mode: TransportMode::Direct,
            criticality: Criticality::High,
            reliability: Reliability::BestEffort,
            queue_name: None,
        };

        router.set_channel_config("new_actor".to_string(), new_config.clone());

        let retrieved_config = router.get_channel_config("node1", "new_actor");
        assert!(retrieved_config.is_some());
        assert_eq!(retrieved_config.unwrap().criticality, Criticality::High);

        // Test removing config
        let removed = router.remove_channel_config("new_actor");
        assert!(removed.is_some());

        let after_removal = router.get_channel_config("node1", "new_actor");
        assert!(after_removal.is_none());
    }
}
