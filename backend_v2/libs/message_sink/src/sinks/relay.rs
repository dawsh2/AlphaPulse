//! Unix socket relay sink for connecting to local relay services
//!
//! RelaySink provides high-performance local connections to relay services using
//! Unix domain sockets. This is the primary communication method between services
//! and the relay infrastructure in AlphaPulse.

use crate::{BatchResult, ConnectionHealth, ConnectionState, ExtendedSinkMetadata, Message, MessageSink, SinkError, SinkMetadata};
use async_trait::async_trait;
use std::collections::VecDeque;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use tokio::sync::{Mutex, RwLock};

/// Unix socket relay sink for connecting to relay services
#[derive(Debug)]
pub struct RelaySink {
    /// Unix socket connection
    connection: Arc<RwLock<Option<UnixStream>>>,
    
    /// Socket path
    socket_path: String,
    
    /// Connection state
    state: Arc<RwLock<ConnectionState>>,
    
    /// Message buffer
    buffer: Arc<Mutex<VecDeque<Message>>>,
    
    /// Buffer size limit
    buffer_size: usize,
    
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
}

impl RelaySink {
    /// Create a new relay sink
    pub fn new(socket_path: impl Into<String>, buffer_size: usize) -> Result<Self, SinkError> {
        let socket_path = socket_path.into();
        
        // Validate socket path
        if socket_path.is_empty() {
            return Err(SinkError::invalid_config("Socket path cannot be empty"));
        }
        
        let name = Path::new(&socket_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("relay")
            .to_string();
        
        Ok(Self {
            connection: Arc::new(RwLock::new(None)),
            socket_path,
            state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
            buffer: Arc::new(Mutex::new(VecDeque::with_capacity(buffer_size))),
            buffer_size,
            connection_mutex: Arc::new(Mutex::new(())),
            messages_sent: AtomicU64::new(0),
            messages_failed: AtomicU64::new(0),
            bytes_sent: AtomicU64::new(0),
            connection_attempts: AtomicU64::new(0),
            last_successful_send: Arc::new(RwLock::new(None)),
            connected_at: Arc::new(RwLock::new(None)),
            name,
            connected: AtomicBool::new(false),
        })
    }
    
    /// Create with default buffer size
    pub fn with_default_buffer(socket_path: impl Into<String>) -> Result<Self, SinkError> {
        Self::new(socket_path, 10000)
    }
    
    /// Get socket path
    pub fn socket_path(&self) -> &str {
        &self.socket_path
    }
    
    /// Get buffer usage
    pub async fn buffer_usage(&self) -> (usize, usize) {
        let buffer = self.buffer.lock().await;
        (buffer.len(), self.buffer_size)
    }
    
    /// Get connection uptime
    pub async fn connection_uptime(&self) -> Option<Duration> {
        let connected_at = self.connected_at.read().await;
        connected_at.map(|t| t.elapsed())
    }
    
    /// Establish connection to relay service
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
        
        tracing::debug!("Connecting to relay at {}", self.socket_path);
        
        match UnixStream::connect(&self.socket_path).await {
            Ok(stream) => {
                // Store connection
                {
                    let mut connection = self.connection.write().await;
                    *connection = Some(stream);
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
                
                tracing::info!("Connected to relay {}", self.socket_path);
                Ok(())
            }
            Err(e) => {
                // Update state to failed
                {
                    let mut state = self.state.write().await;
                    *state = ConnectionState::Failed;
                }
                
                let error = SinkError::connection_failed(
                    format!("Failed to connect to relay {}: {}", self.socket_path, e)
                );
                tracing::error!("Relay connection failed: {}", error);
                Err(error)
            }
        }
    }
    
    /// Send message with buffering
    async fn send_message(&self, message: Message) -> Result<(), SinkError> {
        // Check if buffer is full
        {
            let mut buffer = self.buffer.lock().await;
            if buffer.len() >= self.buffer_size {
                return Err(SinkError::buffer_full_with_context(
                    crate::SendContext::new(
                        message.payload.len(), 
                        crate::fast_timestamp_ns()
                    )
                ));
            }
            buffer.push_back(message.clone());
        }
        
        // Try to flush buffer
        self.flush_buffer().await
    }
    
    /// Flush buffered messages to connection
    async fn flush_buffer(&self) -> Result<(), SinkError> {
        self.ensure_connection().await?;
        
        let mut messages_to_send = Vec::new();
        
        // Take messages from buffer
        {
            let mut buffer = self.buffer.lock().await;
            while let Some(message) = buffer.pop_front() {
                messages_to_send.push(message);
                if messages_to_send.len() >= 100 { // Batch limit
                    break;
                }
            }
        }
        
        if messages_to_send.is_empty() {
            return Ok(());
        }
        
        // Send messages through connection
        let mut connection = self.connection.write().await;
        if let Some(stream) = connection.as_mut() {
            for message in messages_to_send {
                match self.write_message(stream, &message).await {
                    Ok(bytes_written) => {
                        self.messages_sent.fetch_add(1, Ordering::Relaxed);
                        self.bytes_sent.fetch_add(bytes_written as u64, Ordering::Relaxed);
                        
                        // Update last successful send
                        {
                            let mut last_send = self.last_successful_send.write().await;
                            *last_send = Some(SystemTime::now());
                        }
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
                        
                        return Err(e);
                    }
                }
            }
        } else {
            return Err(SinkError::connection_failed("No connection available"));
        }
        
        Ok(())
    }
    
    /// Write a single message to the stream
    async fn write_message(&self, stream: &mut UnixStream, message: &Message) -> Result<usize, SinkError> {
        // Protocol: [length: 4 bytes][payload]
        let payload_len = message.payload.len() as u32;
        let length_bytes = payload_len.to_le_bytes();
        
        // Write length header
        stream.write_all(&length_bytes).await
            .map_err(|e| SinkError::send_failed_with_context(
                format!("Failed to write length header: {}", e),
                crate::SendContext::new(message.payload.len(), crate::fast_timestamp_ns())
            ))?;
        
        // Write payload
        stream.write_all(&message.payload).await
            .map_err(|e| SinkError::send_failed_with_context(
                format!("Failed to write payload: {}", e),
                crate::SendContext::new(message.payload.len(), crate::fast_timestamp_ns())
            ))?;
        
        // Flush to ensure data is sent
        stream.flush().await
            .map_err(|e| SinkError::send_failed_with_context(
                format!("Failed to flush stream: {}", e),
                crate::SendContext::new(message.payload.len(), crate::fast_timestamp_ns())
            ))?;
        
        Ok(4 + message.payload.len())
    }
}

#[async_trait]
impl MessageSink for RelaySink {
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
            if let Some(stream) = connection.take() {
                drop(stream); // UnixStream's Drop impl closes the connection
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
        
        tracing::info!("Disconnected from relay {}", self.socket_path);
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
        
        SinkMetadata {
            name: format!("relay-{}", self.name),
            sink_type: "relay".to_string(),
            endpoint: Some(format!("unix://{}", self.socket_path)),
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
        
        ExtendedSinkMetadata {
            metadata,
            health: self.connection_health(),
            last_successful_send: last_send,
            avg_latency_ns: Some(1000), // Low latency for Unix sockets
            error_rate,
            active_connections: if self.is_connected() { 1 } else { 0 },
            preferred_connections: 1,
            supports_multiplexing: true,
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
        1 // Unix sockets typically use single connections
    }
    
    fn supports_multiplexing(&self) -> bool {
        true // Unix sockets can handle multiplexed messages
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::net::UnixListener;
    
    async fn create_test_relay_server(socket_path: &str) -> UnixListener {
        // Remove socket file if it exists
        let _ = std::fs::remove_file(socket_path);
        
        UnixListener::bind(socket_path).unwrap()
    }
    
    #[tokio::test]
    async fn test_relay_sink_creation() {
        let sink = RelaySink::new("/tmp/test_relay.sock", 1000).unwrap();
        
        assert_eq!(sink.socket_path(), "/tmp/test_relay.sock");
        assert!(!sink.is_connected());
        assert_eq!(sink.buffer_usage().await, (0, 1000));
    }
    
    #[tokio::test]
    async fn test_invalid_socket_path() {
        let result = RelaySink::new("", 1000);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));
    }
    
    #[tokio::test]
    async fn test_connection_failure() {
        let sink = RelaySink::new("/nonexistent/socket.sock", 1000).unwrap();
        
        let message = Message::new_unchecked(b"test".to_vec());
        let result = sink.send(message).await;
        
        assert!(result.is_err());
        assert!(!sink.is_connected());
    }
    
    #[tokio::test]
    async fn test_metadata() {
        let sink = RelaySink::new("/tmp/metadata_test.sock", 1000).unwrap();
        
        let metadata = sink.metadata();
        assert_eq!(metadata.sink_type, "relay");
        assert_eq!(metadata.endpoint, Some("unix:///tmp/metadata_test.sock".to_string()));
        assert_eq!(metadata.state, ConnectionState::Disconnected);
        assert_eq!(metadata.messages_sent, 0);
        
        let ext_metadata = sink.extended_metadata();
        assert_eq!(ext_metadata.health, ConnectionHealth::Degraded);
        assert!(ext_metadata.last_successful_send.is_none());
    }
}