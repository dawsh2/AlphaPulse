//! Shared metrics for adapter monitoring

use dashmap::DashMap;
use protocol_v2::VenueId;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Core metrics shared between all adapters
#[derive(Debug, Clone)]
pub struct AdapterMetrics {
    // Connection metrics
    pub connections_established: Arc<AtomicU64>,
    pub connections_failed: Arc<AtomicU64>,
    pub reconnection_attempts: Arc<AtomicU64>,
    pub active_connections: Arc<AtomicU64>,

    // Message metrics
    pub messages_received: Arc<AtomicU64>,
    pub messages_processed: Arc<AtomicU64>,
    pub messages_failed: Arc<AtomicU64>,
    pub bytes_received: Arc<AtomicU64>,

    // Performance metrics
    pub processing_times: Arc<DashMap<VenueId, Vec<Duration>>>,
    pub last_message_times: Arc<DashMap<VenueId, Instant>>,

    // Error metrics
    pub parse_errors: Arc<AtomicU64>,
    pub timeout_errors: Arc<AtomicU64>,
    pub protocol_errors: Arc<AtomicU64>,

    // State management
    pub state_invalidations: Arc<AtomicU64>,
    pub instruments_tracked: Arc<DashMap<VenueId, usize>>,
}

impl AdapterMetrics {
    /// Create new metrics instance
    pub fn new() -> Self {
        Self {
            connections_established: Arc::new(AtomicU64::new(0)),
            connections_failed: Arc::new(AtomicU64::new(0)),
            reconnection_attempts: Arc::new(AtomicU64::new(0)),
            active_connections: Arc::new(AtomicU64::new(0)),
            messages_received: Arc::new(AtomicU64::new(0)),
            messages_processed: Arc::new(AtomicU64::new(0)),
            messages_failed: Arc::new(AtomicU64::new(0)),
            bytes_received: Arc::new(AtomicU64::new(0)),
            processing_times: Arc::new(DashMap::new()),
            last_message_times: Arc::new(DashMap::new()),
            parse_errors: Arc::new(AtomicU64::new(0)),
            timeout_errors: Arc::new(AtomicU64::new(0)),
            protocol_errors: Arc::new(AtomicU64::new(0)),
            state_invalidations: Arc::new(AtomicU64::new(0)),
            instruments_tracked: Arc::new(DashMap::new()),
        }
    }

    /// Record successful connection
    pub fn record_connection(&self, venue: VenueId) {
        self.connections_established.fetch_add(1, Ordering::Relaxed);
        self.active_connections.fetch_add(1, Ordering::Relaxed);
        tracing::info!("Connection established for venue {:?}", venue);
    }

    /// Record failed connection
    pub fn record_connection_failure(&self, venue: VenueId) {
        self.connections_failed.fetch_add(1, Ordering::Relaxed);
        tracing::warn!("Connection failed for venue {:?}", venue);
    }

    /// Record disconnection
    pub fn record_disconnection(&self, venue: VenueId) {
        let active = self.active_connections.fetch_sub(1, Ordering::Relaxed);
        tracing::info!(
            "Disconnected from venue {:?}, {} connections remain",
            venue,
            active - 1
        );
    }

    /// Record message received
    pub fn record_message(&self, venue: VenueId, bytes: usize) {
        self.messages_received.fetch_add(1, Ordering::Relaxed);
        self.bytes_received
            .fetch_add(bytes as u64, Ordering::Relaxed);
        self.last_message_times.insert(venue, Instant::now());
    }

    /// Record message processing time
    pub fn record_processing_time(&self, venue: VenueId, duration: Duration) {
        self.messages_processed.fetch_add(1, Ordering::Relaxed);

        // Keep last 1000 processing times for analysis
        self.processing_times
            .entry(venue)
            .and_modify(|times| {
                times.push(duration);
                if times.len() > 1000 {
                    times.remove(0);
                }
            })
            .or_insert_with(|| vec![duration]);
    }

    /// Record processing error
    pub fn record_processing_error(&self, error_type: ErrorType) {
        self.messages_failed.fetch_add(1, Ordering::Relaxed);

        match error_type {
            ErrorType::Parse => self.parse_errors.fetch_add(1, Ordering::Relaxed),
            ErrorType::Timeout => self.timeout_errors.fetch_add(1, Ordering::Relaxed),
            ErrorType::Protocol => self.protocol_errors.fetch_add(1, Ordering::Relaxed),
        };
    }

