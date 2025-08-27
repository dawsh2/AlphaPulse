//! Lazy Connection Wrapper for MessageSink
//!
//! Implements the "wake on data" pattern where connections are established only
//! when data needs to be sent. This enables services to start in any order
//! without complex initialization dependencies.
//!
//! ## Key Features
//!
//! - **Lazy Connection**: Establishes connection only on first send() call
//! - **Thread-Safe**: Multiple threads can safely attempt connection simultaneously
//! - **Retry Logic**: Configurable exponential backoff on connection failures
//! - **Auto-Reconnection**: Automatic reconnection on connection loss (optional)
//! - **Connection Pooling**: Support for efficient connection reuse
//!
//! ## Usage
//!
//! ```rust
//! use crate::{LazyMessageSink, LazyConfig};
//!
//! // Create factory function
//! let factory = || async {
//!     // Your connection logic here
//!     Ok(YourSinkImplementation::new())
//! };
//!
//! let lazy_sink = LazyMessageSink::new(factory, LazyConfig::default());
//! 
//! // Connection happens here, not during construction
//! lazy_sink.send(message).await?;
//! ```

use crate::{BatchResult, ConnectionHealth, ConnectionState, ExtendedSinkMetadata, Message, MessageSink, SinkError, SinkMetadata, MessageDomain, TLVMessage};
use async_trait::async_trait;
use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::{Mutex, RwLock};

/// Type alias for boxed async factory functions
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Connection states for lazy sink
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LazyConnectionState {
    /// Not connected yet (initial state)
    Disconnected,
    /// Currently attempting to connect
    Connecting,
    /// Successfully connected
    Connected,
    /// Connection failed
    Failed,
}

/// Configuration for lazy connection behavior
#[derive(Debug, Clone)]
pub struct LazyConfig {
    /// Max connection attempts before giving up
    pub max_retries: u32,
    
    /// Initial delay between retry attempts
    pub retry_delay: Duration,
    
    /// Exponential backoff multiplier
    pub backoff_multiplier: f64,
    
    /// Maximum retry delay cap
    pub max_retry_delay: Duration,
    
    /// Whether to reconnect on connection loss
    pub auto_reconnect: bool,
    
    /// Connection timeout per attempt
    pub connect_timeout: Duration,
    
    /// Timeout for waiting on other threads' connection attempts
    pub wait_timeout: Duration,
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
            wait_timeout: Duration::from_secs(10),
        }
    }
}

impl LazyConfig {
    /// Create a fast-recovery configuration for low-latency systems
    pub fn fast_recovery() -> Self {
        Self {
            max_retries: 2,
            retry_delay: Duration::from_millis(50),
            backoff_multiplier: 1.5,
            max_retry_delay: Duration::from_secs(5),
            auto_reconnect: true,
            connect_timeout: Duration::from_secs(2),
            wait_timeout: Duration::from_secs(5),
        }
    }

    /// Create a conservative configuration for reliable connections
    pub fn conservative() -> Self {
        Self {
            max_retries: 5,
            retry_delay: Duration::from_millis(500),
            backoff_multiplier: 2.5,
            max_retry_delay: Duration::from_secs(60),
            auto_reconnect: true,
            connect_timeout: Duration::from_secs(10),
            wait_timeout: Duration::from_secs(30),
        }
    }
}

/// Metrics for monitoring lazy connection behavior
#[derive(Debug)]
pub struct LazyMetrics {
    /// Total connection attempts
    pub connection_attempts: AtomicU64,
    /// Successful connections
    pub successful_connects: AtomicU64,
    /// Failed connection attempts
    pub failed_connects: AtomicU64,
    /// Messages sent successfully
    pub messages_sent: AtomicU64,
    /// Messages that failed to send
    pub messages_failed: AtomicU64,
    /// Times we waited for another thread's connection
    pub connection_waits: AtomicU64,
    /// Automatic reconnection attempts
    pub reconnection_attempts: AtomicU64,
}

impl Default for LazyMetrics {
    fn default() -> Self {
        Self {
            connection_attempts: AtomicU64::new(0),
            successful_connects: AtomicU64::new(0),
            failed_connects: AtomicU64::new(0),
            messages_sent: AtomicU64::new(0),
            messages_failed: AtomicU64::new(0),
            connection_waits: AtomicU64::new(0),
            reconnection_attempts: AtomicU64::new(0),
        }
    }
}

impl LazyMetrics {
    /// Get connection success rate
    pub fn connection_success_rate(&self) -> f64 {
        let total = self.connection_attempts.load(Ordering::Relaxed);
        if total == 0 {
            return 1.0;
        }
        let successful = self.successful_connects.load(Ordering::Relaxed);
        successful as f64 / total as f64
    }

    /// Get message success rate
    pub fn message_success_rate(&self) -> f64 {
        let sent = self.messages_sent.load(Ordering::Relaxed);
        let failed = self.messages_failed.load(Ordering::Relaxed);
        let total = sent + failed;
        if total == 0 {
            return 1.0;
        }
        sent as f64 / total as f64
    }
}

