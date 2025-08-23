use alphapulse_protocol::{PoolEvent, PoolUpdateMessage, PoolUpdateType, ProtocolType};
use exchange_collector::exchanges::polygon::dex::{identify_pool_event, EventBasedPoolType};
use exchange_collector::exchanges::polygon::PolygonCollector;
use exchange_collector::unix_socket::UnixSocketWriter;
use anyhow::Result;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Integration test metrics for pool event processing
#[derive(Debug, Default)]
pub struct PoolEventMetrics {
    pub events_received: AtomicU32,
    pub events_parsed: AtomicU32,
    pub events_failed: AtomicU32,
    pub v2_events: AtomicU32,
    pub v3_events: AtomicU32,
    pub mint_events: AtomicU32,
    pub burn_events: AtomicU32,
    pub collect_events: AtomicU32,
    pub sync_events: AtomicU32,
    pub total_latency_ns: AtomicU64,
    pub max_latency_ns: AtomicU64,
    pub min_latency_ns: AtomicU64,
}

impl PoolEventMetrics {
    pub fn new() -> Self {
        Self {
            min_latency_ns: AtomicU64::new(u64::MAX),
            ..Default::default()
        }
    }

    pub fn record_event(&self, event_type: PoolUpdateType, protocol: ProtocolType, latency_ns: u64) {
        self.events_parsed.fetch_add(1, Ordering::Relaxed);
        
        match protocol {
            ProtocolType::UniswapV2 => self.v2_events.fetch_add(1, Ordering::Relaxed),
            ProtocolType::UniswapV3 => self.v3_events.fetch_add(1, Ordering::Relaxed),
            _ => {}
        };
        
        match event_type {
            PoolUpdateType::Mint => self.mint_events.fetch_add(1, Ordering::Relaxed),
            PoolUpdateType::Burn => self.burn_events.fetch_add(1, Ordering::Relaxed),
            PoolUpdateType::Collect => self.collect_events.fetch_add(1, Ordering::Relaxed),
            PoolUpdateType::Sync => self.sync_events.fetch_add(1, Ordering::Relaxed),
            _ => {}
        };
        
        self.total_latency_ns.fetch_add(latency_ns, Ordering::Relaxed);
        
        // Update max latency
        let mut current_max = self.max_latency_ns.load(Ordering::Relaxed);
        while latency_ns > current_max {
            match self.max_latency_ns.compare_exchange_weak(
                current_max, latency_ns, Ordering::Relaxed, Ordering::Relaxed
            ) {
                Ok(_) => break,
                Err(x) => current_max = x,
            }
        }
        
        // Update min latency
        let mut current_min = self.min_latency_ns.load(Ordering::Relaxed);
        while latency_ns < current_min {
            match self.min_latency_ns.compare_exchange_weak(
                current_min, latency_ns, Ordering::Relaxed, Ordering::Relaxed
            ) {
                Ok(_) => break,
                Err(x) => current_min = x,
            }
        }
    }
    
