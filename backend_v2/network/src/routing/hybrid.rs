//! Transport Router
//!
//! Routes messages to appropriate transports based on configuration,
//! actor requirements, and current network conditions.

use super::config::{ChannelConfig, TransportConfig, TransportMode};
use super::{RouterConfig, RoutingDecision, Router, RouteRequest, RouterStats, LatencyRequirement, ReliabilityRequirement};
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
// Remove RoutingDecision enum - use RoutingDecision instead

impl TransportRouter {
    /// Create new transport router
    pub fn new(router_config: RouterConfig) -> Result<Self> {
        let transport_config = router_config.transport_config.unwrap_or_default();
        let channel_cache = transport_config.channels.clone();

        Ok(Self {
            config: transport_config,
            channel_cache,
        })
    }

    /// Make routing decision for a message
    pub fn route_decision(
        &self,
        target_node: &str,
        target_actor: &str,
        priority: Priority,
    ) -> Result<RoutingDecision> {
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
    ) -> Result<RoutingDecision> {
        match &channel_config.mode {
            TransportMode::Direct => Ok(RoutingDecision::UnixSocket { socket_path: "/tmp/transport".to_string(), connection_pool: true }),

            TransportMode::MessageQueue => {
                #[cfg(feature = "message-queues")]
                {
                    let queue_name = format!("queue_{}", channel_config.name);
                    Ok(RoutingDecision::MessageQueue { queue_name, exchange: None, routing_key: None })
                }
                #[cfg(not(feature = "message-queues"))]
                Ok(RoutingDecision::UnixSocket { socket_path: "/tmp/transport".to_string(), connection_pool: true })
            }

            TransportMode::DirectWithMqFallback => {
                // Try direct first, MQ is fallback
                Ok(RoutingDecision::UnixSocket { socket_path: "/tmp/transport".to_string(), connection_pool: true })
            }

            TransportMode::MqWithDirectFallback => {
                #[cfg(feature = "message-queues")]
                {
                    let queue_name = format!("queue_{}", channel_config.name);
                    Ok(RoutingDecision::MessageQueue { queue_name, exchange: None, routing_key: None })
                }
                #[cfg(not(feature = "message-queues"))]
                Ok(RoutingDecision::UnixSocket { socket_path: "/tmp/transport".to_string(), connection_pool: true })
            }

            TransportMode::Auto => {
                // Auto mode: choose based on priority and reliability requirements
                match priority {
                    Priority::Critical => Ok(RoutingDecision::UnixSocket { socket_path: "/tmp/transport".to_string(), connection_pool: true }),
                    Priority::High => {
                        // High priority: prefer direct transport for speed
                        Ok(RoutingDecision::UnixSocket { socket_path: "/tmp/transport".to_string(), connection_pool: true })
                    }
                    Priority::Normal | Priority::Background => {
                        #[cfg(feature = "message-queues")]
                        {
                            let queue_name = format!("queue_{}", channel_config.name);
                            Ok(RoutingDecision::MessageQueue { queue_name, exchange: None, routing_key: None })
                        }
                        #[cfg(not(feature = "message-queues"))]
                        Ok(RoutingDecision::UnixSocket { socket_path: "/tmp/transport".to_string(), connection_pool: true })
                    }
                }
            }
        }
    }

