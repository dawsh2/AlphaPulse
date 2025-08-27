//! Connection Pool for Lazy MessageSinks
//!
//! Provides efficient connection pooling for LazyMessageSink instances,
//! enabling resource reuse and load balancing across multiple connections.
//!
//! ## Features
//!
//! - **Lazy Pool Management**: Connections are created on-demand
//! - **Load Balancing**: Round-robin distribution across healthy connections
//! - **Health Monitoring**: Automatic detection and removal of unhealthy connections
//! - **Resource Limits**: Configurable min/max pool sizes with idle timeouts
//! - **Connection Reuse**: RAII guards ensure connections are returned to pool

use crate::{BatchResult, ConnectionHealth, ExtendedSinkMetadata, LazyMessageSink, LazyConfig, Message, MessageSink, SinkError, SinkMetadata};
use async_trait::async_trait;
use std::collections::VecDeque;
use std::fmt::Debug;
use std::future::Future;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::{Mutex, RwLock};
use tokio::time::interval;

/// Configuration for lazy connection pooling
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// Maximum connections in pool
    pub max_size: usize,
    
    /// Minimum idle connections to maintain
    pub min_idle: usize,
    
    /// Connection idle timeout before cleanup
    pub idle_timeout: Duration,
    
    /// How often to run pool maintenance
    pub maintenance_interval: Duration,
    
    /// Lazy connection configuration for each connection
    pub lazy_config: LazyConfig,
    
    /// Enable connection health monitoring
    pub health_monitoring: bool,
    
    /// Timeout for acquiring connection from pool
    pub acquire_timeout: Duration,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_size: 10,
            min_idle: 2,
            idle_timeout: Duration::from_secs(300), // 5 minutes
            maintenance_interval: Duration::from_secs(60), // 1 minute
            lazy_config: LazyConfig::default(),
            health_monitoring: true,
            acquire_timeout: Duration::from_secs(5),
        }
    }
}

impl PoolConfig {
    /// Create high-throughput configuration
    pub fn high_throughput() -> Self {
        Self {
            max_size: 20,
            min_idle: 5,
            idle_timeout: Duration::from_secs(600),
            maintenance_interval: Duration::from_secs(30),
            lazy_config: LazyConfig::fast_recovery(),
            health_monitoring: true,
            acquire_timeout: Duration::from_secs(2),
        }
    }

    /// Create conservative configuration
    pub fn conservative() -> Self {
        Self {
            max_size: 5,
            min_idle: 1,
            idle_timeout: Duration::from_secs(900),
            maintenance_interval: Duration::from_secs(120),
            lazy_config: LazyConfig::conservative(),
            health_monitoring: true,
            acquire_timeout: Duration::from_secs(10),
        }
    }
}

/// Statistics for pool monitoring
#[derive(Debug, Clone, Default)]
pub struct PoolStats {
    /// Current number of connections in pool
    pub pool_size: usize,
    /// Number of active (checked out) connections
    pub active_connections: usize,
    /// Number of idle connections available
    pub idle_connections: usize,
    /// Total connections created
    pub total_created: usize,
    /// Total connections destroyed
    pub total_destroyed: usize,
    /// Current wait queue size
    pub wait_queue_size: usize,
    /// Average connection acquire time (nanoseconds)
    pub avg_acquire_time_ns: u64,
    /// Pool hit rate (successful immediate acquisitions)
    pub hit_rate: f64,
}

/// Connection wrapper with metadata
#[derive(Debug)]
struct PooledConnection<S: MessageSink> {
    sink: LazyMessageSink<S>,
    created_at: Instant,
    last_used: Instant,
    checkout_count: usize,
    health_check_failures: usize,
}

impl<S: MessageSink> PooledConnection<S> {
    fn new(sink: LazyMessageSink<S>) -> Self {
        let now = Instant::now();
        Self {
            sink,
            created_at: now,
            last_used: now,
            checkout_count: 0,
            health_check_failures: 0,
        }
    }

    fn is_idle_expired(&self, timeout: Duration) -> bool {
        self.last_used.elapsed() > timeout
    }

    fn should_remove(&self, idle_timeout: Duration, max_failures: usize) -> bool {
        self.is_idle_expired(idle_timeout) || self.health_check_failures > max_failures
    }

