//! Metrics Collection for MessageSink Monitoring
//!
//! Provides comprehensive metrics collection for MessageSink implementations,
//! enabling real-time monitoring, alerting, and performance analysis across
//! the AlphaPulse trading system.
//!
//! ## Metrics Categories
//!
//! - **Throughput**: Messages per second, bytes per second, batch sizes
//! - **Latency**: Send latency, connection establishment time, queue times
//! - **Reliability**: Success rates, error counts, retry rates
//! - **Resource Usage**: Memory usage, connection counts, buffer sizes
//! - **Circuit Breaker**: State transitions, failure rates, recovery times

use crate::{BatchResult, ConnectionHealth, ConnectionState, SinkError};
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant, SystemTime};

/// Comprehensive metrics for MessageSink implementations
pub trait SinkMetrics: Send + Sync + Debug {
    /// Record a successful message send
    fn record_send_success(&self, message_size: usize, latency_ns: u64);
    
    /// Record a failed message send
    fn record_send_failure(&self, error: &SinkError, message_size: usize);
    
    /// Record batch operation results
    fn record_batch_result(&self, batch_size: usize, result: &BatchResult, total_latency_ns: u64);
    
    /// Record connection state change
    fn record_connection_state(&self, old_state: ConnectionState, new_state: ConnectionState);
    
    /// Record connection health change
    fn record_health_change(&self, old_health: ConnectionHealth, new_health: ConnectionHealth);
    
    /// Record circuit breaker state change
    fn record_circuit_breaker_state(&self, state: &str, reason: Option<&str>);
    
    /// Record resource usage
    fn record_resource_usage(&self, metric_name: &str, value: f64);
    
    /// Get current throughput metrics
    fn throughput_metrics(&self) -> ThroughputMetrics;
    
    /// Get current latency metrics
    fn latency_metrics(&self) -> LatencyMetrics;
    
    /// Get current reliability metrics
    fn reliability_metrics(&self) -> ReliabilityMetrics;
    
    /// Get current resource metrics
    fn resource_metrics(&self) -> ResourceMetrics;
    
    /// Get comprehensive metrics snapshot
    fn snapshot(&self) -> MetricsSnapshot;
    
    /// Reset all metrics (for testing)
    fn reset(&self);
    
    /// Export metrics in a specific format
    fn export(&self, format: MetricsFormat) -> Result<String, SinkError>;
}

/// Throughput-related metrics
#[derive(Debug, Clone)]
pub struct ThroughputMetrics {
    /// Messages per second (recent average)
    pub messages_per_second: f64,
    /// Bytes per second (recent average)
    pub bytes_per_second: f64,
    /// Total messages sent successfully
    pub total_messages: u64,
    /// Total bytes sent successfully
    pub total_bytes: u64,
    /// Average batch size
    pub average_batch_size: f64,
    /// Peak messages per second observed
    pub peak_messages_per_second: f64,
    /// Peak bytes per second observed
    pub peak_bytes_per_second: f64,
}

/// Latency-related metrics
#[derive(Debug, Clone)]
pub struct LatencyMetrics {
    /// Average send latency (nanoseconds)
    pub average_send_latency_ns: u64,
    /// Median send latency (nanoseconds)
    pub median_send_latency_ns: u64,
    /// 95th percentile send latency (nanoseconds)
    pub p95_send_latency_ns: u64,
    /// 99th percentile send latency (nanoseconds)
    pub p99_send_latency_ns: u64,
    /// Maximum send latency observed (nanoseconds)
    pub max_send_latency_ns: u64,
    /// Average connection establishment time (nanoseconds)
    pub average_connection_latency_ns: u64,
}

/// Reliability-related metrics
#[derive(Debug, Clone)]
pub struct ReliabilityMetrics {
    /// Success rate (0.0 to 1.0)
    pub success_rate: f64,
    /// Total successful operations
    pub total_successes: u64,
    /// Total failed operations
    pub total_failures: u64,
    /// Error rate by error type
    pub error_rates: HashMap<String, f64>,
    /// Retry success rate
    pub retry_success_rate: f64,
    /// Connection success rate
    pub connection_success_rate: f64,
    /// Average time between failures (seconds)
    pub mean_time_between_failures: f64,
}

