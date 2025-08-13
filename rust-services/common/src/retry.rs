// Retry logic and circuit breaker for resilient connections
use std::time::Duration;
use std::sync::atomic::{AtomicU32, AtomicBool, Ordering};
use std::sync::Arc;
use tokio::time::sleep;
use tracing::{info, warn};

#[derive(Clone)]
pub struct RetryPolicy {
    max_retries: u32,
    initial_delay: Duration,
    max_delay: Duration,
    exponential_base: f64,
}

impl RetryPolicy {
    pub fn new() -> Self {
        Self {
            max_retries: 5,
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(60),
            exponential_base: 2.0,
        }
    }
    
    pub async fn execute<F, Fut, T, E>(&self, mut f: F) -> Result<T, E>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T, E>>,
        E: std::fmt::Display,
    {
        let mut attempt = 0;
        let mut delay = self.initial_delay;
        
        loop {
            match f().await {
                Ok(result) => return Ok(result),
                Err(e) if attempt >= self.max_retries => {
                    warn!("Max retries ({}) reached. Last error: {}", self.max_retries, e);
                    return Err(e);
                }
                Err(e) => {
                    attempt += 1;
                    warn!("Attempt {} failed: {}. Retrying in {:?}", attempt, e, delay);
                    sleep(delay).await;
                    
                    // Exponential backoff with jitter
                    let jitter = Duration::from_millis(rand::random::<u64>() % 1000);
                    delay = Duration::from_secs_f64(
                        (delay.as_secs_f64() * self.exponential_base).min(self.max_delay.as_secs_f64())
                    ) + jitter;
                }
            }
        }
    }
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self::new()
    }
}

// Circuit breaker to prevent cascading failures
pub struct CircuitBreaker {
    failure_count: AtomicU32,
    is_open: AtomicBool,
    failure_threshold: u32,
    reset_timeout: Duration,
    last_failure_time: Arc<RwLock<Option<std::time::Instant>>>,
}

use tokio::sync::RwLock;

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, reset_timeout: Duration) -> Self {
        Self {
            failure_count: AtomicU32::new(0),
            is_open: AtomicBool::new(false),
            failure_threshold,
            reset_timeout,
            last_failure_time: Arc::new(RwLock::new(None)),
        }
    }
    
    pub async fn is_open(&self) -> bool {
        if !self.is_open.load(Ordering::Relaxed) {
            return false;
        }
        
        // Check if we should reset
        if let Some(last_failure) = *self.last_failure_time.read().await {
            if last_failure.elapsed() > self.reset_timeout {
                info!("Circuit breaker reset after timeout");
                self.reset();
                return false;
            }
        }
        
        true
    }
    
    pub fn record_success(&self) {
        let previous_count = self.failure_count.swap(0, Ordering::Relaxed);
        if previous_count > 0 {
            info!("Circuit breaker: success recorded, failure count reset");
        }
        self.is_open.store(false, Ordering::Relaxed);
    }
    
    pub async fn record_failure(&self) {
        let count = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
        *self.last_failure_time.write().await = Some(std::time::Instant::now());
        
        if count >= self.failure_threshold {
            self.is_open.store(true, Ordering::Relaxed);
            warn!("Circuit breaker opened after {} failures", count);
        }
    }
    
    pub fn reset(&self) {
        self.failure_count.store(0, Ordering::Relaxed);
        self.is_open.store(false, Ordering::Relaxed);
        info!("Circuit breaker reset");
    }
}

// Generate random u64 for jitter
mod rand {
    pub fn random<T>() -> T 
    where
        T: From<u64>
    {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        T::from(seed)
    }
}