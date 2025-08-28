# MYC-007: Integration Testing

## Status
- **Status**: pending
- **Assignee**: TBD
- **Estimated Effort**: 3 days
- **Priority**: Critical (validates complete migration)

## Description
Validate the complete Mycelium broker migration with comprehensive end-to-end tests, performance benchmarks, and backwards compatibility verification. This ensures the broker architecture maintains all functionality while meeting strict performance requirements before relay removal.

## Objectives
1. Validate complete message flow: Producer → Broker → Consumer
2. Verify performance maintains >1M msg/s construction, >1.6M msg/s parsing
3. Test backwards compatibility with existing service interfaces
4. Ensure zero message loss during normal and failure scenarios
5. Validate broker behavior under high load and edge conditions

## Technical Approach

### End-to-End Message Flow Testing
```rust
// tests/integration/end_to_end_flow.rs
#[tokio::test]
async fn complete_message_flow_validation() {
    // Setup test environment
    let test_env = TestEnvironment::new().await;
    
    // Start Mycelium broker
    let broker = test_env.start_broker().await;
    
    // Start producer services
    let polygon_adapter = test_env.start_polygon_adapter(&broker.socket_path()).await;
    let kraken_collector = test_env.start_kraken_collector(&broker.socket_path()).await;
    
    // Start consumer services
    let flash_arbitrage = test_env.start_flash_arbitrage(&broker.socket_path()).await;
    let portfolio_service = test_env.start_portfolio_service(&broker.socket_path()).await;
    let dashboard = test_env.start_dashboard(&broker.socket_path()).await;
    
    // Wait for all services to establish connections
    test_env.wait_for_service_health().await;
    
    // Generate test events
    let test_events = create_comprehensive_test_events();
    
    for event in test_events {
        // Inject event into appropriate producer
        match event.source {
            TestEventSource::Polygon => polygon_adapter.inject_event(event.clone()).await,
            TestEventSource::Kraken => kraken_collector.inject_event(event.clone()).await,
        }
        
        // Verify all appropriate consumers receive the message
        let expected_consumers = get_expected_consumers_for_event(&event);
        
        for consumer in expected_consumers {
            let received_message = consumer.wait_for_message(Duration::from_secs(5)).await
                .expect("Consumer should receive message");
                
            validate_message_content(&event, &received_message);
        }
    }
    
    // Validate service metrics
    assert_broker_metrics(&broker).await;
    assert_producer_metrics(&[&polygon_adapter, &kraken_collector]).await;
    assert_consumer_metrics(&[&flash_arbitrage, &portfolio_service, &dashboard]).await;
}

#[tokio::test]
async fn topic_routing_validation() {
    let test_env = TestEnvironment::new().await;
    let broker = test_env.start_broker().await;
    
    // Create test consumers for each topic
    let market_data_consumer = test_env.create_consumer(&broker.socket_path(), "market_data").await;
    let signals_consumer = test_env.create_consumer(&broker.socket_path(), "signals").await;
    let execution_consumer = test_env.create_consumer(&broker.socket_path(), "execution").await;
    
    // Create test producer
    let producer = test_env.create_producer(&broker.socket_path()).await;
    
    // Test market data routing
    let trade_message = create_test_trade_tlv_message();
    producer.publish("market_data", trade_message.clone()).await.unwrap();
    
    // Only market data consumer should receive
    let received = market_data_consumer.recv_with_timeout(Duration::from_secs(1)).await.unwrap();
    assert_messages_equal(&trade_message, &received);
    
    // Other consumers should not receive
    assert!(signals_consumer.recv_with_timeout(Duration::from_millis(100)).await.is_err());
    assert!(execution_consumer.recv_with_timeout(Duration::from_millis(100)).await.is_err());
    
    // Test signals routing
    let signal_message = create_test_signal_tlv_message();
    producer.publish("signals", signal_message.clone()).await.unwrap();
    
    let received = signals_consumer.recv_with_timeout(Duration::from_secs(1)).await.unwrap();
    assert_messages_equal(&signal_message, &received);
    
    // Verify isolation
    assert!(market_data_consumer.recv_with_timeout(Duration::from_millis(100)).await.is_err());
    assert!(execution_consumer.recv_with_timeout(Duration::from_millis(100)).await.is_err());
}
```