    async fn is_healthy(&self) -> bool {
        matches!(self.sink.connection_health(), ConnectionHealth::Healthy | ConnectionHealth::Degraded)
    }

    fn checkout(&mut self) {
        self.last_used = Instant::now();
        self.checkout_count += 1;
    }

    fn checkin(&mut self) {
        self.last_used = Instant::now();
    }
}

/// RAII guard that returns connection to pool when dropped
pub struct PooledSinkGuard<S: MessageSink + 'static> {
    sink: Option<LazyMessageSink<S>>,
    pool: Arc<LazyConnectionPool<S>>,
}

impl<S: MessageSink> PooledSinkGuard<S> {
    fn new(sink: LazyMessageSink<S>, pool: Arc<LazyConnectionPool<S>>) -> Self {
        Self {
            sink: Some(sink),
            pool,
        }
    }

    /// Get reference to the underlying sink
    pub fn sink(&self) -> &LazyMessageSink<S> {
        self.sink.as_ref().expect("Guard used after drop")
    }

    /// Consume guard and return owned sink (removes from pool)
    pub fn into_sink(mut self) -> LazyMessageSink<S> {
        self.sink.take().expect("Guard already consumed")
    }
}

impl<S: MessageSink + 'static> Drop for PooledSinkGuard<S> {
    fn drop(&mut self) {
        if let Some(sink) = self.sink.take() {
            let pool = self.pool.clone();
            tokio::spawn(async move {
                pool.return_connection(sink).await;
            });
        }
    }
}

impl<S: MessageSink> std::ops::Deref for PooledSinkGuard<S> {
    type Target = LazyMessageSink<S>;

    fn deref(&self) -> &Self::Target {
        self.sink()
    }
}

impl<S: MessageSink> std::ops::DerefMut for PooledSinkGuard<S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.sink.as_mut().expect("Guard used after drop")
    }
}

/// Pool of lazy connections for efficient resource usage
pub struct LazyConnectionPool<S: MessageSink> {
    /// Available connections
    pool: Arc<RwLock<VecDeque<PooledConnection<S>>>>,
    
    /// Factory for creating new connections
    factory: Arc<dyn Fn() -> std::pin::Pin<Box<dyn Future<Output = Result<S, SinkError>> + Send>> + Send + Sync>,
    
    /// Pool configuration
    config: PoolConfig,
    
    /// Current pool size (including checked out connections)
    current_size: AtomicUsize,
    
    /// Number of connections currently checked out
    active_count: AtomicUsize,
    
    /// Statistics for monitoring
    stats: Arc<RwLock<PoolStats>>,
    
    /// Pool name for debugging
    name: String,
    
    /// Shutdown flag
    shutdown: Arc<RwLock<bool>>,
}

impl<S: MessageSink> Debug for LazyConnectionPool<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LazyConnectionPool")
            .field("name", &self.name)
            .field("current_size", &self.current_size.load(Ordering::Relaxed))
            .field("active_count", &self.active_count.load(Ordering::Relaxed))
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

