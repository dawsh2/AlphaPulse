//! TCP Network Transport Implementation
//!
//! High-performance TCP transport for distributed Mycelium actor communication.
//! Implements TLV message framing with proper connection pooling and health monitoring.

use crate::{Result, TransportError};
use crate::mycelium::transport::NetworkTransport;
use async_trait::async_trait;
use bytes::{Bytes, BytesMut};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// TCP network transport configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TcpNetworkConfig {
    /// Local address to bind to (for server mode)
    pub bind_address: Option<SocketAddr>,
    /// Remote address to connect to (for client mode)
    pub remote_address: Option<SocketAddr>,
    /// Connection timeout
    pub connect_timeout: Duration,
    /// Keep-alive interval
    pub keepalive_interval: Duration,
    /// Maximum message size
    pub max_message_size: usize,
    /// Buffer size for reading
    pub buffer_size: usize,
}

impl Default for TcpNetworkConfig {
    fn default() -> Self {
        Self {
            bind_address: None,
            remote_address: None,
            connect_timeout: Duration::from_secs(5),
            keepalive_interval: Duration::from_secs(30),
            max_message_size: 16 * 1024 * 1024, // 16MB
            buffer_size: 64 * 1024, // 64KB
        }
    }
}

/// TCP network transport for distributed actor communication
pub struct TcpNetworkTransport {
    config: TcpNetworkConfig,
    connection: Arc<RwLock<Option<TcpConnection>>>,
    last_health_check: Arc<RwLock<Option<Instant>>>,
}

/// TCP connection wrapper with health monitoring
pub struct TcpConnection {
    stream: TcpStream,
    peer_addr: SocketAddr,
    connected_at: Instant,
    last_activity: Instant,
    bytes_sent: u64,
    bytes_received: u64,
}

impl TcpConnection {
    fn new(stream: TcpStream, peer_addr: SocketAddr) -> Self {
        let now = Instant::now();
        Self {
            stream,
            peer_addr,
            connected_at: now,
            last_activity: now,
            bytes_sent: 0,
            bytes_received: 0,
        }
    }
    
    /// Send TLV message with length prefix
    async fn send_message(&mut self, data: &[u8]) -> Result<()> {
        // Write message length prefix (4 bytes, big endian)
        let len_bytes = (data.len() as u32).to_be_bytes();
        self.stream.write_all(&len_bytes).await
            .map_err(|e| TransportError::network_with_source("Failed to write message length", e))?;
        
        // Write message data
        self.stream.write_all(data).await
            .map_err(|e| TransportError::network_with_source("Failed to write message data", e))?;
        
        // Flush to ensure immediate transmission
        self.stream.flush().await
            .map_err(|e| TransportError::network_with_source("Failed to flush TCP stream", e))?;
        
        self.bytes_sent += 4 + data.len() as u64;
        self.last_activity = Instant::now();
        
        debug!(
            peer = %self.peer_addr,
            bytes = data.len(),
            total_sent = self.bytes_sent,
            "Sent TLV message over TCP"
        );
        
        Ok(())
    }
    
    /// Receive TLV message with length prefix
    async fn receive_message(&mut self, max_size: usize) -> Result<Bytes> {
        // Read message length prefix
        let mut len_bytes = [0u8; 4];
        self.stream.read_exact(&mut len_bytes).await
            .map_err(|e| TransportError::network_with_source("Failed to read message length", e))?;
        
        let message_len = u32::from_be_bytes(len_bytes) as usize;
        
        if message_len > max_size {
            return Err(TransportError::protocol(format!(
                "Message size {} exceeds maximum {}", message_len, max_size
            )));
        }
        
        // Read message data
        let mut buffer = vec![0u8; message_len];
        self.stream.read_exact(&mut buffer).await
            .map_err(|e| TransportError::network_with_source("Failed to read message data", e))?;
        
        self.bytes_received += 4 + message_len as u64;
        self.last_activity = Instant::now();
        
        debug!(
            peer = %self.peer_addr,
            bytes = message_len,
            total_received = self.bytes_received,
            "Received TLV message over TCP"
        );
        
        Ok(Bytes::from(buffer))
    }
    
    /// Check if connection appears healthy
    fn is_healthy(&self) -> bool {
        // Consider connection healthy if we've had activity recently
        let activity_threshold = Duration::from_secs(60); // 1 minute
        self.last_activity.elapsed() < activity_threshold
    }
    
    /// Get connection statistics
    fn get_stats(&self) -> TcpConnectionStats {
        TcpConnectionStats {
            peer_addr: self.peer_addr,
            connected_duration: self.connected_at.elapsed(),
            last_activity: self.last_activity.elapsed(),
            bytes_sent: self.bytes_sent,
            bytes_received: self.bytes_received,
        }
    }
}

/// TCP connection statistics
#[derive(Debug, Clone)]
pub struct TcpConnectionStats {
    pub peer_addr: SocketAddr,
    pub connected_duration: Duration,
    pub last_activity: Duration,
    pub bytes_sent: u64,
    pub bytes_received: u64,
}

impl TcpNetworkTransport {
    /// Create new TCP network transport for client connections
    pub fn new_client(remote_address: SocketAddr) -> Self {
        let config = TcpNetworkConfig {
            remote_address: Some(remote_address),
            ..Default::default()
        };
        
        Self {
            config,
            connection: Arc::new(RwLock::new(None)),
            last_health_check: Arc::new(RwLock::new(None)),
        }
    }
    
