//! Shared Strategy Framework
//!
//! Common utilities and traits for trading strategy implementations.

pub mod config;
pub mod metrics;
pub mod testing;
pub mod traits;

pub use config::*;
pub use metrics::*;
pub use testing::*;
pub use traits::*;