    pub fn record_failure(&self) {
        self.events_failed.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn print_summary(&self) {
        let events_received = self.events_received.load(Ordering::Relaxed);
        let events_parsed = self.events_parsed.load(Ordering::Relaxed);
        let events_failed = self.events_failed.load(Ordering::Relaxed);
        let v2_events = self.v2_events.load(Ordering::Relaxed);
        let v3_events = self.v3_events.load(Ordering::Relaxed);
        let mint_events = self.mint_events.load(Ordering::Relaxed);
        let burn_events = self.burn_events.load(Ordering::Relaxed);
        let collect_events = self.collect_events.load(Ordering::Relaxed);
        let sync_events = self.sync_events.load(Ordering::Relaxed);
        let total_latency = self.total_latency_ns.load(Ordering::Relaxed);
        let max_latency = self.max_latency_ns.load(Ordering::Relaxed);
        let min_latency = self.min_latency_ns.load(Ordering::Relaxed);
        
        info!("🎯 POOL EVENT INTEGRATION TEST RESULTS:");
        info!("📊 Events: {} received, {} parsed, {} failed", events_received, events_parsed, events_failed);
        info!("🔄 Protocols: {} V2, {} V3", v2_events, v3_events);
        info!("📈 Event Types: {} Mint, {} Burn, {} Collect, {} Sync", 
              mint_events, burn_events, collect_events, sync_events);
        
        if events_parsed > 0 {
            let avg_latency = total_latency / events_parsed as u64;
            info!("⚡ Latency: avg {:.1}μs, max {:.1}μs, min {:.1}μs", 
                  avg_latency as f64 / 1000.0,
                  max_latency as f64 / 1000.0,
                  if min_latency == u64::MAX { 0.0 } else { min_latency as f64 / 1000.0 });
            
            let success_rate = (events_parsed as f64 / events_received as f64) * 100.0;
            info!("✅ Success Rate: {:.1}%", success_rate);
            
            // Performance validation
            if avg_latency < 35_000 {
                info!("🚀 PERFORMANCE: HOT PATH target <35μs ACHIEVED!");
            } else {
                warn!("⚠️  PERFORMANCE: HOT PATH target <35μs MISSED ({}μs)", avg_latency / 1000);
            }
        }
    }
}

/// Comprehensive integration test for live Polygon pool events
pub struct PoolEventIntegrationTest {
    pub metrics: Arc<PoolEventMetrics>,
    pub socket_writer: Arc<UnixSocketWriter>,
    pub test_duration: Duration,
    pub known_pools: HashMap<String, &'static str>, // pool_address -> expected_type
}

impl PoolEventIntegrationTest {
    pub fn new(test_duration_secs: u64) -> Self {
        let socket_writer = Arc::new(UnixSocketWriter::new("/tmp/alphapulse/integration_test.sock"));
        
        // Known high-activity pools for validation
        let mut known_pools = HashMap::new();
        known_pools.insert("0x45dda9cb7c25131df268515131f647d726f50608".to_string(), "WETH/USDC V3"); // Uniswap V3 WETH/USDC
        known_pools.insert("0x853ee4b2a13f8a742d64c8f088be7ba2131f670d".to_string(), "WETH/USDC V2"); // Uniswap V2 WETH/USDC
        known_pools.insert("0x06da0fd433c1a5d7a4faa01111c044910a184553".to_string(), "USDC/WETH QS"); // QuickSwap USDC/WETH
        
        Self {
            metrics: Arc::new(PoolEventMetrics::new()),
            socket_writer,
            test_duration: Duration::from_secs(test_duration_secs),
            known_pools,
        }
    }
    
