//! Topology Integration Module
//!
//! Integrates the transport system with the AlphaPulse topology system
//! for automatic transport selection and configuration based on actor
//! placement and channel requirements.

pub mod resolver;
pub mod factory;

// Re-export main types
pub use factory::{TopologyTransportResolver, TransportResolution, TransportCriteria, TransportFactory};

use crate::{TransportError, Result, Transport};
use crate::hybrid::{TransportConfig, ChannelConfig};
use alphapulse_topology::{TopologyConfig, Actor, Node};
use alphapulse_topology::resolution::TopologyResolver as AlphaPulseTopologyResolver;
use std::sync::Arc;

/// Integration layer between topology and transport systems
pub struct TopologyIntegration {
    topology_resolver: Arc<AlphaPulseTopologyResolver>,
    transport_resolver: TopologyTransportResolver,
    factory: TransportFactory,
    transport_config: TransportConfig,
}

impl TopologyIntegration {
    /// Create new topology integration
    pub async fn new(
        topology_config: TopologyConfig,
        transport_config: TransportConfig,
    ) -> Result<Self> {
        transport_config.validate()?;

        let topology_resolver = Arc::new(AlphaPulseTopologyResolver::new(topology_config));
        // Create our own transport resolver
        let transport_resolver = TopologyTransportResolver::new(60);
        let factory = TransportFactory::new(Arc::new(transport_resolver.clone()));

        Ok(Self {
            topology_resolver,
            transport_resolver,
            factory,
            transport_config,
        })
    }

    /// Resolve optimal transport for communication between two actors
    pub async fn resolve_transport(
        &self,
        source_actor: &str,
        target_actor: &str,
        channel: &str,
    ) -> Result<TransportResolution> {
        self.transport_resolver
            .resolve_transport(source_actor, target_actor, channel)
            .await
    }

    /// Create transport instance based on resolution
    pub async fn create_transport(
        &self,
        resolution: &TransportResolution,
    ) -> Result<Box<dyn Transport>> {
        self.factory.create_transport(&resolution.node_id).await
    }

    /// Get complete transport configuration for an actor
    pub fn get_actor_transport_config(&self, actor_id: &str) -> Result<ActorTransportConfig> {
        let actor = self.topology_resolver
            .get_actor(actor_id)
            .ok_or_else(|| TransportError::topology(format!("Actor not found: {}", actor_id)))?;

        let node = self.topology_resolver
            .get_actor_node_object(actor_id)
            .ok_or_else(|| TransportError::topology(format!("Node not found for actor: {}", actor_id)))?;

        // Determine transport configuration based on actor placement and requirements
        let mut channels = Vec::new();
        
        // Add input channels
        for input_channel in &actor.inputs {
            let channel_config = self.transport_config.get_channel_config(input_channel);
            channels.push(ChannelTransportConfig {
                name: input_channel.to_string(),
                direction: ChannelDirection::Input,
                config: channel_config,
                transport_mode: self.determine_channel_transport_mode(&actor, &node, input_channel)?,
            });
        }

        // Add output channels
        for output_channel in &actor.outputs {
            let channel_config = self.transport_config.get_channel_config(output_channel);
            channels.push(ChannelTransportConfig {
                name: output_channel.to_string(),
                direction: ChannelDirection::Output,
                config: channel_config,
                transport_mode: self.determine_channel_transport_mode(&actor, &node, output_channel)?,
            });
        }

        Ok(ActorTransportConfig {
            actor_id: actor_id.to_string(),
            node_id: node.hostname.clone(),
            numa_node: self.get_actor_numa_node(actor_id)?,
            channels,
            security_requirements: self.determine_security_requirements(&actor, &node)?,
        })
    }

    /// Update transport configuration
    pub async fn update_config(&mut self, config: TransportConfig) -> Result<()> {
        config.validate()?;
        self.transport_resolver.update_config(config.clone()).await?;
        self.factory.update_config(config.clone()).await?;
        self.transport_config = config;
        Ok(())
    }

    /// Get topology resolver
    pub fn topology_resolver(&self) -> &AlphaPulseTopologyResolver {
        &self.topology_resolver
    }

    /// Get transport resolver
    pub fn transport_resolver(&self) -> &TopologyTransportResolver {
        &self.transport_resolver
    }

    /// Get transport factory
    pub fn factory(&self) -> &TransportFactory {
        &self.factory
    }