impl<S: MessageSink + 'static> LazyConnectionPool<S> {
    /// Create new connection pool
    pub fn new<F, Fut>(factory: F, config: PoolConfig) -> Arc<Self>
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<S, SinkError>> + Send + 'static,
    {
        let pool = Arc::new(Self {
            pool: Arc::new(RwLock::new(VecDeque::new())),
            factory: Arc::new(move || Box::pin(factory())),
            config,
            current_size: AtomicUsize::new(0),
            active_count: AtomicUsize::new(0),
            stats: Arc::new(RwLock::new(PoolStats::default())),
            name: "lazy-pool".to_string(),
            shutdown: Arc::new(RwLock::new(false)),
        });

        // Start background maintenance task
        let pool_clone = pool.clone();
        tokio::spawn(async move {
            pool_clone.maintenance_loop().await;
        });

        pool
    }

    /// Create new connection pool with name
    pub fn with_name<F, Fut>(factory: F, config: PoolConfig, name: impl Into<String>) -> Arc<Self>
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<S, SinkError>> + Send + 'static,
    {
        let name_str = name.into();
        let pool = Arc::new(Self {
            pool: Arc::new(RwLock::new(VecDeque::new())),
            factory: Arc::new(move || Box::pin(factory())),
            config,
            current_size: AtomicUsize::new(0),
            active_count: AtomicUsize::new(0),
            stats: Arc::new(RwLock::new(PoolStats::default())),
            name: name_str,
            shutdown: Arc::new(RwLock::new(false)),
        });

        // Start background maintenance task
        let pool_clone = pool.clone();
        tokio::spawn(async move {
            pool_clone.maintenance_loop().await;
        });

        pool
    }

    /// Acquire a connection from the pool
    pub async fn acquire(self: &Arc<Self>) -> Result<PooledSinkGuard<S>, SinkError> {
        let start_time = std::time::Instant::now();
        
        // Try to get existing healthy connection first
        if let Some(mut connection) = self.try_get_existing_connection().await {
            connection.checkout();
            let sink = connection.sink;
            self.active_count.fetch_add(1, Ordering::Relaxed);
            
            let acquire_time = start_time.elapsed();
            self.update_acquire_stats(acquire_time, true).await; // Cache hit
            
            return Ok(PooledSinkGuard::new(sink, self.clone()));
        }
        
        // Try to create new connection if under limit
        if self.current_size.load(Ordering::Relaxed) < self.config.max_size {
            match self.create_connection().await {
                Ok(sink) => {
                    self.active_count.fetch_add(1, Ordering::Relaxed);
                    
                    let acquire_time = start_time.elapsed();
                    self.update_acquire_stats(acquire_time, false).await; // Cache miss
                    
                    return Ok(PooledSinkGuard::new(sink, self.clone()));
                },
                Err(e) => {
                    tracing::warn!("Failed to create new connection in pool '{}': {}", self.name, e);
                    // Fall through to wait for available connection
                }
            }
        }
        
        // Wait for connection to become available
        tokio::time::timeout(self.config.acquire_timeout, self.wait_for_available_connection()).await
            .map_err(|_| SinkError::connection_failed(format!("Timeout acquiring connection from pool '{}'", self.name)))?
    }

    /// Get current pool statistics
    pub async fn stats(&self) -> PoolStats {
        let mut stats = self.stats.read().await.clone();
        stats.pool_size = self.current_size.load(Ordering::Relaxed);
        stats.active_connections = self.active_count.load(Ordering::Relaxed);
        stats.idle_connections = stats.pool_size.saturating_sub(stats.active_connections);
        stats
    }

    /// Shutdown the pool and close all connections
    pub async fn shutdown(&self) -> Result<(), SinkError> {
        // Set shutdown flag
        *self.shutdown.write().await = true;

        // Close all connections
        let mut pool = self.pool.write().await;
        let mut errors = Vec::new();
        
        while let Some(connection) = pool.pop_front() {
            if let Err(e) = connection.sink.disconnect().await {
                errors.push(e);
            }
        }

        // Update stats
        self.current_size.store(0, Ordering::Relaxed);
        self.active_count.store(0, Ordering::Relaxed);

        if errors.is_empty() {
            Ok(())
        } else {
            Err(SinkError::connection_failed(format!("Errors during shutdown: {:?}", errors)))
        }
    }

    /// Try to get an existing healthy connection from the pool
    async fn try_get_existing_connection(&self) -> Option<PooledConnection<S>> {
        let mut pool = self.pool.write().await;
        
        // Look for healthy connections
        while let Some(mut connection) = pool.pop_front() {
            if self.config.health_monitoring && !connection.is_healthy().await {
                connection.health_check_failures += 1;
                if connection.should_remove(self.config.idle_timeout, 3) {
                    // Remove unhealthy connection
                    self.current_size.fetch_sub(1, Ordering::Relaxed);
                    tokio::spawn(async move {
                        let _ = connection.sink.disconnect().await;
                    });
                    continue;
                }
                // Put back for retry later
                pool.push_back(connection);
                continue;
            }
            
            return Some(connection);
        }
        
        None
    }

    /// Create a new connection
    async fn create_connection(&self) -> Result<LazyMessageSink<S>, SinkError> {
        let inner_factory = self.factory.clone();
        let lazy_config = self.config.lazy_config.clone();
        let name = format!("{}-connection-{}", self.name, self.current_size.load(Ordering::Relaxed));
        
        let factory = move || {
            let inner = inner_factory.clone();
            async move { inner().await }
        };
        
        let lazy_sink = LazyMessageSink::with_name(factory, lazy_config, name);
        
        self.current_size.fetch_add(1, Ordering::Relaxed);
        
        let mut stats = self.stats.write().await;
        stats.total_created += 1;
        
        Ok(lazy_sink)
    }

    /// Wait for a connection to become available (internal helper)
    async fn wait_for_available_connection(self: &Arc<Self>) -> Result<PooledSinkGuard<S>, SinkError> {
        let mut interval = tokio::time::interval(Duration::from_millis(100));
        
        loop {
            interval.tick().await;
            
            if let Some(mut connection) = self.try_get_existing_connection().await {
                connection.checkout();
                let sink = connection.sink;
                self.active_count.fetch_add(1, Ordering::Relaxed);
                return Ok(PooledSinkGuard::new(sink, self.clone()));
            }
            
            // Try to create new connection if possible
            if self.current_size.load(Ordering::Relaxed) < self.config.max_size {
                match self.create_connection().await {
                    Ok(sink) => {
                        self.active_count.fetch_add(1, Ordering::Relaxed);
                        return Ok(PooledSinkGuard::new(sink, self.clone()));
                    }
                    Err(_) => continue,
                }
            }
        }
    }

    /// Return a connection to the pool
    async fn return_connection(&self, sink: LazyMessageSink<S>) {
        // Don't return connections if shutting down
        if *self.shutdown.read().await {
            let _ = sink.disconnect().await;
            self.current_size.fetch_sub(1, Ordering::Relaxed);
            self.active_count.fetch_sub(1, Ordering::Relaxed);
            return;
        }

        let mut connection = PooledConnection::new(sink);
        connection.checkin();
        
        let mut pool = self.pool.write().await;
        pool.push_back(connection);
        
        self.active_count.fetch_sub(1, Ordering::Relaxed);
    }

    /// Update acquire statistics with bounded memory usage
    async fn update_acquire_stats(&self, acquire_time: Duration, was_hit: bool) {
        let mut stats = self.stats.write().await;
        
        // Update average acquire time (exponential moving average for bounded memory)
        let acquire_ns = acquire_time.as_nanos() as u64;
        stats.avg_acquire_time_ns = if stats.avg_acquire_time_ns == 0 {
            acquire_ns
        } else {
            // Use exponential moving average with alpha = 0.1 to prevent runaway values
            let alpha = 0.1f64;
            let old_avg = stats.avg_acquire_time_ns as f64;
            let new_sample = acquire_ns as f64;
            (old_avg * (1.0 - alpha) + new_sample * alpha) as u64
        };
        
        // Update hit rate with exponential decay to prevent unbounded accumulation
        let alpha = 0.1f64; // Weight for new samples
        if was_hit {
            stats.hit_rate = stats.hit_rate * (1.0 - alpha) + alpha;
        } else {
            stats.hit_rate = stats.hit_rate * (1.0 - alpha);
        }
        
        // Bound hit rate to [0.0, 1.0] to prevent numerical drift
        stats.hit_rate = stats.hit_rate.clamp(0.0, 1.0);
    }

    /// Background maintenance loop
    async fn maintenance_loop(&self) {
        let mut interval = interval(self.config.maintenance_interval);
        
        while !*self.shutdown.read().await {
            interval.tick().await;
            
            if let Err(e) = self.perform_maintenance().await {
                tracing::warn!("Pool '{}' maintenance error: {}", self.name, e);
            }
        }
    }

    /// Perform pool maintenance
    async fn perform_maintenance(&self) -> Result<(), SinkError> {
        let mut pool = self.pool.write().await;
        let mut to_remove = Vec::new();
        let mut healthy_count = 0;
        
        // Check connections for removal
        for (index, connection) in pool.iter_mut().enumerate() {
            if connection.should_remove(self.config.idle_timeout, 3) {
                to_remove.push(index);
            } else {
                if self.config.health_monitoring && !connection.is_healthy().await {
                    connection.health_check_failures += 1;
                } else {
                    connection.health_check_failures = 0;
                    healthy_count += 1;
                }
            }
        }
        
        // Remove unhealthy/expired connections
        for &index in to_remove.iter().rev() {
            if let Some(connection) = pool.remove(index) {
                self.current_size.fetch_sub(1, Ordering::Relaxed);
                tokio::spawn(async move {
                    let _ = connection.sink.disconnect().await;
                });
                
                let mut stats = self.stats.write().await;
                stats.total_destroyed += 1;
            }
        }
        
        // Ensure minimum idle connections
        let needed = self.config.min_idle.saturating_sub(healthy_count);
        for _ in 0..needed {
            if self.current_size.load(Ordering::Relaxed) < self.config.max_size {
                match self.create_connection().await {
                    Ok(sink) => {
                        let connection = PooledConnection::new(sink);
                        pool.push_back(connection);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to create minimum idle connection: {}", e);
                        break;
                    }
                }
            }
        }
        
        // Perform memory cleanup for bounded statistics
        // Reset counters periodically to prevent overflow
        let mut stats = self.stats.write().await;
        if stats.total_created > 1_000_000 {
            tracing::debug!("Pool '{}' resetting statistics to prevent overflow", self.name);
            // Keep ratios, reset absolute counters
            let hit_rate = stats.hit_rate;
            let avg_acquire_time = stats.avg_acquire_time_ns;
            *stats = PoolStats::default();
            stats.hit_rate = hit_rate;
            stats.avg_acquire_time_ns = avg_acquire_time;
        }
        
        Ok(())
    }
}

