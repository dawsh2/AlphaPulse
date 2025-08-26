---
task_id: SINK-002
status: TODO
priority: CRITICAL
estimated_hours: 4
assigned_branch: feat/lazy-connection-wrapper
assignee: TBD
created: 2025-08-26
completed: null
depends_on:
  - SINK-001  # Need MessageSink trait first
blocks:
  - SINK-003  # SinkFactory uses lazy wrapper
scope:
  - "network/transport/src/lazy/"
  - "network/transport/src/messagesink/lazy_wrapper.rs"
---

# SINK-002: Implement Lazy Connection Wrapper

## 🔴 CRITICAL: Enables "Wake on Data" Pattern

### Git Worktree Setup
```bash
# Create worktree for this task
git worktree add -b feat/lazy-connection-wrapper ../messagesink-002
cd ../messagesink-002
```

## Status
**Status**: TODO  
**Priority**: CRITICAL - Core lazy connection functionality
**Branch**: `feat/lazy-connection-wrapper`
**Estimated**: 4 hours
**Depends On**: SINK-001 (MessageSink trait must exist)

## Problem Statement
Services currently establish all connections eagerly at startup, causing:
- Startup order dependencies
- Wasted resources for unused connections
- Complex initialization sequences
- Failures if dependencies aren't ready

We need connections that establish themselves only when data flows.

## Acceptance Criteria
- [ ] LazyMessageSink wrapper implements MessageSink trait
- [ ] Connections established on first send() call
- [ ] Thread-safe connection establishment (no double-connect)
- [ ] Configurable retry logic for failed connections
- [ ] Automatic reconnection on connection loss
- [ ] Connection pooling for efficiency
- [ ] Comprehensive tests for edge cases

## Technical Design

### Core Lazy Wrapper
```rust
// libs/message_sink/src/lazy.rs

use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::Duration;

/// A sink that lazily establishes connections on first use
pub struct LazyMessageSink<S: MessageSink> {
    /// The actual sink (None until connected)
    inner: Arc<RwLock<Option<S>>>,
    
    /// Factory function to create the sink
    factory: Arc<dyn Fn() -> BoxFuture<'static, Result<S, SinkError>> + Send + Sync>,
    
    /// Configuration for lazy behavior
    config: LazyConfig,
    
    /// Connection state tracking
    state: Arc<RwLock<ConnectionState>>,
    
    /// Metrics for monitoring
    metrics: Arc<LazyMetrics>,
}

#[derive(Debug, Clone)]
pub struct LazyConfig {
    /// Max connection attempts before giving up
    pub max_retries: u32,
    
    /// Delay between retry attempts
    pub retry_delay: Duration,
    
    /// Exponential backoff multiplier
    pub backoff_multiplier: f64,
    
    /// Maximum retry delay
    pub max_retry_delay: Duration,
    
    /// Whether to reconnect on connection loss
    pub auto_reconnect: bool,
    
    /// Connection timeout
    pub connect_timeout: Duration,
}

impl Default for LazyConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            retry_delay: Duration::from_millis(100),
            backoff_multiplier: 2.0,
            max_retry_delay: Duration::from_secs(30),
            auto_reconnect: true,
            connect_timeout: Duration::from_secs(5),
        }
    }
}
```

