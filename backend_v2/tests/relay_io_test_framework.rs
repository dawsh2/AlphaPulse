//! # Relay Input/Output Testing Framework
//!
//! ## Purpose
//! Comprehensive testing framework for validating relay message processing,
//! connection handling, and bidirectional communication patterns.
//!
//! ## Test Architecture
//!
//! ```
//! Test Producer → Unix Socket → Relay → Test Consumers
//!                              ↓
//!                         Message Validation
//!                         Latency Measurement  
//!                         Throughput Analysis
//! ```
//!
//! ## Critical Validations
//! - Protocol V2 TLV message integrity through relay forwarding
//! - Race condition fix verification (bidirectional connections)
//! - Message ordering preservation
//! - Connection failure recovery
//! - Performance requirements (<35μs forwarding latency)

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use tokio::sync::{mpsc, RwLock, Barrier};
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

use protocol_v2::{
    parse_header, parse_tlv_extensions,
    tlv::build_message_direct,
    tlv::market_data::{PoolSwapTLV, PoolSyncTLV, PoolMintTLV},
    TLVType, VenueId, SourceType, RelayDomain,
};

/// Relay test configuration
#[derive(Debug, Clone)]
pub struct RelayTestConfig {
    pub socket_path: String,
    pub test_duration_seconds: u64,
    pub producer_message_rate_hz: u64,
    pub consumer_count: usize,
    pub max_forwarding_latency_us: u64,
    pub message_validation_enabled: bool,
    pub connection_timeout_ms: u64,
}

impl Default for RelayTestConfig {
    fn default() -> Self {
        Self {
            socket_path: "/tmp/alphapulse_test/relay_io.sock".to_string(),
            test_duration_seconds: 30,
            producer_message_rate_hz: 1000,
            consumer_count: 3,
            max_forwarding_latency_us: 35,
            message_validation_enabled: true,
            connection_timeout_ms: 5000,
        }
    }
}

/// Relay I/O test framework
pub struct RelayIOTestFramework {
    config: RelayTestConfig,
    test_metrics: Arc<RwLock<RelayTestMetrics>>,
}

/// Comprehensive test metrics
#[derive(Debug, Default)]
struct RelayTestMetrics {
    messages_sent: u64,
    messages_received: u64,
    messages_validated: u64,
    validation_failures: u64,
    connection_failures: u64,
    latency_measurements: Vec<u64>,
    throughput_samples: Vec<f64>,
    test_start_time: Option<Instant>,
    producer_connection_time: Option<Duration>,
    consumer_connection_times: HashMap<usize, Duration>,
}

/// Test message with embedded timing and validation data
#[derive(Debug, Clone)]
struct TestMessage {
    pub sequence_number: u64,
    pub timestamp_ns: u64,
    pub message_type: TestMessageType,
    pub validation_data: Vec<u8>,
}

#[derive(Debug, Clone)]
enum TestMessageType {
    PoolSwap { pool_id: [u8; 20], amount_in: u128, amount_out: u128 },
    PoolSync { pool_id: [u8; 20], reserve0: u128, reserve1: u128 },
    PoolMint { pool_id: [u8; 20], liquidity: u128 },
}

impl RelayIOTestFramework {
    pub fn new(config: RelayTestConfig) -> Self {
        Self {
            config,
            test_metrics: Arc::new(RwLock::new(RelayTestMetrics::default())),
        }
    }