/// Internal state tracking for the lazy sink
#[derive(Debug)]
struct LazyState {
    state: LazyConnectionState,
    last_connection_attempt: Option<Instant>,
    connection_established_at: Option<Instant>,
}

impl LazyState {
    fn new() -> Self {
        Self {
            state: LazyConnectionState::Disconnected,
            last_connection_attempt: None,
            connection_established_at: None,
        }
    }

    fn transition_to(&mut self, new_state: LazyConnectionState) {
        if self.state != new_state {
            tracing::debug!(
                "Lazy connection state transition: {:?} -> {:?}",
                self.state,
                new_state
            );
            self.state = new_state;
            
            if matches!(new_state, LazyConnectionState::Connected) {
                self.connection_established_at = Some(Instant::now());
            } else if matches!(new_state, LazyConnectionState::Disconnected | LazyConnectionState::Failed) {
                self.connection_established_at = None;
            }
        }
    }
}

/// A sink that lazily establishes connections on first use
pub struct LazyMessageSink<S: MessageSink> {
    /// The actual sink (None until connected)
    inner: Arc<RwLock<Option<S>>>,
    
    /// Factory function to create the sink
    factory: Arc<dyn Fn() -> BoxFuture<'static, Result<S, SinkError>> + Send + Sync>,
    
    /// Configuration for lazy behavior
    config: LazyConfig,
    
    /// Connection state tracking
    state: Arc<RwLock<LazyState>>,
    
    /// Metrics for monitoring
    metrics: Arc<LazyMetrics>,
    
    /// Mutex to ensure only one thread attempts connection at a time
    connection_mutex: Arc<Mutex<()>>,
    
    /// Name for debugging and metrics
    name: String,
    
    /// Expected TLV domain for validation (optional)
    expected_domain: Option<MessageDomain>,
    
    /// Cached connection state for performance
    cached_connected: Arc<std::sync::atomic::AtomicBool>,
}

