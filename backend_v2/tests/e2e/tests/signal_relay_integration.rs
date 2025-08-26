//! Comprehensive integration tests for Signal Relay
//!
//! Tests the production signal relay for:
//! - Multiple concurrent publishers and consumers
//! - Message broadcasting correctness (no echo back to sender)
//! - Reconnection resilience
//! - TLV message integrity
//! - Performance under load

use protocol_v2::{
    message::header::MessageHeader,
    tlv::{arbitrage_signal::ArbitrageSignalTLV, builder::TLVMessageBuilder, types::TLVType},
    RelayDomain, SourceType, MESSAGE_MAGIC,
};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use tokio::sync::mpsc;
use tokio::time::timeout;
use zerocopy::AsBytes;

const SIGNAL_RELAY_PATH: &str = "/tmp/alphapulse/signals.sock";
const TEST_TIMEOUT: Duration = Duration::from_secs(10);

/// Helper to create a test ArbitrageSignalTLV
fn create_test_arbitrage_signal(signal_id: u64) -> Vec<u8> {
    let mut builder = TLVMessageBuilder::new(RelayDomain::Signal, SourceType::ArbitrageStrategy);

    let signal = ArbitrageSignalTLV {
        strategy_id: 21, // Flash arbitrage
        signal_id,
        chain_id: 137, // Polygon
        source_pool: [0x01; 20],
        target_pool: [0x02; 20],
        source_venue: 300, // UniswapV2
        target_venue: 301, // UniswapV3
        token_in: [0x03; 20],
        token_out: [0x04; 20],
        expected_profit_usd_q8: 150_000_000,      // $1.50
        required_capital_usd_q8: 100_000_000_000, // $1000
        spread_bps: 150,                          // 1.5%
        dex_fees_usd_q8: 10_000_000,              // $0.10
        gas_cost_usd_q8: 5_000_000,               // $0.05
        slippage_usd_q8: 5_000_000,               // $0.05
        net_profit_usd_q8: 130_000_000,           // $1.30
        slippage_tolerance_bps: 50,               // 0.5%
        max_gas_price_gwei: 30,
        valid_until: (SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + 60) as u32,
        priority: 100,
        reserved: [0; 2],
        timestamp_ns: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64,
    };

    builder.add_tlv(TLVType::ArbitrageSignal, signal.as_bytes());
    builder.build()
}

#[tokio::test]
async fn test_basic_message_forwarding() {
    println!("Testing basic message forwarding");

    // Connect as consumer 1
    let mut consumer1 = UnixStream::connect(SIGNAL_RELAY_PATH)
        .await
        .expect("Failed to connect consumer 1");

    // Connect as consumer 2
    let mut consumer2 = UnixStream::connect(SIGNAL_RELAY_PATH)
        .await
        .expect("Failed to connect consumer 2");

    // Connect as publisher
    let mut publisher = UnixStream::connect(SIGNAL_RELAY_PATH)
        .await
        .expect("Failed to connect publisher");

    // Allow connections to establish
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Send message from publisher
    let test_message = create_test_arbitrage_signal(1);
    publisher
        .write_all(&test_message)
        .await
        .expect("Failed to send message");

    // Consumer 1 should receive the message
    let mut buf1 = vec![0u8; 1024];
    let n1 = timeout(TEST_TIMEOUT, consumer1.read(&mut buf1))
        .await
        .expect("Consumer 1 timeout")
        .expect("Consumer 1 read failed");
    assert_eq!(n1, test_message.len());
    assert_eq!(&buf1[..n1], &test_message[..]);

    // Consumer 2 should receive the message
    let mut buf2 = vec![0u8; 1024];
    let n2 = timeout(TEST_TIMEOUT, consumer2.read(&mut buf2))
        .await
        .expect("Consumer 2 timeout")
        .expect("Consumer 2 read failed");
    assert_eq!(n2, test_message.len());
    assert_eq!(&buf2[..n2], &test_message[..]);

    println!("✅ Basic message forwarding test passed");
}

#[tokio::test]
async fn test_no_echo_back_to_sender() {
    println!("Testing no echo back to sender");

    // Connect two clients
    let mut client1 = UnixStream::connect(SIGNAL_RELAY_PATH)
        .await
        .expect("Failed to connect client 1");

    let mut client2 = UnixStream::connect(SIGNAL_RELAY_PATH)
        .await
        .expect("Failed to connect client 2");

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Send from client 1
    let message1 = create_test_arbitrage_signal(100);
    client1
        .write_all(&message1)
        .await
        .expect("Failed to send from client 1");

    // Client 2 should receive it
    let mut buf2 = vec![0u8; 1024];
    let n2 = timeout(Duration::from_secs(1), client2.read(&mut buf2))
        .await
        .expect("Client 2 timeout")
        .expect("Client 2 read failed");
    assert_eq!(n2, message1.len());

    // Client 1 should NOT receive its own message
    let mut buf1 = vec![0u8; 1024];
    let result = timeout(Duration::from_millis(500), client1.read(&mut buf1)).await;
    assert!(
        result.is_err(),
        "Client 1 should not receive its own message"
    );

    println!("✅ No echo back to sender test passed");
}

