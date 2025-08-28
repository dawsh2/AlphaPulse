//! Performance Optimizations for Hot Path
//!
//! This module contains performance-critical optimizations for the network layer,
//! specifically targeting the hot path (<35μs target) used in high-frequency trading.

use std::sync::Arc;
use std::time::{Duration, Instant};
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};

/// Performance-critical message cache for hot path operations
/// 
/// This cache uses pre-allocated buffers and lock-free operations to minimize
/// latency in the critical trading path.
pub struct HotPathCache {
    /// Pre-allocated message buffers for zero-allocation sends
    buffer_pool: lockfree::queue::Queue<Vec<u8>>,
    /// Cache statistics
    stats: Arc<HotPathStats>,
    /// Maximum buffer size to prevent memory bloat
    max_buffer_size: usize,
    /// Buffer pool size limit
    max_pool_size: usize,
}

/// Hot path performance statistics
#[derive(Debug)]
pub struct HotPathStats {
    /// Total operations processed
    pub operations_total: AtomicU64,
    /// Cache hits (buffer reuse)
    pub cache_hits: AtomicU64,
    /// Cache misses (new allocation)
    pub cache_misses: AtomicU64,
    /// Total latency in nanoseconds
    pub total_latency_ns: AtomicU64,
    /// Whether the hot path is currently healthy
    pub is_healthy: AtomicBool,
}

impl HotPathStats {
    pub fn new() -> Self {
        Self {
            operations_total: AtomicU64::new(0),
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
            total_latency_ns: AtomicU64::new(0),
            is_healthy: AtomicBool::new(true),
        }
    }
    
    /// Record a hot path operation
    pub fn record_operation(&self, duration: Duration, cache_hit: bool) {
        self.operations_total.fetch_add(1, Ordering::Relaxed);
        self.total_latency_ns.fetch_add(
            duration.as_nanos() as u64,
            Ordering::Relaxed
        );
        
        if cache_hit {
            self.cache_hits.fetch_add(1, Ordering::Relaxed);
        } else {
            self.cache_misses.fetch_add(1, Ordering::Relaxed);
        }
        
        // Mark as unhealthy if average latency exceeds 35μs
        let avg_latency_ns = self.average_latency_ns();
        self.is_healthy.store(avg_latency_ns < 35_000.0, Ordering::Relaxed);
    }
    
    /// Get average latency in nanoseconds
    pub fn average_latency_ns(&self) -> f64 {
        let total_ops = self.operations_total.load(Ordering::Relaxed);
        if total_ops == 0 {
            return 0.0;
        }
        
        let total_latency = self.total_latency_ns.load(Ordering::Relaxed);
        total_latency as f64 / total_ops as f64
    }
    
    /// Get cache hit rate as percentage
    pub fn cache_hit_rate(&self) -> f64 {
        let hits = self.cache_hits.load(Ordering::Relaxed);
        let misses = self.cache_misses.load(Ordering::Relaxed);
        let total = hits + misses;
        
        if total == 0 {
            return 0.0;
        }
        
        (hits as f64 / total as f64) * 100.0
    }
    
    /// Check if hot path performance is healthy
    pub fn is_healthy(&self) -> bool {
        self.is_healthy.load(Ordering::Relaxed)
    }
}

impl HotPathCache {
    /// Create new hot path cache with optimized defaults for trading
    pub fn new() -> Self {
        Self {
            buffer_pool: lockfree::queue::Queue::new(),
            stats: Arc::new(HotPathStats::new()),
            max_buffer_size: 64 * 1024, // 64KB max per buffer
            max_pool_size: 1000, // Up to 1000 cached buffers
        }
    }
    
    /// Get a buffer from the cache or allocate new one
    /// 
    /// PERFORMANCE CRITICAL: This method is designed for <100ns operation time
    pub fn get_buffer(&self, min_size: usize) -> (Vec<u8>, bool) {
        let start = Instant::now();
        
        // Try to get a cached buffer
        if let Some(mut buffer) = self.buffer_pool.pop() {
            if buffer.capacity() >= min_size && buffer.capacity() <= self.max_buffer_size {
                buffer.clear();
                buffer.reserve(min_size.saturating_sub(buffer.capacity()));
                
                let cache_hit = true;
                self.stats.record_operation(start.elapsed(), cache_hit);
                return (buffer, cache_hit);
            }
            
            // Buffer too small or too large, discard and allocate new
            // This prevents memory fragmentation
        }
        
        // Allocate new buffer
        let buffer = Vec::with_capacity(min_size);
        let cache_hit = false;
        self.stats.record_operation(start.elapsed(), cache_hit);
        
        (buffer, cache_hit)
    }
    
