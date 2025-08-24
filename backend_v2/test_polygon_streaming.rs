#!/usr/bin/env rust-script
//! Polygon Event Streaming Integration Test
//!
//! Tests the complete flow:
//! Polygon WebSocket → Collector → Market Data Relay → Consumer
//!
//! Usage: cargo run --bin test_polygon_streaming
//!
//! This test will:
//! 1. Start a market data relay in the background
//! 2. Connect the Polygon collector to real WebSocket feeds
//! 3. Monitor for incoming swap/mint/burn events
//! 4. Validate TLV message structure and content
//! 5. Report streaming statistics

use protocol_v2::{parse_header, parse_tlv_extensions, RelayDomain, SourceType, TLVType, VenueId};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::AsyncReadExt;
use tokio::net::UnixStream;
use tokio::sync::RwLock;
use tokio::time::{sleep, timeout};
use tracing::{debug, error, info, warn};

/// Test configuration
#[derive(Debug, Clone)]
pub struct StreamingTestConfig {
    /// How long to run the streaming test (seconds)
    pub test_duration: u64,
    /// Minimum events expected to consider test successful  
    pub min_events_expected: u32,
    /// Market data relay socket path
    pub relay_socket_path: String,
    /// WebSocket endpoint for Polygon
    pub polygon_websocket_url: String,
}

impl Default for StreamingTestConfig {
    fn default() -> Self {
        Self {
            test_duration: 15,      // 15 second test
            min_events_expected: 1, // Expect at least 1 event
            relay_socket_path: "/tmp/alphapulse/market_data.sock".to_string(),
            polygon_websocket_url: "wss://ws-polygon-mainnet.chainstacklabs.com".to_string(),
        }
    }
}

/// Statistics collected during streaming test
#[derive(Debug, Default, Clone)]
pub struct StreamingStats {
    pub total_messages: u64,
    pub swap_events: u64,
    pub mint_events: u64,
    pub burn_events: u64,
    pub sync_events: u64,
    pub parse_errors: u64,
    pub venues_seen: HashMap<VenueId, u64>,
    pub test_start_time: Option<Instant>,
    pub last_message_time: Option<Instant>,
}

impl StreamingStats {
    pub fn new() -> Self {
        let mut stats = Self::default();
        stats.test_start_time = Some(Instant::now());
        stats
    }

    pub fn record_message(&mut self, venue: VenueId) {
        self.total_messages += 1;
        *self.venues_seen.entry(venue).or_insert(0) += 1;
        self.last_message_time = Some(Instant::now());
    }

    pub fn record_swap(&mut self) {
        self.swap_events += 1;
    }
    pub fn record_mint(&mut self) {
        self.mint_events += 1;
    }
    pub fn record_burn(&mut self) {
        self.burn_events += 1;
    }
    pub fn record_sync(&mut self) {
        self.sync_events += 1;
    }
    pub fn record_parse_error(&mut self) {
        self.parse_errors += 1;
    }

    pub fn messages_per_second(&self) -> f64 {
        if let Some(start_time) = self.test_start_time {
            let elapsed = start_time.elapsed().as_secs_f64();
            if elapsed > 0.0 {
                return self.total_messages as f64 / elapsed;
            }
        }
        0.0
    }

    pub fn print_report(&self) {
        println!("\\n📊 STREAMING TEST RESULTS");
        println!("========================================");
        println!("📈 Total Messages: {}", self.total_messages);
        println!("💱 Swap Events: {}", self.swap_events);
        println!("🔵 Mint Events: {}", self.mint_events);
        println!("🔴 Burn Events: {}", self.burn_events);
        println!("🔄 Sync Events: {}", self.sync_events);
        println!("❌ Parse Errors: {}", self.parse_errors);
        println!("⚡ Messages/sec: {:.2}", self.messages_per_second());

        println!("\\n🏪 Venues Seen:");
        for (venue, count) in &self.venues_seen {
            println!("  {:?}: {} messages", venue, count);
        }

        if let (Some(start), Some(last)) = (self.test_start_time, self.last_message_time) {
            println!("\\n⏱️  Duration: {:.1}s", start.elapsed().as_secs_f64());
            println!("📡 Last message: {:.1}s ago", last.elapsed().as_secs_f64());
        }
    }
}

/// Relay consumer that monitors market data messages
pub struct RelayConsumer {
    stats: Arc<RwLock<StreamingStats>>,
    socket_path: String,
}

impl RelayConsumer {
    pub fn new(socket_path: String) -> Self {
        Self {
            stats: Arc::new(RwLock::new(StreamingStats::new())),
            socket_path,
        }
    }

    pub async fn start_consuming(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!(
            "🔌 Connecting to market data relay at: {}",
            self.socket_path
        );

        // Wait for relay socket to be available
        for attempt in 1..=10 {
            match UnixStream::connect(&self.socket_path).await {
                Ok(mut stream) => {
                    info!("✅ Connected to relay socket");
                    return self.consume_messages(&mut stream).await;
                }
                Err(e) => {
                    if attempt == 10 {
                        error!("❌ Failed to connect after {} attempts: {}", attempt, e);
                        return Err(format!("Connection failed: {}", e).into());
                    }
                    warn!("⏳ Attempt {} failed, retrying in 2s: {}", attempt, e);
                    sleep(Duration::from_secs(2)).await;
                }
            }
        }

        unreachable!()
    }