    /// Route using default mode
    fn route_with_default_mode(
        &self,
        _target_node: &str,
        priority: Priority,
    ) -> Result<RoutingDecision> {
        match self.config.default_mode {
            TransportMode::Direct => Ok(RoutingDecision::UnixSocket { socket_path: "/tmp/transport".to_string(), connection_pool: true }),

            TransportMode::MessageQueue => {
                #[cfg(feature = "message-queues")]
                {
                    let queue_name = format!("node_{}", _target_node);
                    Ok(RoutingDecision::MessageQueue { queue_name, exchange: None, routing_key: None })
                }
                #[cfg(not(feature = "message-queues"))]
                Ok(RoutingDecision::UnixSocket { socket_path: "/tmp/transport".to_string(), connection_pool: true })
            }

            TransportMode::DirectWithMqFallback => {
                // Default to direct, MQ is fallback
                Ok(RoutingDecision::UnixSocket { socket_path: "/tmp/transport".to_string(), connection_pool: true })
            }

            TransportMode::MqWithDirectFallback => {
                #[cfg(feature = "message-queues")]
                {
                    let queue_name = format!("node_{}", _target_node);
                    Ok(RoutingDecision::MessageQueue { queue_name, exchange: None, routing_key: None })
                }
                #[cfg(not(feature = "message-queues"))]
                Ok(RoutingDecision::UnixSocket { socket_path: "/tmp/transport".to_string(), connection_pool: true })
            }

            TransportMode::Auto => {
                // Auto mode with default rules
                match priority {
                    Priority::Critical => Ok(RoutingDecision::UnixSocket { socket_path: "/tmp/transport".to_string(), connection_pool: true }),
                    _ => {
                        #[cfg(feature = "message-queues")]
                        {
                            let queue_name = format!("node_{}", _target_node);
                            Ok(RoutingDecision::MessageQueue { queue_name, exchange: None, routing_key: None })
                        }
                        #[cfg(not(feature = "message-queues"))]
                        Ok(RoutingDecision::UnixSocket { socket_path: "/tmp/transport".to_string(), connection_pool: true })
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

impl Router for TransportRouter {
    fn route(&self, request: &RouteRequest) -> Result<RoutingDecision> {
        self.route_decision(&request.target_node, &request.target_actor, request.priority)
    }

    fn update_config(&mut self, config: RouterConfig) -> Result<()> {
        if let Some(transport_config) = config.transport_config {
            self.config = transport_config.clone();
            self.channel_cache = transport_config.channels;
        }
        Ok(())
    }

    fn is_healthy(&self) -> bool {
        // Basic health check - can be extended
        true
    }

    fn stats(&self) -> RouterStats {
        RouterStats {
            total_routes: 0,
            local_routes: 0,
            unix_socket_routes: 0,
            tcp_routes: 0,
            udp_routes: 0,
            failed_routes: 0,
            average_decision_time_ns: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::config::{ChannelConfig, TransportConfig, TransportMode};
    use std::collections::HashMap;

    fn create_test_config() -> TransportConfig {
        let mut channels = HashMap::new();

        // Critical channel
        channels.insert(
            "critical_actor".to_string(),
            ChannelConfig {
                name: "critical_actor".to_string(),
                mode: TransportMode::Direct,
                max_message_size: Some(1024 * 1024),
                priority: Some(Priority::High),
                buffer_size: Some(1024),
            },
        );

        // Normal channel with message queue
        #[cfg(feature = "message-queues")]
        channels.insert(
            "normal_actor".to_string(),
            ChannelConfig {
                name: "normal_actor".to_string(),
                mode: TransportMode::MessageQueue,
                max_message_size: Some(1024 * 1024),
                priority: Some(Priority::Normal),
                buffer_size: Some(2048),
            },
        );

        TransportConfig {
            default_mode: TransportMode::Auto,
            channels,
            global_max_message_size: 16 * 1024 * 1024,
            global_buffer_size: 64 * 1024,
            connection_timeout_secs: 30,
            retry_config: super::config::RetryConfig::default(),
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
            RoutingDecision::UnixSocket { socket_path, connection_pool: true } if socket_path == "/tmp/transport" => {} // Expected
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
            RoutingDecision::MessageQueue { queue_name, exchange: None, routing_key: None } => {
                assert_eq!(queue_name, "queue_normal_actor");
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
            RoutingDecision::UnixSocket { socket_path, connection_pool: true } if socket_path == "/tmp/transport" => {} // Expected for critical priority
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
            name: "new_actor".to_string(),
            mode: TransportMode::Direct,
            criticality: Criticality::Standard,
            reliability: Reliability::BestEffort,
            default_priority: crate::Priority::Normal,
            max_message_size: 1024 * 1024, // 1MB
            timeout: std::time::Duration::from_secs(30),
            retry: crate::hybrid::config::RetryConfig {
                max_attempts: 3,
                initial_delay: std::time::Duration::from_millis(100),
                max_delay: std::time::Duration::from_secs(60),
                backoff_multiplier: 2.0,
                jitter: true,
            },
            circuit_breaker: None,
            #[cfg(feature = "message-queues")]
            mq_config: None,
        };

        router.set_channel_config("new_actor".to_string(), new_config.clone());

        let retrieved_config = router.get_channel_config("node1", "new_actor");
        assert!(retrieved_config.is_some());
        assert_eq!(retrieved_config.unwrap().criticality, Criticality::Standard);

        // Test removing config
        let removed = router.remove_channel_config("new_actor");
        assert!(removed.is_some());

        let after_removal = router.get_channel_config("node1", "new_actor");
        assert!(after_removal.is_none());
    }
}
