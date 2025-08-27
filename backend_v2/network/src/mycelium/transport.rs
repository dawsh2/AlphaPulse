//! Actor Transport Abstraction (MYCEL-001)
//!
//! Core transport system that automatically selects optimal communication method:
//! - Same process: Arc<T> through channels (zero serialization)
//! - Different process: TLV serialization over Unix sockets
//! - Different nodes: TLV over network transport

use crate::{Priority, Result, TransportError, UnixSocketConnection};
use crate::mycelium::messages::Message;
use crate::performance::{FastSerializer, HotPathCache, PerformanceMonitor};
use std::any::Any;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::{debug, trace, warn};

/// Actor transport that adapts to deployment configuration
pub struct ActorTransport {
    /// Fast path: in-process communication via Arc<T>
    local: Option<mpsc::Sender<Arc<dyn Any + Send + Sync>>>,
    
    /// Slow path: cross-process communication via TLV
    remote: Option<Arc<UnixSocketConnection>>,
    
    /// Network path: cross-node communication via TLV
    network: Option<Arc<dyn NetworkTransport>>,
    
    /// Performance metrics
    metrics: Arc<TransportMetrics>,
    
    /// Actor ID for debugging
    actor_id: String,
    
    /// Hot path performance optimizations
    performance_cache: Arc<HotPathCache>,
    
    /// Fast TLV serializer for remote/network transport
    serializer: Arc<FastSerializer>,
    
    /// Performance monitoring for hot path SLA compliance
    performance_monitor: Arc<PerformanceMonitor>,
}

impl std::fmt::Debug for ActorTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ActorTransport")
            .field("actor_id", &self.actor_id)
            .field("has_local", &self.local.is_some())
            .field("has_remote", &self.remote.is_some())
            .field("has_network", &self.network.is_some())
            .finish()
    }
}

impl Clone for ActorTransport {
    fn clone(&self) -> Self {
        Self {
            local: self.local.clone(),
            remote: self.remote.clone(),
            network: self.network.clone(),
            metrics: self.metrics.clone(),
            actor_id: self.actor_id.clone(),
            performance_cache: Arc::clone(&self.performance_cache),
            serializer: Arc::clone(&self.serializer),
            performance_monitor: Arc::clone(&self.performance_monitor),
        }
    }
}

/// Transport type selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportType {
    /// Arc<T> through channels - zero serialization
    Local,
    /// TLV over Unix domain socket  
    UnixSocket,
    /// TLV over network (TCP/UDP/QUIC)
    Network,
}

/// Transport performance metrics
#[derive(Debug, Default)]
pub struct TransportMetrics {
    /// Local messages sent (Arc::clone only)
    pub local_sends: AtomicU64,
    /// Remote messages sent (with serialization)
    pub remote_sends: AtomicU64,
    /// Network messages sent
    pub network_sends: AtomicU64,
    /// Total local latency in nanoseconds
    pub local_latency_total_ns: AtomicU64,
    /// Total remote latency in nanoseconds  
    pub remote_latency_total_ns: AtomicU64,
    /// Total network latency in nanoseconds
    pub network_latency_total_ns: AtomicU64,
    /// Serialization bytes eliminated
    pub serialization_eliminated_bytes: AtomicU64,
}

impl TransportMetrics {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }
    
    /// Record local send metrics
    pub fn record_local_send(&self, duration: Duration, message_size: usize) {
        self.local_sends.fetch_add(1, Ordering::Relaxed);
        self.local_latency_total_ns.fetch_add(
            duration.as_nanos() as u64,
            Ordering::Relaxed
        );
        // Track bytes that would have been serialized
        self.serialization_eliminated_bytes.fetch_add(
            message_size as u64,
            Ordering::Relaxed
        );
    }
    
    /// Record remote send metrics
    pub fn record_remote_send(&self, duration: Duration) {
        self.remote_sends.fetch_add(1, Ordering::Relaxed);
        self.remote_latency_total_ns.fetch_add(
            duration.as_nanos() as u64,
            Ordering::Relaxed
        );
    }
    
    /// Record network send metrics
    pub fn record_network_send(&self, duration: Duration) {
        self.network_sends.fetch_add(1, Ordering::Relaxed);
        self.network_latency_total_ns.fetch_add(
            duration.as_nanos() as u64,
            Ordering::Relaxed
        );
    }
    
    /// Calculate average local latency
    pub fn avg_local_latency_ns(&self) -> f64 {
        let sends = self.local_sends.load(Ordering::Relaxed);
        if sends == 0 {
            return 0.0;
        }
        let total = self.local_latency_total_ns.load(Ordering::Relaxed);
        total as f64 / sends as f64
    }
    
    /// Calculate average remote latency  
    pub fn avg_remote_latency_ns(&self) -> f64 {
        let sends = self.remote_sends.load(Ordering::Relaxed);
        if sends == 0 {
            return 0.0;
        }
        let total = self.remote_latency_total_ns.load(Ordering::Relaxed);
        total as f64 / sends as f64
    }
    
    /// Get total bytes of serialization eliminated
    pub fn serialization_eliminated_mb(&self) -> f64 {
        self.serialization_eliminated_bytes.load(Ordering::Relaxed) as f64 / 1_048_576.0
    }
    
    /// Get performance statistics
    pub fn get_stats(&self) -> TransportStats {
        TransportStats {
            local_sends: self.local_sends.load(Ordering::Relaxed),
            remote_sends: self.remote_sends.load(Ordering::Relaxed),
            network_sends: self.network_sends.load(Ordering::Relaxed),
            avg_local_latency_ns: self.avg_local_latency_ns(),
            avg_remote_latency_ns: self.avg_remote_latency_ns(),
            serialization_eliminated_mb: self.serialization_eliminated_mb(),
        }
    }
}

