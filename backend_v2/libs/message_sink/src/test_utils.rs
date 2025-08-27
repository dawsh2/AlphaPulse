use crate::{
    ConnectionHealth, ConnectionState, ExtendedSinkMetadata, Message, MessageSink, 
    SendContext, SinkError, SinkMetadata,
};
use async_trait::async_trait;
use std::collections::VecDeque;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc, Mutex,
};
use std::time::SystemTime;

/// A sink that just collects messages for testing with bounded storage
#[derive(Debug)]
pub struct CollectorSink {
    /// Bounded message queue to prevent memory leaks
    messages: Arc<Mutex<VecDeque<Message>>>,
    /// Maximum number of messages to store
    max_messages: usize,
    connected: AtomicBool,
    fail_on_send: AtomicBool,
    messages_sent: AtomicU64,
    messages_failed: AtomicU64,
    name: String,
    last_successful_send: Arc<Mutex<Option<SystemTime>>>,
    health: Arc<Mutex<ConnectionHealth>>,
}

impl CollectorSink {
    /// Create a new collector sink with default capacity
    pub fn new() -> Self {
        Self::with_capacity(1000)
    }

    /// Create a new collector sink with specific capacity
    pub fn with_capacity(max_messages: usize) -> Self {
        Self {
            messages: Arc::new(Mutex::new(VecDeque::with_capacity(max_messages))),
            max_messages,
            connected: AtomicBool::new(false),
            fail_on_send: AtomicBool::new(false),
            messages_sent: AtomicU64::new(0),
            messages_failed: AtomicU64::new(0),
            name: "test-collector".to_string(),
            last_successful_send: Arc::new(Mutex::new(None)),
            health: Arc::new(Mutex::new(ConnectionHealth::Unknown)),
        }
    }

    /// Create a new collector sink with a name
    pub fn with_name(name: impl Into<String>) -> Self {
        let mut sink = Self::new();
        sink.name = name.into();
        sink
    }

    /// Create a new collector sink with name and capacity
    pub fn with_name_and_capacity(name: impl Into<String>, max_messages: usize) -> Self {
        let mut sink = Self::with_capacity(max_messages);
        sink.name = name.into();
        sink
    }

    /// Get all received messages
    pub fn received_messages(&self) -> Vec<Message> {
        self.messages.lock().unwrap().iter().cloned().collect()
    }

    /// Get the count of received messages
    pub fn message_count(&self) -> usize {
        self.messages.lock().unwrap().len()
    }

    /// Clear all received messages
    pub fn clear_messages(&self) {
        self.messages.lock().unwrap().clear();
    }

    /// Get current capacity
    pub fn capacity(&self) -> usize {
        self.max_messages
    }

    /// Check if at capacity
    pub fn is_at_capacity(&self) -> bool {
        self.message_count() >= self.max_messages
    }

    /// Configure to fail on next send
    pub fn fail_next_send(&self) {
        self.fail_on_send.store(true, Ordering::Relaxed);
    }

    /// Force connect state (for testing)
    pub fn force_connect(&self) {
        self.connected.store(true, Ordering::Relaxed);
    }

    /// Force disconnect state (for testing)
    pub fn force_disconnect(&self) {
        self.connected.store(false, Ordering::Relaxed);
    }
}

impl Default for CollectorSink {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MessageSink for CollectorSink {
    async fn send(&self, message: Message) -> Result<(), SinkError> {
        if !self.is_connected() {
            self.messages_failed.fetch_add(1, Ordering::Relaxed);
            let timestamp = alphapulse_transport::safe_system_timestamp_ns_checked()
                .unwrap_or_else(|e| {
                    tracing::error!("Timestamp error in test: {}", e);
                    0
                });
            let context = SendContext::new(message.size(), timestamp)
                .with_correlation_id(
                    message.metadata.correlation_id.unwrap_or_else(|| "test".to_string())
                );
            return Err(SinkError::send_failed_with_context("Not connected", context));
        }

        if self.fail_on_send.swap(false, Ordering::Relaxed) {
            self.messages_failed.fetch_add(1, Ordering::Relaxed);
            let timestamp = alphapulse_transport::safe_system_timestamp_ns_checked()
                .unwrap_or_else(|e| {
                    tracing::error!("Timestamp error in test: {}", e);
                    0
                });
            let context = SendContext::new(message.size(), timestamp)
                .with_correlation_id(
                    message.metadata.correlation_id.unwrap_or_else(|| "test".to_string())
                );
            return Err(SinkError::send_failed_with_context("Simulated failure", context));
        }

        // Check capacity and drop oldest if at limit
        {
            let mut messages = self.messages.lock().unwrap();
            if messages.len() >= self.max_messages {
                messages.pop_front(); // Drop oldest message
            }
            messages.push_back(message);
        }

        self.messages_sent.fetch_add(1, Ordering::Relaxed);
        *self.last_successful_send.lock().unwrap() = Some(SystemTime::now());
        *self.health.lock().unwrap() = ConnectionHealth::Healthy;
        
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed)
    }

