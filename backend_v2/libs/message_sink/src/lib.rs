pub mod batch;
pub mod circuit_breaker;
pub mod concurrent;
pub mod config;
pub mod connection;
pub mod error;
pub mod factory;
pub mod lazy;
pub mod message;
pub mod metadata;
pub mod metrics;
pub mod pool;
pub mod registry;
pub mod routing;
pub mod sinks;
pub mod test_utils;
pub mod tlv_types;

use async_trait::async_trait;
use std::fmt::Debug;
use std::time::SystemTime;

pub use torq_network::current_timestamp_ns as fast_timestamp_ns;
pub use batch::BatchResult;
pub use circuit_breaker::{
    CircuitBreakerConfig, CircuitBreakerSink, CircuitBreakerStats, CircuitState,
};
pub use concurrent::{ConcurrentBatchSink, ConcurrentConfig, PipelinedSink};
pub use config::{
    CompositePattern, LazyConfigToml, PrecisionContext, ServiceConfig, ServicesConfig, SinkType,
};
pub use connection::{ConnectionGuard, ConnectionPool};
pub use error::{SendContext, SinkError};
pub use factory::{SinkFactory, SinkFactoryStats};
pub use lazy::{BoxFuture, LazyConfig, LazyConnectionState, LazyMessageSink, LazyMetrics};
pub use message::{Message, MessageMetadata, MessagePriority, DEFAULT_MAX_MESSAGE_SIZE};
pub use metadata::{ConnectionHealth, ConnectionState, ExtendedSinkMetadata, SinkMetadata};
pub use metrics::{
    DefaultSinkMetrics, LatencyMetrics, MetricsFormat, MetricsSnapshot, NoOpMetrics,
    ReliabilityMetrics, ResourceMetrics, SinkMetrics, ThroughputMetrics,
};
pub use pool::{LazyConnectionPool, PoolConfig, PoolStats, PooledSinkGuard};
pub use registry::{RegistryMetadata, ServiceRegistry};
pub use routing::{MessageRouter, RoutingTarget};
pub use sinks::{CompositeMetrics, CompositeSink, ConnectionType, DirectSink, RelaySink};
pub use tlv_types::{MessageDomain, MessageHeader, MessageType, TLVMessage, TLVType};

/// A destination for messages that abstracts away connection details
#[async_trait]
pub trait MessageSink: Send + Sync + Debug {
    /// Send a single message
    async fn send(&self, message: Message) -> Result<(), SinkError>;

    /// Send multiple messages efficiently, returning partial results
    async fn send_batch(&self, messages: Vec<Message>) -> Result<BatchResult, SinkError> {
        let mut result = BatchResult::new(messages.len());

        for (index, message) in messages.into_iter().enumerate() {
            match self.send(message).await {
                Ok(()) => result.record_success(),
                Err(e) => result.record_failure(index, e),
            }
        }

        Ok(result)
    }

    /// Send multiple messages with priority ordering
    async fn send_batch_prioritized(
        &self,
        messages: Vec<Message>,
    ) -> Result<BatchResult, SinkError> {
        let mut sorted_messages: Vec<_> = messages.into_iter().enumerate().collect();
        sorted_messages.sort_by(|(_, a), (_, b)| b.metadata.priority.cmp(&a.metadata.priority));

        let mut result = BatchResult::new(sorted_messages.len());

        for (original_index, message) in sorted_messages {
            match self.send(message).await {
                Ok(()) => result.record_success(),
                Err(e) => result.record_failure(original_index, e),
            }
        }

        Ok(result)
    }

    /// Check if currently connected
    fn is_connected(&self) -> bool;

    /// Establish connection (may be no-op if already connected)
    async fn connect(&self) -> Result<(), SinkError>;

    /// Close connection (may be no-op if not connected)
    async fn disconnect(&self) -> Result<(), SinkError>;

