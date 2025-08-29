//! Transport Configuration Types
//!
//! Configuration types for transport routing and channel management.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Transport mode enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransportMode {
    /// Direct transport only
    Direct,
    /// Message queue only
    MessageQueue,
    /// Direct with message queue fallback
    DirectWithMqFallback,
    /// Message queue with direct fallback
    MqWithDirectFallback,
    /// Automatic mode selection
    Auto,
}

impl Default for TransportMode {
    fn default() -> Self {
        Self::Auto
    }
}

/// Channel configuration for specific actor communications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelConfig {
    /// Channel name or identifier
    pub name: String,
    /// Transport mode for this channel
    pub mode: TransportMode,
    /// Maximum message size for this channel
    pub max_message_size: Option<usize>,
    /// Priority level for messages on this channel
    pub priority: Option<crate::Priority>,
    /// Buffer size for this channel
    pub buffer_size: Option<usize>,
}

impl ChannelConfig {
    /// Create a new channel configuration
    pub fn new(name: String, mode: TransportMode) -> Self {
        Self {
            name,
            mode,
            max_message_size: None,
            priority: None,
            buffer_size: None,
        }
    }
    
    /// Set maximum message size
    pub fn with_max_message_size(mut self, size: usize) -> Self {
        self.max_message_size = Some(size);
        self
    }
    
    /// Set priority level
    pub fn with_priority(mut self, priority: crate::Priority) -> Self {
        self.priority = Some(priority);
        self
    }
    
    /// Set buffer size
    pub fn with_buffer_size(mut self, size: usize) -> Self {
        self.buffer_size = Some(size);
        self
    }
}

/// Transport configuration for the routing system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportConfig {
    /// Default transport mode
    pub default_mode: TransportMode,
    /// Channel-specific configurations
    pub channels: HashMap<String, ChannelConfig>,
    /// Global maximum message size
    pub global_max_message_size: usize,
    /// Global buffer size
    pub global_buffer_size: usize,
    /// Connection timeout in seconds
    pub connection_timeout_secs: u64,
    /// Retry configuration
    pub retry_config: RetryConfig,
}

impl TransportConfig {
    /// Create a new transport configuration with defaults
    pub fn new() -> Self {
        Self {
            default_mode: TransportMode::Auto,
            channels: HashMap::new(),
            global_max_message_size: 1024 * 1024, // 1MB
            global_buffer_size: 64 * 1024,        // 64KB
            connection_timeout_secs: 30,
            retry_config: RetryConfig::default(),
        }
    }
    
    /// Add a channel configuration
    pub fn add_channel(mut self, config: ChannelConfig) -> Self {
        self.channels.insert(config.name.clone(), config);
        self
    }
    
    /// Get channel configuration by name
    pub fn get_channel(&self, name: &str) -> Option<&ChannelConfig> {
        self.channels.get(name)
    }
    
    /// Set default transport mode
    pub fn with_default_mode(mut self, mode: TransportMode) -> Self {
        self.default_mode = mode;
        self
    }

    /// Validate the transport configuration
    pub fn validate(&self) -> Result<(), crate::TransportError> {
        if self.global_max_message_size == 0 {
            return Err(crate::TransportError::configuration(
                "global_max_message_size cannot be zero",
                Some("global_max_message_size"),
            ));
        }

        if self.global_buffer_size == 0 {
            return Err(crate::TransportError::configuration(
                "global_buffer_size cannot be zero", 
                Some("global_buffer_size"),
            ));
        }

        if self.connection_timeout_secs == 0 {
            return Err(crate::TransportError::configuration(
                "connection_timeout_secs cannot be zero",
                Some("connection_timeout_secs"),
            ));
        }

        if self.retry_config.max_attempts == 0 {
            return Err(crate::TransportError::configuration(
                "retry max_attempts cannot be zero",
                Some("retry_config.max_attempts"),
            ));
        }

        // Validate channel configurations
        for (name, channel) in &self.channels {
            if name.is_empty() {
                return Err(crate::TransportError::configuration(
                    "channel name cannot be empty",
                    Some("channels"),
                ));
            }

            if let Some(max_size) = channel.max_message_size {
                if max_size > self.global_max_message_size {
                    return Err(crate::TransportError::configuration(
                        format!("channel '{}' max_message_size exceeds global limit", name),
                        Some("channels"),
                    ));
                }
            }
        }

        Ok(())
    }
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Retry configuration for failed transport operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_attempts: u32,
    /// Base delay between retries in milliseconds
    pub base_delay_ms: u64,
    /// Maximum delay between retries in milliseconds
    pub max_delay_ms: u64,
    /// Whether to use exponential backoff
    pub use_exponential_backoff: bool,
}

