//! Circuit Breaker Pattern Implementation for MessageSink
//!
//! Implements the circuit breaker pattern to prevent cascading failures when
//! a MessageSink is experiencing issues. This provides automatic failure detection,
//! recovery attempts, and fail-fast behavior during outages.
//!
//! ## Circuit Breaker States
//!
//! ```text
//! CLOSED ──failure_threshold──> OPEN ──timeout──> HALF_OPEN
//!   │                            │                   │
//!   └──────────────── success ───┴─── failure ──────┘
//! ```
//!
//! - **CLOSED**: Normal operation, all requests pass through
//! - **OPEN**: Sink is failing, requests immediately fail-fast
//! - **HALF_OPEN**: Testing recovery, limited requests pass through

use crate::{BatchResult, Message, MessageSink, SinkError, SendContext};
use async_trait::async_trait;
use std::fmt::Debug;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::RwLock;

/// Circuit breaker states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Normal operation - requests pass through to underlying sink
    Closed,
    /// Sink is failing - requests fail immediately without hitting sink
    Open,
    /// Testing recovery - limited requests pass through
    HalfOpen,
}

impl CircuitState {
    pub fn is_closed(&self) -> bool {
        matches!(self, CircuitState::Closed)
    }

    pub fn is_open(&self) -> bool {
        matches!(self, CircuitState::Open)
    }

    pub fn is_half_open(&self) -> bool {
        matches!(self, CircuitState::HalfOpen)
    }
}

/// Configuration for circuit breaker behavior
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of consecutive failures before opening circuit
    pub failure_threshold: usize,
    /// How long to wait before attempting recovery (OPEN -> HALF_OPEN)
    pub recovery_timeout: Duration,
    /// Number of successful calls needed to close circuit from half-open
    pub success_threshold: usize,
    /// Maximum number of calls allowed in half-open state
    pub half_open_max_calls: usize,
    /// Time window for measuring failure rate
    pub measurement_window: Duration,
    /// Minimum calls needed before failure rate calculation
    pub minimum_calls: usize,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            recovery_timeout: Duration::from_secs(30),
            success_threshold: 2,
            half_open_max_calls: 5,
            measurement_window: Duration::from_secs(60),
            minimum_calls: 10,
        }
    }
}

impl CircuitBreakerConfig {
    /// Create a fast-recovery configuration for low-latency systems
    pub fn fast_recovery() -> Self {
        Self {
            failure_threshold: 3,
            recovery_timeout: Duration::from_secs(5),
            success_threshold: 2,
            half_open_max_calls: 3,
            measurement_window: Duration::from_secs(30),
            minimum_calls: 5,
        }
    }

    /// Create a conservative configuration for critical systems
    pub fn conservative() -> Self {
        Self {
            failure_threshold: 10,
            recovery_timeout: Duration::from_secs(60),
            success_threshold: 5,
            half_open_max_calls: 10,
            measurement_window: Duration::from_secs(120),
            minimum_calls: 20,
        }
    }
}

/// Statistics for circuit breaker monitoring
#[derive(Debug, Clone)]
pub struct CircuitBreakerStats {
    pub current_state: CircuitState,
    pub consecutive_failures: usize,
    pub consecutive_successes: usize,
    pub total_calls: u64,
    pub total_failures: u64,
    pub total_successes: u64,
    pub calls_rejected: u64,
    pub last_failure_time: Option<SystemTime>,
    pub last_success_time: Option<SystemTime>,
    pub state_changed_at: Instant,
}

impl CircuitBreakerStats {
    pub fn failure_rate(&self) -> f64 {
        if self.total_calls == 0 {
            0.0
        } else {
            self.total_failures as f64 / self.total_calls as f64
        }
    }

    pub fn success_rate(&self) -> f64 {
        1.0 - self.failure_rate()
    }

    pub fn time_in_current_state(&self) -> Duration {
        self.state_changed_at.elapsed()
    }
}