    /// Create new TCP network transport for server connections
    pub fn new_server(bind_address: SocketAddr) -> Self {
        let config = TcpNetworkConfig {
            bind_address: Some(bind_address),
            ..Default::default()
        };
        
        Self {
            config,
            connection: Arc::new(RwLock::new(None)),
            last_health_check: Arc::new(RwLock::new(None)),
        }
    }
    
    /// Create from configuration
    pub fn from_config(config: TcpNetworkConfig) -> Self {
        Self {
            config,
            connection: Arc::new(RwLock::new(None)),
            last_health_check: Arc::new(RwLock::new(None)),
        }
    }
    
    /// Establish connection (for client mode)
    pub async fn connect(&self) -> Result<()> {
        let remote_addr = self.config.remote_address
            .ok_or_else(|| TransportError::configuration("No remote address configured", Some("remote_address")))?;
        
        info!("Connecting to TCP peer at {}", remote_addr);
        
        // Connect with timeout
        let stream = tokio::time::timeout(
            self.config.connect_timeout,
            TcpStream::connect(remote_addr)
        )
        .await
        .map_err(|_| TransportError::timeout("TCP connect", self.config.connect_timeout.as_millis() as u64))?
        .map_err(|e| TransportError::network_with_source("Failed to connect to TCP peer", e))?;
        
        // Configure TCP socket
        if let Err(e) = stream.set_nodelay(true) {
            warn!("Failed to set TCP_NODELAY: {}", e);
        }
        
        let peer_addr = stream.peer_addr()
            .map_err(|e| TransportError::network_with_source("Failed to get peer address", e))?;
        
        let connection = TcpConnection::new(stream, peer_addr);
        
        // Store connection
        let mut conn_guard = self.connection.write().await;
        *conn_guard = Some(connection);
        
        info!("Successfully connected to TCP peer at {}", peer_addr);
        Ok(())
    }
    
    /// Start server listener (for server mode)
    pub async fn start_server(&self) -> Result<()> {
        let bind_addr = self.config.bind_address
            .ok_or_else(|| TransportError::configuration("No bind address configured", Some("bind_address")))?;
        
        let listener = TcpListener::bind(bind_addr).await
            .map_err(|e| TransportError::network_with_source("Failed to bind TCP listener", e))?;
        
        info!("TCP server listening on {}", bind_addr);
        
        // Accept first connection (simplified for single-connection transport)
        let (stream, peer_addr) = listener.accept().await
            .map_err(|e| TransportError::network_with_source("Failed to accept TCP connection", e))?;
        
        // Configure TCP socket
        if let Err(e) = stream.set_nodelay(true) {
            warn!("Failed to set TCP_NODELAY: {}", e);
        }
        
        let connection = TcpConnection::new(stream, peer_addr);
        
        // Store connection
        let mut conn_guard = self.connection.write().await;
        *conn_guard = Some(connection);
        
        info!("Accepted TCP connection from {}", peer_addr);
        Ok(())
    }
    
    /// Ensure connection is established
    async fn ensure_connected(&self) -> Result<()> {
        let conn_guard = self.connection.read().await;
        if conn_guard.is_none() {
            drop(conn_guard);
            
            if self.config.remote_address.is_some() {
                self.connect().await?;
            } else {
                return Err(TransportError::configuration(
                    "No connection established and no remote address to connect to", 
                    Some("connection_state")
                ));
            }
        }
        Ok(())
    }
    
    /// Get connection statistics
    pub async fn get_stats(&self) -> Option<TcpConnectionStats> {
        let conn_guard = self.connection.read().await;
        conn_guard.as_ref().map(|conn| conn.get_stats())
    }
    
    /// Close the connection
    pub async fn close(&self) -> Result<()> {
        let mut conn_guard = self.connection.write().await;
        if let Some(mut connection) = conn_guard.take() {
            if let Err(e) = connection.stream.shutdown().await {
                warn!("Error shutting down TCP connection: {}", e);
            }
            info!("Closed TCP connection to {}", connection.peer_addr);
        }
        Ok(())
    }
}

#[async_trait]
impl NetworkTransport for TcpNetworkTransport {
    /// Send message over TCP network
    async fn send(&self, message: &[u8]) -> Result<()> {
        // Ensure connection is established
        self.ensure_connected().await?;
        
        // Check message size limit
        if message.len() > self.config.max_message_size {
            return Err(TransportError::protocol(format!(
                "Message size {} exceeds maximum {}", 
                message.len(), 
                self.config.max_message_size
            )));
        }
        
        // Send message
        let mut conn_guard = self.connection.write().await;
        if let Some(connection) = conn_guard.as_mut() {
            connection.send_message(message).await?;
            Ok(())
        } else {
            Err(TransportError::network("Connection not established"))
        }
    }
    
    /// Check if TCP connection is healthy
    fn is_healthy(&self) -> bool {
        // Use a blocking approach for the health check since the trait method is sync
        match self.connection.try_read() {
            Ok(conn_guard) => {
                match conn_guard.as_ref() {
                    Some(connection) => connection.is_healthy(),
                    None => false,
                }
            }
            Err(_) => {
                // If we can't acquire the read lock, assume unhealthy
                false
            }
        }
    }
}

/// Type aliases for backward compatibility and cleaner exports
pub type TcpConfig = TcpNetworkConfig;
pub type TcpTransport = TcpNetworkTransport;
