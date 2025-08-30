//! Concrete MessageSink implementations for different connection types
//!
//! This module provides implementations for the three main sink types:
//! - **RelaySink**: Unix socket connections to relay services
//! - **DirectSink**: Direct TCP/WebSocket connections
//! - **CompositeSink**: Multi-target patterns (fanout, round-robin, failover)

pub mod composite;
pub mod direct;
pub mod relay;

pub use composite::{CompositeMetrics, CompositePattern, CompositeSink};
pub use direct::{ConnectionType, DirectSink};
pub use relay::RelaySink;
