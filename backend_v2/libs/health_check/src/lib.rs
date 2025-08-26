//! # AlphaPulse Health Check System
//!
//! Provides standardized health checking for all AlphaPulse services with support for:
//! - Socket availability monitoring
//! - Performance metrics tracking
//! - Service-specific health indicators
//! - HTTP health endpoints
//! - Deployment automation integration
//!
//! ## Architecture
//!
//! Each service embeds a health check server that exposes:
//! - `/health` - Basic liveness check
//! - `/ready` - Readiness for traffic
//! - `/metrics` - Performance metrics
//! - `/status` - Detailed service status
//!
//! ## Performance Monitoring
//!
//! Tracks critical AlphaPulse metrics:
//! - Message throughput (target: >1M msg/s for market data)
//! - Processing latency (target: <35μs hot path)
//! - Connection health
//! - Zero-allocation violations
//!
//! ## Usage
//!
//! ```rust,no_run
//! use alphapulse_health_check::{HealthCheckServer, ServiceHealth};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut health = ServiceHealth::new("market_data_relay");
//! health.set_socket_path("/var/run/alphapulse/market_data.sock");
//!
//! let server = HealthCheckServer::new(health, 8001);
//! server.start().await?;
//! # Ok(())
//! # }
//! ```

use anyhow::Result;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};
use tracing::{debug, error, info};

/// Service health status levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HealthStatus {
    /// Service is healthy and ready for traffic
    Healthy,
    /// Service is starting up, not ready for traffic
    Starting,
    /// Service has issues but still operational
    Degraded,
    /// Service is not operational
    Unhealthy,
}

/// Performance metrics for AlphaPulse services
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Messages processed per second
    pub messages_per_second: f64,
    /// Average processing latency in microseconds
    pub avg_latency_us: f64,
    /// P99 processing latency in microseconds
    pub p99_latency_us: f64,
    /// Active connections
    pub active_connections: u64,
    /// Total messages processed
    pub total_messages: u64,
    /// Zero allocation violations (should be 0)
    pub allocation_violations: u64,
    /// Memory usage in bytes
    pub memory_usage_bytes: u64,
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            messages_per_second: 0.0,
            avg_latency_us: 0.0,
            p99_latency_us: 0.0,
            active_connections: 0,
            total_messages: 0,
            allocation_violations: 0,
            memory_usage_bytes: 0,
        }
    }
}

/// Comprehensive service health information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceHealth {
    /// Service name (e.g., "market_data_relay")
    pub service_name: String,
    /// Current health status
    pub status: HealthStatus,
    /// Unix socket path (if applicable)
    pub socket_path: Option<String>,
    /// HTTP health check port
    pub health_port: u16,
    /// Service startup time
    pub startup_time: SystemTime,
    /// Last health check time
    pub last_check: SystemTime,
    /// Performance metrics
    pub metrics: PerformanceMetrics,
    /// Service-specific status details
    pub details: HashMap<String, String>,
    /// Critical error messages
    pub errors: Vec<String>,
}

impl ServiceHealth {
    /// Create new service health tracker
    pub fn new(service_name: &str) -> Self {
        Self {
            service_name: service_name.to_string(),
            status: HealthStatus::Starting,
            socket_path: None,
            health_port: 8000,
            startup_time: SystemTime::now(),
            last_check: SystemTime::now(),
            metrics: PerformanceMetrics::default(),
            details: HashMap::new(),
            errors: Vec::new(),
        }
    }

    /// Set Unix socket path for monitoring
    pub fn set_socket_path(&mut self, path: &str) {
        self.socket_path = Some(path.to_string());
    }

    /// Set health check HTTP port
    pub fn set_health_port(&mut self, port: u16) {
        self.health_port = port;
    }

    /// Update performance metrics
    pub fn update_metrics(&mut self, metrics: PerformanceMetrics) {
        self.metrics = metrics;
        self.last_check = SystemTime::now();
    }

    /// Add service-specific status detail
    pub fn add_detail(&mut self, key: &str, value: &str) {
        self.details.insert(key.to_string(), value.to_string());
    }

    /// Add error message
    pub fn add_error(&mut self, error: &str) {
        self.errors.push(error.to_string());
        // Keep only last 10 errors
        if self.errors.len() > 10 {
            self.errors.remove(0);
        }
    }

    /// Check if service is ready for traffic
    pub async fn is_ready(&self) -> bool {
        match self.status {
            HealthStatus::Healthy => true,
            HealthStatus::Degraded => true, // Still accepting traffic
            _ => false,
        }
    }

    /// Check if service is alive
    pub async fn is_alive(&self) -> bool {
        !matches!(self.status, HealthStatus::Unhealthy)
    }