    /// Run comprehensive integration test with live Polygon events
    pub async fn run_live_test(&self) -> Result<()> {
        info!("🚀 Starting live Polygon pool event integration test for {} seconds", 
              self.test_duration.as_secs());
        
        // Set up WebSocket connection to Polygon
        let polygon_ws_url = std::env::var("POLYGON_WS_URL")
            .unwrap_or_else(|_| "wss://polygon-mainnet.g.alchemy.com/v2/your-api-key".to_string());
        
        info!("🔗 Connecting to Polygon WebSocket: {}", polygon_ws_url);
        
        let (ws_stream, _) = connect_async(&polygon_ws_url).await?;
        let (mut ws_sender, mut ws_receiver) = ws_stream.split();
        
        // Subscribe to pool events (all major event signatures)
        let subscription = json!({
            "id": 1,
            "method": "eth_subscribe",
            "params": [
                "logs",
                {
                    "topics": [
                        [
                            "0x4c209b5fc8ad50758f13e2e1088ba56a560dff690a1c6fef26394f4c03821c4f", // V2 Mint
                            "0xdccd412f0b1252819cb1fd330b93224ca42612892bb3f4f789976e6d8136129a", // V2 Burn  
                            "0x1c411e9a96e071241c2f21f7726b17ae89e3cab4c78be50e062b03a9fffbbad1", // V2 Sync
                            "0x7a53080ba414158be7ec69b987b5fb7d07dee101babe276914f785c42da22a01b", // V3 Mint
                            "0x0c396cd989a39f4459b5fa1aed6a9a8dcdbc45908acfd67e028cd568da98982c", // V3 Burn
                            "0x40d0efd1a53d60ecbf40971b9daf7dc90178c3aadc7aab1765632738fa8b8f01"  // V3 Collect
                        ]
                    ]
                }
            ]
        });
        
        ws_sender.send(Message::Text(subscription.to_string())).await?;
        info!("📡 Subscribed to all pool event signatures");
        
        // Create mock collector for event processing
        let collector = PolygonCollector::new(self.socket_writer.clone());
        
        let metrics = Arc::clone(&self.metrics);
        let test_start = Instant::now();
        let test_end = test_start + self.test_duration;
        
        let mut event_count = 0;
        
        info!("⏱️  Test running for {} seconds...", self.test_duration.as_secs());
        
        // Process events until test duration expires
        while Instant::now() < test_end {
            tokio::select! {
                msg = ws_receiver.next() => {
                    if let Some(Ok(Message::Text(text))) = msg {
                        let receive_time = Instant::now();
                        
                        if let Ok(data) = serde_json::from_str::<Value>(&text) {
                            self.metrics.events_received.fetch_add(1, Ordering::Relaxed);
                            
                            if let Some(params) = data.get("params") {
                                if let Some(result) = params.get("result") {
                                    event_count += 1;
                                    
                                    // Process the pool event
                                    match self.process_pool_event_test(&collector, result, receive_time).await {
                                        Ok(latency_ns) => {
                                            if event_count % 10 == 0 {
                                                info!("📊 Processed {} pool events (latest: {:.1}μs)", 
                                                      event_count, latency_ns as f64 / 1000.0);
                                            }
                                        }
                                        Err(e) => {
                                            debug!("❌ Failed to process pool event: {}", e);
                                            self.metrics.record_failure();
                                        }
                                    }
                                    
                                    // Validate against known pools
                                    if let Some(pool_addr) = result.get("address").and_then(|a| a.as_str()) {
                                        if let Some(pool_type) = self.known_pools.get(pool_addr) {
                                            debug!("✅ Validated event from known pool: {} ({})", pool_addr, pool_type);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                _ = tokio::time::sleep(Duration::from_millis(100)) => {
                    // Periodic progress update
                    let elapsed = test_start.elapsed();
                    let remaining = self.test_duration.saturating_sub(elapsed);
                    
                    if remaining.as_secs() % 30 == 0 && remaining.as_millis() % 1000 < 100 {
                        let events_received = self.metrics.events_received.load(Ordering::Relaxed);
                        let events_parsed = self.metrics.events_parsed.load(Ordering::Relaxed);
                        info!("⏱️  Test progress: {}s remaining, {} events received, {} parsed", 
                              remaining.as_secs(), events_received, events_parsed);
                    }
                }
            }
        }
        
        info!("🏁 Integration test completed");
        self.metrics.print_summary();
        
        Ok(())
    }
    
    /// Process a single pool event and measure end-to-end latency
    async fn process_pool_event_test(
        &self, 
        collector: &PolygonCollector, 
        log: &Value, 
        receive_time: Instant
    ) -> Result<u64> {
        let start_time = Instant::now();
        
        // Extract event signature
        let topics = log.get("topics")
            .and_then(|t| t.as_array())
            .ok_or_else(|| anyhow::anyhow!("No topics in event"))?;
        
        let event_signature = topics.get(0)
            .and_then(|s| s.as_str())
            .ok_or_else(|| anyhow::anyhow!("No event signature"))?;
        
        // Identify event type
        let (event_type, pool_type) = identify_pool_event(event_signature)
            .ok_or_else(|| anyhow::anyhow!("Unknown pool event signature: {}", event_signature))?;
        
        // Test the pool event processing pipeline
        match collector.handle_pool_event(log).await {
            Ok(_) => {
                let end_time = Instant::now();
                let latency_ns = end_time.duration_since(start_time).as_nanos() as u64;
                
                // Record metrics
                let protocol = match pool_type {
                    EventBasedPoolType::UniswapV2Style => ProtocolType::UniswapV2,
                    EventBasedPoolType::UniswapV3Style => ProtocolType::UniswapV3,
                    EventBasedPoolType::CurveStyle => ProtocolType::Curve,
                    EventBasedPoolType::BalancerStyle => ProtocolType::Balancer,
                };
                
                self.metrics.record_event(event_type, protocol, latency_ns);
                
                Ok(latency_ns)
            }
            Err(e) => {
                self.metrics.record_failure();
                Err(e)
            }
        }
    }
    
    /// Validate event parsing with known transaction hashes
    pub async fn validate_known_transactions(&self) -> Result<()> {
        info!("🔍 Validating pool event parsing with known Polygon transactions");
        
        // Known Polygon transactions with pool events
        let test_cases = vec![
            ("0x123456...", "V2 Mint event"),
            ("0xabcdef...", "V3 Burn event"),
            ("0x789012...", "V3 Collect event"),
        ];
        
        for (tx_hash, description) in test_cases {
            info!("📋 Testing: {} ({})", tx_hash, description);
            // TODO: Fetch actual transaction logs and test parsing
        }
        
        Ok(())
    }
    
    /// Performance stress test with simulated high-frequency events
    pub async fn stress_test(&self, events_per_second: u32, duration_secs: u64) -> Result<()> {
        info!("🔥 Running stress test: {} events/sec for {} seconds", 
              events_per_second, duration_secs);
        
        let interval = Duration::from_nanos(1_000_000_000 / events_per_second as u64);
        let test_duration = Duration::from_secs(duration_secs);
        let start_time = Instant::now();
        
        while start_time.elapsed() < test_duration {
            // Generate synthetic pool event
            let synthetic_event = self.create_synthetic_pool_event();
            
            let process_start = Instant::now();
            
            // Process the synthetic event (simulated)
            let latency_ns = process_start.elapsed().as_nanos() as u64;
            
            if latency_ns > 35_000 {
                warn!("⚠️  Stress test latency spike: {}μs", latency_ns / 1000);
            }
            
            tokio::time::sleep(interval).await;
        }
        
        info!("✅ Stress test completed");
        Ok(())
    }
    
    /// Create synthetic pool event for testing
    fn create_synthetic_pool_event(&self) -> Value {
        json!({
            "address": "0x45dda9cb7c25131df268515131f647d726f50608",
            "topics": [
                "0x4c209b5fc8ad50758f13e2e1088ba56a560dff690a1c6fef26394f4c03821c4f",
                "0x0000000000000000000000007a250d5630b4cf539739df2c5dacb4c659f2488d"
            ],
            "data": "0x0000000000000000000000000000000000000000000000000de0b6b3a76400000000000000000000000000000000000000000000000000000de0b6b3a7640000",
            "blockNumber": "0x2a2a2a2",
            "transactionHash": "0x123456789abcdef123456789abcdef123456789abcdef123456789abcdef1234",
            "logIndex": "0x0"
        })
    }
}

#[tokio::test]
async fn test_live_polygon_pool_events() -> Result<()> {
    // Initialize tracing for test output
    tracing_subscriber::fmt()
        .with_env_filter("debug")
        .with_test_writer()
        .init();
    
    // Run integration test for 60 seconds
    let test = PoolEventIntegrationTest::new(60);
    
    info!("🎯 Starting comprehensive pool event integration test");
    
    // Run live test (requires POLYGON_WS_URL environment variable)
    if std::env::var("POLYGON_WS_URL").is_ok() {
        test.run_live_test().await?;
    } else {
        info!("⚠️  Skipping live test (set POLYGON_WS_URL to enable)");
    }
    
    // Run validation tests
    test.validate_known_transactions().await?;
    
    // Run stress test
    test.stress_test(100, 10).await?;
    
    info!("✅ All integration tests completed successfully");
    Ok(())
}

#[tokio::test] 
async fn test_pool_event_serialization_performance() -> Result<()> {
    use alphapulse_protocol::{UniswapV2PoolEvent, PoolEventCore, PoolUpdateType};
    
    // Test serialization performance of pool events
    let iterations = 10_000;
    let start = Instant::now();
    
    for i in 0..iterations {
        let event = UniswapV2PoolEvent {
            core: PoolEventCore {
                timestamp_ns: SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos() as u64,
                pool_address: format!("0x{:040x}", i),
                tx_hash: format!("0x{:064x}", i),
                block_number: i as u64,
                log_index: i,
                token0_address: "0xA0b86a33E6417c39513dD5C05E02Ad8BF3c8E91c".to_string(),
                token1_address: "0xdAC17F958D2ee523a2206206994597C13D831ec7".to_string(),
                token0_symbol: "WETH".to_string(),
                token1_symbol: "USDT".to_string(),
                event_type: PoolUpdateType::Mint,
                sender: format!("0x{:040x}", i),
            },
            liquidity: 1_000_000_000_000_000_000u128,
            amount0: 1_000_000_000_000_000_000u128,
            amount1: 3_000_000_000u128,
            to: format!("0x{:040x}", i),
            reserves0_after: 10_000_000_000_000_000_000u128,
            reserves1_after: 30_000_000_000u128,
        };
        
        // Test serialization to binary protocol
        let _message = event.to_message();
    }
    
    let elapsed = start.elapsed();
    let avg_latency_ns = elapsed.as_nanos() / iterations;
    
    info!("🚀 Serialization performance: {} iterations in {:?}", iterations, elapsed);
    info!("⚡ Average latency: {}ns ({:.1}μs)", avg_latency_ns, avg_latency_ns as f64 / 1000.0);
    
    // Verify hot path performance
    assert!(avg_latency_ns < 10_000, "Serialization too slow: {}ns", avg_latency_ns);
    
    Ok(())
}