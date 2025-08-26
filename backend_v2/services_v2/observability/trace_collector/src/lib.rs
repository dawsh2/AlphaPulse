//! # TraceCollector Service
//!
//! Central aggregation service for AlphaPulse distributed tracing system.
//! Provides real-time message flow observability across all components.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    TraceCollector                           │
//! │                                                             │
//! │  ┌─────────────┐    ┌─────────────────┐    ┌─────────────┐ │
//! │  │ Unix Socket │ -> │ Event Processor │ -> │ Web API     │ │
//! │  │ Listener    │    │                 │    │ Server      │ │
//! │  └─────────────┘    └─────────────────┘    └─────────────┘ │
//! │                               │                             │
//! │  ┌─────────────┐    ┌─────────────────┐    ┌─────────────┐ │
//! │  │ Active      │    │ Trace Timeline  │    │ Completed   │ │
//! │  │ Traces      │    │ Builder         │    │ Traces      │ │
//! │  └─────────────┘    └─────────────────┘    └─────────────┘ │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Features
//!
//! - **Real-time trace aggregation** - Collects trace events from all services
//! - **Timeline construction** - Builds complete message flow timelines
//! - **JSON API** - Provides data for web visualization
//! - **Health monitoring** - Reports service health and performance
//! - **Ring buffer storage** - Efficiently manages completed traces
//!
//! ## Usage
//!
//! ```rust
//! use trace_collector::TraceCollector;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let collector = TraceCollector::new("/tmp/alphapulse/traces.sock").await?;
//!     collector.start().await?;
//!     Ok(())
//! }
//! ```

pub mod api;
pub mod collector;
pub mod events;
pub mod health;
pub mod timeline;

pub use api::{TraceApiServer, TraceQuery, TraceResponse};
pub use collector::TraceCollector;
pub use events::{EventBuffer, TraceEventProcessor};
pub use health::{CollectorHealth, HealthReporter};
pub use timeline::{MessageFlow, TraceSpan, TraceTimeline};

// use alphapulse_types::{SourceType, TraceEvent, TraceEventType};
use thiserror::Error;

/// Errors that can occur in the TraceCollector service
#[derive(Debug, Clone, Error, serde::Serialize, serde::Deserialize)]
pub enum TraceError {
    #[error("IO error: {0}")]
    Io(String),

    #[error("JSON serialization error: {0}")]
    Json(String),

    #[error("Invalid trace ID: {0}")]
    InvalidTraceId(String),

    #[error("Timeline construction error: {0}")]
    Timeline(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Configuration error: {0}")]
    Config(String),
}

/// Result type for TraceCollector operations
pub type Result<T> = std::result::Result<T, TraceError>;

/// Configuration for TraceCollector service
#[derive(Debug, Clone)]
pub struct TraceCollectorConfig {
    /// Unix socket path for receiving trace events
    pub socket_path: String,

    /// HTTP port for web API server
    pub api_port: u16,

    /// Maximum number of active traces to keep in memory
    pub max_active_traces: usize,

    /// Maximum number of completed traces to keep in ring buffer
    pub max_completed_traces: usize,

    /// Timeout for inactive traces (seconds)
    pub trace_timeout_seconds: u64,

    /// Health check interval (seconds)
    pub health_check_interval_seconds: u64,

    /// Enable debug logging
    pub debug_mode: bool,
}

impl Default for TraceCollectorConfig {
    fn default() -> Self {
        Self {
            socket_path: "/tmp/alphapulse/traces.sock".to_string(),
            api_port: 8080,
            max_active_traces: 10_000,
            max_completed_traces: 1_000,
            trace_timeout_seconds: 300, // 5 minutes
            health_check_interval_seconds: 30,
            debug_mode: false,
        }
    }
}

/// Statistics about trace collection performance
#[derive(Debug, Clone, serde::Serialize)]
pub struct TraceCollectorStats {
    /// Total number of events processed
    pub events_processed: u64,

    /// Number of active traces being tracked
    pub active_traces: usize,

    /// Number of completed traces in buffer
    pub completed_traces: usize,

    /// Average events per trace
    pub avg_events_per_trace: f64,

    /// Average trace duration in milliseconds
    pub avg_trace_duration_ms: f64,

    /// Number of timeout traces (incomplete)
    pub timed_out_traces: u64,

    /// Events processed per second (last minute)
    pub events_per_second: f64,

    /// Memory usage estimate in bytes
    pub memory_usage_bytes: usize,

    /// Uptime in seconds
    pub uptime_seconds: u64,
}

/// Trace ID type alias for clarity (matches protocol_v2 definition)
pub type TraceId = [u8; 8];

/// Convert trace ID to hex string for logging and JSON
pub fn trace_id_to_hex(trace_id: &TraceId) -> String {
    hex::encode(trace_id)
}

/// Convert hex string back to trace ID
pub fn hex_to_trace_id(hex: &str) -> Result<TraceId> {
    if hex.len() != 16 {
        return Err(TraceError::InvalidTraceId(format!(
            "Expected 16 hex chars, got {}",
            hex.len()
        )));
    }

    let bytes = hex::decode(hex).map_err(|_| TraceError::InvalidTraceId(hex.to_string()))?;
    let mut trace_id = [0u8; 8];
    trace_id.copy_from_slice(&bytes);
    Ok(trace_id)
}