### Performance Benchmarks
```rust
// tests/integration/performance_benchmarks.rs
#[tokio::test]
#[ignore]
async fn broker_throughput_benchmark() {
    let test_env = TestEnvironment::new().await;
    let broker = test_env.start_broker_high_performance().await;
    
    // Create high-throughput producer and consumer
    let producer = test_env.create_high_perf_producer(&broker.socket_path()).await;
    let consumer = test_env.create_high_perf_consumer(&broker.socket_path(), "perf_test").await;
    
    let num_messages = 2_000_000; // 2M messages for thorough testing
    let message_size = 1024; // 1KB messages
    let test_message = vec![0u8; message_size];
    
    // Start consumer in background
    let consumer_handle = tokio::spawn(async move {
        let mut received_count = 0;
        let start_time = std::time::Instant::now();
        
        while received_count < num_messages {
            match consumer.recv().await {
                Ok(_) => received_count += 1,
                Err(e) => {
                    eprintln!("Consumer error after {} messages: {}", received_count, e);
                    break;
                }
            }
        }
        
        let elapsed = start_time.elapsed();
        let receive_rate = received_count as f64 / elapsed.as_secs_f64();
        println!("Consumer receive rate: {:.0} msg/s", receive_rate);
        receive_rate
    });
    
    // Benchmark producer throughput
    let start_time = std::time::Instant::now();
    
    for i in 0..num_messages {
        producer.publish("perf_test", test_message.clone()).await.unwrap();
        
        if i % 100_000 == 0 {
            let elapsed = start_time.elapsed();
            let current_rate = (i + 1) as f64 / elapsed.as_secs_f64();
            println!("Producer progress: {}/{} ({:.0} msg/s)", i + 1, num_messages, current_rate);
        }
    }
    
    let producer_elapsed = start_time.elapsed();
    let send_rate = num_messages as f64 / producer_elapsed.as_secs_f64();
    
    println!("Producer send rate: {:.0} msg/s", send_rate);
    
    // Wait for consumer to finish
    let receive_rate = consumer_handle.await.unwrap();
    
    // Validate performance requirements
    assert!(send_rate > 1_000_000.0, "Producer throughput: {:.0} msg/s (required: >1M)", send_rate);
    assert!(receive_rate > 1_600_000.0, "Consumer throughput: {:.0} msg/s (required: >1.6M)", receive_rate);
    
    // Validate end-to-end latency
    let latency_us = measure_end_to_end_latency(&test_env).await;
    assert!(latency_us < 100, "End-to-end latency: {}μs (required: <100μs)", latency_us);
}

#[tokio::test]
#[ignore]
async fn concurrent_consumer_scaling() {
    let test_env = TestEnvironment::new().await;
    let broker = test_env.start_broker().await;
    
    let num_consumers = 50; // Test with many consumers
    let messages_per_consumer = 10_000;
    
    // Create multiple consumers
    let mut consumer_handles = Vec::new();
    
    for consumer_id in 0..num_consumers {
        let broker_path = broker.socket_path().clone();
        let handle = tokio::spawn(async move {
            let consumer = TestConsumer::new(&broker_path, "scaling_test").await.unwrap();
            
            let mut received = 0;
            let start = std::time::Instant::now();
            
            while received < messages_per_consumer {
                match consumer.recv().await {
                    Ok(_) => received += 1,
                    Err(e) => {
                        eprintln!("Consumer {} error: {}", consumer_id, e);
                        break;
                    }
                }
            }
            
            let rate = received as f64 / start.elapsed().as_secs_f64();
            (consumer_id, received, rate)
        });
        
        consumer_handles.push(handle);
    }
    
    // Create producer
    let producer = test_env.create_producer(&broker.socket_path()).await;
    
    // Send messages (fanout to all consumers)
    let total_messages = messages_per_consumer;
    let test_message = create_test_message();
    
    for _ in 0..total_messages {
        producer.publish("scaling_test", test_message.clone()).await.unwrap();
    }
    
    // Wait for all consumers to complete
    let mut total_received = 0;
    let mut min_rate = f64::MAX;
    let mut max_rate = 0.0;
    
    for handle in consumer_handles {
        let (consumer_id, received, rate) = handle.await.unwrap();
        total_received += received;
        min_rate = min_rate.min(rate);
        max_rate = max_rate.max(rate);
        
        println!("Consumer {}: {} messages at {:.0} msg/s", consumer_id, received, rate);
    }
    
    let expected_total = num_consumers * messages_per_consumer;
    let delivery_ratio = total_received as f64 / expected_total as f64;
    
    println!("Message delivery: {}/{} ({:.1}%)", total_received, expected_total, delivery_ratio * 100.0);
    println!("Consumer rate range: {:.0} - {:.0} msg/s", min_rate, max_rate);
    
    // Validate scaling behavior
    assert!(delivery_ratio > 0.99, "Message delivery ratio too low: {:.1}%", delivery_ratio * 100.0);
    assert!(min_rate > 10_000.0, "Slowest consumer too slow: {:.0} msg/s", min_rate);
}
```