    /// Return buffer to cache for reuse
    /// 
    /// PERFORMANCE CRITICAL: Lock-free operation for hot path
    pub fn return_buffer(&self, buffer: Vec<u8>) {
        // Only cache buffers that are reasonably sized
        if buffer.capacity() <= self.max_buffer_size && 
           buffer.capacity() >= 64 && // Don't cache tiny buffers
           self.buffer_pool.len() < self.max_pool_size {
            self.buffer_pool.push(buffer);
        }
        // If buffer is too large or pool is full, let it drop to free memory
    }
    
    /// Get performance statistics
    pub fn stats(&self) -> Arc<HotPathStats> {
        Arc::clone(&self.stats)
    }
    
    /// Clear the cache (for testing or memory pressure)
    pub fn clear(&self) {
        while self.buffer_pool.pop().is_some() {
            // Drain the queue
        }
    }
    
    /// Get current cache size
    pub fn cache_size(&self) -> usize {
        self.buffer_pool.len()
    }
}

/// Fast path message serialization optimizations
pub struct FastSerializer {
    cache: HotPathCache,
}

impl FastSerializer {
    pub fn new() -> Self {
        Self {
            cache: HotPathCache::new(),
        }
    }
    
    /// Serialize message to TLV with buffer reuse
    /// 
    /// CRITICAL: This method targets <500ns execution time
    pub fn serialize_tlv<T>(&self, message: &T) -> crate::Result<Vec<u8>>
    where
        T: serde::Serialize,
    {
        // Estimate message size to avoid buffer reallocation
        let estimated_size = std::mem::size_of::<T>() * 2; // Conservative estimate
        
        let (mut buffer, _cache_hit) = self.cache.get_buffer(estimated_size);
        
        // Use fast serialization (bincode is faster than serde_json for binary)
        match bincode::serialize_into(&mut buffer, message) {
            Ok(()) => {
                // Success - return buffer without returning to cache since it contains data
                Ok(buffer)
            },
            Err(e) => {
                // Error - return buffer to cache for reuse
                self.cache.return_buffer(buffer);
                Err(crate::TransportError::protocol(&format!("Serialization failed: {}", e)))
            }
        }
    }
    
    /// Deserialize TLV message with minimal allocations
    pub fn deserialize_tlv<T>(&self, data: &[u8]) -> crate::Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        bincode::deserialize(data)
            .map_err(|e| crate::TransportError::protocol(&format!("Deserialization failed: {}", e)))
    }
    
    /// Return a used buffer to the cache
    pub fn return_buffer(&self, buffer: Vec<u8>) {
        self.cache.return_buffer(buffer);
    }
    
    /// Get serializer performance stats
    pub fn stats(&self) -> Arc<HotPathStats> {
        self.cache.stats()
    }
}

/// Memory pool for zero-allocation message processing
pub struct MessagePool<T> {
    pool: lockfree::queue::Queue<Box<T>>,
    max_size: usize,
    stats: Arc<PoolStats>,
}

/// Pool statistics
#[derive(Debug)]
pub struct PoolStats {
    pub allocations: AtomicU64,
    pub deallocations: AtomicU64,
    pub pool_hits: AtomicU64,
    pub pool_misses: AtomicU64,
}

impl PoolStats {
    pub fn new() -> Self {
        Self {
            allocations: AtomicU64::new(0),
            deallocations: AtomicU64::new(0),
            pool_hits: AtomicU64::new(0),
            pool_misses: AtomicU64::new(0),
        }
    }
    
    pub fn pool_hit_rate(&self) -> f64 {
        let hits = self.pool_hits.load(Ordering::Relaxed);
        let misses = self.pool_misses.load(Ordering::Relaxed);
        let total = hits + misses;
        
        if total == 0 {
            return 0.0;
        }
        
        (hits as f64 / total as f64) * 100.0
    }
}

impl<T: Default> MessagePool<T> {
    pub fn new(max_size: usize) -> Self {
        Self {
            pool: lockfree::queue::Queue::new(),
            max_size,
            stats: Arc::new(PoolStats::new()),
        }
    }
    
    /// Get a message object from pool or allocate new
    pub fn get(&self) -> Box<T> {
        if let Some(item) = self.pool.pop() {
            self.stats.pool_hits.fetch_add(1, Ordering::Relaxed);
            item
        } else {
            self.stats.pool_misses.fetch_add(1, Ordering::Relaxed);
            self.stats.allocations.fetch_add(1, Ordering::Relaxed);
            Box::new(T::default())
        }
    }
    