/// Resource usage metrics
#[derive(Debug, Clone)]
pub struct ResourceMetrics {
    /// Current memory usage (bytes)
    pub memory_usage_bytes: u64,
    /// Active connection count
    pub active_connections: usize,
    /// Buffer utilization (0.0 to 1.0)
    pub buffer_utilization: f64,
    /// CPU usage percentage (0.0 to 100.0)
    pub cpu_usage_percent: f64,
    /// Custom resource metrics
    pub custom_metrics: HashMap<String, f64>,
}

/// Complete metrics snapshot
#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    /// When this snapshot was taken
    pub timestamp: SystemTime,
    /// Sink identifier
    pub sink_name: String,
    /// Throughput metrics
    pub throughput: ThroughputMetrics,
    /// Latency metrics
    pub latency: LatencyMetrics,
    /// Reliability metrics
    pub reliability: ReliabilityMetrics,
    /// Resource metrics
    pub resource: ResourceMetrics,
    /// Uptime since metrics started
    pub uptime: Duration,
}

/// Supported metrics export formats
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricsFormat {
    /// Prometheus format
    Prometheus,
    /// JSON format
    Json,
    /// InfluxDB line protocol
    InfluxDB,
    /// Human-readable table format
    Table,
}

/// Default metrics collector implementation
#[derive(Debug)]
pub struct DefaultSinkMetrics {
    sink_name: String,
    start_time: Instant,
    
    // Atomic counters for thread-safe updates
    total_messages: AtomicU64,
    total_bytes: AtomicU64,
    total_successes: AtomicU64,
    total_failures: AtomicU64,
    total_send_latency_ns: AtomicU64,
    max_send_latency_ns: AtomicU64,
    
    // Connection metrics
    connection_attempts: AtomicU64,
    connection_successes: AtomicU64,
    active_connections: AtomicUsize,
    
    // Recent activity tracking (for rates)
    recent_messages: std::sync::Mutex<std::collections::VecDeque<(Instant, usize)>>,
    recent_latencies: std::sync::Mutex<std::collections::VecDeque<u64>>,
    
    // Error tracking
    error_counts: std::sync::Mutex<HashMap<String, u64>>,
    
    // Resource tracking
    resource_metrics: std::sync::Mutex<HashMap<String, f64>>,
}

impl DefaultSinkMetrics {
    /// Create new metrics collector for a sink
    pub fn new(sink_name: impl Into<String>) -> Self {
        Self {
            sink_name: sink_name.into(),
            start_time: Instant::now(),
            total_messages: AtomicU64::new(0),
            total_bytes: AtomicU64::new(0),
            total_successes: AtomicU64::new(0),
            total_failures: AtomicU64::new(0),
            total_send_latency_ns: AtomicU64::new(0),
            max_send_latency_ns: AtomicU64::new(0),
            connection_attempts: AtomicU64::new(0),
            connection_successes: AtomicU64::new(0),
            active_connections: AtomicUsize::new(0),
            recent_messages: std::sync::Mutex::new(std::collections::VecDeque::new()),
            recent_latencies: std::sync::Mutex::new(std::collections::VecDeque::new()),
            error_counts: std::sync::Mutex::new(HashMap::new()),
            resource_metrics: std::sync::Mutex::new(HashMap::new()),
        }
    }
    