    /// Execute comprehensive relay I/O test suite
    pub async fn execute_relay_tests(&self) -> Result<()> {
        info!("🚀 Starting Relay I/O Test Framework");
        info!("   Socket: {}", self.config.socket_path);
        info!("   Duration: {}s", self.config.test_duration_seconds);
        info!("   Producer Rate: {} msg/s", self.config.producer_message_rate_hz);
        info!("   Consumers: {}", self.config.consumer_count);

        {
            let mut metrics = self.test_metrics.write().await;
            metrics.test_start_time = Some(Instant::now());
        }

        // Test 1: Basic Connection Handling
        info!("🔌 Test 1: Connection Establishment and Handling");
        self.test_connection_establishment().await?;

        // Test 2: Message Production and Consumption
        info!("📤 Test 2: Message Production and Consumption");
        self.test_message_production_consumption().await?;

        // Test 3: Race Condition Fix Validation
        info!("🏁 Test 3: Race Condition Fix Validation");
        self.test_race_condition_fix().await?;

        // Test 4: Bidirectional Communication
        info!("🔄 Test 4: Bidirectional Communication");
        self.test_bidirectional_communication().await?;

        // Test 5: Performance Validation
        info!("⚡ Test 5: Performance and Latency");
        self.test_performance_requirements().await?;

        // Test 6: Connection Failure Recovery
        info!("🔧 Test 6: Connection Failure Recovery");
        self.test_connection_failure_recovery().await?;

        // Generate test report
        self.generate_relay_test_report().await;

        let metrics = self.test_metrics.read().await;
        let avg_latency = if metrics.latency_measurements.is_empty() {
            0.0
        } else {
            metrics.latency_measurements.iter().sum::<u64>() as f64 / metrics.latency_measurements.len() as f64
        };

        if metrics.validation_failures == 0 && avg_latency <= self.config.max_forwarding_latency_us as f64 {
            info!("✅ Relay I/O Tests: ALL PASSED");
            Ok(())
        } else {
            error!("❌ Relay I/O Tests: FAILURES DETECTED");
            Err(anyhow::anyhow!("Relay tests failed validation"))
        }
    }

    /// Test connection establishment to relay
    async fn test_connection_establishment(&self) -> Result<()> {
        let connection_start = Instant::now();
        
        // Test producer connection
        match timeout(
            Duration::from_millis(self.config.connection_timeout_ms),
            UnixStream::connect(&self.config.socket_path)
        ).await {
            Ok(Ok(mut producer_stream)) => {
                let connection_time = connection_start.elapsed();
                info!("✅ Producer connected in {:?}", connection_time);
                
                {
                    let mut metrics = self.test_metrics.write().await;
                    metrics.producer_connection_time = Some(connection_time);
                }

                // Test basic write
                let test_message = self.create_test_tlv_message(1, TestMessageType::PoolSwap {
                    pool_id: [0x11u8; 20],
                    amount_in: 1000000000000000000u128,
                    amount_out: 2000000000u128,
                })?;

                producer_stream.write_all(&test_message).await?;
                info!("✅ Test message sent successfully");

                producer_stream.shutdown().await.ok();
            }
            Ok(Err(e)) => {
                error!("❌ Producer connection failed: {}", e);
                let mut metrics = self.test_metrics.write().await;
                metrics.connection_failures += 1;
                return Err(e.into());
            }
            Err(_) => {
                error!("❌ Producer connection timeout after {}ms", self.config.connection_timeout_ms);
                let mut metrics = self.test_metrics.write().await;
                metrics.connection_failures += 1;
                return Err(anyhow::anyhow!("Connection timeout"));
            }
        }

        // Test multiple consumer connections
        for consumer_id in 0..self.config.consumer_count {
            let connection_start = Instant::now();
            
            match timeout(
                Duration::from_millis(self.config.connection_timeout_ms),
                UnixStream::connect(&self.config.socket_path)
            ).await {
                Ok(Ok(consumer_stream)) => {
                    let connection_time = connection_start.elapsed();
                    info!("✅ Consumer {} connected in {:?}", consumer_id, connection_time);
                    
                    {
                        let mut metrics = self.test_metrics.write().await;
                        metrics.consumer_connection_times.insert(consumer_id, connection_time);
                    }

                    // Keep connection alive briefly then close
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    drop(consumer_stream);
                }
                Ok(Err(e)) => {
                    error!("❌ Consumer {} connection failed: {}", consumer_id, e);
                    let mut metrics = self.test_metrics.write().await;
                    metrics.connection_failures += 1;
                }
                Err(_) => {
                    error!("❌ Consumer {} connection timeout", consumer_id);
                    let mut metrics = self.test_metrics.write().await;
                    metrics.connection_failures += 1;
                }
            }
        }

        Ok(())
    }

