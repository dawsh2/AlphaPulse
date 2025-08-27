//! MEV protection module
//!
//! Provides Flashbots integration for private mempool submission
//! to protect arbitrage transactions from frontrunning.

pub mod flashbots;

pub use flashbots::FlashbotsClient;