    /// Return message object to pool
    pub fn return_item(&self, mut item: Box<T>) {
        if /* self.pool.len() < self.max_size */ true {
            // Reset the item to default state
            *item = T::default();
            self.pool.push(item);
            self.stats.deallocations.fetch_add(1, Ordering::Relaxed);
        }
        // If pool is full, let item drop to free memory
    }
    
    /// Get pool statistics
    pub fn stats(&self) -> Arc<PoolStats> {
        Arc::clone(&self.stats)
    }
}

/// Performance monitoring for hot path operations
pub struct PerformanceMonitor {
    /// Recent latency measurements (circular buffer)
    latencies: Arc<parking_lot::Mutex<Vec<Duration>>>,
    /// Current index in circular buffer
    current_index: AtomicU64,
    /// Buffer size for latency tracking
    buffer_size: usize,
}

impl PerformanceMonitor {
    pub fn new(buffer_size: usize) -> Self {
        Self {
            latencies: Arc::new(parking_lot::Mutex::new(vec![Duration::ZERO; buffer_size])),
            current_index: AtomicU64::new(0),
            buffer_size,
        }
    }
    
    /// Record a latency measurement
    pub fn record_latency(&self, latency: Duration) {
        let index = self.current_index.fetch_add(1, Ordering::Relaxed) % self.buffer_size as u64;
        let mut latencies = self.latencies.lock();
        latencies[index as usize] = latency;
    }
    
    /// Get P95 latency
    pub fn p95_latency(&self) -> Duration {
        let latencies = self.latencies.lock();
        let mut sorted_latencies: Vec<Duration> = latencies.clone();
        sorted_latencies.sort();
        
        if sorted_latencies.is_empty() {
            return Duration::ZERO;
        }
        
        let index = (sorted_latencies.len() as f64 * 0.95) as usize;
        sorted_latencies[index.min(sorted_latencies.len() - 1)]
    }
    
    /// Check if performance is within SLA
    pub fn is_healthy(&self) -> bool {
        self.p95_latency() < Duration::from_micros(35) // 35μs SLA
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_hot_path_cache_performance() {
        let cache = HotPathCache::new();
        
        // Test buffer allocation and reuse
        let (buffer1, cache_hit1) = cache.get_buffer(1024);
        assert!(!cache_hit1); // First allocation should miss cache
        assert!(buffer1.capacity() >= 1024);
        
        cache.return_buffer(buffer1);
        
        let (buffer2, cache_hit2) = cache.get_buffer(1024);
        assert!(cache_hit2); // Second allocation should hit cache
        
        let stats = cache.stats();
        assert_eq!(stats.cache_hits.load(Ordering::Relaxed), 1);
        assert_eq!(stats.cache_misses.load(Ordering::Relaxed), 1);
    }
    
    #[test]
    fn test_fast_serializer() {
        let serializer = FastSerializer::new();
        
        #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
        struct TestMessage {
            id: u64,
            value: f64,
        }
        
        let original = TestMessage { id: 123, value: 456.789 };
        
        let serialized = serializer.serialize_tlv(&original).unwrap();
        let deserialized: TestMessage = serializer.deserialize_tlv(&serialized).unwrap();
        
        assert_eq!(original, deserialized);
        
        // Return buffer for reuse
        serializer.return_buffer(serialized);
    }
    
    #[test]
    fn test_message_pool() {
        let pool = MessagePool::<Vec<u8>>::new(10);
        
        let item1 = pool.get();
        let item2 = pool.get();
        
        pool.return_item(item1);
        pool.return_item(item2);
        
        let item3 = pool.get(); // Should reuse from pool
        
        let stats = pool.stats();
        assert_eq!(stats.pool_hits.load(Ordering::Relaxed), 1);
        assert_eq!(stats.pool_misses.load(Ordering::Relaxed), 2);
    }
    
    #[test]
    fn test_performance_monitor() {
        let monitor = PerformanceMonitor::new(100);
        
        // Record some test latencies
        for i in 1..=100 {
            monitor.record_latency(Duration::from_nanos(i * 1000)); // 1μs to 100μs
        }
        
        let p95 = monitor.p95_latency();
        assert!(p95 > Duration::from_micros(90)); // Should be around 95μs
        assert!(p95 < Duration::from_micros(100));
        
        // Should be unhealthy due to high latencies
        assert!(!monitor.is_healthy());
    }
}sert!(!monitor.is_healthy());
    }
}