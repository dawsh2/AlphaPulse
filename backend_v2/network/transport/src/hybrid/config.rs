//! Hybrid Transport Configuration
//!
//! Configuration structures for hybrid transport that supports both
//! direct network transport and message queue routing.

use crate::{Criticality, Priority, Reliability};
use crate::{Result, TransportError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Main transport configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportConfig {
    /// Default transport mode for unconfigured channels
    pub default_mode: TransportMode,
    /// Per-channel transport configuration
    pub channels: HashMap<String, ChannelConfig>,
    /// Routing rules for specific node pairs
    pub routes: Vec<RouteConfig>,
    /// Enable transport bridge for protocol conversion
    pub enable_bridge: bool,
    /// Bridge configuration
    pub bridge: BridgeConfig,
}

/// Transport mode selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransportMode {
    /// Automatic selection based on requirements
    Auto,
    /// Direct network transport only
    Direct,
    /// Message queue transport only
    MessageQueue,
    /// Try direct first, fallback to message queue
    DirectWithMqFallback,
    /// Try message queue first, fallback to direct
    MqWithDirectFallback,
}

/// Channel-specific transport configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelConfig {
    /// Channel name/identifier
    pub name: String,
    /// Transport mode for this channel
    pub mode: TransportMode,
    /// Message criticality level
    pub criticality: Criticality,
    /// Reliability requirements
    pub reliability: Reliability,
    /// Default message priority
    pub default_priority: Priority,
    /// Maximum message size in bytes
    pub max_message_size: usize,
    /// Message timeout
    pub timeout: Duration,
    /// Retry configuration
    pub retry: RetryConfig,
    /// Circuit breaker configuration
    pub circuit_breaker: Option<CircuitBreakerConfig>,
    /// Message queue specific settings
    #[cfg(feature = "message-queues")]
    pub mq_config: Option<MessageQueueChannelConfig>,
}

/// Route configuration for specific node pairs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteConfig {
    /// Source node pattern (supports wildcards)
    pub source_pattern: String,
    /// Target node pattern (supports wildcards)
    pub target_pattern: String,
    /// Channels this route applies to (empty = all)
    pub channels: Vec<String>,
    /// Forced transport mode for this route
    pub transport_mode: TransportMode,
    /// Route priority (higher = more specific)
    pub priority: u32,
    /// Additional route metadata
    pub metadata: HashMap<String, String>,
}

/// Retry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum retry attempts
    pub max_attempts: u32,
    /// Initial retry delay
    pub initial_delay: Duration,
    /// Maximum retry delay
    pub max_delay: Duration,
    /// Backoff multiplier
    pub backoff_multiplier: f64,
    /// Jitter to add to delays
    pub jitter: bool,
}

/// Circuit breaker configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    /// Failure threshold to open circuit
    pub failure_threshold: u32,
    /// Success threshold to close circuit
    pub success_threshold: u32,
    /// Timeout before trying to close circuit
    pub timeout: Duration,
    /// Half-open state max concurrent calls
    pub half_open_max_calls: u32,
}

/// Message queue channel configuration
#[cfg(feature = "message-queues")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageQueueChannelConfig {
    /// Queue name template
    pub queue_name_template: String,
    /// Exchange name (for RabbitMQ)
    pub exchange: Option<String>,
    /// Routing key template
    pub routing_key_template: Option<String>,
    /// Message persistence
    pub persistent: bool,
    /// Message TTL
    pub ttl: Option<Duration>,
    /// Dead letter queue
    pub dead_letter_queue: Option<String>,
    /// Consumer configuration
    pub consumer: ConsumerConfig,
}

/// Consumer configuration for message queues
#[cfg(feature = "message-queues")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsumerConfig {
    /// Number of concurrent consumers
    pub concurrency: u32,
    /// Prefetch count
    pub prefetch_count: u32,
    /// Auto-acknowledge messages
    pub auto_ack: bool,
    /// Consumer timeout
    pub timeout: Duration,
}