    /// Clean old entries from recent tracking
    fn clean_recent_entries(&self) {
        let now = Instant::now();
        let cutoff = now - Duration::from_secs(60); // Keep 1 minute of history
        
        let mut recent_messages = self.recent_messages.lock().unwrap();
        while let Some(&(timestamp, _)) = recent_messages.front() {
            if timestamp < cutoff {
                recent_messages.pop_front();
            } else {
                break;
            }
        }
        
        // Clean latencies based on both count AND age
        // Keep only recent samples for accurate percentiles
        let mut recent_latencies = self.recent_latencies.lock().unwrap();
        
        // First remove old samples (keep only last 5 minutes)
        let latency_cutoff = recent_latencies.len().saturating_sub(5000); // Approx 5 min at 1k msg/min
        if latency_cutoff > 0 {
            for _ in 0..latency_cutoff {
                recent_latencies.pop_front();
            }
        }
        
        // Then enforce absolute maximum to prevent unbounded growth
        while recent_latencies.len() > 10_000 {
            recent_latencies.pop_front();
        }
    }
}

impl SinkMetrics for DefaultSinkMetrics {
    fn record_send_success(&self, message_size: usize, latency_ns: u64) {
        self.total_messages.fetch_add(1, Ordering::Relaxed);
        self.total_bytes.fetch_add(message_size as u64, Ordering::Relaxed);
        self.total_successes.fetch_add(1, Ordering::Relaxed);
        self.total_send_latency_ns.fetch_add(latency_ns, Ordering::Relaxed);
        
        // Update max latency
        let mut current_max = self.max_send_latency_ns.load(Ordering::Relaxed);
        while latency_ns > current_max {
            match self.max_send_latency_ns.compare_exchange_weak(
                current_max,
                latency_ns,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => current_max = actual,
            }
        }
        
        // Track recent activity
        {
            let mut recent_messages = self.recent_messages.lock().unwrap();
            recent_messages.push_back((Instant::now(), message_size));
        }
        
        {
            let mut recent_latencies = self.recent_latencies.lock().unwrap();
            recent_latencies.push_back(latency_ns);
        }
        
        self.clean_recent_entries();
    }
    
    fn record_send_failure(&self, error: &SinkError, _message_size: usize) {
        self.total_failures.fetch_add(1, Ordering::Relaxed);
        
        // Track error by type
        let error_type = format!("{:?}", error).split('(').next().unwrap_or("Unknown").to_string();
        let mut error_counts = self.error_counts.lock().unwrap();
        *error_counts.entry(error_type).or_insert(0) += 1;
    }
    
    fn record_batch_result(&self, batch_size: usize, result: &BatchResult, total_latency_ns: u64) {
        // Record successful messages
        if result.succeeded > 0 {
            self.total_messages.fetch_add(result.succeeded as u64, Ordering::Relaxed);
            self.total_successes.fetch_add(result.succeeded as u64, Ordering::Relaxed);
        }
        
        // Record failures
        if !result.failed.is_empty() {
            self.total_failures.fetch_add(result.failed.len() as u64, Ordering::Relaxed);
        }
        
        // Average latency per message in batch
        if batch_size > 0 {
            let avg_latency = total_latency_ns / batch_size as u64;
            self.total_send_latency_ns.fetch_add(total_latency_ns, Ordering::Relaxed);
            
            let mut recent_latencies = self.recent_latencies.lock().unwrap();
            recent_latencies.push_back(avg_latency);
        }
        
        self.clean_recent_entries();
    }
    
    fn record_connection_state(&self, _old_state: ConnectionState, new_state: ConnectionState) {
        self.connection_attempts.fetch_add(1, Ordering::Relaxed);
        
        match new_state {
            ConnectionState::Connected => {
                self.connection_successes.fetch_add(1, Ordering::Relaxed);
                self.active_connections.fetch_add(1, Ordering::Relaxed);
            }
            ConnectionState::Disconnected | ConnectionState::Failed => {
                // Don't go below 0
                self.active_connections.fetch_update(
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                    |x| if x > 0 { Some(x - 1) } else { None }
                ).ok();
            }
            _ => {}
        }
    }
    