impl<S: MessageSink> Debug for LazyMessageSink<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LazyMessageSink")
            .field("name", &self.name)
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

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
            state: Arc::new(RwLock::new(LazyState::new())),
            metrics: Arc::new(LazyMetrics::default()),
            connection_mutex: Arc::new(Mutex::new(())),
            name: "lazy-sink".to_string(),
            expected_domain: None,
            cached_connected: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Create new lazy sink with a name for debugging
    pub fn with_name<F, Fut>(factory: F, config: LazyConfig, name: impl Into<String>) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<S, SinkError>> + Send + 'static,
    {
        let mut sink = Self::new(factory, config);
        sink.name = name.into();
        sink
    }

    /// Create new lazy sink with TLV domain validation
    pub fn with_domain<F, Fut>(factory: F, config: LazyConfig, domain: MessageDomain) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<S, SinkError>> + Send + 'static,
    {
        let mut sink = Self::new(factory, config);
        sink.expected_domain = Some(domain);
        sink
    }

    /// Create new lazy sink with name and TLV domain validation
    pub fn with_name_and_domain<F, Fut>(
        factory: F, 
        config: LazyConfig, 
        name: impl Into<String>,
        domain: MessageDomain
    ) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<S, SinkError>> + Send + 'static,
    {
        let mut sink = Self::new(factory, config);
        sink.name = name.into();
        sink.expected_domain = Some(domain);
        sink
    }

    /// Create with default configuration
    pub fn with_default_config<F, Fut>(factory: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<S, SinkError>> + Send + 'static,
    {
        Self::new(factory, LazyConfig::default())
    }

    /// Get current lazy connection metrics
    pub fn lazy_metrics(&self) -> &LazyMetrics {
        &self.metrics
    }

    /// Get current connection state
    pub async fn connection_state(&self) -> LazyConnectionState {
        let state = self.state.read().await;
        state.state
    }

    /// Check if connection is currently established
    pub async fn is_lazy_connected(&self) -> bool {
        let state = self.state.read().await;
        let connected = matches!(state.state, LazyConnectionState::Connected);
        self.cached_connected.store(connected, Ordering::Relaxed);
        connected
    }

    /// Fast cached connection check (avoids async operations)
    pub fn is_connected_cached(&self) -> bool {
        self.cached_connected.load(Ordering::Relaxed)
    }

    /// Get time since connection was established
    pub async fn connection_uptime(&self) -> Option<Duration> {
        let state = self.state.read().await;
        state.connection_established_at.map(|t| t.elapsed())
    }

    /// Validate message for TLV compliance and precision
    fn validate_message(&self, message: &Message) -> Result<(), SinkError> {
        // TLV domain validation - for now, we'll skip complex parsing
        // In a real implementation, this would parse the TLV header and validate domain
        if let Some(_expected_domain) = self.expected_domain {
            // TODO: Implement proper TLV parsing to extract domain from message
            // For now, we'll accept the message and let the downstream sink validate
            tracing::debug!("TLV domain validation deferred to downstream sink");
        }

        // Precision validation - determine if this is DEX or traditional exchange
        let is_dex = self.expected_domain.map(|d| matches!(d, MessageDomain::MarketData | MessageDomain::Execution))
            .unwrap_or(false);
        
        message.validate_precision(is_dex)?;
        
        Ok(())
    }

    /// Force disconnect (for testing or maintenance)
    pub async fn force_disconnect(&self) -> Result<(), SinkError> {
        let _guard = self.connection_mutex.lock().await;
        
        // Disconnect inner sink if present
        {
            let mut inner = self.inner.write().await;
            if let Some(sink) = inner.take() {
                sink.disconnect().await?;
            }
        }
        
        // Update state
        let mut state = self.state.write().await;
        state.transition_to(LazyConnectionState::Disconnected);
        
        tracing::info!("Lazy sink '{}' forcefully disconnected", self.name);
        Ok(())
    }

    /// Ensure connection is established (thread-safe)
    async fn ensure_connected(&self) -> Result<(), SinkError> {
        // Fast path: already connected
        {
            let state = self.state.read().await;
            if matches!(state.state, LazyConnectionState::Connected) {
                // Double-check we have an inner sink
                let inner = self.inner.read().await;
                if inner.is_some() {
                    return Ok(());
                }
            }
        }

        // Slow path: need to connect
        let _guard = self.connection_mutex.lock().await;
        
        // Double-check under mutex
        {
            let state = self.state.read().await;
            if matches!(state.state, LazyConnectionState::Connected) {
                let inner = self.inner.read().await;
                if inner.is_some() {
                    return Ok(());
                }
            }
            
            // Check if another thread is connecting
            if matches!(state.state, LazyConnectionState::Connecting) {
                drop(state);
                // Wait for connection with timeout
                self.metrics.connection_waits.fetch_add(1, Ordering::Relaxed);
                return self.wait_for_connection().await;
            }
        }

        // We're the ones connecting
        {
            let mut state = self.state.write().await;
            state.transition_to(LazyConnectionState::Connecting);
            state.last_connection_attempt = Some(Instant::now());
        }

        tracing::debug!("Lazy sink '{}' attempting connection", self.name);
        
        // Attempt connection with retries
        match self.connect_with_retries().await {
            Ok(sink) => {
                // Store the connected sink
                {
                    let mut inner = self.inner.write().await;
                    *inner = Some(sink);
                }
                
                // Update state
                {
                    let mut state = self.state.write().await;
                    state.transition_to(LazyConnectionState::Connected);
                }
                
                // Update cached connection state
                self.cached_connected.store(true, Ordering::Relaxed);
                
                self.metrics.successful_connects.fetch_add(1, Ordering::Relaxed);
                tracing::info!("Lazy sink '{}' connected successfully", self.name);
                Ok(())
            }
            Err(e) => {
                // Update state to failed
                {
                    let mut state = self.state.write().await;
                    state.transition_to(LazyConnectionState::Failed);
                }
                
                // Update cached connection state
                self.cached_connected.store(false, Ordering::Relaxed);
                
                self.metrics.failed_connects.fetch_add(1, Ordering::Relaxed);
                tracing::error!("Lazy sink '{}' connection failed: {}", self.name, e);
                Err(e)
            }
        }
    }
    
    /// Wait for another thread's connection attempt to complete
    async fn wait_for_connection(&self) -> Result<(), SinkError> {
        let start = Instant::now();
        let timeout = self.config.wait_timeout;
        
        while start.elapsed() < timeout {
            tokio::time::sleep(Duration::from_millis(50)).await;
            
            let state = self.state.read().await;
            match state.state {
                LazyConnectionState::Connected => {
                    // Check if we actually have a sink
                    let inner = self.inner.read().await;
                    if inner.is_some() {
                        return Ok(());
                    }
                }
                LazyConnectionState::Failed => {
                    return Err(SinkError::connection_failed(
                        "Connection attempt by another thread failed"
                    ));
                }
                LazyConnectionState::Disconnected => {
                    // Another thread gave up or was interrupted
                    return Err(SinkError::connection_failed(
                        "Connection attempt was abandoned"
                    ));
                }
                LazyConnectionState::Connecting => {
                    // Still connecting, keep waiting
                    continue;
                }
            }
        }
        
        Err(SinkError::timeout(self.config.wait_timeout.as_secs()))
    }
    
    /// Attempt connection with retry logic and exponential backoff
    async fn connect_with_retries(&self) -> Result<S, SinkError> {
        let mut delay = self.config.retry_delay;
        
        for attempt in 0..=self.config.max_retries {
            self.metrics.connection_attempts.fetch_add(1, Ordering::Relaxed);
            
            tracing::debug!("Lazy sink '{}' connection attempt {}/{}", 
                          self.name, attempt + 1, self.config.max_retries + 1);
            
            match tokio::time::timeout(self.config.connect_timeout, (self.factory)()).await {
                Ok(Ok(sink)) => {
                    tracing::debug!("Lazy sink '{}' connected on attempt {}", self.name, attempt + 1);
                    return Ok(sink);
                }
                Ok(Err(e)) if attempt < self.config.max_retries => {
                    tracing::warn!(
                        "Lazy sink '{}' connection attempt {} failed: {}, retrying in {:?}",
                        self.name, attempt + 1, e, delay
                    );
                    tokio::time::sleep(delay).await;
                    
                    // Exponential backoff
                    let next_delay_secs = delay.as_secs_f64() * self.config.backoff_multiplier;
                    delay = Duration::from_secs_f64(
                        next_delay_secs.min(self.config.max_retry_delay.as_secs_f64())
                    );
                }
                Ok(Err(e)) => {
                    tracing::error!("Lazy sink '{}' final connection attempt failed: {}", self.name, e);
                    return Err(e);
                }
                Err(_) => {
                    if attempt < self.config.max_retries {
                        tracing::warn!("Lazy sink '{}' connection timeout, retrying...", self.name);
                        tokio::time::sleep(delay).await;
                    } else {
                        tracing::error!("Lazy sink '{}' connection timed out after {} seconds", 
                                      self.name, self.config.connect_timeout.as_secs());
                        return Err(SinkError::timeout(self.config.connect_timeout.as_secs()));
                    }
                }
            }
        }
        
        Err(SinkError::connection_failed("Max retries exceeded"))
    }
    
    /// Check if an error indicates connection loss
    fn is_connection_error(error: &SinkError) -> bool {
        match error {
            // Explicit connection errors
            SinkError::ConnectionFailed(_) => true,
            
            // Timeout errors are usually connection-related
            SinkError::Timeout(_) => true,
            
            // Check send failures more precisely  
            SinkError::SendFailed { error, context: _ } => {
                let msg_lower = error.to_lowercase();
                // More specific patterns to avoid false positives
                msg_lower.contains("connection reset") 
                    || msg_lower.contains("connection refused")
                    || msg_lower.contains("connection lost")
                    || msg_lower.contains("connection closed")
                    || msg_lower.contains("broken pipe")
                    || msg_lower.contains("network unreachable")
                    || msg_lower.contains("socket closed")
                    || msg_lower.contains("disconnect")
            }
            
            // Buffer full is usually temporary, not a connection error
            SinkError::BufferFull { .. } => false,
            
            // Message format errors are not connection errors
            SinkError::MessageTooLarge { .. } => false,
            SinkError::InvalidConfig(_) => false,
            
            // Other errors might be connection-related, be conservative
            _ => false,
        }
    }
    
    /// Handle reconnection on connection loss
    async fn handle_connection_loss(&self) -> Result<(), SinkError> {
        if !self.config.auto_reconnect {
            return Err(SinkError::connection_failed("Auto-reconnect disabled"));
        }
        
        self.metrics.reconnection_attempts.fetch_add(1, Ordering::Relaxed);
        tracing::info!("Lazy sink '{}' attempting automatic reconnection", self.name);
        
        // Clear the failed connection
        {
            let mut inner = self.inner.write().await;
            *inner = None;
        }
        
        // Reset state to disconnected
        {
            let mut state = self.state.write().await;
            state.transition_to(LazyConnectionState::Disconnected);
        }
        
        // Attempt to reconnect
        self.ensure_connected().await
    }
}