/// Bridge configuration for protocol conversion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeConfig {
    /// Enable direct to message queue bridging
    pub direct_to_mq: bool,
    /// Enable message queue to direct bridging
    pub mq_to_direct: bool,
    /// Bridge buffer size
    pub buffer_size: usize,
    /// Bridge worker threads
    pub worker_threads: usize,
    /// Message transformation rules
    pub transformations: Vec<TransformationRule>,
}

/// Message transformation rule for bridging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformationRule {
    /// Pattern to match messages
    pub pattern: String,
    /// Transformation type
    pub transformation: TransformationType,
    /// Target configuration
    pub target: TransformationTarget,
}

/// Message transformation type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransformationType {
    /// Pass through without modification
    PassThrough,
    /// Convert message format
    FormatConversion,
    /// Compress message
    Compress,
    /// Decompress message
    Decompress,
    /// Encrypt message
    Encrypt,
    /// Decrypt message
    Decrypt,
}

/// Transformation target configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformationTarget {
    /// Target transport mode
    pub transport_mode: TransportMode,
    /// Target queue or channel
    pub target: String,
    /// Additional target parameters
    pub parameters: HashMap<String, String>,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            default_mode: TransportMode::Auto,
            channels: HashMap::new(),
            routes: Vec::new(),
            enable_bridge: false,
            bridge: BridgeConfig::default(),
        }
    }
}

impl Default for ChannelConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            mode: TransportMode::Auto,
            criticality: Criticality::Standard,
            reliability: Reliability::BestEffort,
            default_priority: Priority::Normal,
            max_message_size: crate::MAX_MESSAGE_SIZE,
            timeout: Duration::from_secs(30),
            retry: RetryConfig::default(),
            circuit_breaker: None,
            #[cfg(feature = "message-queues")]
            mq_config: None,
        }
    }
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
            jitter: true,
        }
    }
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 3,
            timeout: Duration::from_secs(60),
            half_open_max_calls: 3,
        }
    }
}

#[cfg(feature = "message-queues")]
impl Default for MessageQueueChannelConfig {
    fn default() -> Self {
        Self {
            queue_name_template: "{channel}".to_string(),
            exchange: None,
            routing_key_template: Some("{channel}.{node}".to_string()),
            persistent: true,
            ttl: None,
            dead_letter_queue: None,
            consumer: ConsumerConfig::default(),
        }
    }
}

#[cfg(feature = "message-queues")]
impl Default for ConsumerConfig {
    fn default() -> Self {
        Self {
            concurrency: 1,
            prefetch_count: 10,
            auto_ack: false,
            timeout: Duration::from_secs(30),
        }
    }
}

impl Default for BridgeConfig {
    fn default() -> Self {
        Self {
            direct_to_mq: true,
            mq_to_direct: true,
            buffer_size: 10000,
            worker_threads: 2,
            transformations: Vec::new(),
        }
    }
}

impl TransportConfig {
    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        // Validate channels
        for (name, channel) in &self.channels {
            channel.validate(name)?;
        }

        // Validate routes
        for (i, route) in self.routes.iter().enumerate() {
            route.validate().map_err(|e| {
                TransportError::configuration(
                    format!("Route {} validation failed: {}", i, e),
                    Some("routes"),
                )
            })?;
        }

        // Validate bridge configuration
        if self.enable_bridge {
            self.bridge.validate()?;
        }

        Ok(())
    }

    /// Get channel configuration
    pub fn get_channel_config(&self, channel_name: &str) -> ChannelConfig {
        self.channels.get(channel_name).cloned().unwrap_or_else(|| {
            let mut config = ChannelConfig::default();
            config.name = channel_name.to_string();
            config.mode = self.default_mode;
            config
        })
    }

    /// Find matching route for source and target nodes
    pub fn find_route(
        &self,
        source_node: &str,
        target_node: &str,
        channel: &str,
    ) -> Option<&RouteConfig> {
        self.routes
            .iter()
            .filter(|route| route.matches(source_node, target_node, channel))
            .max_by_key(|route| route.priority)
    }

    /// Add channel configuration
    pub fn add_channel(&mut self, config: ChannelConfig) {
        self.channels.insert(config.name.clone(), config);
    }

    /// Add route configuration
    pub fn add_route(&mut self, route: RouteConfig) {
        self.routes.push(route);
        // Sort by priority (highest first)
        self.routes.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    /// Load configuration from YAML
    pub fn from_yaml(yaml: &str) -> Result<Self> {
        serde_yaml::from_str(yaml).map_err(|e| {
            TransportError::configuration(
                format!("Failed to parse YAML configuration: {}", e),
                None,
            )
        })
    }

    /// Save configuration to YAML
    pub fn to_yaml(&self) -> Result<String> {
        serde_yaml::to_string(self).map_err(|e| {
            TransportError::configuration(
                format!("Failed to serialize configuration to YAML: {}", e),
                None,
            )
        })
    }
}