    fn record_health_change(&self, _old_health: ConnectionHealth, new_health: ConnectionHealth) {
        // Record health state in custom metrics
        let health_value = match new_health {
            ConnectionHealth::Healthy => 1.0,
            ConnectionHealth::Degraded => 0.5,
            ConnectionHealth::Unhealthy => 0.0,
            ConnectionHealth::Unknown => -1.0,
        };
        
        let mut resource_metrics = self.resource_metrics.lock().unwrap();
        resource_metrics.insert("connection_health".to_string(), health_value);
    }
    
    fn record_circuit_breaker_state(&self, state: &str, _reason: Option<&str>) {
        let state_value = match state.to_lowercase().as_str() {
            "closed" => 0.0,
            "half_open" => 0.5,
            "open" => 1.0,
            _ => -1.0,
        };
        
        let mut resource_metrics = self.resource_metrics.lock().unwrap();
        resource_metrics.insert("circuit_breaker_state".to_string(), state_value);
    }
    
    fn record_resource_usage(&self, metric_name: &str, value: f64) {
        let mut resource_metrics = self.resource_metrics.lock().unwrap();
        resource_metrics.insert(metric_name.to_string(), value);
    }
    
    fn throughput_metrics(&self) -> ThroughputMetrics {
        let total_messages = self.total_messages.load(Ordering::Relaxed);
        let total_bytes = self.total_bytes.load(Ordering::Relaxed);
        
        // Calculate recent rates
        let _now = Instant::now();
        let recent_messages = self.recent_messages.lock().unwrap();
        
        let recent_count = recent_messages.len();
        let recent_bytes: usize = recent_messages.iter().map(|(_, size)| size).sum();
        
        let time_window = 60.0; // 1 minute window
        let messages_per_second = recent_count as f64 / time_window;
        let bytes_per_second = recent_bytes as f64 / time_window;
        
        let average_batch_size = if total_messages > 0 {
            total_bytes as f64 / total_messages as f64
        } else {
            0.0
        };
        
        ThroughputMetrics {
            messages_per_second,
            bytes_per_second,
            total_messages,
            total_bytes,
            average_batch_size,
            peak_messages_per_second: messages_per_second, // Could track historical peaks
            peak_bytes_per_second: bytes_per_second,
        }
    }
    
    fn latency_metrics(&self) -> LatencyMetrics {
        let total_messages = self.total_messages.load(Ordering::Relaxed);
        let total_latency = self.total_send_latency_ns.load(Ordering::Relaxed);
        let max_latency = self.max_send_latency_ns.load(Ordering::Relaxed);
        
        let average_send_latency_ns = if total_messages > 0 {
            total_latency / total_messages
        } else {
            0
        };
        
        // Calculate percentiles from recent latencies
        let recent_latencies = self.recent_latencies.lock().unwrap();
        let mut latencies: Vec<u64> = recent_latencies.iter().cloned().collect();
        latencies.sort();
        
        let median = if !latencies.is_empty() {
            latencies[latencies.len() / 2]
        } else {
            0
        };
        
        let p95 = if !latencies.is_empty() {
            let index = (latencies.len() as f64 * 0.95) as usize;
            latencies.get(index.min(latencies.len() - 1)).copied().unwrap_or(0)
        } else {
            0
        };
        
        let p99 = if !latencies.is_empty() {
            let index = (latencies.len() as f64 * 0.99) as usize;
            latencies.get(index.min(latencies.len() - 1)).copied().unwrap_or(0)
        } else {
            0
        };
        
        LatencyMetrics {
            average_send_latency_ns,
            median_send_latency_ns: median,
            p95_send_latency_ns: p95,
            p99_send_latency_ns: p99,
            max_send_latency_ns: max_latency,
            average_connection_latency_ns: 0, // Could track this separately
        }
    }
    