impl RetryConfig {
    /// Create a new retry configuration
    pub fn new(max_attempts: u32, base_delay_ms: u64) -> Self {
        Self {
            max_attempts,
            base_delay_ms,
            max_delay_ms: base_delay_ms * 60, // 60x base delay max
            use_exponential_backoff: true,
        }
    }
    
    /// Calculate delay for given attempt number
    pub fn calculate_delay(&self, attempt: u32) -> u64 {
        if self.use_exponential_backoff {
            let delay = self.base_delay_ms * (2_u64.pow(attempt.min(10))); // Cap at 2^10
            delay.min(self.max_delay_ms)
        } else {
            self.base_delay_ms
        }
    }
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self::new(3, 100) // 3 attempts, 100ms base delay
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NetworkPriority;

    #[test]
    fn test_channel_config() {
        let config = ChannelConfig::new("test_channel".to_string(), TransportMode::Direct)
            .with_max_message_size(1024)
            .with_priority(NetworkPriority::High)
            .with_buffer_size(2048);
        
        assert_eq!(config.name, "test_channel");
        assert_eq!(config.mode, TransportMode::Direct);
        assert_eq!(config.max_message_size, Some(1024));
        assert_eq!(config.priority, Some(NetworkPriority::High));
        assert_eq!(config.buffer_size, Some(2048));
    }
    
    #[test]
    fn test_transport_config() {
        let channel = ChannelConfig::new("test".to_string(), TransportMode::MessageQueue);
        let config = TransportConfig::new()
            .with_default_mode(TransportMode::DirectWithMqFallback)
            .add_channel(channel);
        
        assert_eq!(config.default_mode, TransportMode::DirectWithMqFallback);
        assert!(config.get_channel("test").is_some());
        assert_eq!(config.get_channel("test").unwrap().mode, TransportMode::MessageQueue);
    }
    
    #[test]
    fn test_retry_config() {
        let config = RetryConfig::new(5, 50);
        
        assert_eq!(config.max_attempts, 5);
        assert_eq!(config.base_delay_ms, 50);
        assert_eq!(config.calculate_delay(0), 50);  // 2^0 * 50 = 50
        assert_eq!(config.calculate_delay(1), 100); // 2^1 * 50 = 100
        assert_eq!(config.calculate_delay(2), 200); // 2^2 * 50 = 200
        
        let no_backoff = RetryConfig {
            use_exponential_backoff: false,
            ..config
        };
        assert_eq!(no_backoff.calculate_delay(0), 50);
        assert_eq!(no_backoff.calculate_delay(5), 50); // Same delay always
    }
    
    #[test]
    fn test_transport_modes() {
        assert_eq!(TransportMode::default(), TransportMode::Auto);
        
        // Test serialization roundtrip
        let modes = vec![
            TransportMode::Direct,
            TransportMode::MessageQueue,
            TransportMode::DirectWithMqFallback,
            TransportMode::MqWithDirectFallback,
            TransportMode::Auto,
        ];
        
        for mode in modes {
            let json = serde_json::to_string(&mode).unwrap();
            let deserialized: TransportMode = serde_json::from_str(&json).unwrap();
            assert_eq!(mode, deserialized);
        }
    }
}