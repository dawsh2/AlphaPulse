//! Web API Server for TraceCollector
//!
//! Provides HTTP endpoints for the trace visualization web interface.

use crate::{
    hex_to_trace_id, CollectorHealth, HealthReporter, Result, TraceCollectorStats, TraceError,
    TraceId, TraceTimeline,
};

use dashmap::DashMap;
use parking_lot::RwLock;
use ringbuffer::{AllocRingBuffer, RingBuffer};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{error, info};

/// HTTP API server for trace data
#[derive(Clone)]
pub struct TraceApiServer {
    /// HTTP port to bind to
    port: u16,

    /// Active traces reference
    active_traces: Arc<DashMap<TraceId, TraceTimeline>>,

    /// Completed traces reference
    completed_traces: Arc<RwLock<AllocRingBuffer<TraceTimeline>>>,

    /// Statistics reference
    stats: Arc<RwLock<TraceCollectorStats>>,

    /// Health reporter
    health_reporter: HealthReporter,
}

/// Query parameters for trace API
#[derive(Debug, Deserialize)]
pub struct TraceQuery {
    /// Limit number of results
    pub limit: Option<usize>,

    /// Filter by service
    pub service: Option<String>,

    /// Filter by status (active, completed, healthy, unhealthy)
    pub status: Option<String>,

    /// Minimum duration in milliseconds
    pub min_duration_ms: Option<f64>,

    /// Maximum duration in milliseconds
    pub max_duration_ms: Option<f64>,

    /// Search in metadata
    pub search: Option<String>,
}

/// API response wrapper
#[derive(Debug, Serialize)]
pub struct TraceResponse<T> {
    /// Response data
    pub data: T,

    /// Response metadata
    pub meta: ResponseMetadata,
}

/// Response metadata
#[derive(Debug, Serialize)]
pub struct ResponseMetadata {
    /// Total number of items (before limit)
    pub total: usize,

    /// Number of items returned
    pub count: usize,

    /// Query processing time in milliseconds
    pub query_time_ms: f64,

    /// Server timestamp
    pub timestamp: u64,
}

/// Summary statistics for dashboard
#[derive(Debug, Serialize)]
pub struct DashboardSummary {
    /// Overall system health
    pub health: CollectorHealth,

    /// Collector statistics
    pub stats: TraceCollectorStats,

    /// Recent trace summaries
    pub recent_traces: Vec<TraceSummary>,

    /// Service performance breakdown
    pub service_performance: HashMap<String, ServiceMetrics>,

    /// System recommendations
    pub recommendations: Vec<String>,
}

/// Condensed trace summary for lists
#[derive(Debug, Serialize)]
pub struct TraceSummary {
    /// Trace ID as hex string
    pub trace_id: String,

    /// Origin service
    pub origin: String,

    /// Destination service (if known)
    pub destination: Option<String>,

    /// Number of hops
    pub hop_count: usize,

    /// Total duration in milliseconds
    pub duration_ms: f64,

    /// Whether execution was triggered
    pub execution_triggered: bool,

    /// Whether trace has errors
    pub has_errors: bool,

    /// Trace status
    pub status: String,

    /// Start timestamp
    pub start_time_ns: u64,
}

/// Service performance metrics
#[derive(Debug, Serialize)]
pub struct ServiceMetrics {
    /// Service name
    pub service: String,

    /// Number of traces processed
    pub trace_count: usize,

    /// Average processing latency
    pub avg_latency_ms: f64,

    /// 95th percentile latency
    pub p95_latency_ms: f64,

    /// Error rate percentage
    pub error_rate: f64,

    /// Throughput (traces per minute)
    pub throughput: f64,
}

impl TraceApiServer {
    /// Create new API server
    pub async fn new(
        port: u16,
        active_traces: Arc<DashMap<TraceId, TraceTimeline>>,
        completed_traces: Arc<RwLock<AllocRingBuffer<TraceTimeline>>>,
        stats: Arc<RwLock<TraceCollectorStats>>,
    ) -> Result<Self> {
        let health_reporter = HealthReporter::new(stats.clone());

        Ok(Self {
            port,
            active_traces,
            completed_traces,
            stats,
            health_reporter,
        })
    }

