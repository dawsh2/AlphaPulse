use crate::SinkError;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::SystemTime;

/// Connection pool management for MessageSink implementations
#[derive(Debug, Clone)]
pub struct ConnectionPool {
    /// Target node identifier
    pub target_node: String,
    /// Current active connections
    active_connections: Arc<AtomicUsize>,
    /// Maximum allowed connections
    max_connections: usize,
    /// Preferred connection count for optimal performance
    preferred_connections: usize,
    /// Whether this endpoint supports connection multiplexing
    supports_multiplexing: bool,
    /// Last connection attempt timestamp
    last_attempt: Arc<std::sync::Mutex<Option<SystemTime>>>,
}

impl ConnectionPool {
    /// Create a new connection pool
    pub fn new(
        target_node: impl Into<String>,
        max_connections: usize,
        preferred_connections: usize,
        supports_multiplexing: bool,
    ) -> Self {
        Self {
            target_node: target_node.into(),
            active_connections: Arc::new(AtomicUsize::new(0)),
            max_connections,
            preferred_connections,
            supports_multiplexing,
            last_attempt: Arc::new(std::sync::Mutex::new(None)),
        }
    }

    /// Get current active connection count
    pub fn active_connections(&self) -> usize {
        self.active_connections.load(Ordering::Relaxed)
    }

    /// Get maximum connection count
    pub fn max_connections(&self) -> usize {
        self.max_connections
    }

    /// Get preferred connection count
    pub fn preferred_connections(&self) -> usize {
        self.preferred_connections
    }

    /// Check if multiplexing is supported
    pub fn supports_multiplexing(&self) -> bool {
        self.supports_multiplexing
    }

    /// Attempt to acquire a connection slot
    pub fn try_acquire_connection(&self) -> Result<ConnectionGuard, SinkError> {
        let current = self.active_connections.load(Ordering::Relaxed);
        
        if current >= self.max_connections {
            return Err(SinkError::connection_failed(format!(
                "Connection pool exhausted: {}/{} connections active for node {}",
                current, self.max_connections, self.target_node
            )));
        }

        // Optimistically increment
        let new_count = self.active_connections.fetch_add(1, Ordering::Relaxed) + 1;
        
        if new_count > self.max_connections {
            // Rollback and fail
            self.active_connections.fetch_sub(1, Ordering::Relaxed);
            return Err(SinkError::connection_failed(format!(
                "Connection pool exhausted: {}/{} connections active for node {}",
                new_count - 1, self.max_connections, self.target_node
            )));
        }

        *self.last_attempt.lock().unwrap() = Some(SystemTime::now());
        
        Ok(ConnectionGuard {
            pool: self.clone(),
        })
    }

    /// Check if additional connections are needed
    pub fn needs_more_connections(&self) -> bool {
        self.active_connections.load(Ordering::Relaxed) < self.preferred_connections
    }

    /// Check if connection count is optimal
    pub fn is_optimal(&self) -> bool {
        let current = self.active_connections.load(Ordering::Relaxed);
        current >= self.preferred_connections && current <= self.max_connections
    }

    /// Get connection pool statistics
    pub fn pool_stats(&self) -> ConnectionPoolStats {
        ConnectionPoolStats {
            target_node: self.target_node.clone(),
            active_connections: self.active_connections(),
            max_connections: self.max_connections,
            preferred_connections: self.preferred_connections,
            supports_multiplexing: self.supports_multiplexing,
            utilization: self.active_connections() as f64 / self.max_connections as f64,
            last_attempt: *self.last_attempt.lock().unwrap(),
        }
    }
}

/// RAII guard that automatically releases connection when dropped
#[derive(Debug)]
pub struct ConnectionGuard {
    pool: ConnectionPool,
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        self.pool.active_connections.fetch_sub(1, Ordering::Relaxed);
    }
}

impl ConnectionGuard {
    /// Get the target node for this connection
    pub fn target_node(&self) -> &str {
        &self.pool.target_node
    }

    /// Check if multiplexing is supported on this connection
    pub fn supports_multiplexing(&self) -> bool {
        self.pool.supports_multiplexing
    }
}