/// Internal state for the circuit breaker
#[derive(Debug)]
struct CircuitBreakerState {
    state: CircuitState,
    consecutive_failures: usize,
    consecutive_successes: usize,
    half_open_calls: usize,
    last_failure_time: Option<Instant>,
    state_changed_at: Instant,
}

impl CircuitBreakerState {
    fn new() -> Self {
        Self {
            state: CircuitState::Closed,
            consecutive_failures: 0,
            consecutive_successes: 0,
            half_open_calls: 0,
            last_failure_time: None,
            state_changed_at: Instant::now(),
        }
    }

    fn transition_to(&mut self, new_state: CircuitState) {
        if self.state != new_state {
            tracing::info!("Circuit breaker state transition: {:?} -> {:?}", self.state, new_state);
            self.state = new_state;
            self.state_changed_at = Instant::now();
            
            match new_state {
                CircuitState::Closed => {
                    self.consecutive_failures = 0;
                    self.half_open_calls = 0;
                }
                CircuitState::Open => {
                    self.consecutive_successes = 0;
                    self.half_open_calls = 0;
                }
                CircuitState::HalfOpen => {
                    self.half_open_calls = 0;
                }
            }
        }
    }
}

/// Circuit breaker wrapper that protects a MessageSink from cascading failures
#[derive(Debug)]
pub struct CircuitBreakerSink<T: MessageSink> {
    inner: Arc<T>,
    config: CircuitBreakerConfig,
    state: Arc<RwLock<CircuitBreakerState>>,
    
    // Atomic counters for statistics
    total_calls: AtomicU64,
    total_failures: AtomicU64,
    total_successes: AtomicU64,
    calls_rejected: AtomicU64,
    
    // Recent call tracking
    recent_calls: Arc<RwLock<Vec<(Instant, bool)>>>, // (timestamp, success)
}

