//! Main TraceCollector implementation
//!
//! Handles Unix socket communication, event aggregation, and timeline management.

use crate::{
    trace_id_to_hex, HealthReporter, Result, TraceApiServer, TraceCollectorConfig,
    TraceCollectorStats, TraceError, TraceEventProcessor, TraceId, TraceTimeline,
};

use dashmap::DashMap;
use parking_lot::RwLock;
use protocol_v2::{SourceType, TraceEvent, TraceEventType};
use ringbuffer::{AllocRingBuffer, RingBuffer};
use std::path::Path;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::mpsc;
use tokio::time::{interval, Duration, Instant};
use tracing::{debug, error, info, warn};

/// Main TraceCollector service
///
/// Aggregates trace events from all AlphaPulse services and builds
/// complete message flow timelines for observability.
pub struct TraceCollector {
    config: TraceCollectorConfig,

    /// Active traces being built (trace_id -> timeline)
    active_traces: Arc<DashMap<TraceId, TraceTimeline>>,

    /// Completed traces (ring buffer for efficiency)
    completed_traces: Arc<RwLock<AllocRingBuffer<TraceTimeline>>>,

    /// Statistics and performance metrics
    stats: Arc<RwLock<TraceCollectorStats>>,

    /// Health reporter for system monitoring
    health_reporter: HealthReporter,

    /// Event processor for handling incoming events
    event_processor: TraceEventProcessor,

    /// API server for web interface
    api_server: Option<TraceApiServer>,

    /// Service start time for uptime calculation
    start_time: Instant,
}

impl TraceCollector {
    /// Create new TraceCollector with default configuration
    pub async fn new() -> Result<Self> {
        Self::with_config(TraceCollectorConfig::default()).await
    }

    /// Create new TraceCollector with custom configuration
    pub async fn with_config(config: TraceCollectorConfig) -> Result<Self> {
        let stats = Arc::new(RwLock::new(TraceCollectorStats {
            events_processed: 0,
            active_traces: 0,
            completed_traces: 0,
            avg_events_per_trace: 0.0,
            avg_trace_duration_ms: 0.0,
            timed_out_traces: 0,
            events_per_second: 0.0,
            memory_usage_bytes: 0,
            uptime_seconds: 0,
        }));

        let active_traces = Arc::new(DashMap::with_capacity(config.max_active_traces));
        let completed_traces = Arc::new(RwLock::new(AllocRingBuffer::with_capacity(
            config.max_completed_traces,
        )));

        let health_reporter = HealthReporter::new(stats.clone());
        let event_processor = TraceEventProcessor::new(
            active_traces.clone(),
            completed_traces.clone(),
            stats.clone(),
        );

        // Initialize API server if port is configured
        let api_server = if config.api_port > 0 {
            Some(
                TraceApiServer::new(
                    config.api_port,
                    active_traces.clone(),
                    completed_traces.clone(),
                    stats.clone(),
                )
                .await?,
            )
        } else {
            None
        };

        Ok(Self {
            config,
            active_traces,
            completed_traces,
            stats,
            health_reporter,
            event_processor,
            api_server,
            start_time: Instant::now(),
        })
    }

    /// Start the TraceCollector service
    ///
    /// This will:
    /// 1. Start Unix socket listener for trace events
    /// 2. Start API server for web interface
    /// 3. Start background cleanup tasks
    /// 4. Start health monitoring
    pub async fn start(&mut self) -> Result<()> {
        info!(
            "Starting TraceCollector on socket: {} (API port: {})",
            self.config.socket_path, self.config.api_port
        );

        // Remove existing socket file if it exists
        if Path::new(&self.config.socket_path).exists() {
            std::fs::remove_file(&self.config.socket_path).map_err(|e| TraceError::Io(e))?;
        }

        // Create parent directory if it doesn't exist
        if let Some(parent) = Path::new(&self.config.socket_path).parent() {
            std::fs::create_dir_all(parent).map_err(|e| TraceError::Io(e))?;
        }

        let listener =
            UnixListener::bind(&self.config.socket_path).map_err(|e| TraceError::Io(e))?;

        info!("TraceCollector listening on {}", self.config.socket_path);

        // Start background tasks
        let cleanup_handle = self.start_cleanup_task().await;
        let health_handle = self.start_health_monitoring().await;
        let api_handle = self.start_api_server().await;

        // Start accepting connections
        self.accept_connections(listener).await?;

        // Wait for background tasks to complete (they shouldn't unless there's an error)
        if let Some(handle) = cleanup_handle {
            let _ = handle.await;
        }
        if let Some(handle) = health_handle {
            let _ = handle.await;
        }
        if let Some(handle) = api_handle {
            let _ = handle.await;
        }

        Ok(())
    }