    /// Start the API server
    pub async fn start(&self) -> Result<()> {
        let addr = format!("0.0.0.0:{}", self.port);
        let listener = TcpListener::bind(&addr)
            .await
            .map_err(|e| TraceError::Network(format!("Failed to bind to {}: {}", addr, e)))?;

        info!("TraceCollector API server listening on http://{}", addr);

        // In a real implementation, this would use a proper HTTP framework like axum or warp
        // For now, this is a placeholder showing the intended API structure

        loop {
            match listener.accept().await {
                Ok((_stream, peer_addr)) => {
                    info!("API connection from {}", peer_addr);
                    // TODO: Handle HTTP request/response cycle
                    // This would parse the request, route to appropriate handler,
                    // and send back JSON responses
                }
                Err(e) => {
                    error!("Failed to accept API connection: {}", e);
                }
            }
        }
    }

    /// Get dashboard summary
    pub async fn get_dashboard_summary(
        &self,
        query: Option<TraceQuery>,
    ) -> Result<DashboardSummary> {
        let _start_time = std::time::Instant::now();

        // Get health status
        let health = self.health_reporter.check_health().await?;

        // Get current statistics
        let stats = self.stats.read().clone();

        // Get recent traces
        let recent_traces =
            self.get_recent_trace_summaries(query.as_ref().and_then(|q| q.limit).unwrap_or(20));

        // Calculate service performance metrics
        let service_performance = self.calculate_service_metrics().await;

        // Get recommendations
        let recommendations = self.health_reporter.get_recommendations().await;

        Ok(DashboardSummary {
            health,
            stats,
            recent_traces,
            service_performance,
            recommendations,
        })
    }

    /// Get trace by ID
    pub async fn get_trace(
        &self,
        trace_id_hex: &str,
    ) -> Result<TraceResponse<Option<TraceTimeline>>> {
        let start_time = std::time::Instant::now();

        let trace_id = hex_to_trace_id(trace_id_hex)?;

        // Check active traces first
        let trace = if let Some(timeline) = self.active_traces.get(&trace_id) {
            Some(timeline.clone())
        } else {
            // Check completed traces
            let completed = self.completed_traces.read();
            completed.iter().find(|t| t.trace_id() == trace_id).cloned()
        };

        let query_time_ms = start_time.elapsed().as_secs_f64() * 1000.0;

        let count = if trace.is_some() { 1 } else { 0 };
        Ok(TraceResponse {
            data: trace,
            meta: ResponseMetadata {
                total: count,
                count,
                query_time_ms,
                timestamp: current_timestamp_ns(),
            },
        })
    }

    /// Get list of traces
    pub async fn get_traces(&self, query: TraceQuery) -> Result<TraceResponse<Vec<TraceSummary>>> {
        let start_time = std::time::Instant::now();

        let mut traces = Vec::new();

        // Get active traces
        for trace in self.active_traces.iter() {
            let summary = self.trace_to_summary(&trace);
            if self.matches_query(&summary, &query) {
                traces.push(summary);
            }
        }

        // Get completed traces
        {
            let completed = self.completed_traces.read();
            for trace in completed.iter() {
                let summary = self.trace_to_summary(trace);
                if self.matches_query(&summary, &query) {
                    traces.push(summary);
                }
            }
        }

        // Sort by start time (most recent first)
        traces.sort_by(|a, b| b.start_time_ns.cmp(&a.start_time_ns));

        let total = traces.len();

        // Apply limit
        if let Some(limit) = query.limit {
            traces.truncate(limit);
        }

        let count = traces.len();
        let query_time_ms = start_time.elapsed().as_secs_f64() * 1000.0;

        Ok(TraceResponse {
            data: traces,
            meta: ResponseMetadata {
                total,
                count,
                query_time_ms,
                timestamp: current_timestamp_ns(),
            },
        })
    }

    /// Get collector health status
    pub async fn get_health(&self) -> Result<TraceResponse<CollectorHealth>> {
        let start_time = std::time::Instant::now();
        let health = self.health_reporter.check_health().await?;
        let query_time_ms = start_time.elapsed().as_secs_f64() * 1000.0;

        Ok(TraceResponse {
            data: health,
            meta: ResponseMetadata {
                total: 1,
                count: 1,
                query_time_ms,
                timestamp: current_timestamp_ns(),
            },
        })
    }

    /// Get collector statistics
    pub async fn get_stats(&self) -> Result<TraceResponse<TraceCollectorStats>> {
        let start_time = std::time::Instant::now();
        let stats = self.stats.read().clone();
        let query_time_ms = start_time.elapsed().as_secs_f64() * 1000.0;

        Ok(TraceResponse {
            data: stats,
            meta: ResponseMetadata {
                total: 1,
                count: 1,
                query_time_ms,
                timestamp: current_timestamp_ns(),
            },
        })
    }