### Thread-Safe Connection Management
```rust
impl<S: MessageSink> LazyMessageSink<S> {
    /// Create new lazy sink with factory function
    pub fn new<F, Fut>(factory: F, config: LazyConfig) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<S, SinkError>> + Send + 'static,
    {
        Self {
            inner: Arc::new(RwLock::new(None)),
            factory: Arc::new(move || Box::pin(factory())),
            config,
            state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
            metrics: Arc::new(LazyMetrics::default()),
        }
    }
    
    /// Ensure connection is established (thread-safe)
    async fn ensure_connected(&self) -> Result<(), SinkError> {
        // Fast path: already connected
        if self.is_connected() {
            return Ok(());
        }
        
        // Slow path: need to connect
        let mut state = self.state.write().await;
        
        // Double-check under lock
        if *state == ConnectionState::Connected {
            return Ok(());
        }
        
        // Prevent concurrent connection attempts
        if *state == ConnectionState::Connecting {
            // Wait for other thread's connection attempt
            drop(state);
            return self.wait_for_connection().await;
        }
        
        // We're the ones connecting
        *state = ConnectionState::Connecting;
        drop(state);
        
        // Attempt connection with retries
        match self.connect_with_retries().await {
            Ok(sink) => {
                *self.inner.write().await = Some(sink);
                *self.state.write().await = ConnectionState::Connected;
                self.metrics.successful_connects.fetch_add(1, Ordering::Relaxed);
                Ok(())
            }
            Err(e) => {
                *self.state.write().await = ConnectionState::Failed;
                self.metrics.failed_connects.fetch_add(1, Ordering::Relaxed);
                Err(e)
            }
        }
    }
    
    async fn connect_with_retries(&self) -> Result<S, SinkError> {
        let mut delay = self.config.retry_delay;
        
        for attempt in 0..=self.config.max_retries {
            match tokio::time::timeout(
                self.config.connect_timeout,
                (self.factory)()
            ).await {
                Ok(Ok(sink)) => return Ok(sink),
                Ok(Err(e)) if attempt < self.config.max_retries => {
                    tracing::warn!(
                        "Connection attempt {} failed: {}, retrying in {:?}",
                        attempt + 1, e, delay
                    );
                    tokio::time::sleep(delay).await;
                    
                    // Exponential backoff
                    delay = Duration::from_secs_f64(
                        (delay.as_secs_f64() * self.config.backoff_multiplier)
                            .min(self.config.max_retry_delay.as_secs_f64())
                    );
                }
                Ok(Err(e)) => return Err(e),
                Err(_) => {
                    if attempt < self.config.max_retries {
                        tracing::warn!("Connection timeout, retrying...");
                        tokio::time::sleep(delay).await;
                    } else {
                        return Err(SinkError::Timeout(
                            self.config.connect_timeout.as_secs()
                        ));
                    }
                }
            }
        }
        
        Err(SinkError::ConnectionFailed("Max retries exceeded".into()))
    }
}
```

### MessageSink Implementation
```rust
#[async_trait]
impl<S: MessageSink> MessageSink for LazyMessageSink<S> {
    async fn send(&self, message: Message) -> Result<(), SinkError> {
        // Ensure connected (lazy connection happens here)
        self.ensure_connected().await?;
        
        // Get inner sink
        let inner = self.inner.read().await;
        let sink = inner.as_ref()
            .ok_or_else(|| SinkError::ConnectionFailed("No sink after connect".into()))?;
        
        // Send through inner sink
        match sink.send(message).await {
            Ok(()) => {
                self.metrics.messages_sent.fetch_add(1, Ordering::Relaxed);
                Ok(())
            }
            Err(e) if self.config.auto_reconnect && is_connection_error(&e) => {
                drop(inner);
                // Connection lost, try to reconnect
                *self.state.write().await = ConnectionState::Disconnected;
                *self.inner.write().await = None;
                
                // Retry with reconnection
                self.ensure_connected().await?;
                let inner = self.inner.read().await;
                let sink = inner.as_ref().unwrap();
                sink.send(message).await
            }
            Err(e) => {
                self.metrics.messages_failed.fetch_add(1, Ordering::Relaxed);
                Err(e)
            }
        }
    }
    
    async fn send_batch(&self, messages: Vec<Message>) -> Result<(), SinkError> {
        self.ensure_connected().await?;
        
        let inner = self.inner.read().await;
        let sink = inner.as_ref()
            .ok_or_else(|| SinkError::ConnectionFailed("No sink after connect".into()))?;
        
        sink.send_batch(messages).await
    }
    
    fn is_connected(&self) -> bool {
        // Non-blocking check
        if let Ok(state) = self.state.try_read() {
            *state == ConnectionState::Connected
        } else {
            false
        }
    }
    
    async fn connect(&self) -> Result<(), SinkError> {
        self.ensure_connected().await
    }
    
    async fn disconnect(&self) -> Result<(), SinkError> {
        let mut inner = self.inner.write().await;
        if let Some(sink) = inner.take() {
            sink.disconnect().await?;
        }
        *self.state.write().await = ConnectionState::Disconnected;
        Ok(())
    }
}
```

