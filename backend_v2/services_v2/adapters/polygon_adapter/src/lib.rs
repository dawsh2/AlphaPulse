//! Polygon DEX Adapter Plugin
//!
//! This adapter implements the Torq Adapter trait for Polygon DEX data collection.
//! It connects to Polygon's WebSocket endpoint, processes DEX events (swaps, mints, burns),
//! and converts them to Protocol V2 TLV messages with proper zero-copy serialization.
//!
//! ## Features
//! - **Production-Ready**: Handles real Polygon WebSocket events
//! - **Protocol V2 Compliant**: Builds proper 32-byte header + TLV messages
//! - **Performance Optimized**: <35μs hot path latency monitoring
//! - **Safety Mechanisms**: Circuit breaker, rate limiting, error tracking
//! - **Zero-Copy**: Uses zerocopy traits for efficient serialization

pub mod adapter;
pub mod config;
pub mod constants;

pub use adapter::PolygonAdapter;
pub use config::PolygonConfig;