impl<T: MessageSink> CircuitBreakerSink<T> {
    /// Wrap a MessageSink with circuit breaker protection
    pub fn new(inner: T, config: CircuitBreakerConfig) -> Self {
        Self {
            inner: Arc::new(inner),
            config,
            state: Arc::new(RwLock::new(CircuitBreakerState::new())),
            total_calls: AtomicU64::new(0),
            total_failures: AtomicU64::new(0),
            total_successes: AtomicU64::new(0),
            calls_rejected: AtomicU64::new(0),
            recent_calls: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Create with default configuration
    pub fn with_default_config(inner: T) -> Self {
        Self::new(inner, CircuitBreakerConfig::default())
    }

    /// Create with fast recovery configuration
    pub fn with_fast_recovery(inner: T) -> Self {
        Self::new(inner, CircuitBreakerConfig::fast_recovery())
    }

    /// Create with conservative configuration
    pub fn with_conservative(inner: T) -> Self {
        Self::new(inner, CircuitBreakerConfig::conservative())
    }

    /// Get current circuit breaker statistics
    pub async fn stats(&self) -> CircuitBreakerStats {
        let state_guard = self.state.read().await;
        
        CircuitBreakerStats {
            current_state: state_guard.state,
            consecutive_failures: state_guard.consecutive_failures,
            consecutive_successes: state_guard.consecutive_successes,
            total_calls: self.total_calls.load(Ordering::Relaxed),
            total_failures: self.total_failures.load(Ordering::Relaxed),
            total_successes: self.total_successes.load(Ordering::Relaxed),
            calls_rejected: self.calls_rejected.load(Ordering::Relaxed),
            last_failure_time: state_guard.last_failure_time.map(|i| {
                SystemTime::now() - i.elapsed()
            }),
            last_success_time: None, // Could add this tracking if needed
            state_changed_at: state_guard.state_changed_at,
        }
    }

    /// Force circuit breaker to specific state (for testing)
    pub async fn force_state(&self, state: CircuitState) {
        let mut state_guard = self.state.write().await;
        state_guard.transition_to(state);
    }

    /// Reset circuit breaker to initial state
    pub async fn reset(&self) {
        let mut state_guard = self.state.write().await;
        state_guard.transition_to(CircuitState::Closed);
        state_guard.consecutive_failures = 0;
        state_guard.consecutive_successes = 0;
        state_guard.half_open_calls = 0;
        state_guard.last_failure_time = None;
        
        // Reset atomic counters
        self.total_calls.store(0, Ordering::Relaxed);
        self.total_failures.store(0, Ordering::Relaxed);
        self.total_successes.store(0, Ordering::Relaxed);
        self.calls_rejected.store(0, Ordering::Relaxed);
        
        // Clear recent calls
        let mut recent_calls = self.recent_calls.write().await;
        recent_calls.clear();
    }

    /// Check if a call should be allowed through
    async fn should_allow_call(&self) -> bool {
        // Use write lock to ensure atomic state transitions
        let mut state_guard = self.state.write().await;
        
        match state_guard.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check if enough time has passed to attempt recovery
                if let Some(last_failure) = state_guard.last_failure_time {
                    if last_failure.elapsed() >= self.config.recovery_timeout {
                        // Transition to half-open atomically
                        state_guard.transition_to(CircuitState::HalfOpen);
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => {
                // Allow limited calls in half-open state
                state_guard.half_open_calls < self.config.half_open_max_calls
            }
        }
    }

    /// Record call result and update circuit state
    async fn record_result(&self, success: bool) {
        // Update statistics
        self.total_calls.fetch_add(1, Ordering::Relaxed);
        if success {
            self.total_successes.fetch_add(1, Ordering::Relaxed);
        } else {
            self.total_failures.fetch_add(1, Ordering::Relaxed);
        }

        // Add to recent calls for rate calculation
        {
            let mut recent_calls = self.recent_calls.write().await;
            let now = Instant::now();
            recent_calls.push((now, success));
            
            // Clean old entries outside measurement window
            let cutoff = now - self.config.measurement_window;
            recent_calls.retain(|(timestamp, _)| *timestamp > cutoff);
        }

        // Update circuit state
        let mut state_guard = self.state.write().await;
        
        if success {
            state_guard.consecutive_failures = 0;
            state_guard.consecutive_successes += 1;
            
            match state_guard.state {
                CircuitState::HalfOpen => {
                    // Check if we've had enough successes to close the circuit
                    if state_guard.consecutive_successes >= self.config.success_threshold {
                        state_guard.transition_to(CircuitState::Closed);
                    }
                }
                _ => {} // Stay in current state
            }
        } else {
            state_guard.consecutive_successes = 0;
            state_guard.consecutive_failures += 1;
            state_guard.last_failure_time = Some(Instant::now());
            
            match state_guard.state {
                CircuitState::Closed | CircuitState::HalfOpen => {
                    // Check if we should open the circuit
                    if state_guard.consecutive_failures >= self.config.failure_threshold {
                        state_guard.transition_to(CircuitState::Open);
                    }
                }
                CircuitState::Open => {
                    // Stay open, reset recovery timer
                    state_guard.last_failure_time = Some(Instant::now());
                }
            }
        }

        // Update half-open call counter
        if matches!(state_guard.state, CircuitState::HalfOpen) {
            state_guard.half_open_calls += 1;
        }

        // Check for state transition from Open to HalfOpen (already under write lock)
        if matches!(state_guard.state, CircuitState::Open) {
            if let Some(last_failure) = state_guard.last_failure_time {
                if last_failure.elapsed() >= self.config.recovery_timeout {
                    // Transition is atomic since we hold the write lock
                    state_guard.transition_to(CircuitState::HalfOpen);
                }
            }
        }
    }

    /// Create a circuit breaker error
    fn create_circuit_breaker_error(&self, message: Message) -> SinkError {
        let context = SendContext::new(
            message.size(), 
            alphapulse_network::safe_system_timestamp_ns()
        )
        .with_correlation_id(
            message.metadata.correlation_id.unwrap_or_else(|| "unknown".to_string())
        )
        .with_target(
            message.metadata.target.unwrap_or_else(|| "circuit-breaker".to_string())
        );
        
        SinkError::send_failed_with_context("Circuit breaker is OPEN", context)
    }
}

#[async_trait]
impl<T: MessageSink> MessageSink for CircuitBreakerSink<T> {
    async fn send(&self, message: Message) -> Result<(), SinkError> {
        // Check if call should be allowed
        if !self.should_allow_call().await {
            self.calls_rejected.fetch_add(1, Ordering::Relaxed);
            return Err(self.create_circuit_breaker_error(message));
        }

        // Attempt the call
        let result = self.inner.send(message).await;
        
        // Record the result
        self.record_result(result.is_ok()).await;
        
        result
    }

    async fn send_batch(&self, messages: Vec<Message>) -> Result<BatchResult, SinkError> {
        // For batch operations, we allow the call if circuit is not fully open
        // Individual message failures will be tracked normally
        let state = {
            let state_guard = self.state.read().await;
            state_guard.state
        };

        match state {
            CircuitState::Open => {
                // In open state, reject the entire batch
                let batch_size = messages.len();
                self.calls_rejected.fetch_add(batch_size as u64, Ordering::Relaxed);
                
                let mut result = BatchResult::new(batch_size);
                for (index, message) in messages.into_iter().enumerate() {
                    let error = self.create_circuit_breaker_error(message);
                    result.record_failure(index, error);
                }
                return Ok(result);
            }
            _ => {
                // Allow batch through, but track results
                let result = self.inner.send_batch(messages).await?;
                
                // Record batch results
                self.record_result(result.is_complete_success()).await;
                
                Ok(result)
            }
        }
    }

    async fn send_batch_prioritized(&self, messages: Vec<Message>) -> Result<BatchResult, SinkError> {
        // Similar logic to send_batch but with priority ordering
        let state = {
            let state_guard = self.state.read().await;
            state_guard.state
        };

        match state {
            CircuitState::Open => {
                let batch_size = messages.len();
                self.calls_rejected.fetch_add(batch_size as u64, Ordering::Relaxed);
                
                let mut result = BatchResult::new(batch_size);
                for (index, message) in messages.into_iter().enumerate() {
                    let error = self.create_circuit_breaker_error(message);
                    result.record_failure(index, error);
                }
                return Ok(result);
            }
            _ => {
                let result = self.inner.send_batch_prioritized(messages).await?;
                self.record_result(result.is_complete_success()).await;
                Ok(result)
            }
        }
    }

    fn is_connected(&self) -> bool {
        // Circuit breaker doesn't affect connection status
        self.inner.is_connected()
    }

    async fn connect(&self) -> Result<(), SinkError> {
        // Connection attempts are always allowed (not subject to circuit breaking)
        let result = self.inner.connect().await;
        
        // Record connection result for circuit breaker state
        self.record_result(result.is_ok()).await;
        
        result
    }

    async fn disconnect(&self) -> Result<(), SinkError> {
        // Disconnection is always allowed
        self.inner.disconnect().await
    }

    fn metadata(&self) -> crate::SinkMetadata {
        let mut metadata = self.inner.metadata();
        
        // Add circuit breaker information to metadata name
        let stats = tokio::task::block_in_place(|| {
            let stats_future = self.stats();
            tokio::runtime::Handle::current().block_on(stats_future)
        });
        metadata.name = format!("{} (CB: {:?})", metadata.name, stats.current_state);
        
        metadata
    }

    fn extended_metadata(&self) -> crate::ExtendedSinkMetadata {
        self.inner.extended_metadata()
    }

    fn connection_health(&self) -> crate::ConnectionHealth {
        // Circuit breaker state affects health reporting
        let stats = tokio::task::block_in_place(|| {
            let stats_future = self.stats();
            tokio::runtime::Handle::current().block_on(stats_future)
        });
        
        match stats.current_state {
            CircuitState::Open => crate::ConnectionHealth::Unhealthy,
            CircuitState::HalfOpen => crate::ConnectionHealth::Degraded,
            CircuitState::Closed => self.inner.connection_health(),
        }
    }

    fn last_successful_send(&self) -> Option<SystemTime> {
        self.inner.last_successful_send()
    }

    fn preferred_connection_count(&self) -> usize {
        self.inner.preferred_connection_count()
    }

    fn supports_multiplexing(&self) -> bool {
        self.inner.supports_multiplexing()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{CollectorSink, FailingSink};
    use crate::{Message, MessageMetadata};
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_circuit_breaker_normal_operation() {
        let sink = CollectorSink::new();
        sink.connect().await.unwrap();
        
        let cb_sink = CircuitBreakerSink::with_default_config(sink);
        
        // Normal operation should pass through
        let message = Message::new_unchecked(b"test".to_vec());
        cb_sink.send(message).await.unwrap();
        
        let stats = cb_sink.stats().await;
        assert_eq!(stats.current_state, CircuitState::Closed);
        assert_eq!(stats.total_calls, 1);
        assert_eq!(stats.total_successes, 1);
    }

    #[tokio::test]
    async fn test_circuit_breaker_opens_on_failures() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            ..Default::default()
        };
        
        let failing_sink = FailingSink::new("Test failure");
        let cb_sink = CircuitBreakerSink::new(failing_sink, config);
        
        // First failure
        let message1 = Message::new_unchecked(b"test1".to_vec());
        let result1 = cb_sink.send(message1).await;
        assert!(result1.is_err());
        
        let stats = cb_sink.stats().await;
        assert_eq!(stats.current_state, CircuitState::Closed);
        assert_eq!(stats.consecutive_failures, 1);
        
        // Second failure should open circuit
        let message2 = Message::new_unchecked(b"test2".to_vec());
        let result2 = cb_sink.send(message2).await;
        assert!(result2.is_err());
        
        let stats = cb_sink.stats().await;
        assert_eq!(stats.current_state, CircuitState::Open);
        assert_eq!(stats.consecutive_failures, 2);
        
        // Third call should be rejected by circuit breaker
        let message3 = Message::new_unchecked(b"test3".to_vec());
        let result3 = cb_sink.send(message3).await;
        assert!(result3.is_err());
        assert!(result3.unwrap_err().to_string().contains("Circuit breaker is OPEN"));
        
        let stats = cb_sink.stats().await;
        assert_eq!(stats.calls_rejected, 1);
    }

    #[tokio::test]
    async fn test_circuit_breaker_recovery() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            recovery_timeout: Duration::from_millis(10),
            success_threshold: 1,
            ..Default::default()
        };
        
        let collector = CollectorSink::new();
        let cb_sink = CircuitBreakerSink::new(collector, config);
        
        // Force into open state
        cb_sink.force_state(CircuitState::Open).await;
        
        let stats = cb_sink.stats().await;
        assert_eq!(stats.current_state, CircuitState::Open);
        
        // Wait for recovery timeout
        sleep(Duration::from_millis(15)).await;
        
        // Connect the underlying sink
        cb_sink.inner.connect().await.unwrap();
        
        // Next call should be allowed (transitions to half-open)
        let message = Message::new_unchecked(b"recovery_test".to_vec());
        let result = cb_sink.send(message).await;
        assert!(result.is_ok());
        
        let stats = cb_sink.stats().await;
        assert_eq!(stats.current_state, CircuitState::Closed); // Should close after 1 success
    }