### Connection Pool Support
```rust
/// Pool of lazy connections for efficiency
pub struct LazyConnectionPool<S: MessageSink> {
    /// Available connections
    pool: Arc<RwLock<Vec<Arc<LazyMessageSink<S>>>>>,
    
    /// Factory for creating new connections
    factory: Arc<dyn Fn() -> BoxFuture<'static, Result<S, SinkError>> + Send + Sync>,
    
    /// Pool configuration
    config: PoolConfig,
}

#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// Maximum connections in pool
    pub max_size: usize,
    
    /// Minimum idle connections
    pub min_idle: usize,
    
    /// Connection idle timeout
    pub idle_timeout: Duration,
    
    /// Lazy connection config
    pub lazy_config: LazyConfig,
}
```

## Testing Requirements

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_lazy_connection_on_first_send() {
        let connect_count = Arc::new(AtomicU32::new(0));
        let count_clone = connect_count.clone();
        
        let lazy = LazyMessageSink::new(
            move || {
                count_clone.fetch_add(1, Ordering::Relaxed);
                async { Ok(CollectorSink::new()) }
            },
            LazyConfig::default()
        );
        
        // Not connected yet
        assert!(!lazy.is_connected());
        assert_eq!(connect_count.load(Ordering::Relaxed), 0);
        
        // First send triggers connection
        lazy.send(Message::default()).await.unwrap();
        assert!(lazy.is_connected());
        assert_eq!(connect_count.load(Ordering::Relaxed), 1);
        
        // Second send doesn't reconnect
        lazy.send(Message::default()).await.unwrap();
        assert_eq!(connect_count.load(Ordering::Relaxed), 1);
    }
    
    #[tokio::test]
    async fn test_concurrent_connection_attempts() {
        // Test that multiple threads don't create multiple connections
        let connect_count = Arc::new(AtomicU32::new(0));
        
        let lazy = Arc::new(LazyMessageSink::new(
            // ... factory that increments count
        ));
        
        // Spawn multiple concurrent sends
        let mut handles = vec![];
        for _ in 0..10 {
            let lazy_clone = lazy.clone();
            handles.push(tokio::spawn(async move {
                lazy_clone.send(Message::default()).await
            }));
        }
        
        // Wait for all
        for handle in handles {
            handle.await.unwrap().unwrap();
        }
        
        // Should only connect once
        assert_eq!(connect_count.load(Ordering::Relaxed), 1);
    }
}
```

## Files to Create/Modify
- `libs/message_sink/src/lazy.rs` - Lazy wrapper implementation
- `libs/message_sink/src/pool.rs` - Connection pool
- `libs/message_sink/src/metrics.rs` - Metrics tracking
- `libs/message_sink/src/lib.rs` - Export lazy module

## Completion Checklist
- [ ] Worktree created
- [ ] LazyMessageSink implemented
- [ ] Thread-safe connection management
- [ ] Retry logic with backoff
- [ ] Auto-reconnection on failure
- [ ] Connection pool support
- [ ] All tests passing
- [ ] Performance benchmarks
- [ ] PR created
- [ ] Status updated to COMPLETE

## Why This Matters
The lazy connection wrapper is the key innovation that enables:
- Services to start in any order
- Connections only when data flows ("wake on data")
- Automatic recovery from connection failures
- Efficient resource usage in development/testing
- Foundation for Mycelium's lazy provisioning