    /// Convert trace timeline to summary
    fn trace_to_summary(&self, trace: &TraceTimeline) -> TraceSummary {
        TraceSummary {
            trace_id: trace.trace_id_hex(),
            origin: format!("{:?}", trace.message_flow.origin),
            destination: trace.message_flow.destination.map(|d| format!("{:?}", d)),
            hop_count: trace.message_flow.hop_count,
            duration_ms: trace.total_duration_ns as f64 / 1_000_000.0,
            execution_triggered: trace.message_flow.execution_triggered,
            has_errors: trace.message_flow.has_errors,
            status: if trace.is_complete {
                if trace.message_flow.has_errors {
                    "error"
                } else {
                    "completed"
                }
            } else {
                "active"
            }
            .to_string(),
            start_time_ns: trace.spans.first().map(|s| s.timestamp_ns).unwrap_or(0),
        }
    }

    /// Check if trace summary matches query filters
    fn matches_query(&self, summary: &TraceSummary, query: &TraceQuery) -> bool {
        // Filter by service
        if let Some(ref service_filter) = query.service {
            if !summary
                .origin
                .to_lowercase()
                .contains(&service_filter.to_lowercase())
                && !summary
                    .destination
                    .as_ref()
                    .unwrap_or(&String::new())
                    .to_lowercase()
                    .contains(&service_filter.to_lowercase())
            {
                return false;
            }
        }

        // Filter by status
        if let Some(ref status_filter) = query.status {
            match status_filter.as_str() {
                "active" => {
                    if summary.status != "active" {
                        return false;
                    }
                }
                "completed" => {
                    if summary.status != "completed" {
                        return false;
                    }
                }
                "healthy" => {
                    if summary.has_errors {
                        return false;
                    }
                }
                "unhealthy" => {
                    if !summary.has_errors {
                        return false;
                    }
                }
                _ => {}
            }
        }

        // Filter by duration
        if let Some(min_duration) = query.min_duration_ms {
            if summary.duration_ms < min_duration {
                return false;
            }
        }

        if let Some(max_duration) = query.max_duration_ms {
            if summary.duration_ms > max_duration {
                return false;
            }
        }

        true
    }

    /// Get recent trace summaries
    fn get_recent_trace_summaries(&self, limit: usize) -> Vec<TraceSummary> {
        let mut summaries = Vec::new();

        // Get from completed traces
        {
            let completed = self.completed_traces.read();
            for trace in completed.iter().rev().take(limit) {
                summaries.push(self.trace_to_summary(trace));
            }
        }

        // Get from active traces (most recent)
        let mut active_summaries: Vec<_> = self
            .active_traces
            .iter()
            .map(|trace| self.trace_to_summary(&trace))
            .collect();
        active_summaries.sort_by(|a, b| b.start_time_ns.cmp(&a.start_time_ns));

        summaries.extend(active_summaries.into_iter().take(limit - summaries.len()));

        summaries.sort_by(|a, b| b.start_time_ns.cmp(&a.start_time_ns));
        summaries.truncate(limit);

        summaries
    }

    /// Calculate service performance metrics
    async fn calculate_service_metrics(&self) -> HashMap<String, ServiceMetrics> {
        let metrics = HashMap::new();

        // TODO: Implement service metrics calculation
        // This would analyze all traces to calculate:
        // - Average latency per service
        // - Throughput per service
        // - Error rates per service
        // - 95th percentile latencies

        metrics
    }
}

/// Get current timestamp in nanoseconds
fn current_timestamp_ns() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    // use std::collections::HashMap;

    #[tokio::test]
    async fn test_api_server_creation() {
        let active_traces = Arc::new(DashMap::new());
        let completed_traces = Arc::new(RwLock::new(AllocRingBuffer::with_capacity(100)));
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

        let api_server = TraceApiServer::new(8080, active_traces, completed_traces, stats).await;
        assert!(api_server.is_ok());
    }

    #[tokio::test]
    async fn test_trace_query_filtering() {
        let active_traces = Arc::new(DashMap::new());
        let completed_traces = Arc::new(RwLock::new(AllocRingBuffer::with_capacity(100)));
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

        let api_server = TraceApiServer::new(8080, active_traces, completed_traces, stats)
            .await
            .unwrap();

        let query = TraceQuery {
            limit: Some(10),
            service: Some("polygon".to_string()),
            status: Some("completed".to_string()),
            min_duration_ms: Some(1.0),
            max_duration_ms: Some(100.0),
            search: None,
        };

        let response = api_server.get_traces(query).await.unwrap();
        assert_eq!(response.data.len(), 0); // No traces in empty collector
    }
}