/// Transport statistics snapshot
#[derive(Debug, Clone)]
pub struct TransportStats {
    pub local_sends: u64,
    pub remote_sends: u64,  
    pub network_sends: u64,
    pub avg_local_latency_ns: f64,
    pub avg_remote_latency_ns: f64,
    pub serialization_eliminated_mb: f64,
}

/// Transport health status for monitoring and debugging
#[derive(Debug, Clone)]
pub struct TransportHealthStatus {
    pub transport_type: TransportType,
    pub status: HealthState,
    pub details: String,
    pub last_message_sent: Option<Instant>,
    pub error_count: u64,
}

/// Health state enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthState {
    Healthy,
    Unhealthy,
    Unknown,
}

impl ActorTransport {
    /// Create new transport for local (bundled) communication
    pub fn new_local(
        sender: mpsc::Sender<Arc<dyn Any + Send + Sync>>,
        actor_id: String,
    ) -> Self {
        debug!("Creating local transport for actor {}", actor_id);
        Self {
            local: Some(sender),
            remote: None,
            network: None,
            metrics: TransportMetrics::new(),
            actor_id,
            performance_cache: Arc::new(HotPathCache::new()),
            serializer: Arc::new(FastSerializer::new()),
            performance_monitor: Arc::new(PerformanceMonitor::new(1000)), // Track 1000 samples
        }
    }
    
    /// Create new transport for remote (same-node) communication
    pub fn new_remote(
        connection: Arc<UnixSocketConnection>,
        actor_id: String,
    ) -> Self {
        debug!("Creating remote transport for actor {}", actor_id);
        Self {
            local: None,
            remote: Some(connection),
            network: None,
            metrics: TransportMetrics::new(),
            actor_id,
            performance_cache: Arc::new(HotPathCache::new()),
            serializer: Arc::new(FastSerializer::new()),
            performance_monitor: Arc::new(PerformanceMonitor::new(1000)), // Track 1000 samples
        }
    }
    
    /// Create new transport for network communication
    pub fn new_network(
        transport: Arc<dyn NetworkTransport>,
        actor_id: String,
    ) -> Self {
        debug!("Creating network transport for actor {}", actor_id);
        Self {
            local: None,
            remote: None,
            network: Some(transport),
            metrics: TransportMetrics::new(),
            actor_id,
            performance_cache: Arc::new(HotPathCache::new()),
            serializer: Arc::new(FastSerializer::new()),
            performance_monitor: Arc::new(PerformanceMonitor::new(1000)), // Track 1000 samples
        }
    }
    