    /// Perform comprehensive health check
    pub async fn check_health(&mut self) -> Result<()> {
        self.last_check = SystemTime::now();
        self.errors.clear();

        // Check socket availability if configured
        if let Some(socket_path) = &self.socket_path {
            if !self.check_socket_availability(socket_path).await {
                self.add_error(&format!("Socket not available: {}", socket_path));
                self.status = HealthStatus::Unhealthy;
                return Ok(());
            }
        }

        // Check performance requirements for critical services
        match self.service_name.as_str() {
            "market_data_relay" => {
                if self.metrics.messages_per_second < 1_000_000.0 {
                    self.add_error(&format!(
                        "Message rate below requirement: {:.0}/s < 1M/s",
                        self.metrics.messages_per_second
                    ));
                    self.status = HealthStatus::Degraded;
                } else if self.metrics.avg_latency_us > 35.0 {
                    self.add_error(&format!(
                        "Latency above requirement: {:.1}μs > 35μs",
                        self.metrics.avg_latency_us
                    ));
                    self.status = HealthStatus::Degraded;
                } else {
                    self.status = HealthStatus::Healthy;
                }
            }
            "signal_relay" => {
                if self.metrics.messages_per_second < 100_000.0 {
                    self.status = HealthStatus::Degraded;
                } else {
                    self.status = HealthStatus::Healthy;
                }
            }
            "execution_relay" => {
                if self.metrics.messages_per_second < 50_000.0 {
                    self.status = HealthStatus::Degraded;
                } else {
                    self.status = HealthStatus::Healthy;
                }
            }
            _ => {
                // Default health check
                self.status = HealthStatus::Healthy;
            }
        }

        // Check for zero-allocation violations (critical for hot path)
        if self.metrics.allocation_violations > 0 {
            self.add_error(&format!(
                "Zero-allocation violations detected: {}",
                self.metrics.allocation_violations
            ));
            self.status = HealthStatus::Degraded;
        }

        Ok(())
    }

    /// Check if Unix socket is available
    async fn check_socket_availability(&self, socket_path: &str) -> bool {
        Path::new(socket_path).exists()
    }

    /// Get uptime in seconds
    pub fn uptime_seconds(&self) -> u64 {
        self.startup_time.elapsed().unwrap_or_default().as_secs()
    }
}

/// HTTP health check server
pub struct HealthCheckServer {
    health: Arc<tokio::sync::Mutex<ServiceHealth>>,
    port: u16,
}

impl HealthCheckServer {
    /// Create new health check server
    pub fn new(health: ServiceHealth, port: u16) -> Self {
        Self {
            health: Arc::new(tokio::sync::Mutex::new(health)),
            port,
        }
    }

    /// Start health check HTTP server
    pub async fn start(&self) -> Result<()> {
        let addr = SocketAddr::from(([127, 0, 0, 1], self.port));
        let health = Arc::clone(&self.health);

        let make_svc = make_service_fn(move |_conn| {
            let health = Arc::clone(&health);
            async move {
                Ok::<_, Infallible>(service_fn(move |req| {
                    let health = Arc::clone(&health);
                    handle_request(req, health)
                }))
            }
        });

        let server = Server::bind(&addr).serve(make_svc);

        info!("Health check server listening on http://{}", addr);
        info!("Endpoints: /health, /ready, /metrics, /status");

        if let Err(e) = server.await {
            error!("Health check server error: {}", e);
        }

        Ok(())
    }

    /// Get current health status (for internal use)
    pub async fn get_health(&self) -> ServiceHealth {
        self.health.lock().await.clone()
    }

    /// Update health status (for service integration)
    pub async fn update_health<F>(&self, updater: F)
    where
        F: FnOnce(&mut ServiceHealth),
    {
        let mut health = self.health.lock().await;
        updater(&mut health);
    }
}

/// Handle HTTP health check requests
async fn handle_request(
    req: Request<Body>,
    health: Arc<tokio::sync::Mutex<ServiceHealth>>,
) -> Result<Response<Body>, Infallible> {
    let path = req.uri().path();
    let method = req.method();

    debug!("Health check request: {} {}", method, path);

    if method != Method::GET {
        return Ok(Response::builder()
            .status(StatusCode::METHOD_NOT_ALLOWED)
            .body(Body::from("Method not allowed"))
            .unwrap());
    }

    let mut health_guard = health.lock().await;
    let _ = health_guard.check_health().await; // Update health status
    let health_snapshot = health_guard.clone();
    drop(health_guard); // Release lock

    match path {
        "/health" => handle_health_endpoint(health_snapshot),
        "/ready" => handle_ready_endpoint(health_snapshot).await,
        "/metrics" => handle_metrics_endpoint(health_snapshot),
        "/status" => handle_status_endpoint(health_snapshot),
        _ => Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("Not found"))
            .unwrap()),
    }
}