### Failure Recovery Testing
```rust
// tests/integration/failure_recovery.rs
#[tokio::test]
async fn broker_restart_recovery() {
    let test_env = TestEnvironment::new().await;
    let mut broker = test_env.start_broker().await;
    
    // Start producer and consumer
    let producer = test_env.create_producer(&broker.socket_path()).await;
    let consumer = test_env.create_consumer(&broker.socket_path(), "recovery_test").await;
    
    // Send initial messages
    for i in 0..100 {
        let message = format!("message_{}", i).into_bytes();
        producer.publish("recovery_test", message).await.unwrap();
    }
    
    // Verify initial messages received
    for i in 0..100 {
        let received = consumer.recv_with_timeout(Duration::from_secs(1)).await.unwrap();
        let expected = format!("message_{}", i).into_bytes();
        assert_eq!(received, expected);
    }
    
    // Restart broker
    broker.stop().await;
    broker = test_env.start_broker().await;
    
    // Wait for reconnection
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // Send post-restart messages
    for i in 100..200 {
        let message = format!("message_{}", i).into_bytes();
        producer.publish("recovery_test", message).await.unwrap();
    }
    
    // Verify post-restart messages received
    for i in 100..200 {
        let received = consumer.recv_with_timeout(Duration::from_secs(5)).await.unwrap();
        let expected = format!("message_{}", i).into_bytes();
        assert_eq!(received, expected);
    }
}

#[tokio::test]
async fn producer_failure_isolation() {
    let test_env = TestEnvironment::new().await;
    let broker = test_env.start_broker().await;
    
    // Start multiple producers and consumers
    let producer1 = test_env.create_producer(&broker.socket_path()).await;
    let producer2 = test_env.create_producer(&broker.socket_path()).await;
    let consumer = test_env.create_consumer(&broker.socket_path(), "isolation_test").await;
    
    // Both producers send messages
    producer1.publish("isolation_test", b"from_producer_1".to_vec()).await.unwrap();
    producer2.publish("isolation_test", b"from_producer_2".to_vec()).await.unwrap();
    
    // Consumer receives both
    let msg1 = consumer.recv().await.unwrap();
    let msg2 = consumer.recv().await.unwrap();
    
    // Simulate producer1 failure
    drop(producer1);
    
    // Producer2 should continue working
    producer2.publish("isolation_test", b"after_failure".to_vec()).await.unwrap();
    let msg3 = consumer.recv().await.unwrap();
    
    assert_eq!(msg3, b"after_failure");
}
```

### Backwards Compatibility Validation
```rust
// tests/integration/backwards_compatibility.rs
#[tokio::test]
async fn tlv_message_format_compatibility() {
    // Generate messages using old relay system (if still available)
    let relay_messages = generate_relay_system_messages().await;
    
    // Parse with new broker system
    let parser = TLVParser::new();
    
    for (description, message_data) in relay_messages {
        match parser.parse_message(&message_data) {
            Ok(parsed) => {
                println!("✓ Compatible: {}", description);
                validate_parsed_message_structure(&parsed);
            }
            Err(e) => {
                panic!("✗ Incompatible {}: {}", description, e);
            }
        }
    }
}

#[tokio::test]
async fn service_interface_compatibility() {
    // Test that existing service interfaces still work
    
    // FlashArbitrage strategy
    let strategy_config = FlashArbitrageConfig::default();
    let strategy = FlashArbitrageStrategy::new(strategy_config).await;
    assert!(strategy.is_ok(), "FlashArbitrage should initialize with broker config");
    
    // Portfolio service
    let portfolio_config = PortfolioConfig::default();
    let portfolio = PortfolioService::new(portfolio_config).await;
    assert!(portfolio.is_ok(), "Portfolio should initialize with broker config");
    
    // Dashboard service
    let dashboard_config = DashboardConfig::default();
    let dashboard = DashboardService::new(dashboard_config).await;
    assert!(dashboard.is_ok(), "Dashboard should initialize with broker config");
}
```