#[async_trait]
impl<S: MessageSink + 'static> MessageSink for Arc<LazyConnectionPool<S>> {
    async fn send(&self, message: Message) -> Result<(), SinkError> {
        let guard = self.acquire().await?;
        guard.send(message).await
    }

    async fn send_batch(&self, messages: Vec<Message>) -> Result<BatchResult, SinkError> {
        let guard = self.acquire().await?;
        guard.send_batch(messages).await
    }

    async fn send_batch_prioritized(&self, messages: Vec<Message>) -> Result<BatchResult, SinkError> {
        let guard = self.acquire().await?;
        guard.send_batch_prioritized(messages).await
    }

    fn is_connected(&self) -> bool {
        self.current_size.load(Ordering::Relaxed) > 0
    }

    async fn connect(&self) -> Result<(), SinkError> {
        // For pools, we pre-create minimum connections
        let needed = self.config.min_idle;
        for _ in 0..needed {
            if self.current_size.load(Ordering::Relaxed) < self.config.max_size {
                let sink = self.create_connection().await?;
                let connection = PooledConnection::new(sink);
                let mut pool = self.pool.write().await;
                pool.push_back(connection);
            }
        }
        Ok(())
    }

    async fn disconnect(&self) -> Result<(), SinkError> {
        self.shutdown().await
    }

    fn metadata(&self) -> SinkMetadata {
        SinkMetadata::new(format!("pool-{}", self.name), "pool")
    }

    fn extended_metadata(&self) -> ExtendedSinkMetadata {
        let stats = tokio::task::block_in_place(|| {
            let handle = tokio::runtime::Handle::current();
            handle.block_on(self.stats())
        });

        ExtendedSinkMetadata {
            metadata: self.metadata(),
            health: self.connection_health(),
            last_successful_send: None, // Pools don't track individual send times
            avg_latency_ns: Some(stats.avg_acquire_time_ns),
            error_rate: Some(1.0 - stats.hit_rate), // Convert hit rate to error rate
            active_connections: stats.active_connections,
            preferred_connections: self.config.max_size,
            supports_multiplexing: true,
        }
    }

    fn connection_health(&self) -> ConnectionHealth {
        let size = self.current_size.load(Ordering::Relaxed);
        let active = self.active_count.load(Ordering::Relaxed);
        
        if size == 0 {
            ConnectionHealth::Unhealthy
        } else if size < self.config.min_idle {
            ConnectionHealth::Degraded
        } else if active as f64 / size as f64 > 0.8 {
            ConnectionHealth::Degraded
        } else {
            ConnectionHealth::Healthy
        }
    }

    fn last_successful_send(&self) -> Option<SystemTime> {
        // Would need to track this across all connections
        None
    }

    fn preferred_connection_count(&self) -> usize {
        self.config.max_size
    }

    fn supports_multiplexing(&self) -> bool {
        true // Pools inherently support multiplexing
    }
}