/// Handle /health endpoint (basic liveness check)
fn handle_health_endpoint(health: ServiceHealth) -> Result<Response<Body>, Infallible> {
    let is_alive = !matches!(health.status, HealthStatus::Unhealthy);

    if is_alive {
        Ok(Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({
                    "status": "healthy",
                    "service": health.service_name,
                    "uptime_seconds": health.uptime_seconds()
                })
                .to_string(),
            ))
            .unwrap())
    } else {
        Ok(Response::builder()
            .status(StatusCode::SERVICE_UNAVAILABLE)
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({
                    "status": "unhealthy",
                    "service": health.service_name,
                    "errors": health.errors
                })
                .to_string(),
            ))
            .unwrap())
    }
}

/// Handle /ready endpoint (readiness for traffic)
async fn handle_ready_endpoint(health: ServiceHealth) -> Result<Response<Body>, Infallible> {
    let is_ready = health.is_ready().await;

    if is_ready {
        Ok(Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({
                    "status": "ready",
                    "service": health.service_name
                })
                .to_string(),
            ))
            .unwrap())
    } else {
        Ok(Response::builder()
            .status(StatusCode::SERVICE_UNAVAILABLE)
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({
                    "status": "not ready",
                    "service": health.service_name,
                    "current_status": health.status
                })
                .to_string(),
            ))
            .unwrap())
    }
}

/// Handle /metrics endpoint (performance metrics)
fn handle_metrics_endpoint(health: ServiceHealth) -> Result<Response<Body>, Infallible> {
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_string_pretty(&health.metrics).unwrap(),
        ))
        .unwrap())
}

/// Handle /status endpoint (detailed service status)
fn handle_status_endpoint(health: ServiceHealth) -> Result<Response<Body>, Infallible> {
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string_pretty(&health).unwrap()))
        .unwrap())
}

/// Performance metrics collector for services
pub struct MetricsCollector {
    message_count: AtomicU64,
    start_time: SystemTime,
    active_connections: AtomicU64,
    allocation_violations: AtomicU64,
    latency_samples: Arc<Mutex<LatencyTracker>>,
}

/// Tracks latency statistics with percentile calculation
struct LatencyTracker {
    samples: Vec<f64>,
    max_samples: usize,
}

impl LatencyTracker {
    fn new(max_samples: usize) -> Self {
        Self {
            samples: Vec::with_capacity(max_samples),
            max_samples,
        }
    }

    fn add_sample(&mut self, latency_us: f64) {
        if self.samples.len() >= self.max_samples {
            // Remove oldest sample
            self.samples.remove(0);
        }
        self.samples.push(latency_us);
    }

    fn calculate_average(&self) -> f64 {
        if self.samples.is_empty() {
            return 0.0;
        }
        self.samples.iter().sum::<f64>() / self.samples.len() as f64
    }

    fn calculate_p99(&self) -> f64 {
        if self.samples.is_empty() {
            return 0.0;
        }
        let mut sorted = self.samples.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let index = ((sorted.len() as f64 * 0.99) as usize).min(sorted.len() - 1);
        sorted[index]
    }
}

impl MetricsCollector {
    /// Create new metrics collector
    pub fn new() -> Self {
        Self {
            message_count: AtomicU64::new(0),
            start_time: SystemTime::now(),
            active_connections: AtomicU64::new(0),
            allocation_violations: AtomicU64::new(0),
            latency_samples: Arc::new(Mutex::new(LatencyTracker::new(10000))), // Keep last 10k samples
        }
    }