    /// Send message using optimal transport
    pub async fn send<T>(&self, msg: T) -> Result<()>
    where 
        T: crate::mycelium::messages::Message + Send + Sync + 'static
    {
        let start = Instant::now();
        
        // CRITICAL: Protocol V2 validation happens at message construction time
        // Individual transport layers trust the message has already been validated
        
        let message_size = std::mem::size_of_val(&msg);
        
        if let Some(local) = &self.local {
            // FAST PATH: Zero serialization - just Arc::clone()
            // PERFORMANCE CRITICAL: Use try_send for <100ns target latency
            trace!("Sending local message for actor {}", self.actor_id);
            let arc_msg = Arc::new(msg) as Arc<dyn Any + Send + Sync>;
            
            // Try non-blocking send first (optimal for <100ns target)
            match local.try_send(arc_msg) {
                Ok(()) => {
                    // Success - fastest path achieved
                }
                Err(mpsc::error::TrySendError::Full(arc_msg)) => {
                    // Channel full - fall back to async send but log performance issue
                    warn!("Local channel full for actor {} - falling back to async send (performance impact)", self.actor_id);
                    local.send(arc_msg).await
                        .map_err(|_| TransportError::network("Local channel closed"))?;
                }
                Err(mpsc::error::TrySendError::Closed(_)) => {
                    return Err(TransportError::network("Local channel closed"));
                }
            }
            
            self.metrics.record_local_send(start.elapsed(), message_size);
            
        } else if let Some(remote) = &self.remote {
            // SLOW PATH: Serialize to TLV for Unix socket
            trace!("Sending remote TLV message for actor {}", self.actor_id);
            let tlv_bytes = msg.to_tlv()
                .map_err(|e| TransportError::protocol(&format!("TLV serialization failed: {}", e)))?;
            
            // CRITICAL: Real Unix socket send - implements "no deception" principle
            // Now using internal mutability with async Mutex for thread-safe access
            
            if remote.is_connected() {
                debug!(
                    actor_id = %self.actor_id,
                    bytes = tlv_bytes.len(),
                    "Sending TLV message via Unix socket"
                );
                
                // Send the TLV message through the Unix socket connection
                remote.send(&tlv_bytes).await
                    .map_err(|e| {
                        warn!("Failed to send message via Unix socket: {}", e);
                        TransportError::network_with_source("Unix socket send failed", e)
                    })?;
                
                debug!("Successfully sent {} bytes via Unix socket", tlv_bytes.len());
            } else {
                return Err(TransportError::network("Unix socket connection not active"));
            }
            
            self.metrics.record_remote_send(start.elapsed());
            
        } else if let Some(network) = &self.network {
            // NETWORK PATH: Serialize to TLV for network
            trace!("Sending network TLV message for actor {}", self.actor_id);
            let tlv_bytes = msg.to_tlv()
                .map_err(|e| TransportError::protocol(&format!("TLV serialization failed: {}", e)))?;
            
            network.send(&tlv_bytes).await
                .map_err(|_| TransportError::network("Network send failed"))?;
            
            self.metrics.record_network_send(start.elapsed());
            
        } else {
            return Err(TransportError::configuration(
                "No transport configured", 
                Some("transport_config")
            ));
        }
        
        Ok(())
    }
    
    /// Send message with priority (only affects non-local transports)
    pub async fn send_with_priority<T>(&self, msg: T, _priority: Priority) -> Result<()>
    where 
        T: crate::mycelium::messages::Message + Send + Sync + 'static
    {
        if self.local.is_some() {
            // Local transport ignores priority - always fast
            self.send(msg).await
        } else {
            // TODO: Implement priority handling for remote/network transports
            self.send(msg).await
        }
    }
    
    /// Get transport type
    pub fn transport_type(&self) -> TransportType {
        if self.local.is_some() {
            TransportType::Local
        } else if self.remote.is_some() {
            TransportType::UnixSocket
        } else if self.network.is_some() {
            TransportType::Network
        } else {
            // This shouldn't happen in practice
            TransportType::Local
        }
    }
    
    /// Get performance metrics
    pub fn metrics(&self) -> Arc<TransportMetrics> {
        Arc::clone(&self.metrics)
    }
    
    /// Get system metrics (placeholder for system-level metrics access)
    fn get_system_metrics(&self) -> Option<&crate::mycelium::system::SystemMetrics> {
        // In a real implementation, this would access the system metrics
        // For now, return None as this is just a performance optimization hook
        None
    }
    
    /// Check if transport is healthy
    /// 
    /// CRITICAL: Real health validation - never return fake success
    /// This implements AlphaPulse's "no deception" principle
    pub fn is_healthy(&self) -> bool {
        match self.transport_type() {
            TransportType::Local => {
                // Local channel health check
                match &self.local {
                    Some(channel) => !channel.is_closed(),
                    None => false, // Should not happen, but be explicit
                }
            },
            TransportType::UnixSocket => {
                // Unix socket health check - verify connection is active
                match &self.remote {
                    Some(connection) => {
                        // REAL health check: Attempt to verify socket connection
                        // In production, this might send a heartbeat or check socket status
                        connection.is_connected()
                    },
                    None => false,
                }
            },
            TransportType::Network => {
                // Network transport health check - verify connectivity
                match &self.network {
                    Some(transport) => {
                        // REAL health check: Use the NetworkTransport trait method
                        transport.is_healthy()
                    },
                    None => false,
                }
            }
        }
    }
    
