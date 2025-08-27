//! # Coinbase Adapter Plugin
//!
//! Implementation of the Coinbase exchange adapter following the plugin architecture.
//! This adapter connects to Coinbase WebSocket API and converts market data to TLV format.

pub mod adapter;
pub mod config;

pub use adapter::CoinbasePluginAdapter;
pub use config::CoinbaseAdapterConfig;