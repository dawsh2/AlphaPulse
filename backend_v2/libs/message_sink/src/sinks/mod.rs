//! Concrete MessageSink implementations for different connection types
//!
//! This module provides implementations for the three main sink types:
//! - **RelaySink**: Unix socket connections to relay services
//! - **DirectSink**: Direct TCP/WebSocket connections 
//! - **CompositeSink**: Multi-target patterns (fanout, round-robin, failover)

pub mod relay;
pub mod direct;
pub mod composite;

pub use relay::RelaySink;
pub use direct::{DirectSink, ConnectionType};
pub use composite::{CompositeSink, CompositePattern, CompositeMetrics};