impl ChannelConfig {
    /// Validate channel configuration
    pub fn validate(&self, _name: &str) -> Result<()> {
        if self.name.is_empty() {
            return Err(TransportError::configuration(
                "Channel name cannot be empty",
                Some("name"),
            ));
        }

        if self.max_message_size == 0 {
            return Err(TransportError::configuration(
                "Max message size must be greater than 0",
                Some("max_message_size"),
            ));
        }

        if self.timeout.as_secs() == 0 {
            return Err(TransportError::configuration(
                "Timeout must be greater than 0",
                Some("timeout"),
            ));
        }

        self.retry.validate()?;

        if let Some(ref cb_config) = self.circuit_breaker {
            cb_config.validate()?;
        }

        #[cfg(feature = "message-queues")]
        if let Some(ref mq_config) = self.mq_config {
            mq_config.validate(name)?;
        }

        Ok(())
    }

    /// Create configuration for ultra-low latency channel
    pub fn ultra_low_latency(name: &str) -> Self {
        Self {
            name: name.to_string(),
            mode: TransportMode::Direct,
            criticality: Criticality::UltraLowLatency,
            reliability: Reliability::BestEffort,
            default_priority: Priority::Critical,
            max_message_size: 8192, // 8KB max for low latency
            timeout: Duration::from_millis(100),
            retry: RetryConfig {
                max_attempts: 1, // No retries for ultra-low latency
                ..Default::default()
            },
            circuit_breaker: None, // No circuit breaker for simplicity
            #[cfg(feature = "message-queues")]
            mq_config: None,
        }
    }

    /// Create configuration for reliable delivery channel
    pub fn reliable_delivery(name: &str) -> Self {
        Self {
            name: name.to_string(),
            mode: TransportMode::MessageQueue,
            criticality: Criticality::HighLatency,
            reliability: Reliability::GuaranteedDelivery,
            default_priority: Priority::Normal,
            max_message_size: crate::MAX_MESSAGE_SIZE,
            timeout: Duration::from_secs(300), // 5 minutes
            retry: RetryConfig {
                max_attempts: 10,
                initial_delay: Duration::from_millis(500),
                max_delay: Duration::from_secs(300),
                backoff_multiplier: 2.0,
                jitter: true,
            },
            circuit_breaker: Some(CircuitBreakerConfig::default()),
            #[cfg(feature = "message-queues")]
            mq_config: Some(MessageQueueChannelConfig {
                persistent: true,
                ttl: Some(Duration::from_secs(3600)), // 1 hour TTL
                dead_letter_queue: Some(format!("{}.dlq", name)),
                ..Default::default()
            }),
        }
    }
}

impl RouteConfig {
    /// Check if route matches the given source, target, and channel
    pub fn matches(&self, source_node: &str, target_node: &str, channel: &str) -> bool {
        if !self.matches_pattern(&self.source_pattern, source_node) {
            return false;
        }

        if !self.matches_pattern(&self.target_pattern, target_node) {
            return false;
        }

        if !self.channels.is_empty() && !self.channels.contains(&channel.to_string()) {
            return false;
        }

        true
    }

    /// Match pattern with wildcards
    fn matches_pattern(&self, pattern: &str, value: &str) -> bool {
        if pattern == "*" {
            return true;
        }

        if pattern.contains('*') {
            // Simple wildcard matching
            let parts: Vec<&str> = pattern.split('*').collect();
            if parts.len() == 2 {
                let prefix = parts[0];
                let suffix = parts[1];
                return value.starts_with(prefix) && value.ends_with(suffix);
            }
        }

        pattern == value
    }