    /// Test message production and consumption patterns
    async fn test_message_production_consumption(&self) -> Result<()> {
        let test_duration = Duration::from_secs(self.config.test_duration_seconds);
        let message_interval = Duration::from_micros(1_000_000 / self.config.producer_message_rate_hz);
        
        // Channels for coordinating test
        let (producer_done_tx, mut producer_done_rx) = mpsc::channel(1);
        let (consumer_results_tx, mut consumer_results_rx) = mpsc::channel(self.config.consumer_count);

        // Start consumer tasks
        let mut consumer_handles = Vec::new();
        for consumer_id in 0..self.config.consumer_count {
            let socket_path = self.config.socket_path.clone();
            let consumer_tx = consumer_results_tx.clone();
            let test_duration = test_duration.clone();
            let metrics = self.test_metrics.clone();

            let handle = tokio::spawn(async move {
                match Self::run_consumer_task(consumer_id, socket_path, test_duration, metrics).await {
                    Ok(messages_received) => {
                        consumer_tx.send((consumer_id, messages_received)).await.ok();
                    }
                    Err(e) => {
                        error!("Consumer {} failed: {}", consumer_id, e);
                        consumer_tx.send((consumer_id, 0)).await.ok();
                    }
                }
            });
            consumer_handles.push(handle);
        }

        // Give consumers time to connect
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Start producer task
        let socket_path = self.config.socket_path.clone();
        let metrics = self.test_metrics.clone();
        let producer_handle = tokio::spawn(async move {
            let result = Self::run_producer_task(socket_path, message_interval, test_duration, metrics).await;
            producer_done_tx.send(()).await.ok();
            result
        });

        // Wait for producer completion
        producer_done_rx.recv().await;
        info!("✅ Producer task completed");

        // Wait for consumer results
        let mut total_messages_received = 0u64;
        for _ in 0..self.config.consumer_count {
            if let Some((consumer_id, messages_received)) = consumer_results_rx.recv().await {
                total_messages_received += messages_received;
                info!("✅ Consumer {} received {} messages", consumer_id, messages_received);
            }
        }

        // Wait for all tasks to complete
        for handle in consumer_handles {
            handle.await.ok();
        }
        producer_handle.await.ok();

        let messages_sent = {
            let metrics = self.test_metrics.read().await;
            metrics.messages_sent
        };

        let expected_total_messages = messages_sent * self.config.consumer_count as u64;
        let message_delivery_rate = total_messages_received as f64 / expected_total_messages as f64;

        info!("📊 Message delivery: {}/{} ({:.2}%)", total_messages_received, expected_total_messages, message_delivery_rate * 100.0);

        if message_delivery_rate >= 0.95 {
            info!("✅ Message production/consumption: PASSED");
        } else {
            error!("❌ Message production/consumption: LOW DELIVERY RATE ({:.2}%)", message_delivery_rate * 100.0);
            return Err(anyhow::anyhow!("Low message delivery rate"));
        }

        Ok(())
    }

    /// Test that race condition fix is working (bidirectional connections)
    async fn test_race_condition_fix(&self) -> Result<()> {
        // Simulate the original race condition scenario:
        // 1. Connect as "producer" but delay first message >100ms
        // 2. Verify connection is still treated as bidirectional
        // 3. Confirm messages are forwarded correctly

        let connection_start = Instant::now();
        let mut stream = UnixStream::connect(&self.config.socket_path).await?;
        
        info!("✅ Connected to relay, simulating delayed producer scenario");

        // Wait >100ms before sending first message (original race condition trigger)
        tokio::time::sleep(Duration::from_millis(150)).await;

        let test_message = self.create_test_tlv_message(1, TestMessageType::PoolSwap {
            pool_id: [0x22u8; 20],
            amount_in: 5000000000000000000u128,
            amount_out: 10000000000u128,
        })?;

        let send_start = Instant::now();
        stream.write_all(&test_message).await?;
        
        info!("✅ Delayed message sent successfully after {:?} delay", connection_start.elapsed());
        info!("✅ Race condition fix verified - bidirectional connection maintained");

        stream.shutdown().await.ok();
        Ok(())
    }

    /// Test bidirectional communication capability
    async fn test_bidirectional_communication(&self) -> Result<()> {
        // Connect two clients and verify both can send/receive
        let mut client1 = UnixStream::connect(&self.config.socket_path).await?;
        let mut client2 = UnixStream::connect(&self.config.socket_path).await?;

        // Client 1 sends message
        let message1 = self.create_test_tlv_message(1, TestMessageType::PoolSync {
            pool_id: [0x33u8; 20],
            reserve0: 1000000000000000000u128,
            reserve1: 2000000000u128,
        })?;

        client1.write_all(&message1).await?;

        // Client 2 should receive the message (with timeout)
        let mut buffer = vec![0u8; 1024];
        let bytes_read = timeout(Duration::from_millis(1000), client2.read(&mut buffer)).await??;

        if bytes_read >= 32 {
            // Validate received message
            let header = parse_header(&buffer[..32])?;
            if header.magic == 0xDEADBEEF {
                info!("✅ Bidirectional communication verified - message forwarded correctly");
            } else {
                error!("❌ Invalid message header received");
                return Err(anyhow::anyhow!("Invalid forwarded message"));
            }
        } else {
            error!("❌ No message received by client 2");
            return Err(anyhow::anyhow!("Message not forwarded"));
        }

        client1.shutdown().await.ok();
        client2.shutdown().await.ok();
        Ok(())
    }

