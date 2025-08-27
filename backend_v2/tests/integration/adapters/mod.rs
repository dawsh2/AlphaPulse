//! Integration tests for AlphaPulse adapters
//!
//! This module contains comprehensive integration tests for all adapter implementations,
//! including real exchange connection tests, TLV message validation, and performance benchmarks.

pub mod coinbase_plugin_test;
pub mod coinbase_roundtrip_test;
pub mod e2e_pool_cache_validation;
pub mod ethabi_parsing_test;
pub mod gemini_roundtrip_test;
pub mod plugin_architecture_integration;
pub mod polygon_event_debug;
pub mod polygon_pool_cache_integration;
pub mod polygon_subscription_test;

pub mod fixtures;
pub mod integration;
pub mod performance;
pub mod unit;
pub mod validation;