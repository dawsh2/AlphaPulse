//! Direct connection sink for TCP, WebSocket, and Unix socket connections
//!
//! DirectSink provides direct connections to services using various protocols:
//! - TCP connections for remote services
//! - WebSocket connections for web-based services
//! - Unix socket connections for local services (alternative to relay)

use crate::{BatchResult, ConnectionHealth, ConnectionState, ExtendedSinkMetadata, Message, MessageSink, SinkError, SinkMetadata};
use async_trait::async_trait;
use futures_util::sink::SinkExt;
use futures_util::stream::StreamExt;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::{Mutex, RwLock};

/// Connection type for DirectSink
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionType {
    /// TCP connection
    Tcp(String), // host:port
    /// WebSocket connection
    WebSocket(String), // ws://... or wss://...
    /// Unix socket connection
    Unix(String), // path
}

/// Connection wrapper for different protocols
#[derive(Debug)]
enum Connection {
    Tcp(tokio::net::TcpStream),
    WebSocket(tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>),
    Unix(tokio::net::UnixStream),
}

impl Connection {
    async fn write_all(&mut self, data: &[u8]) -> std::io::Result<()> {
        match self {
            Connection::Tcp(stream) => stream.write_all(data).await,
            Connection::WebSocket(ws) => {
                use tokio_tungstenite::tungstenite::Message as WsMessage;
                let msg = WsMessage::Binary(data.to_vec());
                ws.send(msg).await
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
            }
            Connection::Unix(stream) => stream.write_all(data).await,
        }
    }
    
    async fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Connection::Tcp(stream) => stream.flush().await,
            Connection::WebSocket(_) => Ok(()), // WebSocket handles flushing automatically
            Connection::Unix(stream) => stream.flush().await,
        }
    }
}

/// Direct connection sink for various protocols
#[derive(Debug)]
pub struct DirectSink {
    /// Connection wrapper
    connection: Arc<RwLock<Option<Connection>>>,
    
    /// Connection type and endpoint
    connection_type: ConnectionType,
    
    /// Connection state
    state: Arc<RwLock<ConnectionState>>,
    
    /// Connection mutex for thread safety
    connection_mutex: Arc<Mutex<()>>,
    
    /// Metrics
    messages_sent: AtomicU64,
    messages_failed: AtomicU64,
    bytes_sent: AtomicU64,
    connection_attempts: AtomicU64,
    last_successful_send: Arc<RwLock<Option<SystemTime>>>,
    connected_at: Arc<RwLock<Option<Instant>>>,
    
    /// Sink name for debugging
    name: String,
    
    /// Whether this sink is connected
    connected: AtomicBool,
    
    /// Connection timeout
    connect_timeout: Duration,
}

impl DirectSink {
    /// Create a new direct sink
    pub fn new(connection_type: ConnectionType) -> Self {
        let name = match &connection_type {
            ConnectionType::Tcp(addr) => format!("tcp-{}", addr),
            ConnectionType::WebSocket(url) => format!("ws-{}", url),
            ConnectionType::Unix(path) => format!("unix-{}", path),
        };
        
        Self {
            connection: Arc::new(RwLock::new(None)),
            connection_type,
            state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
            connection_mutex: Arc::new(Mutex::new(())),
            messages_sent: AtomicU64::new(0),
            messages_failed: AtomicU64::new(0),
            bytes_sent: AtomicU64::new(0),
            connection_attempts: AtomicU64::new(0),
            last_successful_send: Arc::new(RwLock::new(None)),
            connected_at: Arc::new(RwLock::new(None)),
            name,
            connected: AtomicBool::new(false),
            connect_timeout: Duration::from_secs(5),
        }
    }
    
    /// Create TCP connection
    pub async fn tcp(address: &str) -> Result<Self, SinkError> {
        let sink = Self::new(ConnectionType::Tcp(address.to_string()));
        Ok(sink)
    }
    
    /// Create WebSocket connection
    pub async fn websocket(url: &str) -> Result<Self, SinkError> {
        // Validate WebSocket URL
        if !url.starts_with("ws://") && !url.starts_with("wss://") {
            return Err(SinkError::invalid_config(
                "WebSocket URL must start with ws:// or wss://"
            ));
        }
        
        let sink = Self::new(ConnectionType::WebSocket(url.to_string()));
        Ok(sink)
    }
    