    /// Validate route configuration
    pub fn validate(&self) -> Result<()> {
        if self.source_pattern.is_empty() {
            return Err(TransportError::configuration(
                "Source pattern cannot be empty",
                Some("source_pattern"),
            ));
        }

        if self.target_pattern.is_empty() {
            return Err(TransportError::configuration(
                "Target pattern cannot be empty",
                Some("target_pattern"),
            ));
        }

        Ok(())
    }
}

impl RetryConfig {
    /// Validate retry configuration
    pub fn validate(&self) -> Result<()> {
        if self.backoff_multiplier <= 0.0 {
            return Err(TransportError::configuration(
                "Backoff multiplier must be positive",
                Some("backoff_multiplier"),
            ));
        }

        if self.initial_delay >= self.max_delay {
            return Err(TransportError::configuration(
                "Initial delay must be less than max delay",
                Some("initial_delay"),
            ));
        }

        Ok(())
    }
}

impl CircuitBreakerConfig {
    /// Validate circuit breaker configuration
    pub fn validate(&self) -> Result<()> {
        if self.failure_threshold == 0 {
            return Err(TransportError::configuration(
                "Failure threshold must be greater than 0",
                Some("failure_threshold"),
            ));
        }

        if self.success_threshold == 0 {
            return Err(TransportError::configuration(
                "Success threshold must be greater than 0",
                Some("success_threshold"),
            ));
        }

        if self.half_open_max_calls == 0 {
            return Err(TransportError::configuration(
                "Half open max calls must be greater than 0",
                Some("half_open_max_calls"),
            ));
        }

        Ok(())
    }
}

#[cfg(feature = "message-queues")]
impl MessageQueueChannelConfig {
    /// Validate message queue channel configuration
    pub fn validate(&self, channel_name: &str) -> Result<()> {
        if self.queue_name_template.is_empty() {
            return Err(TransportError::configuration(
                "Queue name template cannot be empty",
                Some("queue_name_template"),
            ));
        }

        self.consumer.validate()?;

        Ok(())
    }

    /// Expand template with variables
    pub fn expand_queue_name(&self, channel: &str, node: &str) -> String {
        self.queue_name_template
            .replace("{channel}", channel)
            .replace("{node}", node)
    }

    /// Expand routing key template
    pub fn expand_routing_key(&self, channel: &str, node: &str) -> Option<String> {
        self.routing_key_template.as_ref().map(|template| {
            template
                .replace("{channel}", channel)
                .replace("{node}", node)
        })
    }
}

#[cfg(feature = "message-queues")]
impl ConsumerConfig {
    /// Validate consumer configuration
    pub fn validate(&self) -> Result<()> {
        if self.concurrency == 0 {
            return Err(TransportError::configuration(
                "Consumer concurrency must be greater than 0",
                Some("concurrency"),
            ));
        }

        if self.prefetch_count == 0 {
            return Err(TransportError::configuration(
                "Prefetch count must be greater than 0",
                Some("prefetch_count"),
            ));
        }

        Ok(())
    }
}

impl BridgeConfig {
    /// Validate bridge configuration
    pub fn validate(&self) -> Result<()> {
        if self.buffer_size == 0 {
            return Err(TransportError::configuration(
                "Bridge buffer size must be greater than 0",
                Some("buffer_size"),
            ));
        }

        if self.worker_threads == 0 {
            return Err(TransportError::configuration(
                "Bridge worker threads must be greater than 0",
                Some("worker_threads"),
            ));
        }

        for (i, transformation) in self.transformations.iter().enumerate() {
            transformation.validate().map_err(|e| {
                TransportError::configuration(
                    format!("Transformation {} validation failed: {}", i, e),
                    Some("transformations"),
                )
            })?;
        }

        Ok(())
    }
}

impl TransformationRule {
    /// Validate transformation rule
    pub fn validate(&self) -> Result<()> {
        if self.pattern.is_empty() {
            return Err(TransportError::configuration(
                "Transformation pattern cannot be empty",
                Some("pattern"),
            ));
        }

        self.target.validate()?;

        Ok(())
    }
}

