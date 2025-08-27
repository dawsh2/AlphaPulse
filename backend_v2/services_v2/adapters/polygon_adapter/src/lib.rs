//! Polygon DEX Adapter Plugin
//!
//! This adapter implements the AlphaPulse Adapter trait for Polygon DEX data collection.
//! It connects to Polygon's WebSocket endpoint, processes DEX events (swaps, mints, burns),
//! and converts them to Protocol V2 TLV messages.

pub mod adapter;
pub mod config;
pub mod parser;

pub use adapter::PolygonAdapter;
pub use config::PolygonConfig;