#[async_trait]
impl<S: MessageSink> MessageSink for LazyMessageSink<S> {
    async fn send(&self, message: Message) -> Result<(), SinkError> {
        // Validate message for TLV compliance and precision
        self.validate_message(&message)?;
        
        // Ensure connected (lazy connection happens here)
        if let Err(e) = self.ensure_connected().await {
            self.metrics.messages_failed.fetch_add(1, Ordering::Relaxed);
            return Err(e);
        }
        
        // Send through inner sink while holding the read lock (simplified approach)
        
        // Send through inner sink while holding the read lock
        let result = {
            let inner_guard = self.inner.read().await;
            let sink = inner_guard.as_ref()
                .ok_or_else(|| SinkError::connection_failed("Sink disappeared during send"))?;
            
            sink.send(message.clone()).await
        };
        
        match result {
            Ok(()) => {
                self.metrics.messages_sent.fetch_add(1, Ordering::Relaxed);
                Ok(())
            }
            Err(e) if self.config.auto_reconnect && Self::is_connection_error(&e) => {
                tracing::warn!("Lazy sink '{}' detected connection loss during send: {}", self.name, e);
                
                // Try to reconnect and retry once
                match self.handle_connection_loss().await {
                    Ok(()) => {
                        // Retry the send with the new connection
                        let inner_guard = self.inner.read().await;
                        let sink = inner_guard.as_ref()
                            .ok_or_else(|| SinkError::connection_failed("No sink after reconnection"))?;
                        
                        match sink.send(message).await {
                            Ok(()) => {
                                self.metrics.messages_sent.fetch_add(1, Ordering::Relaxed);
                                Ok(())
                            }
                            Err(retry_err) => {
                                self.metrics.messages_failed.fetch_add(1, Ordering::Relaxed);
                                Err(retry_err)
                            }
                        }
                    }
                    Err(reconnect_err) => {
                        self.metrics.messages_failed.fetch_add(1, Ordering::Relaxed);
                        Err(reconnect_err)
                    }
                }
            }
            Err(e) => {
                self.metrics.messages_failed.fetch_add(1, Ordering::Relaxed);
                Err(e)
            }
        }
    }

