//! # Adapter Plugin Implementations
//!
//! This module contains adapter implementations that follow the new plugin architecture
//! defined by the `Adapter` and `SafeAdapter` traits.
//!
//! ## Plugin Architecture Benefits
//! - **Standardized Interface**: All adapters implement the same traits
//! - **Safety Mechanisms**: Built-in circuit breakers, rate limiting, and timeouts
//! - **Zero-Copy Performance**: Buffer-based message processing for hot paths
//! - **Configuration Management**: Standardized configuration with environment overrides
//! - **Health Monitoring**: Comprehensive health reporting and metrics

pub mod coinbase_plugin;

pub use coinbase_plugin::{CoinbaseAdapterConfig, CoinbasePluginAdapter};