    // Private helper methods

    fn determine_channel_transport_mode(
        &self,
        actor: &Actor,
        node: &Node,
        channel: &str,
    ) -> Result<crate::hybrid::TransportMode> {
        let channel_config = self.transport_config.get_channel_config(channel);
        
        // If explicitly configured, use that mode
        if !matches!(channel_config.mode, crate::hybrid::TransportMode::Auto) {
            return Ok(channel_config.mode);
        }

        // Auto-select based on requirements
        match (channel_config.criticality, channel_config.reliability) {
            (crate::Criticality::UltraLowLatency, _) => {
                Ok(crate::hybrid::TransportMode::Direct)
            }
            (_, crate::Reliability::GuaranteedDelivery) => {
                Ok(crate::hybrid::TransportMode::MessageQueue)
            }
            (crate::Criticality::LowLatency, crate::Reliability::AtLeastOnce) => {
                Ok(crate::hybrid::TransportMode::DirectWithMqFallback)
            }
            _ => {
                Ok(crate::hybrid::TransportMode::Direct)
            }
        }
    }

    fn get_actor_numa_node(&self, actor_id: &str) -> Result<Option<u8>> {
        if let Some(placement) = self.topology_resolver.get_actor_placement(actor_id) {
            Ok(placement.numa)
        } else {
            Ok(None)
        }
    }

    fn determine_security_requirements(
        &self,
        actor: &Actor,
        node: &Node,
    ) -> Result<SecurityRequirements> {
        // Determine security requirements based on actor type and node configuration
        let encryption_required = match actor.actor_type {
            alphapulse_topology::ActorType::Producer => {
                // Producers handling sensitive market data may need encryption
                false // Default to no encryption for performance
            }
            alphapulse_topology::ActorType::Transformer => {
                // Transformers doing trading logic may need encryption
                true // Default to encryption for trading strategies
            }
            alphapulse_topology::ActorType::Consumer => {
                // Consumers writing to external systems may need encryption
                true // Default to encryption for external communication
            }
        };

        Ok(SecurityRequirements {
            encryption_required,
            tls_required: false, // Use application-layer encryption instead
            authentication_required: true,
            authorized_nodes: vec![node.hostname.clone()],
        })
    }
}

/// Transport configuration for a specific actor
#[derive(Debug, Clone)]
pub struct ActorTransportConfig {
    /// Actor identifier
    pub actor_id: String,
    /// Node where actor is placed
    pub node_id: String,
    /// NUMA node for optimization
    pub numa_node: Option<u8>,
    /// Channel configurations
    pub channels: Vec<ChannelTransportConfig>,
    /// Security requirements
    pub security_requirements: SecurityRequirements,
}

/// Transport configuration for a specific channel
#[derive(Debug, Clone)]
pub struct ChannelTransportConfig {
    /// Channel name
    pub name: String,
    /// Channel direction relative to actor
    pub direction: ChannelDirection,
    /// Channel configuration
    pub config: ChannelConfig,
    /// Resolved transport mode
    pub transport_mode: crate::hybrid::TransportMode,
}

/// Channel direction relative to an actor
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelDirection {
    /// Actor receives messages from this channel
    Input,
    /// Actor sends messages to this channel
    Output,
}

/// Security requirements for actor communication
#[derive(Debug, Clone)]
pub struct SecurityRequirements {
    /// Whether encryption is required
    pub encryption_required: bool,
    /// Whether TLS is required
    pub tls_required: bool,
    /// Whether authentication is required
    pub authentication_required: bool,
    /// List of authorized nodes
    pub authorized_nodes: Vec<String>,
}

/// Helper trait for topology-aware transport creation
#[async_trait::async_trait]
pub trait TopologyAwareTransport: Transport {
    /// Initialize transport with topology context
    async fn init_with_topology(
        &mut self,
        actor_config: &ActorTransportConfig,
        topology_resolver: &AlphaPulseTopologyResolver,
    ) -> Result<()>;

    /// Update actor placement (for migration)
    async fn update_placement(
        &mut self,
        new_node: &str,
        new_numa_node: Option<u8>,
    ) -> Result<()>;

    /// Get current placement info
    fn placement_info(&self) -> Option<PlacementInfo>;
}