    /// Create Unix socket connection
    pub async fn unix(path: &str) -> Result<Self, SinkError> {
        if path.is_empty() {
            return Err(SinkError::invalid_config("Unix socket path cannot be empty"));
        }
        
        let sink = Self::new(ConnectionType::Unix(path.to_string()));
        Ok(sink)
    }
    
    /// Get connection type
    pub fn connection_type(&self) -> &ConnectionType {
        &self.connection_type
    }
    
    /// Get connection uptime
    pub async fn connection_uptime(&self) -> Option<Duration> {
        let connected_at = self.connected_at.read().await;
        connected_at.map(|t| t.elapsed())
    }
    
    /// Set connection timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }
    
    /// Establish connection based on connection type
    async fn ensure_connection(&self) -> Result<(), SinkError> {
        // Fast path: already connected
        if self.connected.load(Ordering::Relaxed) {
            return Ok(());
        }
        
        // Acquire connection mutex to prevent duplicate connections
        let _guard = self.connection_mutex.lock().await;
        
        // Double check under mutex
        if self.connected.load(Ordering::Relaxed) {
            return Ok(());
        }
        
        self.connection_attempts.fetch_add(1, Ordering::Relaxed);
        
        // Update state to connecting
        {
            let mut state = self.state.write().await;
            *state = ConnectionState::Connecting;
        }
        
        tracing::debug!("Connecting direct sink: {}", self.name);
        
        let connection_result = match &self.connection_type {
            ConnectionType::Tcp(address) => self.connect_tcp(address).await,
            ConnectionType::WebSocket(url) => self.connect_websocket(url).await,
            ConnectionType::Unix(path) => self.connect_unix(path).await,
        };
        
        match connection_result {
            Ok(connection) => {
                // Store connection
                {
                    let mut conn = self.connection.write().await;
                    *conn = Some(connection);
                }
                
                // Update state and metrics
                {
                    let mut state = self.state.write().await;
                    *state = ConnectionState::Connected;
                }
                
                {
                    let mut connected_at = self.connected_at.write().await;
                    *connected_at = Some(Instant::now());
                }
                
                self.connected.store(true, Ordering::Relaxed);
                
                tracing::info!("Direct sink connected: {}", self.name);
                Ok(())
            }
            Err(e) => {
                // Update state to failed
                {
                    let mut state = self.state.write().await;
                    *state = ConnectionState::Failed;
                }
                
                tracing::error!("Direct sink connection failed: {}", e);
                Err(e)
            }
        }
    }
    
    /// Connect via TCP
    async fn connect_tcp(&self, address: &str) -> Result<Connection, SinkError> {
        let stream = tokio::time::timeout(
            self.connect_timeout,
            tokio::net::TcpStream::connect(address)
        ).await
        .map_err(|_| SinkError::timeout(self.connect_timeout.as_secs()))?
        .map_err(|e| SinkError::connection_failed(
            format!("TCP connection to {} failed: {}", address, e)
        ))?;
        
        Ok(Connection::Tcp(stream))
    }
    
    /// Connect via WebSocket
    async fn connect_websocket(&self, url: &str) -> Result<Connection, SinkError> {
        use tokio_tungstenite::{connect_async, tungstenite::Error as WsError};
        
        let connect_future = connect_async(url);
        let (ws_stream, _response) = tokio::time::timeout(
            self.connect_timeout,
            connect_future
        ).await
        .map_err(|_| SinkError::timeout(self.connect_timeout.as_secs()))?
        .map_err(|e| match e {
            WsError::Io(io_err) => SinkError::connection_failed(
                format!("WebSocket IO error for {}: {}", url, io_err)
            ),
            WsError::ConnectionClosed => SinkError::connection_failed(
                format!("WebSocket connection closed for {}", url)
            ),
            WsError::AlreadyClosed => SinkError::connection_failed(
                format!("WebSocket already closed for {}", url)
            ),
            other => SinkError::connection_failed(
                format!("WebSocket error for {}: {}", url, other)
            ),
        })?;
        
        Ok(Connection::WebSocket(ws_stream))
    }
    
    /// Connect via Unix socket
    async fn connect_unix(&self, path: &str) -> Result<Connection, SinkError> {
        let stream = tokio::time::timeout(
            self.connect_timeout,
            tokio::net::UnixStream::connect(path)
        ).await
        .map_err(|_| SinkError::timeout(self.connect_timeout.as_secs()))?
        .map_err(|e| SinkError::connection_failed(
            format!("Unix socket connection to {} failed: {}", path, e)
        ))?;
        
        Ok(Connection::Unix(stream))
    }
    
