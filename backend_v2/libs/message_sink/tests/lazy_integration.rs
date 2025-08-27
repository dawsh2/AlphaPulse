//! Integration tests for LazyMessageSink
//!
//! Tests the lazy connection wrapper in various scenarios including:
//! - Lazy connection establishment
//! - Thread-safety under concurrent load
//! - Retry logic with exponential backoff
//! - Auto-reconnection on connection loss
//! - Connection pooling behavior

use alphapulse_message_sink::{
    test_utils::{CollectorSink, DelayedConnectionSink, FailingSink, SlowSink},
    LazyConfig, LazyConnectionState, LazyMessageSink, Message, MessageSink, SinkError,
};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::timeout;

#[tokio::test]
async fn test_lazy_connection_on_first_send() {
    // Test that connection only happens on first send, not during construction
    let connect_count = Arc::new(AtomicU32::new(0));
    let count_clone = connect_count.clone();

    let lazy = LazyMessageSink::with_name(
        move || {
            count_clone.fetch_add(1, Ordering::Relaxed);
            async move {
                let sink = CollectorSink::with_name("test-collector");
                sink.force_connect(); // Pre-connect for testing
                Ok(sink)
            }
        },
        LazyConfig::default(),
        "test-lazy",
    );

    // Initially should not be connected
    assert!(!lazy.is_connected());
    assert_eq!(
        lazy.connection_state().await,
        LazyConnectionState::Disconnected
    );
    assert_eq!(connect_count.load(Ordering::Relaxed), 0);

    // First send should trigger connection
    let message = Message::new_unchecked(b"test message".to_vec());
    lazy.send(message).await.unwrap();

    // Should now be connected
    assert!(lazy.is_connected());
    assert_eq!(
        lazy.connection_state().await,
        LazyConnectionState::Connected
    );
    assert_eq!(connect_count.load(Ordering::Relaxed), 1);

    // Second send should not trigger another connection
    let message2 = Message::new_unchecked(b"second message".to_vec());
    lazy.send(message2).await.unwrap();
    assert_eq!(connect_count.load(Ordering::Relaxed), 1);

    // Verify both messages were sent
    let metrics = lazy.lazy_metrics();
    assert_eq!(metrics.messages_sent.load(Ordering::Relaxed), 2);
    assert_eq!(metrics.messages_failed.load(Ordering::Relaxed), 0);
    assert_eq!(metrics.successful_connects.load(Ordering::Relaxed), 1);
}

#[tokio::test]
async fn test_concurrent_connection_attempts() {
    // Test that multiple threads don't create multiple connections
    let connect_count = Arc::new(AtomicU32::new(0));
    let count_clone = connect_count.clone();

    let lazy = Arc::new(LazyMessageSink::with_name(
        move || {
            count_clone.fetch_add(1, Ordering::Relaxed);
            async move {
                // Add small delay to increase chance of race condition
                tokio::time::sleep(Duration::from_millis(10)).await;
                let sink = CollectorSink::with_name("concurrent-test");
                sink.force_connect();
                Ok(sink)
            }
        },
        LazyConfig::default(),
        "concurrent-test",
    ));

    // Spawn multiple concurrent sends
    let mut handles = vec![];
    for i in 0..10 {
        let lazy_clone = lazy.clone();
        handles.push(tokio::spawn(async move {
            let message = Message::new_unchecked(format!("msg{}", i).into_bytes());
            lazy_clone.send(message).await
        }));
    }

    // Wait for all sends to complete
    for handle in handles {
        handle.await.unwrap().unwrap();
    }

    // Should only have connected once despite concurrent attempts
    assert_eq!(connect_count.load(Ordering::Relaxed), 1);
    assert!(lazy.is_connected());

    // All messages should have been sent successfully
    let metrics = lazy.lazy_metrics();
    assert_eq!(metrics.messages_sent.load(Ordering::Relaxed), 10);
    assert_eq!(metrics.successful_connects.load(Ordering::Relaxed), 1);

    // Should have recorded some connection waits
    assert!(metrics.connection_waits.load(Ordering::Relaxed) > 0);
}