/// Actor placement information
#[derive(Debug, Clone)]
pub struct PlacementInfo {
    /// Current node
    pub node_id: String,
    /// Current NUMA node
    pub numa_node: Option<u8>,
    /// Last update timestamp
    pub last_updated: std::time::Instant,
}

/// Integration builder for easy setup
pub struct TopologyIntegrationBuilder {
    topology_config: Option<TopologyConfig>,
    transport_config: Option<TransportConfig>,
}

impl TopologyIntegrationBuilder {
    /// Create new builder
    pub fn new() -> Self {
        Self {
            topology_config: None,
            transport_config: None,
        }
    }

    /// Set topology configuration
    pub fn with_topology_config(mut self, config: TopologyConfig) -> Self {
        self.topology_config = Some(config);
        self
    }

    /// Set transport configuration
    pub fn with_transport_config(mut self, config: TransportConfig) -> Self {
        self.transport_config = Some(config);
        self
    }

    /// Load topology configuration from file
    pub async fn load_topology_config(mut self, path: &str) -> Result<Self> {
        let config = TopologyConfig::from_file(path)
            .map_err(|e| TransportError::topology_with_source(
                "Failed to load topology configuration",
                e
            ))?;
        self.topology_config = Some(config);
        Ok(self)
    }

    /// Load transport configuration from file
    pub async fn load_transport_config(mut self, path: &str) -> Result<Self> {
        let yaml = tokio::fs::read_to_string(path).await
            .map_err(|e| TransportError::configuration(
                format!("Failed to read transport config file: {}", e),
                Some("config_file")
            ))?;
        
        let config = TransportConfig::from_yaml(&yaml)?;
        self.transport_config = Some(config);
        Ok(self)
    }

    /// Build the integration
    pub async fn build(self) -> Result<TopologyIntegration> {
        let topology_config = self.topology_config
            .ok_or_else(|| TransportError::configuration(
                "Topology configuration required",
                Some("topology_config")
            ))?;

        let transport_config = self.transport_config
            .unwrap_or_default();

        TopologyIntegration::new(topology_config, transport_config).await
    }
}

impl Default for TopologyIntegrationBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alphapulse_topology::{Actor, ActorType};
    use std::collections::HashMap;

    #[test]
    fn test_builder_pattern() {
        let builder = TopologyIntegrationBuilder::new()
            .with_transport_config(TransportConfig::default());
            
        // Builder should be ready to build (with default topology config)
        assert!(builder.transport_config.is_some());
    }

    #[test]
    fn test_actor_transport_config() {
        let config = ActorTransportConfig {
            actor_id: "test_actor".to_string(),
            node_id: "node1".to_string(),
            numa_node: Some(0),
            channels: vec![
                ChannelTransportConfig {
                    name: "input_channel".to_string(),
                    direction: ChannelDirection::Input,
                    config: ChannelConfig::default(),
                    transport_mode: crate::hybrid::TransportMode::Direct,
                },
                ChannelTransportConfig {
                    name: "output_channel".to_string(),
                    direction: ChannelDirection::Output,
                    config: ChannelConfig::default(),
                    transport_mode: crate::hybrid::TransportMode::MessageQueue,
                },
            ],
            security_requirements: SecurityRequirements {
                encryption_required: true,
                tls_required: false,
                authentication_required: true,
                authorized_nodes: vec!["node1".to_string()],
            },
        };

        assert_eq!(config.actor_id, "test_actor");
        assert_eq!(config.channels.len(), 2);
        assert!(config.security_requirements.encryption_required);
    }

    #[test]
    fn test_channel_direction() {
        assert_eq!(ChannelDirection::Input, ChannelDirection::Input);
        assert_ne!(ChannelDirection::Input, ChannelDirection::Output);
    }

    #[test]
    fn test_security_requirements() {
        let security = SecurityRequirements {
            encryption_required: false,
            tls_required: true,
            authentication_required: true,
            authorized_nodes: vec!["node1".to_string(), "node2".to_string()],
        };

        assert!(!security.encryption_required);
        assert!(security.tls_required);
        assert_eq!(security.authorized_nodes.len(), 2);
    }

    #[test]
    fn test_placement_info() {
        let placement = PlacementInfo {
            node_id: "test_node".to_string(),
            numa_node: Some(1),
            last_updated: std::time::Instant::now(),
        };

        assert_eq!(placement.node_id, "test_node");
        assert_eq!(placement.numa_node, Some(1));
    }
}