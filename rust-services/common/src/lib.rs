// Common types and utilities shared across AlphaPulse Rust services
// These types mirror the Python schemas for seamless integration

pub mod types;
pub mod error;
pub mod metrics;
pub mod config;
pub mod orderbook_delta;
pub mod retry;
pub mod shared_memory;
pub mod shared_memory_v2;
pub mod shared_memory_registry;
pub mod event_driven_shm;

pub use types::*;
pub use error::*;
pub use metrics::MetricsCollector;
pub use config::{Config, SymbolConverter};
pub use orderbook_delta::{OrderBookTracker, OrderBookSnapshot, OrderBookDelta};
pub use retry::{RetryPolicy, CircuitBreaker};
pub use shared_memory::{SharedMemoryWriter, SharedMemoryReader, SharedTrade};
pub use shared_memory_registry::{SharedMemoryRegistry, FeedMetadata, FeedType, create_feed_metadata, update_feed_heartbeat};
pub use event_driven_shm::{EventDrivenTradeWriter, EventDrivenTradeReader, AtomicReaderRegistry};