### Load Testing and Stress Testing
```rust
// tests/integration/stress_tests.rs
#[tokio::test]
#[ignore] // Long-running test
async fn sustained_high_load_test() {
    let test_env = TestEnvironment::new().await;
    let broker = test_env.start_broker().await;
    
    let duration = Duration::from_secs(300); // 5 minute stress test
    let target_rate = 500_000; // 500k msg/s sustained
    
    let producer = test_env.create_producer(&broker.socket_path()).await;
    let consumer = test_env.create_consumer(&broker.socket_path(), "stress_test").await;
    
    let test_message = vec![0u8; 512]; // 512 byte messages
    
    let start_time = std::time::Instant::now();
    let mut messages_sent = 0;
    let mut messages_received = 0;
    
    // Producer task
    let producer_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_nanos(1_000_000_000 / target_rate as u64));
        
        while start_time.elapsed() < duration {
            interval.tick().await;
            if producer.publish("stress_test", test_message.clone()).await.is_ok() {
                messages_sent += 1;
            }
        }
        
        messages_sent
    });
    
    // Consumer task
    let consumer_handle = tokio::spawn(async move {
        while start_time.elapsed() < duration {
            if consumer.recv().await.is_ok() {
                messages_received += 1;
            }
        }
        
        messages_received
    });
    
    // Wait for completion
    let (sent, received) = tokio::join!(producer_handle, consumer_handle);
    let sent = sent.unwrap();
    let received = received.unwrap();
    
    let actual_duration = start_time.elapsed();
    let send_rate = sent as f64 / actual_duration.as_secs_f64();
    let receive_rate = received as f64 / actual_duration.as_secs_f64();
    let delivery_ratio = received as f64 / sent as f64;
    
    println!("Stress test results:");
    println!("  Duration: {:.1}s", actual_duration.as_secs_f64());
    println!("  Messages sent: {} ({:.0} msg/s)", sent, send_rate);
    println!("  Messages received: {} ({:.0} msg/s)", received, receive_rate);
    println!("  Delivery ratio: {:.1}%", delivery_ratio * 100.0);
    
    // Validate stress test requirements
    assert!(send_rate > target_rate as f64 * 0.95, "Send rate too low: {:.0} msg/s", send_rate);
    assert!(delivery_ratio > 0.99, "Delivery ratio too low: {:.1}%", delivery_ratio * 100.0);
}

#[tokio::test]
async fn memory_leak_detection() {
    let test_env = TestEnvironment::new().await;
    let broker = test_env.start_broker().await;
    
    let initial_memory = get_broker_memory_usage(&broker);
    
    // Run high-volume message traffic
    let producer = test_env.create_producer(&broker.socket_path()).await;
    let consumer = test_env.create_consumer(&broker.socket_path(), "memory_test").await;
    
    let num_rounds = 10;
    let messages_per_round = 100_000;
    
    for round in 0..num_rounds {
        println!("Memory test round {}/{}", round + 1, num_rounds);
        
        // Send burst of messages
        for _ in 0..messages_per_round {
            producer.publish("memory_test", vec![0u8; 1024]).await.unwrap();
        }
        
        // Consume all messages
        for _ in 0..messages_per_round {
            consumer.recv().await.unwrap();
        }
        
        // Force garbage collection
        tokio::time::sleep(Duration::from_secs(1)).await;
        
        let current_memory = get_broker_memory_usage(&broker);
        let memory_growth = current_memory - initial_memory;
        
        println!("  Memory usage: {} MB (growth: {} MB)", 
                 current_memory / 1_000_000, memory_growth / 1_000_000);
        
        // Memory growth should be bounded
        assert!(memory_growth < 100_000_000, "Excessive memory growth: {} MB", memory_growth / 1_000_000);
    }
}
```

## Acceptance Criteria

### End-to-End Functionality
- [ ] Complete producer→broker→consumer message flow working
- [ ] All TLV message types routed correctly through topics
- [ ] Service startup and shutdown sequences work properly
- [ ] Configuration changes take effect without data loss

### Performance Validation
- [ ] Broker throughput ≥ 1M msg/s construction, ≥ 1.6M msg/s parsing
- [ ] End-to-end latency < 100μs for typical message sizes
- [ ] Memory usage stable under sustained high load
- [ ] CPU usage reasonable for target throughput

### Reliability Testing
- [ ] Zero message loss under normal operating conditions
- [ ] Graceful handling of connection failures and reconnections
- [ ] Proper isolation between producers and consumers
- [ ] Service restart scenarios handled correctly

### Backwards Compatibility
- [ ] Existing service configurations work with minimal changes
- [ ] TLV message format parsing identical to relay system
- [ ] Service interfaces preserved during migration
- [ ] Monitoring and metrics collection maintained

## Dependencies
- **Upstream**: MYC-005 (Producer Migration), MYC-006 (Consumer Migration)
- **Downstream**: MYC-008 (Relay Removal)
- **External**: Test infrastructure, monitoring tools

## Testing Requirements

