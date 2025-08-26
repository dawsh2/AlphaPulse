//! Relay Communication Integration Tests
//!
//! Tests that validate message passing between components through relay infrastructure:
//! - MarketData relay routing and filtering
//! - Signal relay coordination
//! - Execution relay validation
//! - Cross-domain message isolation

pub mod market_data_relay_tests;
pub mod signal_relay_tests;
pub mod execution_relay_tests;
pub mod relay_isolation_tests;