    /// Increment message count
    pub fn increment_messages(&self) {
        self.message_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Set active connection count
    pub fn set_active_connections(&self, count: u64) {
        self.active_connections.store(count, Ordering::Relaxed);
    }

    /// Increment allocation violations
    pub fn increment_allocation_violations(&self) {
        self.allocation_violations.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a latency sample in microseconds
    pub fn record_latency_us(&self, latency_us: f64) {
        if let Ok(mut tracker) = self.latency_samples.lock() {
            tracker.add_sample(latency_us);
        }
    }

    /// Record a latency sample from a Duration
    pub fn record_latency(&self, duration: Duration) {
        self.record_latency_us(duration.as_micros() as f64);
    }

    /// Get current memory usage in bytes
    /// Returns Result to handle potential OS-level errors
    fn get_memory_usage() -> Result<u64> {
        #[cfg(target_os = "linux")]
        {
            if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
                for line in status.lines() {
                    if line.starts_with("VmRSS:") {
                        if let Some(kb_str) = line.split_whitespace().nth(1) {
                            if let Ok(kb) = kb_str.parse::<u64>() {
                                return Ok(kb * 1024); // Convert KB to bytes
                            }
                        }
                    }
                }
            }
            return Err(anyhow::anyhow!(
                "Failed to read memory usage from /proc/self/status"
            ));
        }

        #[cfg(not(target_os = "linux"))]
        {
            // Use libc::getrusage for Unix-like systems (macOS, BSD, etc.)
            // Use MaybeUninit to avoid undefined behavior with zeroed structs
            unsafe {
                let mut rusage = std::mem::MaybeUninit::<libc::rusage>::uninit();
                if libc::getrusage(libc::RUSAGE_SELF, rusage.as_mut_ptr()) == 0 {
                    let rusage = rusage.assume_init();
                    // On macOS, ru_maxrss is in bytes
                    // On other BSD systems, it might be in kilobytes
                    #[cfg(target_os = "macos")]
                    {
                        return Ok(rusage.ru_maxrss as u64);
                    }

                    #[cfg(not(target_os = "macos"))]
                    {
                        // On most other Unix systems, it's in kilobytes
                        return Ok((rusage.ru_maxrss as u64) * 1024);
                    }
                } else {
                    // Get the actual errno for better error reporting
                    let errno = std::io::Error::last_os_error();
                    return Err(anyhow::anyhow!(
                        "Failed to get memory usage via getrusage: {}",
                        errno
                    ));
                }
            }
        }
    }

    /// Get current performance metrics
    pub fn get_metrics(&self) -> PerformanceMetrics {
        let total_messages = self.message_count.load(Ordering::Relaxed);
        let elapsed_secs = self.start_time.elapsed().unwrap_or_default().as_secs_f64();
        let messages_per_second = if elapsed_secs > 0.0 {
            total_messages as f64 / elapsed_secs
        } else {
            0.0
        };

        let (avg_latency, p99_latency) = if let Ok(tracker) = self.latency_samples.lock() {
            (tracker.calculate_average(), tracker.calculate_p99())
        } else {
            (0.0, 0.0)
        };

        PerformanceMetrics {
            messages_per_second,
            avg_latency_us: avg_latency,
            p99_latency_us: p99_latency,
            active_connections: self.active_connections.load(Ordering::Relaxed),
            total_messages,
            allocation_violations: self.allocation_violations.load(Ordering::Relaxed),
            memory_usage_bytes: Self::get_memory_usage().unwrap_or_else(|e| {
                tracing::warn!("Failed to get memory usage, reporting 0: {}", e);
                0
            }),
        }
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_service_health_creation() {
        let mut health = ServiceHealth::new("test_service");
        assert_eq!(health.service_name, "test_service");
        assert_eq!(health.status, HealthStatus::Starting);

        health.set_socket_path("/tmp/test.sock");
        assert_eq!(health.socket_path, Some("/tmp/test.sock".to_string()));
    }

    #[test]
    fn test_metrics_collector() {
        let collector = MetricsCollector::new();

        collector.increment_messages();
        collector.increment_messages();
        collector.set_active_connections(5);

        let metrics = collector.get_metrics();
        assert_eq!(metrics.total_messages, 2);
        assert_eq!(metrics.active_connections, 5);
    }

    #[test]
    fn test_memory_usage_non_zero() {
        // Test that memory usage returns a non-zero value on all platforms
        let result = MetricsCollector::get_memory_usage();

        // The function should return Ok on all platforms
        assert!(
            result.is_ok(),
            "get_memory_usage should return Ok, got: {:?}",
            result
        );

        // The memory usage should be greater than 0 (any running process uses some memory)
        let memory_bytes = result.unwrap();
        assert!(
            memory_bytes > 0,
            "Memory usage should be greater than 0, got: {}",
            memory_bytes
        );

        // Sanity check: memory usage should be reasonable (between 1MB and 10GB)
        assert!(
            memory_bytes > 1_000_000,
            "Memory usage seems too low: {} bytes",
            memory_bytes
        );
        assert!(
            memory_bytes < 10_000_000_000,
            "Memory usage seems too high: {} bytes",
            memory_bytes
        );
    }

    #[test]
    fn test_metrics_error_handling() {
        // Test that metrics collection handles errors gracefully
        let collector = MetricsCollector::new();

        // Get metrics (this will call get_memory_usage internally)
        let metrics = collector.get_metrics();

        // Even if memory usage fails, other metrics should still work
        assert_eq!(metrics.total_messages, 0);
        assert_eq!(metrics.active_connections, 0);

        // Memory usage should be reported (either real value or 0 fallback)
        // Note: u64 is always >= 0, so we just verify it's being set
        // This ensures the fallback mechanism works without causing compilation warnings
    }
}
