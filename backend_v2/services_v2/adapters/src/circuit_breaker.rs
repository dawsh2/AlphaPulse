//! Circuit breaker pattern for fault tolerance

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Circuit breaker states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Normal operation - requests pass through
    Closed,
    /// Failing - requests are rejected
    Open,
    /// Testing recovery - limited requests allowed
    HalfOpen,
}

/// Configuration for circuit breaker behavior
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of failures before opening circuit
    pub failure_threshold: u32,
    /// Time to wait before attempting recovery
    pub recovery_timeout: Duration,
    /// Number of successes needed to close circuit from half-open
    pub success_threshold: u32,
    /// Maximum failures allowed in half-open state
    pub half_open_max_failures: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            recovery_timeout: Duration::from_secs(60),
            success_threshold: 3,
            half_open_max_failures: 1,
        }
    }
}

/// Thread-safe circuit breaker implementation
pub struct CircuitBreaker {
    state: Arc<RwLock<CircuitState>>,
    failure_count: Arc<AtomicU32>,
    success_count: Arc<AtomicU32>,
    last_failure_time: Arc<RwLock<Option<Instant>>>,
    config: CircuitBreakerConfig,

    // Metrics
    total_requests: Arc<AtomicU64>,
    total_failures: Arc<AtomicU64>,
    circuit_opens: Arc<AtomicU64>,
}

impl CircuitBreaker {
    /// Create a new circuit breaker
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            state: Arc::new(RwLock::new(CircuitState::Closed)),
            failure_count: Arc::new(AtomicU32::new(0)),
            success_count: Arc::new(AtomicU32::new(0)),
            last_failure_time: Arc::new(RwLock::new(None)),
            config,
            total_requests: Arc::new(AtomicU64::new(0)),
            total_failures: Arc::new(AtomicU64::new(0)),
            circuit_opens: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Execute an operation through the circuit breaker
    pub async fn call<F, T, E>(&self, operation: F) -> Result<T, E>
    where
        F: FnOnce() -> Result<T, E>,
        E: From<crate::AdapterError>,
    {
        self.total_requests.fetch_add(1, Ordering::Relaxed);

        // Check if we should attempt the operation
        if !self.should_attempt().await {
            return Err(E::from(crate::AdapterError::CircuitBreakerOpen {
                venue: alphapulse_types::protocol::VenueId::Generic,
            }));
        }

        // Execute the operation
        match operation() {
            Ok(result) => {
                self.on_success().await;
                Ok(result)
            }
            Err(error) => {
                self.on_failure().await;
                self.total_failures.fetch_add(1, Ordering::Relaxed);
                Err(error)
            }
        }
    }

    /// Check if we should allow an operation
    pub async fn should_attempt(&self) -> bool {
        let mut state = self.state.write().await;

        match *state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check if we should transition to half-open
                let last_failure = self.last_failure_time.read().await;
                if let Some(failure_time) = *last_failure {
                    if failure_time.elapsed() >= self.config.recovery_timeout {
                        *state = CircuitState::HalfOpen;
                        self.failure_count.store(0, Ordering::Relaxed);
                        self.success_count.store(0, Ordering::Relaxed);
                        tracing::info!("Circuit breaker transitioning to half-open");
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => true,
        }
    }

    /// Handle successful operation
    pub async fn on_success(&self) {
        let mut state = self.state.write().await;

        match *state {
            CircuitState::HalfOpen => {
                let successes = self.success_count.fetch_add(1, Ordering::Relaxed) + 1;

                if successes >= self.config.success_threshold {
                    *state = CircuitState::Closed;
                    self.failure_count.store(0, Ordering::Relaxed);
                    tracing::info!("Circuit breaker closed after {} successes", successes);
                }
            }
            CircuitState::Closed => {
                // Reset failure count on any success
                self.failure_count.store(0, Ordering::Relaxed);
            }
            CircuitState::Open => {
                // Shouldn't happen, but handle gracefully
            }
        }
    }

    /// Handle failed operation
    pub async fn on_failure(&self) {
        let mut state = self.state.write().await;
        let failures = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;

        // Update last failure time
        *self.last_failure_time.write().await = Some(Instant::now());

        match *state {
            CircuitState::Closed => {
                if failures >= self.config.failure_threshold {
                    *state = CircuitState::Open;
                    self.circuit_opens.fetch_add(1, Ordering::Relaxed);
                    tracing::warn!("Circuit breaker opened after {} failures", failures);
                }
            }
            CircuitState::HalfOpen => {
                if failures >= self.config.half_open_max_failures {
                    *state = CircuitState::Open;
                    self.circuit_opens.fetch_add(1, Ordering::Relaxed);
                    tracing::warn!("Circuit breaker reopened from half-open state");
                }
            }
            CircuitState::Open => {
                // Already open, no action needed
            }
        }
    }

    /// Get current circuit state
    pub async fn state(&self) -> CircuitState {
        *self.state.read().await
    }

    /// Get circuit breaker metrics
    pub fn metrics(&self) -> CircuitBreakerMetrics {
        CircuitBreakerMetrics {
            total_requests: self.total_requests.load(Ordering::Relaxed),
            total_failures: self.total_failures.load(Ordering::Relaxed),
            circuit_opens: self.circuit_opens.load(Ordering::Relaxed),
            current_failure_count: self.failure_count.load(Ordering::Relaxed),
        }
    }

    /// Reset the circuit breaker
    pub async fn reset(&self) {
        *self.state.write().await = CircuitState::Closed;
        self.failure_count.store(0, Ordering::Relaxed);
        self.success_count.store(0, Ordering::Relaxed);
        *self.last_failure_time.write().await = None;
    }
}

/// Metrics for circuit breaker monitoring
#[derive(Debug, Clone)]
pub struct CircuitBreakerMetrics {
    /// Total requests attempted
    pub total_requests: u64,
    /// Total failed requests
    pub total_failures: u64,
    /// Number of times circuit opened
    pub circuit_opens: u64,
    /// Current consecutive failure count
    pub current_failure_count: u32,
}