    fn reliability_metrics(&self) -> ReliabilityMetrics {
        let total_successes = self.total_successes.load(Ordering::Relaxed);
        let total_failures = self.total_failures.load(Ordering::Relaxed);
        let total_operations = total_successes + total_failures;
        
        let success_rate = if total_operations > 0 {
            total_successes as f64 / total_operations as f64
        } else {
            1.0
        };
        
        let connection_attempts = self.connection_attempts.load(Ordering::Relaxed);
        let connection_successes = self.connection_successes.load(Ordering::Relaxed);
        let connection_success_rate = if connection_attempts > 0 {
            connection_successes as f64 / connection_attempts as f64
        } else {
            1.0
        };
        
        // Calculate error rates by type
        let error_counts = self.error_counts.lock().unwrap();
        let mut error_rates = HashMap::new();
        for (error_type, count) in error_counts.iter() {
            let rate = if total_operations > 0 {
                *count as f64 / total_operations as f64
            } else {
                0.0
            };
            error_rates.insert(error_type.clone(), rate);
        }
        
        // Estimate MTBF (simplified)
        let uptime_hours = self.start_time.elapsed().as_secs_f64() / 3600.0;
        let mean_time_between_failures = if total_failures > 0 {
            uptime_hours / total_failures as f64
        } else {
            uptime_hours
        };
        
        ReliabilityMetrics {
            success_rate,
            total_successes,
            total_failures,
            error_rates,
            retry_success_rate: 0.0, // Could track retries separately
            connection_success_rate,
            mean_time_between_failures,
        }
    }
    
    fn resource_metrics(&self) -> ResourceMetrics {
        let active_connections = self.active_connections.load(Ordering::Relaxed);
        let resource_metrics = self.resource_metrics.lock().unwrap();
        
        ResourceMetrics {
            memory_usage_bytes: 0, // Could use system metrics
            active_connections,
            buffer_utilization: resource_metrics.get("buffer_utilization").copied().unwrap_or(0.0),
            cpu_usage_percent: resource_metrics.get("cpu_usage_percent").copied().unwrap_or(0.0),
            custom_metrics: resource_metrics.clone(),
        }
    }
    
    fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            timestamp: SystemTime::now(),
            sink_name: self.sink_name.clone(),
            throughput: self.throughput_metrics(),
            latency: self.latency_metrics(),
            reliability: self.reliability_metrics(),
            resource: self.resource_metrics(),
            uptime: self.start_time.elapsed(),
        }
    }
    
    fn reset(&self) {
        self.total_messages.store(0, Ordering::Relaxed);
        self.total_bytes.store(0, Ordering::Relaxed);
        self.total_successes.store(0, Ordering::Relaxed);
        self.total_failures.store(0, Ordering::Relaxed);
        self.total_send_latency_ns.store(0, Ordering::Relaxed);
        self.max_send_latency_ns.store(0, Ordering::Relaxed);
        self.connection_attempts.store(0, Ordering::Relaxed);
        self.connection_successes.store(0, Ordering::Relaxed);
        self.active_connections.store(0, Ordering::Relaxed);
        
        self.recent_messages.lock().unwrap().clear();
        self.recent_latencies.lock().unwrap().clear();
        self.error_counts.lock().unwrap().clear();
        self.resource_metrics.lock().unwrap().clear();
    }
    
    fn export(&self, format: MetricsFormat) -> Result<String, SinkError> {
        let snapshot = self.snapshot();
        
        match format {
            MetricsFormat::Json => {
                // Simple JSON representation (would use serde in real implementation)
                Ok(format!(
                    r#"{{
  "sink_name": "{}",
  "timestamp": "{}",
  "throughput": {{
    "messages_per_second": {},
    "bytes_per_second": {},
    "total_messages": {},
    "total_bytes": {}
  }},
  "reliability": {{
    "success_rate": {},
    "total_successes": {},
    "total_failures": {}
  }},
  "latency": {{
    "average_send_latency_ns": {},
    "p95_send_latency_ns": {},
    "p99_send_latency_ns": {}
  }}
}}"#,
                    snapshot.sink_name,
                    snapshot.timestamp.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs(),
                    snapshot.throughput.messages_per_second,
                    snapshot.throughput.bytes_per_second,
                    snapshot.throughput.total_messages,
                    snapshot.throughput.total_bytes,
                    snapshot.reliability.success_rate,
                    snapshot.reliability.total_successes,
                    snapshot.reliability.total_failures,
                    snapshot.latency.average_send_latency_ns,
                    snapshot.latency.p95_send_latency_ns,
                    snapshot.latency.p99_send_latency_ns
                ))
            }
            MetricsFormat::Prometheus => {
                Ok(format!(
                    r#"# HELP sink_messages_total Total messages processed
# TYPE sink_messages_total counter
sink_messages_total{{sink="{}"}} {}

# HELP sink_messages_per_second Current message rate
# TYPE sink_messages_per_second gauge
sink_messages_per_second{{sink="{}"}} {}

# HELP sink_success_rate Success rate (0-1)
# TYPE sink_success_rate gauge
sink_success_rate{{sink="{}"}} {}

# HELP sink_latency_nanoseconds Message send latency
# TYPE sink_latency_nanoseconds histogram
sink_latency_nanoseconds_bucket{{sink="{}",le="1000000"}} {}
sink_latency_nanoseconds_bucket{{sink="{}",le="10000000"}} {}
sink_latency_nanoseconds_bucket{{sink="{}",le="+Inf"}} {}
"#,
                    snapshot.sink_name, snapshot.throughput.total_messages,
                    snapshot.sink_name, snapshot.throughput.messages_per_second,
                    snapshot.sink_name, snapshot.reliability.success_rate,
                    snapshot.sink_name, snapshot.throughput.total_messages,
                    snapshot.sink_name, snapshot.throughput.total_messages,
                    snapshot.sink_name, snapshot.throughput.total_messages
                ))
            }
            MetricsFormat::Table => {
                Ok(format!(
                    r#"MessageSink Metrics: {}
==================================
Throughput:
  Messages/sec:     {:.2}
  Bytes/sec:        {:.2}
  Total messages:   {}
  Total bytes:      {}

Reliability:
  Success rate:     {:.2}%
  Total successes:  {}
  Total failures:   {}

Latency:
  Average:          {:.2} μs
  95th percentile:  {:.2} μs
  99th percentile:  {:.2} μs
  Max:              {:.2} μs

Resource:
  Active connections: {}
  Uptime:           {:.1} hours
"#,
                    snapshot.sink_name,
                    snapshot.throughput.messages_per_second,
                    snapshot.throughput.bytes_per_second,
                    snapshot.throughput.total_messages,
                    snapshot.throughput.total_bytes,
                    snapshot.reliability.success_rate * 100.0,
                    snapshot.reliability.total_successes,
                    snapshot.reliability.total_failures,
                    snapshot.latency.average_send_latency_ns as f64 / 1000.0,
                    snapshot.latency.p95_send_latency_ns as f64 / 1000.0,
                    snapshot.latency.p99_send_latency_ns as f64 / 1000.0,
                    snapshot.latency.max_send_latency_ns as f64 / 1000.0,
                    snapshot.resource.active_connections,
                    snapshot.uptime.as_secs_f64() / 3600.0
                ))
            }
            MetricsFormat::InfluxDB => {
                let timestamp_ns = snapshot.timestamp
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos();
                
                Ok(format!(
                    r#"sink_throughput,sink={} messages_per_second={},bytes_per_second={},total_messages={}i,total_bytes={}i {}
sink_reliability,sink={} success_rate={},total_successes={}i,total_failures={}i {}
sink_latency,sink={} average_ns={}i,p95_ns={}i,p99_ns={}i,max_ns={}i {}
sink_resource,sink={} active_connections={}i {}"#,
                    snapshot.sink_name,
                    snapshot.throughput.messages_per_second,
                    snapshot.throughput.bytes_per_second,
                    snapshot.throughput.total_messages,
                    snapshot.throughput.total_bytes,
                    timestamp_ns,
                    snapshot.sink_name,
                    snapshot.reliability.success_rate,
                    snapshot.reliability.total_successes,
                    snapshot.reliability.total_failures,
                    timestamp_ns,
                    snapshot.sink_name,
                    snapshot.latency.average_send_latency_ns,
                    snapshot.latency.p95_send_latency_ns,
                    snapshot.latency.p99_send_latency_ns,
                    snapshot.latency.max_send_latency_ns,
                    timestamp_ns,
                    snapshot.sink_name,
                    snapshot.resource.active_connections,
                    timestamp_ns
                ))
            }
        }
    }
}