#[tokio::test]
async fn test_connection_retry_logic() {
    // Test retry logic with a sink that fails twice then succeeds
    let attempt_count = Arc::new(AtomicU32::new(0));
    let count_clone = attempt_count.clone();

    let config = LazyConfig {
        max_retries: 3,
        retry_delay: Duration::from_millis(10),
        backoff_multiplier: 2.0,
        ..LazyConfig::default()
    };

    let lazy = LazyMessageSink::with_name(
        move || {
            let attempts = count_clone.fetch_add(1, Ordering::Relaxed);
            async move {
                if attempts < 2 {
                    // Fail first two attempts
                    Err(SinkError::connection_failed(format!(
                        "Attempt {} failed",
                        attempts + 1
                    )))
                } else {
                    // Succeed on third attempt
                    let sink = CollectorSink::with_name("retry-test");
                    sink.force_connect();
                    Ok(sink)
                }
            }
        },
        config,
        "retry-test",
    );

    let start = Instant::now();
    let message = Message::new_unchecked(b"test with retries".to_vec());
    lazy.send(message).await.unwrap();
    let elapsed = start.elapsed();

    // Should have succeeded after 3 attempts
    assert!(lazy.is_connected());
    assert_eq!(attempt_count.load(Ordering::Relaxed), 3);

    // Should have taken time due to retry delays
    assert!(elapsed >= Duration::from_millis(10)); // At least one retry delay

    let metrics = lazy.lazy_metrics();
    assert_eq!(metrics.successful_connects.load(Ordering::Relaxed), 1);
    assert_eq!(metrics.connection_attempts.load(Ordering::Relaxed), 3);
}

#[tokio::test]
async fn test_connection_failure_after_max_retries() {
    // Test that connection fails after exhausting max retries
    let config = LazyConfig {
        max_retries: 2,
        retry_delay: Duration::from_millis(5),
        ..LazyConfig::default()
    };

    let lazy = LazyMessageSink::with_name(
        || async { Err(SinkError::connection_failed("Always fails")) },
        config,
        "fail-test",
    );

    let message = Message::new_unchecked(b"should fail".to_vec());
    let result = lazy.send(message).await;

    assert!(result.is_err());
    assert_eq!(lazy.connection_state().await, LazyConnectionState::Failed);
    assert!(!lazy.is_connected());

    let metrics = lazy.lazy_metrics();
    assert_eq!(metrics.successful_connects.load(Ordering::Relaxed), 0);
    assert_eq!(metrics.failed_connects.load(Ordering::Relaxed), 1);
    assert_eq!(metrics.connection_attempts.load(Ordering::Relaxed), 3); // max_retries + 1
}

#[tokio::test]
async fn test_auto_reconnection() {
    // Test automatic reconnection when connection is lost
    let should_fail = Arc::new(AtomicBool::new(false));
    let fail_clone = should_fail.clone();
    let connect_count = Arc::new(AtomicU32::new(0));
    let count_clone = connect_count.clone();

    let config = LazyConfig {
        auto_reconnect: true,
        max_retries: 1,
        retry_delay: Duration::from_millis(10),
        ..LazyConfig::default()
    };

    let lazy = LazyMessageSink::with_name(
        move || {
            count_clone.fetch_add(1, Ordering::Relaxed);
            let fail_ref = fail_clone.clone();
            async move {
                // Create a sink that can be configured to fail on send
                let sink = if fail_ref.load(Ordering::Relaxed) {
                    // Return a failing sink that will cause reconnection
                    FailingSink::new("Connection lost")
                } else {
                    // Return a working sink (cast to same type)
                    let working = CollectorSink::with_name("reconnect-test");
                    working.force_connect();
                    // We need to return the same type, so we'll use a working collector
                    working
                };
                Ok(sink)
            }
        },
        config,
        "reconnect-test",
    );

    // First send should work
    let message1 = Message::new_unchecked(b"first message".to_vec());
    lazy.send(message1).await.unwrap();
    assert_eq!(connect_count.load(Ordering::Relaxed), 1);

    // Configure next connection to use failing sink
    should_fail.store(true, Ordering::Relaxed);

    // Force disconnect to trigger reconnection
    lazy.force_disconnect().await.unwrap();
    assert_eq!(
        lazy.connection_state().await,
        LazyConnectionState::Disconnected
    );

    // Next send should trigger reconnection, but the new connection will fail
    // In a real scenario, this would be a connection loss during send
    let message2 = Message::new_unchecked(b"second message".to_vec());
    let result = lazy.send(message2).await;

    // The send should fail because we're now using a FailingSink
    // This demonstrates the reconnection attempt occurred
    assert!(result.is_err());

    let metrics = lazy.lazy_metrics();
    assert!(metrics.reconnection_attempts.load(Ordering::Relaxed) > 0);
}