    async fn connect(&self) -> Result<(), SinkError> {
        self.connected.store(true, Ordering::Relaxed);
        Ok(())
    }

    async fn disconnect(&self) -> Result<(), SinkError> {
        self.connected.store(false, Ordering::Relaxed);
        Ok(())
    }

    fn metadata(&self) -> SinkMetadata {
        SinkMetadata {
            name: self.name.clone(),
            sink_type: "collector".to_string(),
            endpoint: Some("memory://test".to_string()),
            state: if self.is_connected() {
                ConnectionState::Connected
            } else {
                ConnectionState::Disconnected
            },
            messages_sent: self.messages_sent.load(Ordering::Relaxed),
            messages_failed: self.messages_failed.load(Ordering::Relaxed),
            last_error: None,
        }
    }

    fn extended_metadata(&self) -> ExtendedSinkMetadata {
        ExtendedSinkMetadata {
            metadata: self.metadata(),
            health: self.health.lock().unwrap().clone(),
            last_successful_send: *self.last_successful_send.lock().unwrap(),
            avg_latency_ns: Some(1000), // Simulated 1μs latency
            error_rate: {
                let sent = self.messages_sent.load(Ordering::Relaxed);
                let failed = self.messages_failed.load(Ordering::Relaxed);
                if sent + failed > 0 {
                    Some(failed as f64 / (sent + failed) as f64)
                } else {
                    Some(0.0)
                }
            },
            active_connections: if self.is_connected() { 1 } else { 0 },
            preferred_connections: 1,
            supports_multiplexing: false,
        }
    }

    fn connection_health(&self) -> ConnectionHealth {
        self.health.lock().unwrap().clone()
    }

    fn last_successful_send(&self) -> Option<SystemTime> {
        *self.last_successful_send.lock().unwrap()
    }
}

/// A sink that always fails for testing error conditions
#[derive(Debug)]
pub struct FailingSink {
    error_message: String,
}

impl FailingSink {
    pub fn new(error_message: impl Into<String>) -> Self {
        Self {
            error_message: error_message.into(),
        }
    }
}

impl Default for FailingSink {
    fn default() -> Self {
        Self::new("Simulated failure")
    }
}

#[async_trait]
impl MessageSink for FailingSink {
    async fn send(&self, message: Message) -> Result<(), SinkError> {
        let timestamp = alphapulse_transport::safe_system_timestamp_ns_checked()
            .unwrap_or_else(|e| {
                tracing::error!("Timestamp error in test: {}", e);
                0
            });
        let context = SendContext::new(message.size(), timestamp)
            .with_correlation_id(
                message.metadata.correlation_id.unwrap_or_else(|| "test".to_string())
            );
        Err(SinkError::send_failed_with_context(&self.error_message, context))
    }

    fn is_connected(&self) -> bool {
        false
    }

    async fn connect(&self) -> Result<(), SinkError> {
        Err(SinkError::connection_failed(&self.error_message))
    }

    async fn disconnect(&self) -> Result<(), SinkError> {
        Ok(())
    }

    fn metadata(&self) -> SinkMetadata {
        SinkMetadata {
            name: "failing-sink".to_string(),
            sink_type: "test-failing".to_string(),
            endpoint: None,
            state: ConnectionState::Failed,
            messages_sent: 0,
            messages_failed: 0,
            last_error: Some(self.error_message.clone()),
        }
    }

    fn connection_health(&self) -> ConnectionHealth {
        ConnectionHealth::Unhealthy
    }
}

/// A sink that simulates slow operations for testing timeouts
#[derive(Debug)]
pub struct SlowSink {
    delay_ms: u64,
    connected: AtomicBool,
}

impl SlowSink {
    pub fn new(delay_ms: u64) -> Self {
        Self {
            delay_ms,
            connected: AtomicBool::new(false),
        }
    }
}

#[async_trait]
impl MessageSink for SlowSink {
    async fn send(&self, _message: Message) -> Result<(), SinkError> {
        if !self.is_connected() {
            return Err(SinkError::connection_failed("Not connected"));
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(self.delay_ms)).await;
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed)
    }

    async fn connect(&self) -> Result<(), SinkError> {
        tokio::time::sleep(tokio::time::Duration::from_millis(self.delay_ms)).await;
        self.connected.store(true, Ordering::Relaxed);
        Ok(())
    }

    async fn disconnect(&self) -> Result<(), SinkError> {
        self.connected.store(false, Ordering::Relaxed);
        Ok(())
    }

    fn metadata(&self) -> SinkMetadata {
        SinkMetadata {
            name: "slow-sink".to_string(),
            sink_type: "test-slow".to_string(),
            endpoint: Some(format!("slow://{}ms", self.delay_ms)),
            state: if self.is_connected() {
                ConnectionState::Connected
            } else {
                ConnectionState::Disconnected
            },
            messages_sent: 0,
            messages_failed: 0,
            last_error: None,
        }
    }
}