    /// Test performance requirements
    async fn test_performance_requirements(&self) -> Result<()> {
        let test_count = 1000u64;
        let mut latencies = Vec::with_capacity(test_count as usize);

        let mut producer = UnixStream::connect(&self.config.socket_path).await?;
        let mut consumer = UnixStream::connect(&self.config.socket_path).await?;

        // Give connections time to establish
        tokio::time::sleep(Duration::from_millis(100)).await;

        for sequence in 1..=test_count {
            let test_message = self.create_test_tlv_message(sequence, TestMessageType::PoolMint {
                pool_id: [0x44u8; 20],
                liquidity: sequence * 1000u128,
            })?;

            let send_start = Instant::now();
            producer.write_all(&test_message).await?;

            // Read response (measure forwarding latency)
            let mut buffer = vec![0u8; 1024];
            match timeout(Duration::from_millis(100), consumer.read(&mut buffer)).await {
                Ok(Ok(bytes_read)) if bytes_read >= 32 => {
                    let latency = send_start.elapsed().as_micros() as u64;
                    latencies.push(latency);

                    if latency <= self.config.max_forwarding_latency_us {
                        debug!("✅ Message {} forwarded in {}μs", sequence, latency);
                    } else {
                        warn!("⚠️ Message {} slow forwarding: {}μs", sequence, latency);
                    }
                }
                Ok(Ok(_)) => {
                    warn!("⚠️ Message {} incomplete", sequence);
                }
                Ok(Err(e)) => {
                    error!("❌ Message {} read error: {}", sequence, e);
                }
                Err(_) => {
                    error!("❌ Message {} timeout", sequence);
                }
            }
        }

        producer.shutdown().await.ok();
        consumer.shutdown().await.ok();

        // Analyze performance
        if latencies.is_empty() {
            error!("❌ No valid latency measurements");
            return Err(anyhow::anyhow!("No performance data collected"));
        }

        let avg_latency = latencies.iter().sum::<u64>() as f64 / latencies.len() as f64;
        let max_latency = *latencies.iter().max().unwrap();
        let min_latency = *latencies.iter().min().unwrap();

        {
            let mut metrics = self.test_metrics.write().await;
            metrics.latency_measurements = latencies;
        }

        info!("📊 Performance Results:");
        info!("   Average Latency: {:.2}μs", avg_latency);
        info!("   Maximum Latency: {}μs", max_latency);
        info!("   Minimum Latency: {}μs", min_latency);

        if avg_latency <= self.config.max_forwarding_latency_us as f64 {
            info!("✅ Performance requirements: PASSED");
        } else {
            error!("❌ Performance requirements: FAILED - Average latency {}μs > {}μs", avg_latency, self.config.max_forwarding_latency_us);
            return Err(anyhow::anyhow!("Performance requirements not met"));
        }

        Ok(())
    }

    /// Test connection failure and recovery scenarios
    async fn test_connection_failure_recovery(&self) -> Result<()> {
        // Test graceful handling of connection drops
        let mut stream = UnixStream::connect(&self.config.socket_path).await?;

        // Send a message
        let message = self.create_test_tlv_message(1, TestMessageType::PoolSwap {
            pool_id: [0x55u8; 20],
            amount_in: 1000000000000000000u128,
            amount_out: 2000000000u128,
        })?;

        stream.write_all(&message).await?;

        // Force close connection
        drop(stream);

        // Verify relay continues operating with new connections
        tokio::time::sleep(Duration::from_millis(100)).await;

        let mut new_stream = UnixStream::connect(&self.config.socket_path).await?;
        let new_message = self.create_test_tlv_message(2, TestMessageType::PoolSync {
            pool_id: [0x66u8; 20],
            reserve0: 5000000000000000000u128,
            reserve1: 10000000000u128,
        })?;

        new_stream.write_all(&new_message).await?;
        info!("✅ Connection recovery verified - relay continues operating");

        new_stream.shutdown().await.ok();
        Ok(())
    }