    /// Start Unix socket listener and process incoming trace events
    async fn accept_connections(&self, listener: UnixListener) -> Result<()> {
        let mut connection_count = 0;

        loop {
            match listener.accept().await {
                Ok((stream, _addr)) => {
                    connection_count += 1;
                    debug!("New trace connection #{}", connection_count);

                    let processor = self.event_processor.clone();
                    let config = self.config.clone();

                    // Spawn task to handle this connection
                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_connection(stream, processor, config).await {
                            warn!("Connection error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    error!("Failed to accept connection: {}", e);
                    // Continue accepting other connections
                }
            }
        }
    }

    /// Handle individual Unix socket connection
    async fn handle_connection(
        mut stream: UnixStream,
        processor: TraceEventProcessor,
        config: TraceCollectorConfig,
    ) -> Result<()> {
        let mut buffer = vec![0u8; 8192];
        let mut incomplete_data = Vec::new();

        loop {
            match stream.read(&mut buffer).await {
                Ok(0) => {
                    debug!("Client disconnected");
                    break;
                }
                Ok(bytes_read) => {
                    // Append new data to any incomplete data from previous reads
                    incomplete_data.extend_from_slice(&buffer[..bytes_read]);

                    // Process complete JSON events
                    let mut start = 0;
                    while let Some(newline_pos) =
                        incomplete_data[start..].iter().position(|&b| b == b'\n')
                    {
                        let end = start + newline_pos;
                        let json_bytes = &incomplete_data[start..end];

                        // Parse and process trace event
                        match Self::parse_trace_event(json_bytes) {
                            Ok(event) => {
                                if config.debug_mode {
                                    debug!(
                                        "Received trace event: {} from {:?}",
                                        trace_id_to_hex(&event.trace_id),
                                        event.service
                                    );
                                }

                                if let Err(e) = processor.process_event(event).await {
                                    warn!("Failed to process trace event: {}", e);
                                }
                            }
                            Err(e) => {
                                warn!("Failed to parse trace event: {}", e);
                            }
                        }

                        start = end + 1; // Skip the newline
                    }

                    // Keep any remaining incomplete data
                    if start < incomplete_data.len() {
                        incomplete_data.drain(..start);
                    } else {
                        incomplete_data.clear();
                    }
                }
                Err(e) => {
                    warn!("Error reading from connection: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }

    /// Parse JSON trace event from bytes
    fn parse_trace_event(json_bytes: &[u8]) -> Result<TraceEvent> {
        let json_str = std::str::from_utf8(json_bytes)
            .map_err(|_| TraceError::InvalidTraceId("Invalid UTF-8 in trace event".to_string()))?;

        let event: TraceEvent = serde_json::from_str(json_str)?;
        Ok(event)
    }

    /// Start background cleanup task for timed-out traces
    async fn start_cleanup_task(&self) -> Option<tokio::task::JoinHandle<()>> {
        let active_traces = self.active_traces.clone();
        let completed_traces = self.completed_traces.clone();
        let stats = self.stats.clone();
        let timeout_duration = Duration::from_secs(self.config.trace_timeout_seconds);

        Some(tokio::spawn(async move {
            let mut cleanup_interval = interval(Duration::from_secs(60)); // Cleanup every minute

            loop {
                cleanup_interval.tick().await;

                let now = Instant::now();
                let mut timed_out = Vec::new();

                // Find timed-out traces
                for trace in active_traces.iter() {
                    let trace_id = *trace.key();
                    let timeline = trace.value();

                    if now.duration_since(timeline.start_time()) > timeout_duration {
                        timed_out.push(trace_id);
                    }
                }

                // Move timed-out traces to completed buffer
                for trace_id in timed_out {
                    if let Some((_, timeline)) = active_traces.remove(&trace_id) {
                        debug!("Trace timed out: {}", trace_id_to_hex(&trace_id));

                        // Add to completed traces
                        {
                            let mut completed = completed_traces.write();
                            completed.push(timeline);
                        }

                        // Update stats
                        {
                            let mut stats = stats.write();
                            stats.timed_out_traces += 1;
                            stats.active_traces = active_traces.len();
                            stats.completed_traces = completed_traces.read().len();
                        }
                    }
                }
            }
        }))
    }

    /// Start health monitoring task
    async fn start_health_monitoring(&self) -> Option<tokio::task::JoinHandle<()>> {
        let health_reporter = self.health_reporter.clone();
        let interval_seconds = self.config.health_check_interval_seconds;

        Some(tokio::spawn(async move {
            let mut health_interval = interval(Duration::from_secs(interval_seconds));

            loop {
                health_interval.tick().await;

                if let Err(e) = health_reporter.report_health().await {
                    warn!("Failed to report health: {}", e);
                }
            }
        }))
    }

    /// Start API server for web interface
    async fn start_api_server(&self) -> Option<tokio::task::JoinHandle<()>> {
        if let Some(api_server) = &self.api_server {
            let server = api_server.clone();
            Some(tokio::spawn(async move {
                if let Err(e) = server.start().await {
                    error!("API server error: {}", e);
                }
            }))
        } else {
            None
        }
    }

    /// Get current collector statistics
    pub fn get_stats(&self) -> TraceCollectorStats {
        let mut stats = self.stats.read().clone();
        stats.uptime_seconds = self.start_time.elapsed().as_secs();
        stats.active_traces = self.active_traces.len();
        stats.completed_traces = self.completed_traces.read().len();
        stats
    }

    /// Get all active traces (for debugging)
    pub fn get_active_traces(&self) -> Vec<(TraceId, TraceTimeline)> {
        self.active_traces
            .iter()
            .map(|entry| (*entry.key(), entry.value().clone()))
            .collect()
    }

    /// Get completed traces (for web interface)
    pub fn get_completed_traces(&self, limit: Option<usize>) -> Vec<TraceTimeline> {
        let completed = self.completed_traces.read();
        let traces: Vec<TraceTimeline> = completed.iter().cloned().collect();

        match limit {
            Some(n) => traces.into_iter().rev().take(n).collect(),
            None => traces.into_iter().rev().collect(),
        }
    }

    /// Find trace by ID (active or completed)
    pub fn find_trace(&self, trace_id: &TraceId) -> Option<TraceTimeline> {
        // First check active traces
        if let Some(timeline) = self.active_traces.get(trace_id) {
            return Some(timeline.clone());
        }

        // Then check completed traces
        let completed = self.completed_traces.read();
        completed
            .iter()
            .find(|timeline| timeline.trace_id() == *trace_id)
            .cloned()
    }
}

impl Drop for TraceCollector {
    fn drop(&mut self) {
        // Clean up socket file
        if Path::new(&self.config.socket_path).exists() {
            let _ = std::fs::remove_file(&self.config.socket_path);
        }
    }
}
