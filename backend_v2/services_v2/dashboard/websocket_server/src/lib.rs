//! Dashboard WebSocket Server
//!
//! Multi-relay consumer that streams real-time data to dashboard frontend.
//! Converts TLV protocol messages to JSON for browser consumption.

#![recursion_limit = "256"]

pub mod client;
pub mod config;
pub mod error;
pub mod message_converter;
pub mod relay_consumer;
pub mod server;

pub use client::{Client, ClientManager};
pub use config::DashboardConfig;
pub use error::{DashboardError, Result};
pub use server::DashboardServer;

/// Re-export key types
pub use protocol_v2::{QuoteTLV, StateInvalidationTLV};