    /// Get sink metadata for debugging/monitoring
    fn metadata(&self) -> SinkMetadata {
        SinkMetadata::default()
    }

    /// Get extended metadata with health monitoring (optional)
    fn extended_metadata(&self) -> ExtendedSinkMetadata {
        ExtendedSinkMetadata::from_metadata(self.metadata())
    }

    /// Get connection health status
    fn connection_health(&self) -> ConnectionHealth {
        self.extended_metadata().health
    }

    /// Get last successful send timestamp
    fn last_successful_send(&self) -> Option<SystemTime> {
        self.extended_metadata().last_successful_send
    }

    /// Get preferred connection count for connection pooling
    fn preferred_connection_count(&self) -> usize {
        1
    }

    /// Check if sink supports connection multiplexing
    fn supports_multiplexing(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{CollectorSink, FailingSink, SlowSink};

    #[tokio::test]
    async fn test_send_requires_connection() {
        let sink = CollectorSink::new();
        let msg = Message::new_unchecked(b"test".to_vec());

        // Should fail when not connected
        let result = sink.send(msg.clone()).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), SinkError::SendFailed { .. }));

        // Should succeed after connecting
        sink.connect().await.unwrap();
        assert!(sink.send(msg).await.is_ok());
        assert_eq!(sink.message_count(), 1);
    }

    #[tokio::test]
    async fn test_batch_send() {
        let sink = CollectorSink::new();
        sink.connect().await.unwrap();

        let messages = vec![
            Message::new_unchecked(b"msg1".to_vec()),
            Message::new_unchecked(b"msg2".to_vec()),
            Message::new_unchecked(b"msg3".to_vec()),
        ];

        let result = sink.send_batch(messages).await.unwrap();
        assert!(result.is_complete_success());
        assert_eq!(result.succeeded, 3);
        assert_eq!(result.failed.len(), 0);
        assert_eq!(sink.message_count(), 3);

        let received = sink.received_messages();
        assert_eq!(received[0].payload, b"msg1");
        assert_eq!(received[1].payload, b"msg2");
        assert_eq!(received[2].payload, b"msg3");
    }

    #[tokio::test]
    async fn test_connection_lifecycle() {
        let sink = CollectorSink::new();

        // Initially disconnected
        assert!(!sink.is_connected());

        // Connect
        sink.connect().await.unwrap();
        assert!(sink.is_connected());

        // Disconnect
        sink.disconnect().await.unwrap();
        assert!(!sink.is_connected());
    }

    #[tokio::test]
    async fn test_metadata() {
        let sink = CollectorSink::with_name("test-sink");
        sink.connect().await.unwrap();

        let metadata = sink.metadata();
        assert_eq!(metadata.name, "test-sink");
        assert_eq!(metadata.sink_type, "collector");
        assert_eq!(metadata.state, ConnectionState::Connected);
        assert_eq!(metadata.messages_sent, 0);
        assert_eq!(metadata.messages_failed, 0);

        // Send a message and check updated metadata
        let msg = Message::new_unchecked(b"test".to_vec());
        sink.send(msg).await.unwrap();

        let metadata = sink.metadata();
        assert_eq!(metadata.messages_sent, 1);
        assert_eq!(metadata.messages_failed, 0);

        // Test extended metadata
        let ext_metadata = sink.extended_metadata();
        assert_eq!(ext_metadata.health, ConnectionHealth::Healthy);
        assert!(ext_metadata.last_successful_send.is_some());
        assert!(ext_metadata.error_rate.unwrap() < 0.1); // Low error rate
    }

    #[tokio::test]
    async fn test_failing_sink() {
        let sink = FailingSink::new("Test failure");

        // Connection should fail
        let result = sink.connect().await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SinkError::ConnectionFailed(_)
        ));

        // Send should fail
        let msg = Message::new_unchecked(b"test".to_vec());
        let result = sink.send(msg).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), SinkError::SendFailed { .. }));

        // Metadata should show failed state
        let metadata = sink.metadata();
        assert_eq!(metadata.state, ConnectionState::Failed);
        assert!(metadata.last_error.is_some());

        // Health should be unhealthy
        assert_eq!(sink.connection_health(), ConnectionHealth::Unhealthy);
    }

    #[tokio::test]
    async fn test_slow_sink() {
        let sink = SlowSink::new(10); // 10ms delay

        let start = std::time::Instant::now();
        sink.connect().await.unwrap();
        let duration = start.elapsed();

        assert!(duration >= std::time::Duration::from_millis(10));
        assert!(sink.is_connected());
    }

    #[test]
    fn test_message_creation() {
        let msg = Message::new(b"test payload".to_vec()).unwrap();
        assert_eq!(msg.payload, b"test payload");
        assert_eq!(msg.metadata.priority, MessagePriority::Normal);
        assert!(msg.metadata.timestamp_ns > 0);

        let metadata = MessageMetadata::new()
            .with_target("test-service")
            .with_priority(MessagePriority::High)
            .with_correlation_id("test-123");

        let msg2 = Message::with_metadata(b"test".to_vec(), metadata.clone()).unwrap();
        assert_eq!(msg2.metadata.target, Some("test-service".to_string()));
        assert_eq!(msg2.metadata.priority, MessagePriority::High);
        assert_eq!(msg2.metadata.correlation_id, Some("test-123".to_string()));
    }

    #[test]
    fn test_message_size_validation() {
        // Should succeed with small message
        let small_msg = Message::new(vec![0u8; 1000]);
        assert!(small_msg.is_ok());

        // Should fail with oversized message using default limit
        let big_msg = Message::new(vec![0u8; DEFAULT_MAX_MESSAGE_SIZE + 1]);
        assert!(big_msg.is_err());
        assert!(matches!(
            big_msg.unwrap_err(),
            SinkError::MessageTooLarge { .. }
        ));

        // Custom limit should work
        let custom_msg = Message::new_with_limit(vec![0u8; 100], 50);
        assert!(custom_msg.is_err());

        let custom_msg2 = Message::new_with_limit(vec![0u8; 50], 100);
        assert!(custom_msg2.is_ok());
    }

    #[tokio::test]
    async fn test_batch_partial_failure() {
        let sink = CollectorSink::new();
        sink.connect().await.unwrap();

        // Configure sink to fail on second message
        let messages = vec![
            Message::new_unchecked(b"msg1".to_vec()),
            Message::new_unchecked(b"msg2".to_vec()),
            Message::new_unchecked(b"msg3".to_vec()),
        ];

        // Send first message successfully
        sink.send(messages[0].clone()).await.unwrap();

        // Configure to fail next send
        sink.fail_next_send();

        // Send batch - should get partial success
        let remaining = vec![messages[1].clone(), messages[2].clone()];
        let result = sink.send_batch(remaining).await.unwrap();

        assert!(!result.is_complete_success());
        assert!(result.has_partial_success());
        assert_eq!(result.succeeded, 1); // msg3 succeeds after msg2 fails
        assert_eq!(result.failed.len(), 1); // msg2 fails
        assert_eq!(result.failed[0].0, 0); // First message in batch (msg2) failed
    }

    #[tokio::test]
    async fn test_priority_batch_send() {
        let sink = CollectorSink::new();
        sink.connect().await.unwrap();

        let messages = vec![
            Message::with_metadata(
                b"low".to_vec(),
                MessageMetadata::new().with_priority(MessagePriority::Low),
            )
            .unwrap(),
            Message::with_metadata(
                b"critical".to_vec(),
                MessageMetadata::new().with_priority(MessagePriority::Critical),
            )
            .unwrap(),
            Message::with_metadata(
                b"normal".to_vec(),
                MessageMetadata::new().with_priority(MessagePriority::Normal),
            )
            .unwrap(),
        ];

        let result = sink.send_batch_prioritized(messages).await.unwrap();
        assert!(result.is_complete_success());

        let received = sink.received_messages();
        // Should be ordered by priority (Critical, Normal, Low)
        assert_eq!(received[0].payload, b"critical");
        assert_eq!(received[1].payload, b"normal");
        assert_eq!(received[2].payload, b"low");
    }

    #[test]
    fn test_collector_sink_bounded_memory() {
        let sink = CollectorSink::with_capacity(2);
        sink.force_connect();

        // Add messages up to capacity
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            sink.send(Message::new_unchecked(b"msg1".to_vec()))
                .await
                .unwrap();
            sink.send(Message::new_unchecked(b"msg2".to_vec()))
                .await
                .unwrap();
            assert_eq!(sink.message_count(), 2);

            // Adding third message should drop the first
            sink.send(Message::new_unchecked(b"msg3".to_vec()))
                .await
                .unwrap();
            assert_eq!(sink.message_count(), 2);

            let messages = sink.received_messages();
            assert_eq!(messages[0].payload, b"msg2"); // msg1 was dropped
            assert_eq!(messages[1].payload, b"msg3");
        });
    }

    #[test]
    fn test_connection_health() {
        let sink = CollectorSink::new();

        // Initially unknown
        assert_eq!(sink.connection_health(), ConnectionHealth::Unknown);

        sink.force_connect();
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            // After successful send, should be healthy
            sink.send(Message::new_unchecked(b"test".to_vec()))
                .await
                .unwrap();
            assert_eq!(sink.connection_health(), ConnectionHealth::Healthy);
        });
    }

    #[test]
    fn test_message_priority_ordering() {
        assert!(MessagePriority::Critical > MessagePriority::High);
        assert!(MessagePriority::High > MessagePriority::Normal);
        assert!(MessagePriority::Normal > MessagePriority::Low);
    }

    #[test]
    fn test_connection_state() {
        assert!(ConnectionState::Connected.is_active());
        assert!(!ConnectionState::Disconnected.is_active());
        assert!(!ConnectionState::Failed.is_active());

        assert!(ConnectionState::Disconnected.can_connect());
        assert!(ConnectionState::Failed.can_connect());
        assert!(!ConnectionState::Connected.can_connect());

        assert!(ConnectionState::Connecting.is_connecting());
        assert!(!ConnectionState::Connected.is_connecting());
    }

    #[test]
    fn test_sink_error_classification() {
        let conn_err = SinkError::connection_failed("test");
        assert!(conn_err.is_connection_error());
        assert!(!conn_err.is_recoverable());

        let timeout_err = SinkError::timeout(30);
        assert!(!timeout_err.is_connection_error());
        assert!(timeout_err.is_recoverable());

        let timestamp = torq_network::safe_system_timestamp_ns_checked().unwrap_or(0);
        let context = SendContext::new(100, timestamp);
        let buffer_err = SinkError::buffer_full_with_context(context);
        assert!(buffer_err.is_recoverable());

        let size_err = SinkError::message_too_large(1000, 500);
        assert!(!size_err.is_recoverable());
    }

    #[test]
    fn test_sink_metadata_builder() {
        let mut metadata = SinkMetadata::new("test-sink", "test-type")
            .with_endpoint("tcp://localhost:8080")
            .with_state(ConnectionState::Connected);

        assert_eq!(metadata.name, "test-sink");
        assert_eq!(metadata.sink_type, "test-type");
        assert_eq!(metadata.endpoint, Some("tcp://localhost:8080".to_string()));
        assert_eq!(metadata.state, ConnectionState::Connected);

        metadata.record_success();
        assert_eq!(metadata.messages_sent, 1);

        metadata.record_failure(Some("Test error".to_string()));
        assert_eq!(metadata.messages_failed, 1);
        assert_eq!(metadata.last_error, Some("Test error".to_string()));
    }
}
