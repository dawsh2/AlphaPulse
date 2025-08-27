//! # Common Adapter Infrastructure
//!
//! Shared trait definitions and utilities for all AlphaPulse adapter implementations.
//! Provides a unified interface for data collection, transformation, and output routing.

use crate::{AdapterError, CircuitState, Result};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

// ============================================================================
// CORE TRAITS
// ============================================================================

/// Core trait that all AlphaPulse adapters must implement
///
/// This trait defines the standard lifecycle and behavior for data collection
/// adapters, ensuring consistent interfaces across all exchange integrations.
#[async_trait]
pub trait Adapter: Send + Sync {
    /// Adapter configuration type
    type Config: Send + Sync + Clone;

    /// Start the adapter data collection process with safety mechanisms
    ///
    /// This method should:
    /// 1. Initialize circuit breaker in CLOSED state
    /// 2. Establish connections with configured timeout limits
    /// 3. Begin continuous data collection with rate limiting
    /// 4. Handle automatic reconnection on failures
    /// 5. Transform raw data into Protocol V2 TLV messages
    async fn start(&mut self) -> Result<()>;

    /// Stop the adapter gracefully
    ///
    /// Clean shutdown process:
    /// 1. Stop accepting new data
    /// 2. Flush pending messages
    /// 3. Close connections cleanly
    /// 4. Report final metrics
    async fn stop(&mut self) -> Result<()>;

    /// Initialize adapter resources
    ///
    /// One-time setup operations:
    /// - Load configuration from environment
    /// - Initialize rate limiters with configured thresholds
    /// - Setup metrics collectors
    /// - Prepare message buffers
    /// - Configure circuit breaker thresholds
    async fn initialize(&mut self) -> Result<()>;

    /// Get adapter's current health status
    ///
    /// Health checks include:
    /// - Connection status (CONNECTED/DISCONNECTED/RECONNECTING)
    /// - Circuit breaker state (OPEN/CLOSED/HALF_OPEN)
    /// - Message rate and latency metrics
    /// - Error counts and types
    /// - Buffer overflow indicators
    async fn health(&self) -> AdapterHealth;

    /// Process incoming data from the source
    ///
    /// Hot path requirements (must complete in <35μs):
    /// 1. Parse raw data with zero-copy techniques
    /// 2. Transform to TLV message format
    /// 3. Apply rate limiting if configured
    /// 4. Write to output channel without blocking
    async fn process_data(&self, data: &[u8]) -> Result<()>;
}

/// Safety wrapper trait for adapters
///
/// Provides additional safety guarantees including:
/// - Circuit breaker pattern for fault tolerance
/// - Rate limiting to prevent overwhelming downstream
/// - Connection timeout enforcement
/// - Automatic retries with exponential backoff
#[async_trait]
pub trait SafeAdapter: Adapter {
    /// Execute operation with circuit breaker protection
    ///
    /// Wraps operations to prevent cascade failures:
    /// - Opens circuit after configured failure threshold
    /// - Provides half-open state for recovery testing
    /// - Tracks success/failure metrics
    async fn with_circuit_breaker<F, T>(&self, operation: F) -> Result<T>
    where
        F: std::future::Future<Output = Result<T>> + Send,
        T: Send;

    /// Apply rate limiting to operations
    ///
    /// Enforces configured rate limits:
    /// - Token bucket algorithm for smooth rate limiting
    /// - Configurable requests per second
    /// - Automatic backpressure when limit exceeded
    async fn rate_limit(&self) -> Result<()>;

    /// Check if circuit breaker allows operation
    ///
    /// Circuit states:
    /// - CLOSED: Normal operation, requests flow through
    /// - OPEN: Failures exceeded threshold, requests blocked
    /// - HALF_OPEN: Testing recovery with limited requests
    fn circuit_state(&self) -> CircuitState;
}

// ============================================================================
// AUTHENTICATION
// ============================================================================

/// Authentication provider for adapters
pub trait AuthProvider: Send + Sync {
    /// Get authentication headers for HTTP requests
    fn get_headers(&self) -> HashMap<String, String>;
    
    /// Get authentication parameters for WebSocket connections
    fn get_ws_auth(&self) -> Option<String>;
    
    /// Refresh authentication tokens if needed
    async fn refresh(&mut self) -> Result<()>;
}

/// API key authentication
#[derive(Debug, Clone)]
pub struct ApiKeyAuth {
    pub api_key: String,
    pub api_secret: Option<String>,
}

impl ApiKeyAuth {
    pub fn new(api_key: String, api_secret: Option<String>) -> Self {
        Self { api_key, api_secret }
    }
}

impl AuthProvider for ApiKeyAuth {
    fn get_headers(&self) -> HashMap<String, String> {
        let mut headers = HashMap::new();
        headers.insert("API-KEY".to_string(), self.api_key.clone());
        if let Some(secret) = &self.api_secret {
            headers.insert("API-SECRET".to_string(), secret.clone());
        }
        headers
    }
    
    fn get_ws_auth(&self) -> Option<String> {
        Some(format!("{{\"api_key\":\"{}\"}}", self.api_key))
    }
    
    async fn refresh(&mut self) -> Result<()> {
        // API keys don't need refresh
        Ok(())
    }
}

/// OAuth2 authentication
#[derive(Debug, Clone)]
pub struct OAuth2Auth {
    pub client_id: String,
    pub client_secret: String,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
}

impl OAuth2Auth {
    pub fn new(client_id: String, client_secret: String) -> Self {
        Self {
            client_id,
            client_secret,
            access_token: None,
            refresh_token: None,
        }
    }
    