#[tokio::test]
async fn test_concurrent_publishers() {
    println!("Testing concurrent publishers");

    const NUM_PUBLISHERS: usize = 5;
    const MESSAGES_PER_PUBLISHER: usize = 10;

    // Connect a consumer
    let mut consumer = UnixStream::connect(SIGNAL_RELAY_PATH)
        .await
        .expect("Failed to connect consumer");

    // Create multiple publishers
    let mut publisher_handles = vec![];

    for publisher_id in 0..NUM_PUBLISHERS {
        let handle = tokio::spawn(async move {
            let mut publisher = UnixStream::connect(SIGNAL_RELAY_PATH)
                .await
                .expect("Failed to connect publisher");

            for msg_id in 0..MESSAGES_PER_PUBLISHER {
                let signal_id = (publisher_id * 1000 + msg_id) as u64;
                let message = create_test_arbitrage_signal(signal_id);
                publisher
                    .write_all(&message)
                    .await
                    .expect("Failed to send message");
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        });
        publisher_handles.push(handle);
    }

    // Receive all messages
    let mut received_count = 0;
    let expected_total = NUM_PUBLISHERS * MESSAGES_PER_PUBLISHER;
    let mut buffer = vec![0u8; 65536];

    while received_count < expected_total {
        match timeout(Duration::from_secs(5), consumer.read(&mut buffer)).await {
            Ok(Ok(n)) if n > 0 => {
                received_count += 1;
                println!("Received message {}/{}", received_count, expected_total);
            }
            _ => break,
        }
    }

    // Wait for all publishers to complete
    for handle in publisher_handles {
        handle.await.expect("Publisher task failed");
    }

    assert_eq!(
        received_count, expected_total,
        "Should receive all messages from concurrent publishers"
    );

    println!("✅ Concurrent publishers test passed");
}

#[tokio::test]
async fn test_reconnection_resilience() {
    println!("Testing reconnection resilience");

    // Initial consumer
    let mut consumer1 = UnixStream::connect(SIGNAL_RELAY_PATH)
        .await
        .expect("Failed to connect consumer 1");

    // Publisher
    let mut publisher = UnixStream::connect(SIGNAL_RELAY_PATH)
        .await
        .expect("Failed to connect publisher");

    // Send initial message
    let message1 = create_test_arbitrage_signal(1000);
    publisher
        .write_all(&message1)
        .await
        .expect("Failed to send message 1");

    // Consumer 1 receives it
    let mut buf = vec![0u8; 1024];
    consumer1
        .read(&mut buf)
        .await
        .expect("Consumer 1 read failed");

    // Consumer 1 disconnects
    drop(consumer1);
    tokio::time::sleep(Duration::from_millis(100)).await;

    // New consumer connects
    let mut consumer2 = UnixStream::connect(SIGNAL_RELAY_PATH)
        .await
        .expect("Failed to connect consumer 2");

    // Send another message
    let message2 = create_test_arbitrage_signal(2000);
    publisher
        .write_all(&message2)
        .await
        .expect("Failed to send message 2");

    // Consumer 2 should receive it
    let n = timeout(Duration::from_secs(1), consumer2.read(&mut buf))
        .await
        .expect("Consumer 2 timeout")
        .expect("Consumer 2 read failed");
    assert!(
        n > 0,
        "Consumer 2 should receive message after reconnection"
    );

    println!("✅ Reconnection resilience test passed");
}

#[tokio::test]
async fn test_tlv_message_integrity() {
    println!("Testing TLV message integrity");

    // Connect publisher and consumer
    let mut publisher = UnixStream::connect(SIGNAL_RELAY_PATH)
        .await
        .expect("Failed to connect publisher");

    let mut consumer = UnixStream::connect(SIGNAL_RELAY_PATH)
        .await
        .expect("Failed to connect consumer");

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Send multiple ArbitrageSignal messages
    for i in 0..5 {
        let message = create_test_arbitrage_signal(i + 1);
        publisher
            .write_all(&message)
            .await
            .expect("Failed to send message");
    }

    // Receive and validate each message
    for i in 0..5 {
        let mut buffer = vec![0u8; 1024];
        let n = timeout(Duration::from_secs(1), consumer.read(&mut buffer))
            .await
            .expect("Timeout receiving message")
            .expect("Failed to read message");

        // Parse header
        assert!(n >= 32, "Message too short for header");
        let header = &buffer[..32];
        let magic = u32::from_le_bytes([header[0], header[1], header[2], header[3]]);
        assert_eq!(magic, MESSAGE_MAGIC, "Invalid message magic");

        let domain = header[4];
        assert_eq!(domain, RelayDomain::Signal as u8, "Should be Signal domain");

        let source = header[5];
        assert_eq!(
            source,
            SourceType::ArbitrageStrategy as u8,
            "Should be ArbitrageStrategy source"
        );

        // Verify we have TLV payload
        let payload_size = u32::from_le_bytes([header[24], header[25], header[26], header[27]]);
        assert!(payload_size > 0, "Should have TLV payload");
        assert_eq!(n, 32 + payload_size as usize, "Message size mismatch");

        println!("Message {} integrity verified", i + 1);
    }

    println!("✅ TLV message integrity test passed");
}

#[tokio::test]
async fn test_high_throughput() {
    println!("Testing high throughput message handling");

    const NUM_MESSAGES: usize = 1000;

    // Connect publisher and consumer
    let mut publisher = UnixStream::connect(SIGNAL_RELAY_PATH)
        .await
        .expect("Failed to connect publisher");

    let mut consumer = UnixStream::connect(SIGNAL_RELAY_PATH)
        .await
        .expect("Failed to connect consumer");

    tokio::time::sleep(Duration::from_millis(100)).await;

    let start = std::time::Instant::now();

    // Publisher task
    let publisher_handle = tokio::spawn(async move {
        for i in 0..NUM_MESSAGES {
            let message = create_test_arbitrage_signal(i as u64);
            publisher
                .write_all(&message)
                .await
                .expect("Failed to send message");
        }
    });

    // Consumer task
    let consumer_handle = tokio::spawn(async move {
        let mut received = 0;
        let mut buffer = vec![0u8; 65536];

        while received < NUM_MESSAGES {
            match timeout(Duration::from_secs(5), consumer.read(&mut buffer)).await {
                Ok(Ok(n)) if n > 0 => {
                    received += 1;
                    if received % 100 == 0 {
                        println!("Received {}/{} messages", received, NUM_MESSAGES);
                    }
                }
                _ => break,
            }
        }

        received
    });

    publisher_handle.await.expect("Publisher task failed");
    let received = consumer_handle.await.expect("Consumer task failed");

    let elapsed = start.elapsed();
    let throughput = NUM_MESSAGES as f64 / elapsed.as_secs_f64();

    println!("Processed {} messages in {:?}", received, elapsed);
    println!("Throughput: {:.0} messages/second", throughput);

    assert_eq!(received, NUM_MESSAGES, "Should receive all messages");
    assert!(throughput > 1000.0, "Should handle >1000 messages/second");

    println!("✅ High throughput test passed");
}

#[tokio::test]
async fn test_large_message_handling() {
    println!("Testing large message handling");

    // Connect publisher and consumer
    let mut publisher = UnixStream::connect(SIGNAL_RELAY_PATH)
        .await
        .expect("Failed to connect publisher");

    let mut consumer = UnixStream::connect(SIGNAL_RELAY_PATH)
        .await
        .expect("Failed to connect consumer");

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Create a large message with multiple TLV extensions
    let mut builder = TLVMessageBuilder::new(RelayDomain::Signal, SourceType::ArbitrageStrategy);

    // Add multiple ArbitrageSignal TLVs
    for i in 0..10 {
        let signal = ArbitrageSignalTLV {
            strategy_id: 21,
            signal_id: i,
            chain_id: 137,
            source_pool: [i as u8; 20],
            target_pool: [(i + 1) as u8; 20],
            source_venue: 300 + i as u16,
            target_venue: 301 + i as u16,
            token_in: [(i * 2) as u8; 20],
            token_out: [(i * 2 + 1) as u8; 20],
            expected_profit_usd_q8: 100_000_000 * (i + 1) as i64,
            required_capital_usd_q8: 1_000_000_000 * (i + 1) as i64,
            spread_bps: 100 + i as u16 * 10,
            dex_fees_usd_q8: 1_000_000 * (i + 1) as i64,
            gas_cost_usd_q8: 500_000 * (i + 1) as i64,
            slippage_usd_q8: 500_000 * (i + 1) as i64,
            net_profit_usd_q8: 95_000_000 * (i + 1) as i64,
            slippage_tolerance_bps: 25 + i as u16 * 5,
            max_gas_price_gwei: 20 + i as u32,
            valid_until: (SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                + 120) as u32,
            priority: 50 + i as u16 * 10,
            reserved: [0; 2],
            timestamp_ns: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
        };

        builder.add_tlv(TLVType::ArbitrageSignal, signal.as_bytes());
    }

    let large_message = builder.build();
    println!("Sending large message: {} bytes", large_message.len());

    // Send large message
    publisher
        .write_all(&large_message)
        .await
        .expect("Failed to send large message");

    // Receive large message
    let mut buffer = vec![0u8; 65536];
    let n = timeout(Duration::from_secs(1), consumer.read(&mut buffer))
        .await
        .expect("Timeout receiving large message")
        .expect("Failed to read large message");

    assert_eq!(
        n,
        large_message.len(),
        "Should receive complete large message"
    );
    assert_eq!(
        &buffer[..n],
        &large_message[..],
        "Large message content should match"
    );

    println!("✅ Large message handling test passed");
}