impl TransformationTarget {
    /// Validate transformation target
    pub fn validate(&self) -> Result<()> {
        if self.target.is_empty() {
            return Err(TransportError::configuration(
                "Transformation target cannot be empty",
                Some("target"),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_configs() {
        let config = TransportConfig::default();
        assert_eq!(config.default_mode, TransportMode::Auto);
        assert!(config.validate().is_ok());

        let channel = ChannelConfig::default();
        assert_eq!(channel.mode, TransportMode::Auto);
        assert!(channel.validate("test").is_ok());
    }

    #[test]
    fn test_ultra_low_latency_config() {
        let config = ChannelConfig::ultra_low_latency("signals");
        assert_eq!(config.mode, TransportMode::Direct);
        assert_eq!(config.criticality, Criticality::UltraLowLatency);
        assert_eq!(config.retry.max_attempts, 1);
        assert!(config.validate("signals").is_ok());
    }

    #[test]
    fn test_reliable_delivery_config() {
        let config = ChannelConfig::reliable_delivery("audit");
        assert_eq!(config.mode, TransportMode::MessageQueue);
        assert_eq!(config.reliability, Reliability::GuaranteedDelivery);
        assert!(config.retry.max_attempts > 1);
        assert!(config.validate("audit").is_ok());
    }

    #[test]
    fn test_route_matching() {
        let route = RouteConfig {
            source_pattern: "node1".to_string(),
            target_pattern: "node*".to_string(),
            channels: vec!["market_data".to_string()],
            transport_mode: TransportMode::Direct,
            priority: 10,
            metadata: HashMap::new(),
        };

        assert!(route.matches("node1", "node2", "market_data"));
        assert!(route.matches("node1", "node_xyz", "market_data"));
        assert!(!route.matches("node2", "node1", "market_data"));
        assert!(!route.matches("node1", "node2", "signals"));
    }

    #[test]
    fn test_wildcard_pattern_matching() {
        let route = RouteConfig {
            source_pattern: "*".to_string(),
            target_pattern: "prod-*".to_string(),
            channels: Vec::new(), // Empty = matches all channels
            transport_mode: TransportMode::MessageQueue,
            priority: 5,
            metadata: HashMap::new(),
        };

        assert!(route.matches("any_node", "prod-node1", "any_channel"));
        assert!(route.matches("dev-node", "prod-db", "signals"));
        assert!(!route.matches("any_node", "dev-node", "any_channel"));
    }

    #[test]
    fn test_config_validation() {
        let mut config = ChannelConfig::default();
        config.name = "test".to_string();
        assert!(config.validate("test").is_ok());

        // Test invalid max message size
        config.max_message_size = 0;
        assert!(config.validate("test").is_err());

        // Test invalid timeout
        config.max_message_size = 1024;
        config.timeout = Duration::from_secs(0);
        assert!(config.validate("test").is_err());
    }

    #[test]
    fn test_retry_config_validation() {
        let mut retry = RetryConfig::default();
        assert!(retry.validate().is_ok());

        // Test invalid backoff multiplier
        retry.backoff_multiplier = 0.0;
        assert!(retry.validate().is_err());

        // Test invalid delay relationship
        retry.backoff_multiplier = 2.0;
        retry.initial_delay = Duration::from_secs(10);
        retry.max_delay = Duration::from_secs(5);
        assert!(retry.validate().is_err());
    }

    #[cfg(feature = "message-queues")]
    #[test]
    fn test_mq_config_template_expansion() {
        let config = MessageQueueChannelConfig::default();
        assert_eq!(
            config.expand_queue_name("market_data", "node1"),
            "market_data"
        );

        let mut config = MessageQueueChannelConfig::default();
        config.routing_key_template = Some("{channel}.{node}".to_string());
        assert_eq!(
            config.expand_routing_key("signals", "node2"),
            Some("signals.node2".to_string())
        );
    }

    #[test]
    fn test_yaml_serialization() {
        let config = TransportConfig::default();
        let yaml = config.to_yaml().unwrap();
        let parsed = TransportConfig::from_yaml(&yaml).unwrap();

        assert_eq!(config.default_mode, parsed.default_mode);
        assert_eq!(config.enable_bridge, parsed.enable_bridge);
    }
}