    /// Increment error counter (legacy compatibility)
    pub fn increment_errors(&self) {
        self.messages_failed.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment messages sent counter
    pub fn increment_messages_sent(&self) {
        self.messages_processed.fetch_add(1, Ordering::Relaxed);
    }

    /// Record state invalidation
    pub fn record_state_invalidation(&self, venue: VenueId, instruments: usize) {
        self.state_invalidations.fetch_add(1, Ordering::Relaxed);
        self.instruments_tracked.insert(venue, 0);
        tracing::info!(
            "Invalidated {} instruments for venue {:?}",
            instruments,
            venue
        );
    }

    /// Update instrument count
    pub fn update_instrument_count(&self, venue: VenueId, count: usize) {
        self.instruments_tracked.insert(venue, count);
    }

    /// Get summary statistics
    pub fn summary(&self) -> MetricsSummary {
        let total_connections = self.connections_established.load(Ordering::Relaxed);
        let failed_connections = self.connections_failed.load(Ordering::Relaxed);
        let messages = self.messages_received.load(Ordering::Relaxed);
        let processed = self.messages_processed.load(Ordering::Relaxed);
        let failed = self.messages_failed.load(Ordering::Relaxed);

        MetricsSummary {
            connection_success_rate: if total_connections > 0 {
                (total_connections - failed_connections) as f64 / total_connections as f64
            } else {
                0.0
            },
            active_connections: self.active_connections.load(Ordering::Relaxed),
            message_success_rate: if messages > 0 {
                processed as f64 / messages as f64
            } else {
                0.0
            },
            total_messages: messages,
            total_bytes: self.bytes_received.load(Ordering::Relaxed),
            average_processing_time: self.calculate_average_processing_time(),
            total_instruments: self.instruments_tracked.iter().map(|e| *e.value()).sum(),
            state_invalidations: self.state_invalidations.load(Ordering::Relaxed),
        }
    }

    fn calculate_average_processing_time(&self) -> Duration {
        let mut total = Duration::ZERO;
        let mut count = 0;

        for entry in self.processing_times.iter() {
            for duration in entry.value() {
                total += *duration;
                count += 1;
            }
        }

        if count > 0 {
            total / count as u32
        } else {
            Duration::ZERO
        }
    }

    /// Reset all metrics
    pub fn reset(&self) {
        self.connections_established.store(0, Ordering::Relaxed);
        self.connections_failed.store(0, Ordering::Relaxed);
        self.reconnection_attempts.store(0, Ordering::Relaxed);
        self.messages_received.store(0, Ordering::Relaxed);
        self.messages_processed.store(0, Ordering::Relaxed);
        self.messages_failed.store(0, Ordering::Relaxed);
        self.bytes_received.store(0, Ordering::Relaxed);
        self.parse_errors.store(0, Ordering::Relaxed);
        self.timeout_errors.store(0, Ordering::Relaxed);
        self.protocol_errors.store(0, Ordering::Relaxed);
        self.state_invalidations.store(0, Ordering::Relaxed);
        self.processing_times.clear();
        self.last_message_times.clear();
        self.instruments_tracked.clear();
    }
}

impl Default for AdapterMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Types of processing errors
#[derive(Debug, Clone, Copy)]
pub enum ErrorType {
    /// JSON or binary parsing error
    Parse,
    /// Connection or message timeout
    Timeout,
    /// Protocol violation or invalid TLV
    Protocol,
}

/// Summary of adapter metrics
#[derive(Debug, Clone)]
pub struct MetricsSummary {
    /// Percentage of successful connections
    pub connection_success_rate: f64,
    /// Current active connections
    pub active_connections: u64,
    /// Percentage of successfully processed messages
    pub message_success_rate: f64,
    /// Total messages received
    pub total_messages: u64,
    /// Total bytes received
    pub total_bytes: u64,
    /// Average message processing time
    pub average_processing_time: Duration,
    /// Total instruments being tracked
    pub total_instruments: usize,
    /// Number of state invalidations
    pub state_invalidations: u64,
}

impl MetricsSummary {
    /// Check if adapter is healthy
    pub fn is_healthy(&self) -> bool {
        self.connection_success_rate > 0.9
            && self.message_success_rate > 0.95
            && self.average_processing_time < Duration::from_millis(1)
    }

    /// Get health status string
    pub fn health_status(&self) -> &'static str {
        if self.active_connections == 0 {
            "disconnected"
        } else if self.is_healthy() {
            "healthy"
        } else if self.message_success_rate > 0.8 {
            "degraded"
        } else {
            "unhealthy"
        }
    }
}
