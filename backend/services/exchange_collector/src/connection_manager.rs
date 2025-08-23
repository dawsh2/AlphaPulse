use anyhow::{Result, anyhow};
use std::future::Future;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, warn, error};
use rand::Rng;

/// Robust connection management with exponential backoff
#[derive(Debug, Clone)]
pub struct ConnectionManager {
    max_retries: u32,
    base_delay_ms: u64,
    max_delay_ms: u64,
    jitter: bool,
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            max_retries: 10,
            base_delay_ms: 100,
            max_delay_ms: 30_000, // 30 seconds max
            jitter: true,
        }
    }

    pub fn with_config(max_retries: u32, base_delay_ms: u64, max_delay_ms: u64) -> Self {
        Self {
            max_retries,
            base_delay_ms,
            max_delay_ms,
            jitter: true,
        }
    }

    /// Connect with exponential backoff retry logic
    pub async fn connect_with_backoff<F, T, Fut>(&self, mut connect_fn: F) -> Result<T>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<T>>,
    {
        let mut delay = self.base_delay_ms;
        let mut rng = rand::thread_rng();
        
        for attempt in 1..=self.max_retries {
            match connect_fn().await {
                Ok(result) => {
                    if attempt > 1 {
                        info!("Successfully connected after {} attempts", attempt);
                    }
                    return Ok(result);
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    
                    if !Self::is_retryable(&error_msg) {
                        error!("Non-retryable error: {}", error_msg);
                        return Err(e);
                    }
                    
                    if attempt == self.max_retries {
                        error!("Max retries ({}) exceeded. Last error: {}", self.max_retries, error_msg);
                        return Err(anyhow!("Max retries exceeded: {}", error_msg));
                    }
                    
                    // Add jitter to prevent thundering herd
                    let jitter = if self.jitter {
                        rng.gen_range(0..=delay / 2)
                    } else {
                        0
                    };
                    
                    let total_delay = delay + jitter;
                    warn!(
                        "Connection attempt {} failed: {}. Retrying in {}ms...",
                        attempt, error_msg, total_delay
                    );
                    
                    sleep(Duration::from_millis(total_delay)).await;
                    
                    // Exponential backoff with cap
                    delay = (delay * 2).min(self.max_delay_ms);
                }
            }
        }
        
        Err(anyhow!("Connection failed after {} attempts", self.max_retries))
    }

    /// Determine if an error is retryable
    fn is_retryable(error_msg: &str) -> bool {
        // Rate limiting errors
        if error_msg.contains("429") || 
           error_msg.contains("rate") || 
           error_msg.contains("Too Many Requests") {
            return true;
        }
        
        // Network errors
        if error_msg.contains("timeout") || 
           error_msg.contains("connection") ||
           error_msg.contains("refused") ||
           error_msg.contains("reset") ||
           error_msg.contains("broken pipe") {
            return true;
        }
        
        // WebSocket specific errors
        if error_msg.contains("WebSocket") ||
           error_msg.contains("disconnected") ||
           error_msg.contains("closed") {
            return true;
        }
        
        // Temporary failures
        if error_msg.contains("temporarily") ||
           error_msg.contains("unavailable") ||
           error_msg.contains("503") {
            return true;
        }
        
        false
    }

    /// Monitor connection health with periodic heartbeats
    pub async fn monitor_health<F, Fut>(&self, check_fn: F, interval_secs: u64) 
    where
        F: Fn() -> Fut,
        Fut: Future<Output = bool>,
    {
        let mut consecutive_failures = 0;
        let max_failures = 3;
        
        loop {
            sleep(Duration::from_secs(interval_secs)).await;
            
            if check_fn().await {
                if consecutive_failures > 0 {
                    info!("Connection health restored");
                }
                consecutive_failures = 0;
            } else {
                consecutive_failures += 1;
                warn!("Health check failed ({}/{})", consecutive_failures, max_failures);
                
                if consecutive_failures >= max_failures {
                    error!("Connection unhealthy after {} consecutive failures", max_failures);
                    // Trigger reconnection logic here
                    break;
                }
            }
        }
    }
}

impl Default for ConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_successful_connection() {
        let manager = ConnectionManager::new();
        let result = manager.connect_with_backoff(|| async {
            Ok::<_, anyhow::Error>("connected")
        }).await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "connected");
    }

    #[tokio::test]
    async fn test_retry_with_eventual_success() {
        let manager = ConnectionManager::with_config(5, 10, 100);
        let counter = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let counter_clone = counter.clone();
        
        let result = manager.connect_with_backoff(|| {
            let count = counter_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            async move {
                if count < 2 {
                    Err(anyhow!("connection timeout"))
                } else {
                    Ok("success")
                }
            }
        }).await;
        
        assert!(result.is_ok());
        assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_non_retryable_error() {
        let manager = ConnectionManager::new();
        let result = manager.connect_with_backoff(|| async {
            Err::<String, _>(anyhow!("authentication failed"))
        }).await;
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("authentication failed"));
    }
}