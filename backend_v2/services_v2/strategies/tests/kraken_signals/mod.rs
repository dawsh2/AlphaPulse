//! Kraken Signals Strategy Tests
//!
//! Comprehensive test suite for the Kraken signals strategy including unit tests,
//! integration tests, and fixtures for real market data validation.

pub mod fixtures;
pub mod integration;
pub mod unit;

// Re-export common test utilities
pub use super::common::*;