// Retry logic with exponential backoff
use std::time::Duration;
use tokio::time::sleep;
use tracing::{warn, info};

pub struct RetryPolicy {
    pub max_attempts: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub exponential_base: f64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 10,
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(300),
            exponential_base: 2.0,
        }
    }
}

impl RetryPolicy {
    pub fn from_config(config: &crate::config::CollectorsConfig) -> Self {
        Self {
            max_attempts: config.max_reconnect_attempts,
            initial_delay: Duration::from_secs(config.reconnect_delay_secs),
            max_delay: Duration::from_secs(config.max_backoff_secs),
            exponential_base: if config.exponential_backoff {
                config.backoff_multiplier
            } else {
                1.0
            },
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
            attempt += 1;
            
            match f().await {
                Ok(result) => {
                    if attempt > 1 {
                        info!("Retry successful after {} attempts", attempt);
                    }
                    return Ok(result);
                }
                Err(e) => {
                    if attempt >= self.max_attempts {
                        warn!("Max retry attempts ({}) reached", self.max_attempts);
                        return Err(e);
                    }
                    
                    warn!(
                        "Attempt {}/{} failed: {}. Retrying in {:?}",
                        attempt, self.max_attempts, e, delay
                    );
                    
                    sleep(delay).await;
                    
                    // Calculate next delay with exponential backoff
                    if self.exponential_base > 1.0 {
                        let next_delay_ms = (delay.as_millis() as f64 * self.exponential_base) as u64;
                        delay = Duration::from_millis(next_delay_ms.min(self.max_delay.as_millis() as u64));
                    }
                }
            }
        }
    }
}

/// Circuit breaker to prevent overwhelming failed services
pub struct CircuitBreaker {
    failure_count: std::sync::Arc<std::sync::atomic::AtomicU32>,
    last_failure: std::sync::Arc<std::sync::RwLock<Option<std::time::Instant>>>,
    threshold: u32,
    reset_timeout: Duration,
}

impl CircuitBreaker {
    pub fn new(threshold: u32, reset_timeout: Duration) -> Self {
        Self {
            failure_count: std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0)),
            last_failure: std::sync::Arc::new(std::sync::RwLock::new(None)),
            threshold,
            reset_timeout,
        }
    }
    
    pub fn is_open(&self) -> bool {
        let count = self.failure_count.load(std::sync::atomic::Ordering::Relaxed);
        
        if count >= self.threshold {
            // Check if we should reset
            if let Ok(last) = self.last_failure.read() {
                if let Some(last_time) = *last {
                    if last_time.elapsed() > self.reset_timeout {
                        // Reset the circuit breaker
                        self.failure_count.store(0, std::sync::atomic::Ordering::Relaxed);
                        return false;
                    }
                }
            }
            true
        } else {
            false
        }
    }
    
    pub fn record_success(&self) {
        self.failure_count.store(0, std::sync::atomic::Ordering::Relaxed);
        if let Ok(mut last) = self.last_failure.write() {
            *last = None;
        }
    }
    
    pub fn record_failure(&self) {
        self.failure_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if let Ok(mut last) = self.last_failure.write() {
            *last = Some(std::time::Instant::now());
        }
    }
}