    async fn request_token(&mut self) -> Result<String> {
        // OAuth2 token request not yet implemented
        // Return error instead of panicking
        Err(AdapterError::NotImplemented(
            "OAuth2 token request not yet implemented. Use API key authentication instead.".to_string()
        ))
    }
}

impl AuthProvider for OAuth2Auth {
    fn get_headers(&self) -> HashMap<String, String> {
        let mut headers = HashMap::new();
        if let Some(token) = &self.access_token {
            headers.insert("Authorization".to_string(), format!("Bearer {}", token));
        }
        headers
    }
    
    fn get_ws_auth(&self) -> Option<String> {
        self.access_token.as_ref().map(|token| {
            format!("{{\"access_token\":\"{}\"}}", token)
        })
    }
    
    async fn refresh(&mut self) -> Result<()> {
        if let Some(refresh_token) = &self.refresh_token {
            // Use refresh token to get new access token
            let new_token = self.request_token().await?;
            self.access_token = Some(new_token);
        }
        Ok(())
    }
}

// ============================================================================
// METRICS
// ============================================================================

/// Performance metrics for adapters
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AdapterMetrics {
    /// Total messages received
    pub messages_received: u64,
    
    /// Total messages sent
    pub messages_sent: u64,
    
    /// Total bytes received
    pub bytes_received: u64,
    
    /// Total bytes sent
    pub bytes_sent: u64,
    
    /// Connection attempts
    pub connection_attempts: u64,
    
    /// Successful connections
    pub successful_connections: u64,
    
    /// Failed connections
    pub failed_connections: u64,
    
    /// Current message rate (messages/sec)
    pub message_rate: f64,
    
    /// Average processing latency (microseconds)
    pub avg_latency_us: f64,
    
    /// P99 latency (microseconds)
    pub p99_latency_us: f64,
    
    /// Circuit breaker trips
    pub circuit_breaker_trips: u64,
    
    /// Rate limit hits
    pub rate_limit_hits: u64,
    
    /// Error count
    pub error_count: u64,
    
    /// Last error message
    pub last_error: Option<String>,
    
    /// Uptime in seconds
    pub uptime_seconds: u64,
}

impl AdapterMetrics {
    /// Create new metrics instance
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Record a received message
    pub fn record_message_received(&mut self, size_bytes: usize) {
        self.messages_received += 1;
        self.bytes_received += size_bytes as u64;
    }
    
    /// Record a sent message
    pub fn record_message_sent(&mut self, size_bytes: usize) {
        self.messages_sent += 1;
        self.bytes_sent += size_bytes as u64;
    }
    
    /// Record connection attempt
    pub fn record_connection_attempt(&mut self, success: bool) {
        self.connection_attempts += 1;
        if success {
            self.successful_connections += 1;
        } else {
            self.failed_connections += 1;
        }
    }
    
    /// Record an error
    pub fn record_error(&mut self, error_msg: String) {
        self.error_count += 1;
        self.last_error = Some(error_msg);
    }
    
    /// Record circuit breaker trip
    pub fn record_circuit_breaker_trip(&mut self) {
        self.circuit_breaker_trips += 1;
    }
    
    /// Record rate limit hit
    pub fn record_rate_limit_hit(&mut self) {
        self.rate_limit_hits += 1;
    }
}

/// Metrics collector for tracking adapter performance
pub struct MetricsCollector {
    metrics: Arc<RwLock<AdapterMetrics>>,
    start_time: std::time::Instant,
}

impl MetricsCollector {
    /// Create new metrics collector
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(AdapterMetrics::new())),
            start_time: std::time::Instant::now(),
        }
    }
    
    /// Get current metrics snapshot
    pub async fn get_metrics(&self) -> AdapterMetrics {
        let mut metrics = self.metrics.read().await.clone();
        metrics.uptime_seconds = self.start_time.elapsed().as_secs();
        metrics
    }
    
    /// Record a message
    pub async fn record_message(&self, size_bytes: usize, sent: bool) {
        let mut metrics = self.metrics.write().await;
        if sent {
            metrics.record_message_sent(size_bytes);
        } else {
            metrics.record_message_received(size_bytes);
        }
    }
    
    /// Record connection event
    pub async fn record_connection(&self, success: bool) {
        let mut metrics = self.metrics.write().await;
        metrics.record_connection_attempt(success);
    }
    
    /// Record an error
    pub async fn record_error(&self, error: String) {
        let mut metrics = self.metrics.write().await;
        metrics.record_error(error);
    }
}

// ============================================================================
// HEALTH STATUS
// ============================================================================

/// Adapter health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterHealth {
    /// Overall health status
    pub status: HealthStatus,
    
    /// Connection status
    pub connection: ConnectionStatus,
    
    /// Circuit breaker state
    pub circuit_state: CircuitState,
    
    /// Current metrics
    pub metrics: AdapterMetrics,
    
    /// Additional status details
    pub details: HashMap<String, String>,
}

/// Overall health status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HealthStatus {
    /// Adapter is healthy and operating normally
    Healthy,
    
    /// Adapter is degraded but still operational
    Degraded,
    
    /// Adapter is unhealthy and may not be operational
    Unhealthy,
}

/// Connection status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConnectionStatus {
    /// Connected and receiving data
    Connected,
    
    /// Not connected
    Disconnected,
    
    /// Attempting to reconnect
    Reconnecting,
}

// Re-export commonly used types
pub use crate::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
pub use crate::rate_limit::RateLimiter;
pub use crate::config::BaseAdapterConfig;