/// Statistics for a connection pool
#[derive(Debug, Clone)]
pub struct ConnectionPoolStats {
    /// Target node identifier
    pub target_node: String,
    /// Current active connections
    pub active_connections: usize,
    /// Maximum allowed connections
    pub max_connections: usize,
    /// Preferred connection count
    pub preferred_connections: usize,
    /// Whether multiplexing is supported
    pub supports_multiplexing: bool,
    /// Pool utilization (0.0 to 1.0)
    pub utilization: f64,
    /// Last connection attempt timestamp
    pub last_attempt: Option<SystemTime>,
}

impl ConnectionPoolStats {
    /// Check if pool is under-utilized
    pub fn is_under_utilized(&self) -> bool {
        self.utilization < 0.5
    }

    /// Check if pool is over-utilized
    pub fn is_over_utilized(&self) -> bool {
        self.utilization > 0.9
    }

    /// Check if pool needs scaling
    pub fn needs_scaling(&self) -> bool {
        self.active_connections < self.preferred_connections
    }
}

/// Connection manager for MessageSink implementations
pub trait ConnectionManager: Send + Sync {
    /// Get or create a connection pool for the target
    fn get_pool(&self, target: &str) -> Result<Arc<ConnectionPool>, SinkError>;
    
    /// Create a new connection pool
    fn create_pool(
        &self, 
        target: &str, 
        max_connections: usize,
        preferred_connections: usize,
        supports_multiplexing: bool
    ) -> Result<Arc<ConnectionPool>, SinkError>;
    
    /// Remove a connection pool
    fn remove_pool(&self, target: &str) -> Result<(), SinkError>;
    
    /// Get statistics for all pools
    fn pool_statistics(&self) -> Vec<ConnectionPoolStats>;
    
    /// Optimize connection pools based on usage patterns
    fn optimize_pools(&self) -> Result<(), SinkError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_pool_basic() {
        let pool = ConnectionPool::new("test-node", 5, 3, false);
        
        assert_eq!(pool.active_connections(), 0);
        assert_eq!(pool.max_connections(), 5);
        assert_eq!(pool.preferred_connections(), 3);
        assert!(!pool.supports_multiplexing());
        assert!(pool.needs_more_connections());
        assert!(!pool.is_optimal());
    }

    #[test]
    fn test_connection_acquisition() {
        let pool = ConnectionPool::new("test-node", 2, 1, false);
        
        // Should acquire successfully
        let _guard1 = pool.try_acquire_connection().unwrap();
        assert_eq!(pool.active_connections(), 1);
        assert!(pool.is_optimal());
        
        // Should acquire second connection
        let _guard2 = pool.try_acquire_connection().unwrap();
        assert_eq!(pool.active_connections(), 2);
        
        // Should fail to acquire third connection
        let result = pool.try_acquire_connection();
        assert!(result.is_err());
        assert_eq!(pool.active_connections(), 2);
    }

    #[test]
    fn test_connection_release() {
        let pool = ConnectionPool::new("test-node", 5, 3, false);
        
        {
            let _guard = pool.try_acquire_connection().unwrap();
            assert_eq!(pool.active_connections(), 1);
        } // Guard drops here
        
        assert_eq!(pool.active_connections(), 0);
    }

    #[test]
    fn test_pool_stats() {
        let pool = ConnectionPool::new("test-node", 10, 5, true);
        let _guard1 = pool.try_acquire_connection().unwrap();
        let _guard2 = pool.try_acquire_connection().unwrap();
        
        let stats = pool.pool_stats();
        assert_eq!(stats.target_node, "test-node");
        assert_eq!(stats.active_connections, 2);
        assert_eq!(stats.max_connections, 10);
        assert_eq!(stats.preferred_connections, 5);
        assert!(stats.supports_multiplexing);
        assert_eq!(stats.utilization, 0.2);
        assert!(stats.is_under_utilized());
        assert!(!stats.is_over_utilized());
        assert!(stats.needs_scaling());
    }
}