### Integration Test Suite
```bash
# Complete integration test suite
cargo test --package mycelium-broker --test integration -- --test-threads=1

# Performance benchmarks
cargo test --package mycelium-broker --test performance --release -- --ignored

# Stress tests (long-running)
cargo test --package mycelium-broker --test stress -- --ignored --nocapture

# Backwards compatibility
cargo test --package mycelium-broker --test compatibility
```

### Test Environment Setup
```rust
// tests/common/test_environment.rs
pub struct TestEnvironment {
    temp_dir: TempDir,
    broker_process: Option<Child>,
    service_processes: Vec<Child>,
}

impl TestEnvironment {
    pub async fn new() -> Self {
        let temp_dir = tempdir().expect("Failed to create temp directory");
        
        Self {
            temp_dir,
            broker_process: None,
            service_processes: Vec::new(),
        }
    }

    pub async fn start_broker(&mut self) -> TestBroker {
        let socket_path = self.temp_dir.path().join("test_broker.sock");
        let config_path = self.temp_dir.path().join("broker.toml");
        
        // Generate test broker configuration
        let config = BrokerConfig {
            server: ServerConfig {
                socket_path: socket_path.to_str().unwrap().to_string(),
                max_connections: 1000,
                connection_timeout_ms: 5000,
            },
            topics: vec![
                TopicConfig {
                    name: "market_data".to_string(),
                    routing: RoutingType::Fanout,
                    buffer_size: 100000,
                    max_message_size: 65536,
                },
                TopicConfig {
                    name: "signals".to_string(),
                    routing: RoutingType::Fanout,
                    buffer_size: 50000,
                    max_message_size: 32768,
                },
                TopicConfig {
                    name: "execution".to_string(),
                    routing: RoutingType::Queue,
                    buffer_size: 25000,
                    max_message_size: 16384,
                },
            ],
            performance: PerformanceConfig::default(),
        };
        
        std::fs::write(&config_path, toml::to_string(&config).unwrap()).unwrap();
        
        // Start broker process
        let mut cmd = std::process::Command::new("mycelium-broker");
        cmd.arg("--config").arg(&config_path);
        
        let process = cmd.spawn().expect("Failed to start broker");
        self.broker_process = Some(process);
        
        // Wait for broker to start
        tokio::time::sleep(Duration::from_millis(500)).await;
        
        TestBroker {
            socket_path: socket_path.to_str().unwrap().to_string(),
            config,
        }
    }
}
```

## Rollback Plan

### If Critical Issues Found
1. **Immediate Rollback**: Restart all services with relay configuration
2. **Issue Analysis**: Identify specific failure modes and root causes
3. **Targeted Fixes**: Address issues in isolated environment
4. **Gradual Re-deployment**: Service-by-service migration with extensive monitoring

### If Performance Issues
1. **Baseline Comparison**: Compare against relay system benchmarks
2. **Profiling**: Identify performance bottlenecks in broker or clients
3. **Optimization**: Implement targeted performance improvements
4. **Re-validation**: Repeat integration tests after optimizations

### If Reliability Issues
1. **Chaos Testing**: Introduce controlled failures to identify weak points
2. **Circuit Breakers**: Implement additional failure protection mechanisms
3. **Monitoring Enhancement**: Add more detailed observability
4. **Graceful Degradation**: Implement fallback mechanisms

## Technical Notes

### Test Environment Design
- **Isolated Environment**: Each test uses separate temp directories and sockets
- **Process Management**: Proper cleanup of broker and service processes
- **Resource Monitoring**: Track memory, CPU, and file descriptor usage
- **Parallel Testing**: Tests designed to run independently

### Performance Measurement
- **High-Resolution Timing**: Use nanosecond precision for latency measurements
- **Statistical Analysis**: Report P50, P95, P99 latencies in addition to averages
- **Throughput Validation**: Sustained load testing over extended periods
- **Resource Efficiency**: Monitor memory usage patterns and CPU utilization

### Reliability Validation
- **Failure Injection**: Simulate various failure scenarios
- **Data Integrity**: Verify no message loss or corruption
- **Recovery Testing**: Validate proper service restart and reconnection
- **Load Isolation**: Ensure one service's load doesn't affect others

## Validation Steps

1. **Smoke Tests**:
   ```bash
   cargo test --package mycelium-broker basic_functionality
   ```

2. **Performance Validation**:
   ```bash
   cargo test --package mycelium-broker --release -- --ignored throughput
   ```

3. **Full Integration Suite**:
   ```bash
   ./scripts/run_integration_tests.sh
   ```

4. **Stress Testing**:
   ```bash
   cargo test --package mycelium-broker --release -- --ignored stress --nocapture
   ```

This comprehensive integration testing validates that the Mycelium broker migration is complete, performant, and reliable before proceeding with relay removal in MYC-008.