    async fn send_batch(&self, messages: Vec<Message>) -> Result<BatchResult, SinkError> {
        // Validate all messages first
        for message in &messages {
            self.validate_message(message)?;
        }
        
        self.ensure_connected().await?;
        
        let inner_guard = self.inner.read().await;
        let sink = inner_guard.as_ref()
            .ok_or_else(|| SinkError::connection_failed("No sink after ensure_connected"))?;
        
        sink.send_batch(messages).await
    }
    
    async fn send_batch_prioritized(&self, messages: Vec<Message>) -> Result<BatchResult, SinkError> {
        // Validate all messages first
        for message in &messages {
            self.validate_message(message)?;
        }
        
        self.ensure_connected().await?;
        
        let inner_guard = self.inner.read().await;
        let sink = inner_guard.as_ref()
            .ok_or_else(|| SinkError::connection_failed("No sink after ensure_connected"))?;
        
        sink.send_batch_prioritized(messages).await
    }

    fn is_connected(&self) -> bool {
        // Use cached state for performance, but verify with inner sink
        if self.cached_connected.load(Ordering::Relaxed) {
            if let Ok(inner) = self.inner.try_read() {
                let actually_connected = inner.as_ref().map(|s| s.is_connected()).unwrap_or(false);
                // Update cache if state changed
                if !actually_connected {
                    self.cached_connected.store(false, Ordering::Relaxed);
                }
                return actually_connected;
            }
        }
        false
    }

    async fn connect(&self) -> Result<(), SinkError> {
        // For lazy sinks, connect() explicitly triggers connection
        self.ensure_connected().await
    }

    async fn disconnect(&self) -> Result<(), SinkError> {
        let _guard = self.connection_mutex.lock().await;
        
        // Disconnect inner sink if present
        let result = {
            let mut inner = self.inner.write().await;
            if let Some(sink) = inner.take() {
                sink.disconnect().await
            } else {
                Ok(())
            }
        };
        
        // Update state
        {
            let mut state = self.state.write().await;
            state.transition_to(LazyConnectionState::Disconnected);
        }
        
        // Update cached connection state
        self.cached_connected.store(false, Ordering::Relaxed);
        
        result
    }

    fn metadata(&self) -> SinkMetadata {
        SinkMetadata::new(format!("lazy-{}", self.name), "lazy")
    }

    fn extended_metadata(&self) -> ExtendedSinkMetadata {
        ExtendedSinkMetadata {
            metadata: self.metadata(),
            health: self.connection_health(),
            last_successful_send: self.last_successful_send(),
            avg_latency_ns: None, // Could be implemented if needed
            error_rate: None, // Could be calculated from metrics
            active_connections: if self.is_connected_cached() { 1 } else { 0 },
            preferred_connections: 1,
            supports_multiplexing: self.supports_multiplexing(),
        }
    }

    fn connection_health(&self) -> ConnectionHealth {
        if let Ok(state) = self.state.try_read() {
            match state.state {
                LazyConnectionState::Connected => {
                    // Delegate to inner sink if available
                    if let Ok(inner) = self.inner.try_read() {
                        if let Some(sink) = inner.as_ref() {
                            return sink.connection_health();
                        }
                    }
                    ConnectionHealth::Healthy
                }
                LazyConnectionState::Connecting => ConnectionHealth::Degraded,
                LazyConnectionState::Failed => ConnectionHealth::Unhealthy,
                LazyConnectionState::Disconnected => ConnectionHealth::Unknown,
            }
        } else {
            ConnectionHealth::Unknown
        }
    }

    fn last_successful_send(&self) -> Option<SystemTime> {
        if let Ok(inner) = self.inner.try_read() {
            inner.as_ref().and_then(|s| s.last_successful_send())
        } else {
            None
        }
    }

    fn preferred_connection_count(&self) -> usize {
        // Lazy sinks prefer single connections by default
        1
    }

