//! Configuration for dynamic TLV payload constraints
//!
//! Provides configurable limits for bounded collections in TLV messages,
//! allowing runtime adjustment without recompilation.

use once_cell::sync::Lazy;
use std::env;

/// Configuration for dynamic payload constraints
#[derive(Debug, Clone)]
pub struct DynamicPayloadConfig {
    /// Maximum number of instruments in state invalidation messages
    pub max_instruments: usize,
    /// Maximum number of tokens in pool liquidity messages
    pub max_pool_tokens: usize,
    /// Maximum number of order levels in orderbook messages
    pub max_order_levels: usize,
}

impl Default for DynamicPayloadConfig {
    fn default() -> Self {
        Self {
            // Default values with generous headroom based on real usage patterns
            max_instruments: 16,  // Real usage: 1-5
            max_pool_tokens: 8,   // Real usage: 2-4 (handles complex Balancer pools)
            max_order_levels: 50, // Real usage: 10-30
        }
    }
}

impl DynamicPayloadConfig {
    /// Create configuration from environment variables
    pub fn from_env() -> Self {
        Self {
            max_instruments: env::var("TORQ_MAX_INSTRUMENTS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(16),
            max_pool_tokens: env::var("TORQ_MAX_POOL_TOKENS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(8),
            max_order_levels: env::var("TORQ_MAX_ORDER_LEVELS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(50),
        }
    }

    /// Validate configuration values
    pub fn validate(&self) -> Result<(), String> {
        if self.max_instruments == 0 {
            return Err("max_instruments must be greater than 0".to_string());
        }
        if self.max_instruments > 256 {
            return Err("max_instruments cannot exceed 256".to_string());
        }
        if self.max_pool_tokens == 0 {
            return Err("max_pool_tokens must be greater than 0".to_string());
        }
        if self.max_pool_tokens > 32 {
            return Err("max_pool_tokens cannot exceed 32".to_string());
        }
        if self.max_order_levels == 0 {
            return Err("max_order_levels must be greater than 0".to_string());
        }
        if self.max_order_levels > 100 {
            return Err("max_order_levels cannot exceed 100".to_string());
        }
        Ok(())
    }
}

/// Global configuration loaded once at startup
pub static CONFIG: Lazy<DynamicPayloadConfig> = Lazy::new(|| {
    let config = DynamicPayloadConfig::from_env();
    if let Err(e) = config.validate() {
        eprintln!("Invalid dynamic payload configuration: {}", e);
        eprintln!("Using default configuration");
        DynamicPayloadConfig::default()
    } else {
        config
    }
});

/// Get maximum instruments from configuration
pub fn max_instruments() -> usize {
    CONFIG.max_instruments
}

/// Get maximum pool tokens from configuration
pub fn max_pool_tokens() -> usize {
    CONFIG.max_pool_tokens
}

/// Get maximum order levels from configuration
pub fn max_order_levels() -> usize {
    CONFIG.max_order_levels
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = DynamicPayloadConfig::default();
        assert_eq!(config.max_instruments, 16);
        assert_eq!(config.max_pool_tokens, 8);
        assert_eq!(config.max_order_levels, 50);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validation() {
        let mut config = DynamicPayloadConfig::default();

        // Test zero values
        config.max_instruments = 0;
        assert!(config.validate().is_err());

        // Test excessive values
        config.max_instruments = 1000;
        assert!(config.validate().is_err());

        // Test valid range
        config.max_instruments = 32;
        config.max_pool_tokens = 16;
        config.max_order_levels = 75;
        assert!(config.validate().is_ok());
    }
}