    /// Send message through established connection
    async fn send_message(&self, message: Message) -> Result<(), SinkError> {
        self.ensure_connection().await?;
        
        let mut connection = self.connection.write().await;
        if let Some(conn) = connection.as_mut() {
            match self.write_message(conn, &message).await {
                Ok(bytes_written) => {
                    self.messages_sent.fetch_add(1, Ordering::Relaxed);
                    self.bytes_sent.fetch_add(bytes_written as u64, Ordering::Relaxed);
                    
                    // Update last successful send
                    {
                        let mut last_send = self.last_successful_send.write().await;
                        *last_send = Some(SystemTime::now());
                    }
                    
                    Ok(())
                }
                Err(e) => {
                    self.messages_failed.fetch_add(1, Ordering::Relaxed);
                    
                    // Connection might be lost, mark as disconnected
                    self.connected.store(false, Ordering::Relaxed);
                    {
                        let mut state = self.state.write().await;
                        *state = ConnectionState::Failed;
                    }
                    *connection = None;
                    
                    Err(e)
                }
            }
        } else {
            Err(SinkError::connection_failed("No connection available"))
        }
    }
    
    /// Write a message to the connection
    async fn write_message(&self, connection: &mut Connection, message: &Message) -> Result<usize, SinkError> {
        // Protocol: [length: 4 bytes][payload]
        let payload_len = message.payload.len() as u32;
        let length_bytes = payload_len.to_le_bytes();
        
        // Write length header
        connection.write_all(&length_bytes).await
            .map_err(|e| SinkError::send_failed_with_context(
                format!("Failed to write length header: {}", e),
                crate::SendContext::new(message.payload.len(), crate::fast_timestamp_ns())
            ))?;
        
        // Write payload
        connection.write_all(&message.payload).await
            .map_err(|e| SinkError::send_failed_with_context(
                format!("Failed to write payload: {}", e),
                crate::SendContext::new(message.payload.len(), crate::fast_timestamp_ns())
            ))?;
        
        // Flush to ensure data is sent
        connection.flush().await
            .map_err(|e| SinkError::send_failed_with_context(
                format!("Failed to flush connection: {}", e),
                crate::SendContext::new(message.payload.len(), crate::fast_timestamp_ns())
            ))?;
        
        Ok(4 + message.payload.len())
    }
}

#[async_trait]
impl MessageSink for DirectSink {
    async fn send(&self, message: Message) -> Result<(), SinkError> {
        self.send_message(message).await
    }
    
    async fn send_batch(&self, messages: Vec<Message>) -> Result<BatchResult, SinkError> {
        let mut result = BatchResult::new(messages.len());
        
        for (index, message) in messages.into_iter().enumerate() {
            match self.send_message(message).await {
                Ok(()) => result.record_success(),
                Err(e) => result.record_failure(index, e),
            }
        }
        
        Ok(result)
    }
    
    fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed)
    }
    
    async fn connect(&self) -> Result<(), SinkError> {
        self.ensure_connection().await
    }
    
    async fn disconnect(&self) -> Result<(), SinkError> {
        let _guard = self.connection_mutex.lock().await;
        
        // Close connection
        {
            let mut connection = self.connection.write().await;
            if let Some(conn) = connection.take() {
                drop(conn); // Connection's Drop impl closes it
            }
        }
        
        // Update state
        {
            let mut state = self.state.write().await;
            *state = ConnectionState::Disconnected;
        }
        
        {
            let mut connected_at = self.connected_at.write().await;
            *connected_at = None;
        }
        
        self.connected.store(false, Ordering::Relaxed);
        
        tracing::info!("Direct sink disconnected: {}", self.name);
        Ok(())
    }
    
    fn metadata(&self) -> SinkMetadata {
        let messages_sent = self.messages_sent.load(Ordering::Relaxed);
        let messages_failed = self.messages_failed.load(Ordering::Relaxed);
        let state = if self.connected.load(Ordering::Relaxed) {
            ConnectionState::Connected
        } else {
            ConnectionState::Disconnected
        };
        
        let endpoint = match &self.connection_type {
            ConnectionType::Tcp(addr) => format!("tcp://{}", addr),
            ConnectionType::WebSocket(url) => url.clone(),
            ConnectionType::Unix(path) => format!("unix://{}", path),
        };
        
        SinkMetadata {
            name: self.name.clone(),
            sink_type: "direct".to_string(),
            endpoint: Some(endpoint),
            state,
            messages_sent,
            messages_failed,
            last_error: None,
        }
    }
    
    fn extended_metadata(&self) -> ExtendedSinkMetadata {
        let metadata = self.metadata();
        let last_send = {
            let last_send = self.last_successful_send.try_read().ok();
            last_send.and_then(|ls| *ls)
        };
        
        let bytes_sent = self.bytes_sent.load(Ordering::Relaxed);
        let total_messages = metadata.messages_sent + metadata.messages_failed;
        let error_rate = if total_messages > 0 {
            Some(metadata.messages_failed as f64 / total_messages as f64)
        } else {
            Some(0.0)
        };
        
        let avg_latency = match &self.connection_type {
            ConnectionType::Unix(_) => Some(1000), // Low latency for Unix sockets
            ConnectionType::Tcp(_) => Some(50000), // Higher latency for TCP
            ConnectionType::WebSocket(_) => Some(75000), // Highest latency for WebSocket
        };
        
        ExtendedSinkMetadata {
            metadata,
            health: self.connection_health(),
            last_successful_send: last_send,
            avg_latency_ns: avg_latency,
            error_rate,
            active_connections: if self.is_connected() { 1 } else { 0 },
            preferred_connections: 1,
            supports_multiplexing: self.supports_multiplexing(),
        }
    }
    
    fn connection_health(&self) -> ConnectionHealth {
        if self.connected.load(Ordering::Relaxed) {
            ConnectionHealth::Healthy
        } else {
            ConnectionHealth::Degraded
        }
    }
    
    fn last_successful_send(&self) -> Option<SystemTime> {
        let last_send = self.last_successful_send.try_read().ok()?;
        *last_send
    }
    
    fn preferred_connection_count(&self) -> usize {
        1 // Direct connections typically use single connections
    }
    
    fn supports_multiplexing(&self) -> bool {
        match &self.connection_type {
            ConnectionType::Tcp(_) => true,
            ConnectionType::WebSocket(_) => true,
            ConnectionType::Unix(_) => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_direct_sink_creation() {
        let tcp_sink = DirectSink::tcp("localhost:8080").await.unwrap();
        assert!(matches!(tcp_sink.connection_type(), ConnectionType::Tcp(_)));
        assert!(!tcp_sink.is_connected());
        
        let ws_sink = DirectSink::websocket("ws://localhost:8080").await.unwrap();
        assert!(matches!(ws_sink.connection_type(), ConnectionType::WebSocket(_)));
        
        let unix_sink = DirectSink::unix("/tmp/test.sock").await.unwrap();
        assert!(matches!(unix_sink.connection_type(), ConnectionType::Unix(_)));
    }
    
    #[tokio::test]
    async fn test_invalid_websocket_url() {
        let result = DirectSink::websocket("http://localhost:8080").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must start with ws://"));
    }
    
    #[tokio::test]
    async fn test_empty_unix_path() {
        let result = DirectSink::unix("").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));
    }
    
    #[tokio::test]
    async fn test_connection_failure() {
        let sink = DirectSink::tcp("nonexistent:9999").await.unwrap();
        
        let message = Message::new_unchecked(b"test".to_vec());
        let result = sink.send(message).await;
        
        assert!(result.is_err());
        assert!(!sink.is_connected());
    }
    
    #[tokio::test]
    async fn test_metadata() {
        let sink = DirectSink::tcp("localhost:8080").await.unwrap();
        
        let metadata = sink.metadata();
        assert_eq!(metadata.sink_type, "direct");
        assert_eq!(metadata.endpoint, Some("tcp://localhost:8080".to_string()));
        assert_eq!(metadata.state, ConnectionState::Disconnected);
        
        let ext_metadata = sink.extended_metadata();
        assert_eq!(ext_metadata.health, ConnectionHealth::Degraded);
        assert!(ext_metadata.avg_latency_ns.is_some());
    }
    
    #[tokio::test]
    async fn test_timeout_configuration() {
        let sink = DirectSink::tcp("localhost:8080")
            .await
            .unwrap()
            .with_timeout(Duration::from_millis(100));
        
        assert_eq!(sink.connect_timeout, Duration::from_millis(100));
    }
}