    /// Detailed health status for monitoring and debugging
    pub fn health_status(&self) -> TransportHealthStatus {
        let transport_type = self.transport_type();
        let is_healthy = self.is_healthy();
        let metrics = self.metrics.get_stats();
        
        let status = if is_healthy {
            HealthState::Healthy
        } else {
            HealthState::Unhealthy
        };
        
        let details = match transport_type {
            TransportType::Local => {
                let channel_closed = self.local.as_ref().map(|c| c.is_closed()).unwrap_or(true);
                format!("Local channel: closed={}", channel_closed)
            },
            TransportType::UnixSocket => {
                let connected = self.remote.as_ref().map(|c| c.is_connected()).unwrap_or(false);
                format!("Unix socket: connected={}", connected)
            },
            TransportType::Network => {
                let network_healthy = self.network.as_ref().map(|n| n.is_healthy()).unwrap_or(false);
                format!("Network transport: healthy={}", network_healthy)
            },
        };
        
        TransportHealthStatus {
            transport_type,
            status,
            details,
            last_message_sent: if metrics.local_sends + metrics.remote_sends + metrics.network_sends > 0 {
                Some(Instant::now()) // Approximation - in production would track actual timestamps
            } else {
                None
            },
            error_count: 0, // Would need to add error tracking
        }
    }
}

/// Trait for network transport implementations
#[async_trait::async_trait]
pub trait NetworkTransport: Send + Sync {
    /// Send message over network
    async fn send(&self, message: &[u8]) -> Result<()>;
    
    /// Check if connection is healthy
    fn is_healthy(&self) -> bool;
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::mycelium::messages::{Message, PoolSwapEvent, QuoteUpdate};
    use std::time::{SystemTime, UNIX_EPOCH};
    
    #[tokio::test]
    async fn test_local_transport_creation() {
        let (tx, _rx) = mpsc::channel(100);
        let transport = ActorTransport::new_local(tx, "test_actor".to_string());
        
        assert_eq!(transport.transport_type(), TransportType::Local);
        assert!(transport.is_healthy());
    }
    
    #[tokio::test]
    async fn test_local_message_send() {
        let (tx, mut rx) = mpsc::channel(100);
        let transport = ActorTransport::new_local(tx, "test_actor".to_string());
        
        // Create real Protocol V2 message with current timestamp
        let timestamp_ns = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
            
        let msg = PoolSwapEvent {
            pool_address: [0x12; 20], // Real Ethereum address format
            token0_in: 1_000_000_000_000_000_000, // 1 WETH (18 decimals)
            token1_out: 2_000_000_000, // 2000 USDC (6 decimals) 
            timestamp_ns,
            tx_hash: [0xab; 32], // Real transaction hash format
            gas_used: 150_000,
        };
        
        transport.send(msg.clone()).await.unwrap();
        
        // Verify message received
        let received = rx.recv().await.unwrap();
        let downcast = received.downcast::<PoolSwapEvent>().unwrap();
        assert_eq!(*downcast, msg);
        
        // Verify metrics
        let stats = transport.metrics().get_stats();
        assert_eq!(stats.local_sends, 1);
        assert_eq!(stats.remote_sends, 0);
        assert!(stats.avg_local_latency_ns > 0.0);
        assert!(stats.serialization_eliminated_mb > 0.0);
    }
    
    #[tokio::test]
    async fn test_metrics_collection() {
        let (tx, _rx) = mpsc::channel(100);
        let transport = ActorTransport::new_local(tx, "test_actor".to_string());
        
        // Send multiple real messages with different data
        for i in 0..10 {
            let timestamp_ns = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64 + i as u64;
                
            let msg = QuoteUpdate {
                instrument_id: 12345 + i as u64,
                bid_price: (1999_00000000_i64) + (i as i64 * 1000000), // $1999 + $0.01*i in 8-decimal
                ask_price: (2001_00000000_i64) + (i as i64 * 1000000), // $2001 + $0.01*i in 8-decimal
                bid_size: 1_000_000 + (i as u64 * 100_000),
                ask_size: 1_000_000 + (i as u64 * 100_000),
                timestamp_ns,
            };
            transport.send(msg).await.unwrap();
        }
        
        let stats = transport.metrics().get_stats();
        assert_eq!(stats.local_sends, 10);
        assert!(stats.avg_local_latency_ns > 0.0);
        assert!(stats.serialization_eliminated_mb > 0.0);
    }
}