    fn supports_multiplexing(&self) -> bool {
        // Delegate to inner sink once connected, otherwise assume no
        if let Ok(inner) = self.inner.try_read() {
            inner.as_ref().map(|s| s.supports_multiplexing()).unwrap_or(false)
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{CollectorSink, FailingSink};
    use std::sync::atomic::{AtomicU32, Ordering};
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_lazy_connection_on_first_send() {
        let connect_count = Arc::new(AtomicU32::new(0));
        let count_clone = connect_count.clone();
        
        let factory = move || {
            count_clone.fetch_add(1, Ordering::Relaxed);
            async { 
                let sink = CollectorSink::new();
                sink.force_connect();
                Ok(sink) 
            }
        };
        
        let lazy = LazyMessageSink::with_name(factory, LazyConfig::default(), "test-lazy");
        
        // Not connected yet
        assert!(!lazy.is_lazy_connected().await);
        assert_eq!(connect_count.load(Ordering::Relaxed), 0);
        assert_eq!(lazy.connection_state().await, LazyConnectionState::Disconnected);
        
        // First send triggers connection
        let message = Message::new_unchecked(b"test".to_vec());
        lazy.send(message).await.unwrap();
        
        assert!(lazy.is_lazy_connected().await);
        assert_eq!(connect_count.load(Ordering::Relaxed), 1);
        assert_eq!(lazy.connection_state().await, LazyConnectionState::Connected);
        
        // Second send doesn't reconnect
        let message2 = Message::new_unchecked(b"test2".to_vec());
        lazy.send(message2).await.unwrap();
        assert_eq!(connect_count.load(Ordering::Relaxed), 1);
        
        // Check metrics
        let metrics = lazy.lazy_metrics();
        assert_eq!(metrics.connection_attempts.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.successful_connects.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.messages_sent.load(Ordering::Relaxed), 2);
        assert!(metrics.connection_success_rate() > 0.9);
    }
    
    #[tokio::test]
    async fn test_concurrent_connection_attempts() {
        let connect_count = Arc::new(AtomicU32::new(0));
        let count_clone = connect_count.clone();
        
        let factory = move || {
            count_clone.fetch_add(1, Ordering::Relaxed);
            async { 
                // Add small delay to simulate real connection
                sleep(Duration::from_millis(10)).await;
                let sink = CollectorSink::new();
                sink.force_connect();
                Ok(sink) 
            }
        };
        
        let lazy = Arc::new(LazyMessageSink::with_name(
            factory, 
            LazyConfig::default(), 
            "concurrent-test"
        ));
        
        // Spawn multiple concurrent sends
        let mut handles = vec![];
        for i in 0..10 {
            let lazy_clone = lazy.clone();
            handles.push(tokio::spawn(async move {
                let message = Message::new_unchecked(format!("msg{}", i).as_bytes().to_vec());
                lazy_clone.send(message).await
            }));
        }
        
        // Wait for all
        for handle in handles {
            handle.await.unwrap().unwrap();
        }
        
        // Should only connect once despite multiple concurrent attempts
        assert_eq!(connect_count.load(Ordering::Relaxed), 1);
        assert!(lazy.is_lazy_connected().await);
        
        let metrics = lazy.lazy_metrics();
        assert_eq!(metrics.successful_connects.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.messages_sent.load(Ordering::Relaxed), 10);
        assert!(metrics.connection_waits.load(Ordering::Relaxed) >= 8); // 9 threads waited
    }
    
    #[tokio::test]
    async fn test_connection_retry_logic() {
        let attempt_count = Arc::new(AtomicU32::new(0));
        let count_clone = attempt_count.clone();
        
        let factory = move || {
            let current_attempt = count_clone.fetch_add(1, Ordering::Relaxed);
            async move {
                if current_attempt < 2 {
                    // Fail first two attempts
                    Err(SinkError::connection_failed(format!("Attempt {} failed", current_attempt)))
                } else {
                    // Succeed on third attempt
                    let sink = CollectorSink::new();
                    sink.force_connect();
                    Ok(sink)
                }
            }
        };
        
        let config = LazyConfig {
            max_retries: 3,
            retry_delay: Duration::from_millis(10),
            backoff_multiplier: 1.5,
            ..LazyConfig::default()
        };
        
        let lazy = LazyMessageSink::with_name(factory, config, "retry-test");
        
        // Should succeed after retries
        let message = Message::new_unchecked(b"test".to_vec());
        lazy.send(message).await.unwrap();
        
        assert!(lazy.is_lazy_connected().await);
        assert_eq!(attempt_count.load(Ordering::Relaxed), 3);
        
        let metrics = lazy.lazy_metrics();
        assert_eq!(metrics.connection_attempts.load(Ordering::Relaxed), 3);
        assert_eq!(metrics.successful_connects.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.failed_connects.load(Ordering::Relaxed), 0); // Final attempt succeeded
    }
    
    #[tokio::test]
    async fn test_connection_failure_after_max_retries() {
        let factory = move || async {
            Err(SinkError::connection_failed("Always fails"))
        };
        
        let config = LazyConfig {
            max_retries: 2,
            retry_delay: Duration::from_millis(1),
            ..LazyConfig::default()
        };
        
        let lazy: LazyMessageSink<CollectorSink> = LazyMessageSink::with_name(factory, config, "always-fail-test");
        
        // Should fail after exhausting retries
        let message = Message::new_unchecked(b"test".to_vec());
        let result = lazy.send(message).await;
        
        assert!(result.is_err());
        assert!(!lazy.is_lazy_connected().await);
        assert_eq!(lazy.connection_state().await, LazyConnectionState::Failed);
        
        let metrics = lazy.lazy_metrics();
        assert_eq!(metrics.connection_attempts.load(Ordering::Relaxed), 3); // 1 + 2 retries
        assert_eq!(metrics.successful_connects.load(Ordering::Relaxed), 0);
        assert_eq!(metrics.failed_connects.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.messages_failed.load(Ordering::Relaxed), 1);
    }
    
    #[tokio::test]
    async fn test_auto_reconnection() {
        let connect_count = Arc::new(AtomicU32::new(0));
        let count_clone = connect_count.clone();
        
        let factory = move || {
            count_clone.fetch_add(1, Ordering::Relaxed);
            async { 
                let sink = CollectorSink::new();
                sink.force_connect();
                Ok(sink) 
            }
        };
        
        let config = LazyConfig {
            auto_reconnect: true,
            retry_delay: Duration::from_millis(5),
            ..LazyConfig::default()
        };
        
        let lazy = LazyMessageSink::with_name(factory, config, "reconnect-test");
        
        // Initial connection
        let message1 = Message::new_unchecked(b"test1".to_vec());
        lazy.send(message1).await.unwrap();
        assert_eq!(connect_count.load(Ordering::Relaxed), 1);
        
        // Force disconnect to simulate connection loss
        lazy.force_disconnect().await.unwrap();
        assert!(!lazy.is_lazy_connected().await);
        assert_eq!(lazy.connection_state().await, LazyConnectionState::Disconnected);
        
        // Next send should trigger reconnection
        let message2 = Message::new_unchecked(b"test2".to_vec());
        lazy.send(message2).await.unwrap();
        
        assert!(lazy.is_lazy_connected().await);
        assert_eq!(connect_count.load(Ordering::Relaxed), 2); // Reconnected
        
        let metrics = lazy.lazy_metrics();
        assert_eq!(metrics.successful_connects.load(Ordering::Relaxed), 2);
        assert_eq!(metrics.messages_sent.load(Ordering::Relaxed), 2);
    }
    
    #[tokio::test]
    async fn test_connection_timeout() {
        let factory = move || async {
            // Simulate slow connection that times out
            sleep(Duration::from_secs(1)).await;
            let sink = CollectorSink::new();
            sink.force_connect();
            Ok(sink)
        };
        
        let config = LazyConfig {
            connect_timeout: Duration::from_millis(50), // Very short timeout
            max_retries: 1,
            ..LazyConfig::default()
        };
        
        let lazy = LazyMessageSink::with_name(factory, config, "timeout-test");
        
        // Should timeout
        let message = Message::new_unchecked(b"test".to_vec());
        let result = lazy.send(message).await;
        
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("timeout") || error_msg.contains("Timeout"));
    }
    
    #[tokio::test]
    async fn test_batch_operations() {
        let factory = move || async {
            let sink = CollectorSink::new();
            sink.force_connect();
            Ok(sink)
        };
        
        let lazy = LazyMessageSink::with_name(factory, LazyConfig::default(), "batch-test");
        
        let messages = vec![
            Message::new_unchecked(b"msg1".to_vec()),
            Message::new_unchecked(b"msg2".to_vec()),
            Message::new_unchecked(b"msg3".to_vec()),
        ];
        
        // Batch send should trigger lazy connection
        let result = lazy.send_batch(messages).await.unwrap();
        
        assert!(result.is_complete_success());
        assert_eq!(result.succeeded, 3);
        assert!(lazy.is_lazy_connected().await);
    }
    
    #[tokio::test]
    async fn test_explicit_connect() {
        let connect_count = Arc::new(AtomicU32::new(0));
        let count_clone = connect_count.clone();
        
        let factory = move || {
            count_clone.fetch_add(1, Ordering::Relaxed);
            async { 
                let sink = CollectorSink::new();
                sink.force_connect();
                Ok(sink) 
            }
        };
        
        let lazy = LazyMessageSink::with_name(factory, LazyConfig::default(), "explicit-connect-test");
        
        // Explicit connect should establish connection
        lazy.connect().await.unwrap();
        
        assert!(lazy.is_connected());
        assert!(lazy.is_lazy_connected().await);
        assert_eq!(connect_count.load(Ordering::Relaxed), 1);
        
        // Subsequent send shouldn't trigger additional connection
        let message = Message::new_unchecked(b"test".to_vec());
        lazy.send(message).await.unwrap();
        assert_eq!(connect_count.load(Ordering::Relaxed), 1);
    }
    
    #[tokio::test]
    async fn test_disconnect() {
        let factory = move || async {
            let sink = CollectorSink::new();
            sink.force_connect();
            Ok(sink)
        };
        
        let lazy = LazyMessageSink::with_name(factory, LazyConfig::default(), "disconnect-test");
        
        // Connect
        let message = Message::new_unchecked(b"test".to_vec());
        lazy.send(message).await.unwrap();
        assert!(lazy.is_lazy_connected().await);
        
        // Disconnect
        lazy.disconnect().await.unwrap();
        assert!(!lazy.is_connected());
        assert!(!lazy.is_lazy_connected().await);
        assert_eq!(lazy.connection_state().await, LazyConnectionState::Disconnected);
    }
    
    #[tokio::test]
    async fn test_metadata() {
        let factory = move || async {
            let sink = CollectorSink::new();
            sink.force_connect();
            Ok(sink)
        };
        
        let lazy = LazyMessageSink::with_name(factory, LazyConfig::default(), "metadata-test");
        
        let metadata = lazy.metadata();
        assert_eq!(metadata.name, "lazy-metadata-test");
        // Basic metadata checks - supports_batching and supports_priorities removed from SinkMetadata
        assert_eq!(metadata.name, "lazy-test-sink");
        assert_eq!(metadata.sink_type, "lazy");
        
        let ext_metadata = lazy.extended_metadata();
        // Extended metadata checks - capabilities field removed from ExtendedSinkMetadata
        assert!(ext_metadata.supports_multiplexing); // LazyMessageSink supports multiplexing
    }
    
    #[tokio::test]
    async fn test_connection_health() {
        let factory = move || async {
            let sink = CollectorSink::new();
            sink.force_connect();
            Ok(sink)
        };
        
        let lazy = LazyMessageSink::with_name(factory, LazyConfig::default(), "health-test");
        
        // Initially unknown
        assert_eq!(lazy.connection_health(), ConnectionHealth::Unknown);
        
        // After connection, should reflect inner sink health
        let message = Message::new_unchecked(b"test".to_vec());
        lazy.send(message).await.unwrap();
        assert!(matches!(lazy.connection_health(), ConnectionHealth::Healthy));
    }
    
    #[tokio::test]
    async fn test_configuration_variants() {
        let factory = move || async {
            let sink = CollectorSink::new();
            sink.force_connect();
            Ok(sink)
        };
        
        // Test fast recovery config
        let fast_lazy = LazyMessageSink::with_name(factory, LazyConfig::fast_recovery(), "fast-test");
        let message = Message::new_unchecked(b"test".to_vec());
        fast_lazy.send(message).await.unwrap();
        assert!(fast_lazy.is_lazy_connected().await);
        
        // Test conservative config
        let conservative_lazy = LazyMessageSink::with_name(factory, LazyConfig::conservative(), "conservative-test");
        let message2 = Message::new_unchecked(b"test2".to_vec());
        conservative_lazy.send(message2).await.unwrap();
        assert!(conservative_lazy.is_lazy_connected().await);
    }
    
    #[tokio::test]
    async fn test_connection_uptime() {
        let factory = move || async {
            let sink = CollectorSink::new();
            sink.force_connect();
            Ok(sink)
        };
        
        let lazy = LazyMessageSink::with_name(factory, LazyConfig::default(), "uptime-test");
        
        // No uptime before connection
        assert!(lazy.connection_uptime().await.is_none());
        
        // Connect
        let message = Message::new_unchecked(b"test".to_vec());
        lazy.send(message).await.unwrap();
        
        // Small delay to ensure uptime is measurable
        sleep(Duration::from_millis(10)).await;
        
        // Should have uptime after connection
        let uptime = lazy.connection_uptime().await;
        assert!(uptime.is_some());
        assert!(uptime.unwrap() >= Duration::from_millis(10));
    }
    
    #[tokio::test]
    async fn test_metrics_collection() {
        let connect_count = Arc::new(AtomicU32::new(0));
        let count_clone = connect_count.clone();
        
        let factory = move || {
            let current = count_clone.fetch_add(1, Ordering::Relaxed);
            async move {
                if current == 0 {
                    // First attempt fails
                    Err(SinkError::connection_failed("First attempt fails"))
                } else {
                    // Second attempt succeeds
                    let sink = CollectorSink::new();
                    sink.force_connect();
                    Ok(sink)
                }
            }
        };
        
        let config = LazyConfig {
            max_retries: 2,
            retry_delay: Duration::from_millis(5),
            ..LazyConfig::default()
        };
        
        let lazy = LazyMessageSink::with_name(factory, config, "metrics-test");
        
        // Send some messages
        for i in 0..5 {
            let message = Message::new_unchecked(format!("msg{}", i).as_bytes().to_vec());
            lazy.send(message).await.unwrap();
        }
        
        let metrics = lazy.lazy_metrics();
        assert_eq!(metrics.connection_attempts.load(Ordering::Relaxed), 2); // 1 fail + 1 success
        assert_eq!(metrics.successful_connects.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.failed_connects.load(Ordering::Relaxed), 0); // Final connect succeeded
        assert_eq!(metrics.messages_sent.load(Ordering::Relaxed), 5);
        assert_eq!(metrics.messages_failed.load(Ordering::Relaxed), 0);
        
        // Test success rates
        assert!(metrics.connection_success_rate() > 0.4); // At least 1/2 successful
        assert_eq!(metrics.message_success_rate(), 1.0); // All messages sent
    }
}