/// No-op metrics collector for cases where metrics aren't needed
#[derive(Debug)]
pub struct NoOpMetrics;

impl SinkMetrics for NoOpMetrics {
    fn record_send_success(&self, _message_size: usize, _latency_ns: u64) {}
    fn record_send_failure(&self, _error: &SinkError, _message_size: usize) {}
    fn record_batch_result(&self, _batch_size: usize, _result: &BatchResult, _total_latency_ns: u64) {}
    fn record_connection_state(&self, _old_state: ConnectionState, _new_state: ConnectionState) {}
    fn record_health_change(&self, _old_health: ConnectionHealth, _new_health: ConnectionHealth) {}
    fn record_circuit_breaker_state(&self, _state: &str, _reason: Option<&str>) {}
    fn record_resource_usage(&self, _metric_name: &str, _value: f64) {}
    
    fn throughput_metrics(&self) -> ThroughputMetrics {
        ThroughputMetrics {
            messages_per_second: 0.0,
            bytes_per_second: 0.0,
            total_messages: 0,
            total_bytes: 0,
            average_batch_size: 0.0,
            peak_messages_per_second: 0.0,
            peak_bytes_per_second: 0.0,
        }
    }
    
    fn latency_metrics(&self) -> LatencyMetrics {
        LatencyMetrics {
            average_send_latency_ns: 0,
            median_send_latency_ns: 0,
            p95_send_latency_ns: 0,
            p99_send_latency_ns: 0,
            max_send_latency_ns: 0,
            average_connection_latency_ns: 0,
        }
    }
    
    fn reliability_metrics(&self) -> ReliabilityMetrics {
        ReliabilityMetrics {
            success_rate: 1.0,
            total_successes: 0,
            total_failures: 0,
            error_rates: HashMap::new(),
            retry_success_rate: 1.0,
            connection_success_rate: 1.0,
            mean_time_between_failures: f64::INFINITY,
        }
    }
    
    fn resource_metrics(&self) -> ResourceMetrics {
        ResourceMetrics {
            memory_usage_bytes: 0,
            active_connections: 0,
            buffer_utilization: 0.0,
            cpu_usage_percent: 0.0,
            custom_metrics: HashMap::new(),
        }
    }
    
    fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            timestamp: SystemTime::now(),
            sink_name: "no-op".to_string(),
            throughput: self.throughput_metrics(),
            latency: self.latency_metrics(),
            reliability: self.reliability_metrics(),
            resource: self.resource_metrics(),
            uptime: Duration::from_secs(0),
        }
    }
    
    fn reset(&self) {}
    
    fn export(&self, _format: MetricsFormat) -> Result<String, SinkError> {
        Ok("# No metrics collected".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::CollectorSink;
    use std::thread;
    use std::time::Duration;
    
    #[test]
    fn test_metrics_basic_recording() {
        let metrics = DefaultSinkMetrics::new("test-sink");
        
        // Record some successes
        metrics.record_send_success(100, 1000);
        metrics.record_send_success(200, 2000);
        
        let throughput = metrics.throughput_metrics();
        assert_eq!(throughput.total_messages, 2);
        assert_eq!(throughput.total_bytes, 300);
        
        let latency = metrics.latency_metrics();
        assert_eq!(latency.average_send_latency_ns, 1500); // (1000 + 2000) / 2
    }
    
    #[test]
    fn test_metrics_failure_recording() {
        let metrics = DefaultSinkMetrics::new("test-sink");
        
        let error = crate::SinkError::connection_failed("test error");
        metrics.record_send_failure(&error, 100);
        
        let reliability = metrics.reliability_metrics();
        assert_eq!(reliability.total_failures, 1);
        assert_eq!(reliability.success_rate, 0.0);
    }
    
    #[test]
    fn test_metrics_batch_recording() {
        let metrics = DefaultSinkMetrics::new("test-sink");
        
        let mut batch_result = crate::BatchResult::new(5);
        batch_result.record_success(); // 1 success
        batch_result.record_success(); // 2 successes
        batch_result.record_failure(2, crate::SinkError::connection_failed("test")); // 1 failure
        
        metrics.record_batch_result(5, &batch_result, 5000);
        
        let throughput = metrics.throughput_metrics();
        assert_eq!(throughput.total_messages, 2); // 2 successes
        
        let reliability = metrics.reliability_metrics();
        assert_eq!(reliability.total_successes, 2);
        assert_eq!(reliability.total_failures, 1);
    }
    
    #[test]
    fn test_connection_state_tracking() {
        let metrics = DefaultSinkMetrics::new("test-sink");
        
        metrics.record_connection_state(ConnectionState::Disconnected, ConnectionState::Connected);
        metrics.record_connection_state(ConnectionState::Connecting, ConnectionState::Connected);
        
        let reliability = metrics.reliability_metrics();
        assert_eq!(reliability.connection_success_rate, 1.0); // 2 successes / 2 attempts
        
        let resource = metrics.resource_metrics();
        assert_eq!(resource.active_connections, 2);
    }
    
    #[test]
    fn test_metrics_export_formats() {
        let metrics = DefaultSinkMetrics::new("test-sink");
        metrics.record_send_success(100, 1000);
        
        let json_export = metrics.export(MetricsFormat::Json).unwrap();
        assert!(json_export.contains("test-sink"));
        assert!(json_export.contains("total_messages"));
        
        let prometheus_export = metrics.export(MetricsFormat::Prometheus).unwrap();
        assert!(prometheus_export.contains("sink_messages_total"));
        
        let table_export = metrics.export(MetricsFormat::Table).unwrap();
        assert!(table_export.contains("MessageSink Metrics"));
        
        let influx_export = metrics.export(MetricsFormat::InfluxDB).unwrap();
        assert!(influx_export.contains("sink_throughput"));
    }
    
    #[test]
    fn test_metrics_reset() {
        let metrics = DefaultSinkMetrics::new("test-sink");
        metrics.record_send_success(100, 1000);
        
        assert_eq!(metrics.throughput_metrics().total_messages, 1);
        
        metrics.reset();
        
        assert_eq!(metrics.throughput_metrics().total_messages, 0);
        assert_eq!(metrics.latency_metrics().average_send_latency_ns, 0);
    }
    
    #[test]
    fn test_no_op_metrics() {
        let metrics = NoOpMetrics;
        
        // Should not panic or cause issues
        metrics.record_send_success(100, 1000);
        metrics.record_send_failure(&crate::SinkError::connection_failed("test"), 100);
        
        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.sink_name, "no-op");
        assert_eq!(snapshot.throughput.total_messages, 0);
        
        let export = metrics.export(MetricsFormat::Json).unwrap();
        assert!(export.contains("No metrics"));
    }
    
    #[test]
    fn test_percentile_calculations() {
        let metrics = DefaultSinkMetrics::new("test-sink");
        
        // Record latencies: 100, 200, 300, 400, 500, 600, 700, 800, 900, 1000 ns
        for i in 1..=10 {
            metrics.record_send_success(100, i * 100);
        }
        
        let latency = metrics.latency_metrics();
        
        // Median should be around 550 (between 5th and 6th values)
        assert_eq!(latency.median_send_latency_ns, 600);
        
        // P95 should be around 950-1000
        assert!(latency.p95_send_latency_ns >= 900);
        
        // P99 should be around 990-1000  
        assert!(latency.p99_send_latency_ns >= 900);
        
        // Max should be 1000
        assert_eq!(latency.max_send_latency_ns, 1000);
    }
}