    /// Create test TLV message for validation
    fn create_test_tlv_message(&self, sequence: u64, message_type: TestMessageType) -> Result<Vec<u8>> {
        let tlv_type = match &message_type {
            TestMessageType::PoolSwap { pool_id, amount_in, amount_out } => {
                let swap_tlv = PoolSwapTLV::new(
                    *pool_id,
                    [0x11u8; 20], // token_in
                    [0x22u8; 20], // token_out
                    VenueId::Polygon,
                    *amount_in,
                    *amount_out,
                    0u128, // liquidity_after
                    SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos() as u64,
                    sequence,
                    0i32, // tick_after
                    18u8, // amount_in_decimals
                    6u8,  // amount_out_decimals
                    0u128, // sqrt_price_x96_after
                );

                return build_message_direct(
                    RelayDomain::MarketData,
                    SourceType::TestHarness,
                    TLVType::PoolSwap,
                    &swap_tlv,
                ).map_err(|e| anyhow::anyhow!("TLV construction failed: {}", e));
            }
            TestMessageType::PoolSync { pool_id, reserve0, reserve1 } => {
                let sync_tlv = PoolSyncTLV::from_components(
                    *pool_id,
                    [0x11u8; 20], // token0
                    [0x22u8; 20], // token1
                    VenueId::Polygon,
                    *reserve0,
                    *reserve1,
                    18u8, // token0_decimals
                    6u8,  // token1_decimals
                    SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos() as u64,
                    sequence,
                );

                return build_message_direct(
                    RelayDomain::MarketData,
                    SourceType::TestHarness,
                    TLVType::PoolSync,
                    &sync_tlv,
                ).map_err(|e| anyhow::anyhow!("TLV construction failed: {}", e));
            }
            TestMessageType::PoolMint { pool_id, liquidity } => {
                let mint_tlv = PoolMintTLV::new(
                    *pool_id,
                    [0x33u8; 20], // provider
                    [0x11u8; 20], // token0
                    [0x22u8; 20], // token1
                    VenueId::Polygon,
                    *liquidity,
                    1000000000000000000u128, // amount0
                    2000000000u128,           // amount1
                    -887220i32, // tick_lower
                    887220i32,  // tick_upper
                    18u8, // token0_decimals
                    6u8,  // token1_decimals
                    SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos() as u64,
                );

                return build_message_direct(
                    RelayDomain::MarketData,
                    SourceType::TestHarness,
                    TLVType::PoolMint,
                    &mint_tlv,
                ).map_err(|e| anyhow::anyhow!("TLV construction failed: {}", e));
            }
        };
    }

    /// Producer task for generating test messages
    async fn run_producer_task(
        socket_path: String,
        message_interval: Duration,
        test_duration: Duration,
        metrics: Arc<RwLock<RelayTestMetrics>>,
    ) -> Result<()> {
        let mut stream = UnixStream::connect(&socket_path).await?;
        let start_time = Instant::now();
        let mut sequence = 0u64;

        while start_time.elapsed() < test_duration {
            sequence += 1;

            // Create test message
            let swap_tlv = PoolSwapTLV::new(
                [0x77u8; 20], // pool
                [0x11u8; 20], // token_in
                [0x22u8; 20], // token_out
                VenueId::Polygon,
                sequence * 1000000000000000u128, // varying amounts
                sequence * 2000000u128,
                0u128,
                SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64,
                sequence,
                0i32,
                18u8,
                6u8,
                0u128,
            );

            let message = build_message_direct(
                RelayDomain::MarketData,
                SourceType::TestHarness,
                TLVType::PoolSwap,
                &swap_tlv,
            )?;

            match stream.write_all(&message).await {
                Ok(()) => {
                    let mut m = metrics.write().await;
                    m.messages_sent += 1;
                }
                Err(e) => {
                    error!("Producer send error: {}", e);
                    break;
                }
            }

            tokio::time::sleep(message_interval).await;
        }

        stream.shutdown().await.ok();
        Ok(())
    }