    async fn consume_messages(
        &self,
        stream: &mut UnixStream,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("📡 Starting message consumption...");
        let mut buffer = vec![0u8; 65536]; // 64KB buffer

        loop {
            match timeout(Duration::from_secs(5), stream.read(&mut buffer)).await {
                Ok(Ok(0)) => {
                    warn!("🔌 Stream closed");
                    break;
                }
                Ok(Ok(bytes_read)) => {
                    self.process_received_data(&buffer[..bytes_read]).await;
                }
                Ok(Err(e)) => {
                    error!("❌ Read error: {}", e);
                    break;
                }
                Err(_) => {
                    // Timeout - this is expected during low activity
                    debug!("⏰ Read timeout (no new messages)");
                }
            }
        }

        Ok(())
    }

    async fn process_received_data(&self, data: &[u8]) {
        // Protocol V2 messages start with 32-byte header
        if data.len() < 32 {
            warn!("⚠️ Message too short: {} bytes", data.len());
            self.stats.write().await.record_parse_error();
            return;
        }

        match self.parse_tlv_message(data).await {
            Ok((venue, tlv_type)) => {
                let mut stats = self.stats.write().await;
                stats.record_message(venue);

                match tlv_type {
                    TLVType::PoolSwap => stats.record_swap(),
                    TLVType::PoolMint => stats.record_mint(),
                    TLVType::PoolBurn => stats.record_burn(),
                    TLVType::PoolSync => stats.record_sync(),
                    _ => debug!("📦 Other TLV type: {:?}", tlv_type),
                }
            }
            Err(e) => {
                warn!("⚠️ Failed to parse message: {}", e);
                self.stats.write().await.record_parse_error();
            }
        }
    }

    async fn parse_tlv_message(
        &self,
        data: &[u8],
    ) -> Result<(VenueId, TLVType), Box<dyn std::error::Error + Send + Sync>> {
        // Parse 32-byte header
        let header = parse_header(data)?;

        // Validate this is a market data message
        if header.relay_domain != RelayDomain::MarketData as u8 {
            return Err(format!("Expected MarketData domain, got {}", header.relay_domain).into());
        }

        // Extract venue from source
        let venue = match header.source {
            s if s == SourceType::PolygonCollector as u8 => VenueId::Polygon,
            s if s == SourceType::BinanceCollector as u8 => VenueId::Binance,
            _ => return Err(format!("Unknown source: {}", header.source).into()),
        };

        // Parse TLV payload
        let tlv_payload = &data[32..32 + header.payload_size as usize];
        let tlvs = parse_tlv_extensions(tlv_payload)?;

        if let Some(tlv) = tlvs.first() {
            let tlv_type_num = match tlv {
                protocol_v2::TLVExtensionEnum::Standard(std_tlv) => std_tlv.header.tlv_type,
                protocol_v2::TLVExtensionEnum::Extended(ext_tlv) => ext_tlv.header.tlv_type,
            };

            let tlv_type = TLVType::try_from(tlv_type_num)
                .map_err(|_| format!("Unknown TLV type: {}", tlv_type_num))?;

            debug!(
                "📦 Received {:?} from {:?} (seq: {})",
                tlv_type, venue, header.sequence
            );
            return Ok((venue, tlv_type));
        }

        Err("No TLV extensions found".into())
    }

    pub async fn get_stats(&self) -> StreamingStats {
        (*self.stats.read().await).clone()
    }
}

/// Test orchestrator
pub struct StreamingTest {
    config: StreamingTestConfig,
    consumer: RelayConsumer,
}

impl StreamingTest {
    pub fn new(config: StreamingTestConfig) -> Self {
        let consumer = RelayConsumer::new(config.relay_socket_path.clone());
        Self { config, consumer }
    }

    pub async fn run(&self) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        println!("🚀 Starting Polygon Event Streaming Test");
        println!("Configuration: {:?}", self.config);

        // Step 1: Start relay consumer in background
        let consumer_clone = RelayConsumer::new(self.config.relay_socket_path.clone());
        let consumer_handle = tokio::spawn(async move {
            if let Err(e) = consumer_clone.start_consuming().await {
                error!("❌ Consumer failed: {}", e);
            }
        });

        // Step 2: Wait for the test duration
        info!(
            "⏳ Running streaming test for {} seconds...",
            self.config.test_duration
        );
        sleep(Duration::from_secs(self.config.test_duration)).await;

        // Step 3: Collect results
        let stats = self.consumer.get_stats().await;
        stats.print_report();

        // Step 4: Evaluate success
        let success = stats.total_messages >= self.config.min_events_expected as u64;

        if success {
            println!(
                "\\n✅ TEST PASSED: Received {} events (min: {})",
                stats.total_messages, self.config.min_events_expected
            );
        } else {
            println!(
                "\\n❌ TEST FAILED: Only received {} events (min: {})",
                stats.total_messages, self.config.min_events_expected
            );
        }

        // Cleanup
        consumer_handle.abort();

        Ok(success)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize tracing
    tracing_subscriber::fmt().init();

    let config = StreamingTestConfig::default();
    let test = StreamingTest::new(config);

    println!("🧪 Polygon Event Streaming Integration Test");
    println!("==========================================");
    println!("This test will monitor real Polygon DEX events streaming");
    println!("through the market data relay for 60 seconds.");
    println!("\\nMAKE SURE to start the following services first:");
    println!("1. Market Data Relay: cargo run --bin relay");
    println!("2. Polygon Collector: cargo run --bin polygon");
    println!("\\nPress Enter to continue or Ctrl+C to cancel...");

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    match test.run().await {
        Ok(true) => {
            println!("\\n🎉 Integration test SUCCESSFUL!");
            std::process::exit(0);
        }
        Ok(false) => {
            println!("\\n💥 Integration test FAILED!");
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("\\n💥 Test error: {}", e);
            std::process::exit(1);
        }
    }
}