    #[tokio::test]
    async fn test_circuit_breaker_half_open_behavior() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            recovery_timeout: Duration::from_millis(10),
            success_threshold: 2,
            half_open_max_calls: 3,
            ..Default::default()
        };
        
        let collector = CollectorSink::new();
        collector.connect().await.unwrap();
        let cb_sink = CircuitBreakerSink::new(collector, config);
        
        // Force to half-open state
        cb_sink.force_state(CircuitState::HalfOpen).await;
        
        // First success
        let message1 = Message::new_unchecked(b"test1".to_vec());
        cb_sink.send(message1).await.unwrap();
        
        let stats = cb_sink.stats().await;
        assert_eq!(stats.current_state, CircuitState::HalfOpen); // Still half-open
        
        // Second success should close circuit
        let message2 = Message::new_unchecked(b"test2".to_vec());
        cb_sink.send(message2).await.unwrap();
        
        let stats = cb_sink.stats().await;
        assert_eq!(stats.current_state, CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_circuit_breaker_batch_operations() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            ..Default::default()
        };
        
        let failing_sink = FailingSink::new("Batch failure");
        let cb_sink = CircuitBreakerSink::new(failing_sink, config);
        
        let messages = vec![
            Message::new_unchecked(b"msg1".to_vec()),
            Message::new_unchecked(b"msg2".to_vec()),
        ];
        
        // First batch should fail and open circuit
        let result = cb_sink.send_batch(messages.clone()).await.unwrap();
        assert!(!result.is_complete_success());
        
        let stats = cb_sink.stats().await;
        assert_eq!(stats.current_state, CircuitState::Open);
        
        // Second batch should be rejected by circuit breaker
        let result2 = cb_sink.send_batch(messages).await.unwrap();
        assert!(!result2.is_complete_success());
        assert_eq!(result2.succeeded, 0);
        
        let stats = cb_sink.stats().await;
        assert!(stats.calls_rejected >= 2);
    }

    #[tokio::test]
    async fn test_circuit_breaker_configurations() {
        let fast_config = CircuitBreakerConfig::fast_recovery();
        let conservative_config = CircuitBreakerConfig::conservative();
        
        assert!(fast_config.failure_threshold < conservative_config.failure_threshold);
        assert!(fast_config.recovery_timeout < conservative_config.recovery_timeout);
        
        let sink = CollectorSink::new();
        let _fast_cb = CircuitBreakerSink::with_fast_recovery(sink);
        
        let sink2 = CollectorSink::new();
        let _conservative_cb = CircuitBreakerSink::with_conservative(sink2);
    }

    #[tokio::test]
    async fn test_circuit_breaker_reset() {
        let failing_sink = FailingSink::new("Test");
        let cb_sink = CircuitBreakerSink::with_default_config(failing_sink);
        
        // Generate some failures
        for _ in 0..3 {
            let message = Message::new_unchecked(b"test".to_vec());
            let _ = cb_sink.send(message).await;
        }
        
        let stats = cb_sink.stats().await;
        assert!(stats.total_failures > 0);
        
        // Reset should clear all stats
        cb_sink.reset().await;
        
        let stats = cb_sink.stats().await;
        assert_eq!(stats.current_state, CircuitState::Closed);
        assert_eq!(stats.total_calls, 0);
        assert_eq!(stats.total_failures, 0);
        assert_eq!(stats.consecutive_failures, 0);
    }

    #[tokio::test]
    async fn test_circuit_breaker_health_reporting() {
        let collector = CollectorSink::new();
        let cb_sink = CircuitBreakerSink::with_default_config(collector);
        
        // Closed state should report underlying health
        assert_eq!(cb_sink.connection_health(), crate::ConnectionHealth::Unknown);
        
        // Force open state
        cb_sink.force_state(CircuitState::Open).await;
        assert_eq!(cb_sink.connection_health(), crate::ConnectionHealth::Unhealthy);
        
        // Force half-open state
        cb_sink.force_state(CircuitState::HalfOpen).await;
        assert_eq!(cb_sink.connection_health(), crate::ConnectionHealth::Degraded);
    }
}