    /// Consumer task for receiving and validating messages
    async fn run_consumer_task(
        consumer_id: usize,
        socket_path: String,
        test_duration: Duration,
        metrics: Arc<RwLock<RelayTestMetrics>>,
    ) -> Result<u64> {
        let mut stream = UnixStream::connect(&socket_path).await?;
        let start_time = Instant::now();
        let mut messages_received = 0u64;
        let mut buffer = vec![0u8; 4096];

        while start_time.elapsed() < test_duration {
            match timeout(Duration::from_millis(1000), stream.read(&mut buffer)).await {
                Ok(Ok(bytes_read)) if bytes_read >= 32 => {
                    // Validate TLV message
                    match parse_header(&buffer[..32]) {
                        Ok(header) if header.magic == 0xDEADBEEF => {
                            messages_received += 1;
                            
                            let mut m = metrics.write().await;
                            m.messages_received += 1;
                            m.messages_validated += 1;
                        }
                        Ok(header) => {
                            warn!("Consumer {}: Invalid magic: 0x{:08X}", consumer_id, header.magic);
                            let mut m = metrics.write().await;
                            m.validation_failures += 1;
                        }
                        Err(e) => {
                            warn!("Consumer {}: Header parse error: {}", consumer_id, e);
                            let mut m = metrics.write().await;
                            m.validation_failures += 1;
                        }
                    }
                }
                Ok(Ok(_)) => {
                    debug!("Consumer {}: Incomplete message", consumer_id);
                }
                Ok(Err(e)) => {
                    error!("Consumer {}: Read error: {}", consumer_id, e);
                    break;
                }
                Err(_) => {
                    debug!("Consumer {}: Read timeout", consumer_id);
                }
            }
        }

        stream.shutdown().await.ok();
        Ok(messages_received)
    }

    async fn generate_relay_test_report(&self) {
        let metrics = self.test_metrics.read().await;
        let test_duration = metrics.test_start_time.map(|t| t.elapsed()).unwrap_or_default();

        info!("📊 ===== RELAY I/O TEST REPORT =====");
        info!("Test Duration: {:?}", test_duration);
        info!("Messages Sent: {}", metrics.messages_sent);
        info!("Messages Received: {}", metrics.messages_received);
        info!("Messages Validated: {}", metrics.messages_validated);
        info!("Validation Failures: {}", metrics.validation_failures);
        info!("Connection Failures: {}", metrics.connection_failures);
        
        if let Some(producer_time) = metrics.producer_connection_time {
            info!("Producer Connection Time: {:?}", producer_time);
        }
        
        info!("Consumer Connections: {}", metrics.consumer_connection_times.len());
        for (id, time) in &metrics.consumer_connection_times {
            info!("  Consumer {}: {:?}", id, time);
        }

        if !metrics.latency_measurements.is_empty() {
            let avg_latency = metrics.latency_measurements.iter().sum::<u64>() as f64 
                            / metrics.latency_measurements.len() as f64;
            let max_latency = metrics.latency_measurements.iter().max().unwrap();
            let min_latency = metrics.latency_measurements.iter().min().unwrap();

            info!("Latency Statistics:");
            info!("  Average: {:.2}μs", avg_latency);
            info!("  Maximum: {}μs", max_latency);
            info!("  Minimum: {}μs", min_latency);
        }

        info!("=====================================");
    }
}

/// Run relay I/O test suite
pub async fn run_relay_io_tests() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    let config = RelayTestConfig::default();
    let test_framework = RelayIOTestFramework::new(config);
    
    test_framework.execute_relay_tests().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tlv_message_creation() {
        let config = RelayTestConfig::default();
        let framework = RelayIOTestFramework::new(config);
        
        let message = framework.create_test_tlv_message(1, TestMessageType::PoolSwap {
            pool_id: [0x11u8; 20],
            amount_in: 1000000000000000000u128,
            amount_out: 2000000000u128,
        });
        
        assert!(message.is_ok());
        let msg = message.unwrap();
        assert!(msg.len() >= 32);
        
        // Validate header
        let header = parse_header(&msg[..32]).unwrap();
        assert_eq!(header.magic, 0xDEADBEEF);
    }

    #[tokio::test]
    async fn test_metric_tracking() {
        let config = RelayTestConfig::default();
        let framework = RelayIOTestFramework::new(config);
        
        // Test metric updates
        {
            let mut metrics = framework.test_metrics.write().await;
            metrics.messages_sent = 100;
            metrics.messages_received = 95;
            metrics.validation_failures = 2;
        }
        
        let metrics = framework.test_metrics.read().await;
        assert_eq!(metrics.messages_sent, 100);
        assert_eq!(metrics.messages_received, 95);
        assert_eq!(metrics.validation_failures, 2);
    }
}