#[tokio::test]
async fn test_connection_timeout() {
    // Test connection timeout handling
    let config = LazyConfig {
        connect_timeout: Duration::from_millis(50),
        max_retries: 1,
        ..LazyConfig::default()
    };

    let lazy = LazyMessageSink::with_name(
        || async {
            // Sleep longer than timeout
            tokio::time::sleep(Duration::from_millis(100)).await;
            let sink = CollectorSink::with_name("timeout-test");
            sink.force_connect();
            Ok(sink)
        },
        config,
        "timeout-test",
    );

    let message = Message::new_unchecked(b"timeout test".to_vec());
    let result = lazy.send(message).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        SinkError::Timeout(_) | SinkError::ConnectionFailed(_) => {
            // Either is acceptable - timeout or connection failure due to timeout
        }
        e => panic!("Expected timeout error, got: {:?}", e),
    }

    assert_eq!(lazy.connection_state().await, LazyConnectionState::Failed);
}

#[tokio::test]
async fn test_batch_operations() {
    // Test that batch operations work correctly with lazy connections
    let connect_count = Arc::new(AtomicU32::new(0));
    let count_clone = connect_count.clone();

    let lazy = LazyMessageSink::with_name(
        move || {
            count_clone.fetch_add(1, Ordering::Relaxed);
            async move {
                let sink = CollectorSink::with_name("batch-test");
                sink.force_connect();
                Ok(sink)
            }
        },
        LazyConfig::default(),
        "batch-test",
    );

    // Prepare batch messages
    let messages = vec![
        Message::new_unchecked(b"msg1".to_vec()),
        Message::new_unchecked(b"msg2".to_vec()),
        Message::new_unchecked(b"msg3".to_vec()),
    ];

    // Should not be connected yet
    assert!(!lazy.is_connected());

    // Batch send should trigger connection
    let result = lazy.send_batch(messages).await.unwrap();

    // Should be connected now and all messages sent
    assert!(lazy.is_connected());
    assert_eq!(connect_count.load(Ordering::Relaxed), 1);
    assert!(result.is_complete_success());
    assert_eq!(result.succeeded, 3);
}

#[tokio::test]
async fn test_metadata_and_health() {
    // Test metadata and health reporting for lazy sinks
    let lazy = LazyMessageSink::with_name(
        || async {
            let sink = CollectorSink::with_name("metadata-test");
            sink.force_connect();
            Ok(sink)
        },
        LazyConfig::default(),
        "metadata-test",
    );

    // Initially should report unknown health
    let health = lazy.connection_health();
    assert_eq!(health, alphapulse_message_sink::ConnectionHealth::Unknown);

    // Metadata should reflect lazy wrapper
    let metadata = lazy.metadata();
    assert!(metadata.name.contains("lazy"));
    assert!(metadata.supports_batching);

    // Extended metadata should include lazy capabilities
    let ext_metadata = lazy.extended_metadata();
    assert!(ext_metadata
        .capabilities
        .contains(&"lazy_connection".to_string()));
    assert!(ext_metadata
        .capabilities
        .contains(&"auto_reconnect".to_string()));

    // Connect and check health again
    let message = Message::new_unchecked(b"health test".to_vec());
    lazy.send(message).await.unwrap();

    // Should now be healthy
    let health = lazy.connection_health();
    assert_eq!(health, alphapulse_message_sink::ConnectionHealth::Healthy);

    // Should have connection uptime
    assert!(lazy.connection_uptime().await.is_some());
}

#[tokio::test]
async fn test_explicit_connect_and_disconnect() {
    // Test explicit connect/disconnect operations
    let connect_count = Arc::new(AtomicU32::new(0));
    let count_clone = connect_count.clone();

    let lazy = LazyMessageSink::with_name(
        move || {
            count_clone.fetch_add(1, Ordering::Relaxed);
            async move {
                let sink = CollectorSink::with_name("explicit-test");
                sink.force_connect();
                Ok(sink)
            }
        },
        LazyConfig::default(),
        "explicit-test",
    );

    // Initially disconnected
    assert!(!lazy.is_connected());
    assert_eq!(
        lazy.connection_state().await,
        LazyConnectionState::Disconnected
    );

    // Explicit connect
    lazy.connect().await.unwrap();
    assert!(lazy.is_connected());
    assert_eq!(
        lazy.connection_state().await,
        LazyConnectionState::Connected
    );
    assert_eq!(connect_count.load(Ordering::Relaxed), 1);

    // Explicit disconnect
    lazy.disconnect().await.unwrap();
    assert!(!lazy.is_connected());
    assert_eq!(
        lazy.connection_state().await,
        LazyConnectionState::Disconnected
    );

    // Sending after disconnect should reconnect
    let message = Message::new_unchecked(b"after disconnect".to_vec());
    lazy.send(message).await.unwrap();
    assert!(lazy.is_connected());
    assert_eq!(connect_count.load(Ordering::Relaxed), 2); // Second connection
}

#[tokio::test]
async fn test_configuration_variants() {
    // Test different configuration variants

    // Fast recovery config
    let fast_config = LazyConfig::fast_recovery();
    let fast_lazy = LazyMessageSink::with_name(
        || async {
            let sink = CollectorSink::with_name("fast-test");
            sink.force_connect();
            Ok(sink)
        },
        fast_config,
        "fast-test",
    );

    let message = Message::new_unchecked(b"fast test".to_vec());
    fast_lazy.send(message).await.unwrap();
    assert!(fast_lazy.is_connected());

    // Conservative config
    let conservative_config = LazyConfig::conservative();
    let conservative_lazy = LazyMessageSink::with_name(
        || async {
            let sink = CollectorSink::with_name("conservative-test");
            sink.force_connect();
            Ok(sink)
        },
        conservative_config,
        "conservative-test",
    );

    let message = Message::new_unchecked(b"conservative test".to_vec());
    conservative_lazy.send(message).await.unwrap();
    assert!(conservative_lazy.is_connected());
}

#[tokio::test]
async fn test_metrics_tracking() {
    // Test comprehensive metrics tracking
    let fail_first = Arc::new(AtomicBool::new(true));
    let fail_clone = fail_first.clone();
    let connect_attempts = Arc::new(AtomicU32::new(0));
    let attempt_clone = connect_attempts.clone();

    let config = LazyConfig {
        max_retries: 2,
        retry_delay: Duration::from_millis(5),
        ..LazyConfig::default()
    };

    let lazy = LazyMessageSink::with_name(
        move || {
            attempt_clone.fetch_add(1, Ordering::Relaxed);
            let should_fail = fail_clone.load(Ordering::Relaxed);
            fail_clone.store(false, Ordering::Relaxed); // Only fail first attempt

            async move {
                if should_fail {
                    Err(SinkError::connection_failed("First attempt fails"))
                } else {
                    let sink = CollectorSink::with_name("metrics-test");
                    sink.force_connect();
                    Ok(sink)
                }
            }
        },
        config,
        "metrics-test",
    );

    // Send message (should succeed after retry)
    let message = Message::new_unchecked(b"metrics test".to_vec());
    lazy.send(message).await.unwrap();

    let metrics = lazy.lazy_metrics();

    // Should have attempted twice (fail then succeed)
    assert_eq!(metrics.connection_attempts.load(Ordering::Relaxed), 2);
    assert_eq!(metrics.successful_connects.load(Ordering::Relaxed), 1);
    assert_eq!(metrics.failed_connects.load(Ordering::Relaxed), 0);
    assert_eq!(metrics.messages_sent.load(Ordering::Relaxed), 1);
    assert_eq!(metrics.messages_failed.load(Ordering::Relaxed), 0);

    // Test success rates
    assert!(metrics.connection_success_rate() > 0.0);
    assert_eq!(metrics.message_success_rate(), 1.0);
}
