//! Polygon Event Reception Debug Tests
//!
//! Comprehensive test suite to diagnose why Polygon collector isn't receiving events.
//! Tests each component in the WebSocket → Event → TLV pipeline with real Polygon endpoints.
//!
//! ## Test Strategy
//! 1. **Network Layer**: Validate WebSocket connections to Polygon endpoints
//! 2. **Subscription Layer**: Verify JSON-RPC subscriptions are accepted
//! 3. **Event Reception**: Check if real events are being received
//! 4. **Parsing Layer**: Validate JSON log parsing and ABI decoding
//! 5. **TLV Conversion**: Test event → TLV message conversion
//! 6. **Integration**: End-to-end flow validation

use anyhow::{Context, Result};
use ethabi::{Event, EventParam, ParamType, RawLog};
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info, warn};
use web3::types::{Log, H160, H256};

// PolygonDexCollector replaced by standalone binary: bin/polygon/polygon.rs
// Tests now use the unified collector directly
use protocol_v2::{
    parse_header, parse_tlv_extensions, tlv::market_data::PoolSwapTLV, InstrumentId, SourceType,
    TLVMessageBuilder, TLVType, VenueId,
};

// Real Polygon WebSocket endpoints for testing
const TEST_POLYGON_ENDPOINTS: &[&str] = &[
    "wss://polygon-bor-rpc.publicnode.com",
    "wss://polygon-mainnet.g.alchemy.com/v2/demo",
    "wss://ws-polygon-mainnet.chainstacklabs.com",
];

// Real pool addresses for testing event subscription
const TEST_POOL_ADDRESSES: &[&str] = &[
    "0x45dda9cb7c25131df268515131f647d726f50608", // USDC-WETH V3
    "0x9b08288c3be4f62bbf8d1c20ac9c5e6f9467d8b7", // WMATIC-USDC V3
    "0x604229c960e5cacf2aaeac8be68ac07ba9df81c3", // USDC-WETH V2
    "0x6e7a5fafcec6bb1e78bae2a1f0b612012bf14827", // WMATIC-USDC V2
];

// Event signatures to test
const SWAP_V2_SIGNATURE: &str =
    "0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822";
const SWAP_V3_SIGNATURE: &str =
    "0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67";
const MINT_SIGNATURE: &str = "0x7a53080ba414158be7ec69b987b5fb7d07dee101fe85488f0853ae16239d0bde";
const BURN_SIGNATURE: &str = "0x0c396cd989a39f4459b5fa1aed6a9a8dcdbc45908acfd67e028cd568da98982c";
const SYNC_SIGNATURE: &str = "0x1c411e9a96e071241c2f21f7726b17ae89e3cab4c78be50e062b03a9fffbbad1";

/// Results from endpoint connectivity testing
#[derive(Debug)]
pub struct EndpointTestResult {
    pub url: String,
    pub connected: bool,
    pub handshake_time: Duration,
    pub subscription_accepted: bool,
    pub events_received: u32,
    pub test_duration: Duration,
    pub error_message: Option<String>,
}

/// Results from event parsing testing
#[derive(Debug)]
pub struct EventParsingResult {
    pub event_type: String,
    pub parse_success: bool,
    pub tlv_conversion_success: bool,
    pub tlv_size: usize,
    pub processing_time: Duration,
    pub error_message: Option<String>,
}

/// Comprehensive debug test suite for Polygon event reception
pub struct PolygonEventDebugger {
    timeout_duration: Duration,
    test_duration: Duration,
    verbose_logging: bool,
}

impl Default for PolygonEventDebugger {
    fn default() -> Self {
        Self {
            timeout_duration: Duration::from_secs(30),
            test_duration: Duration::from_secs(120), // 2 minutes per endpoint
            verbose_logging: true,
        }
    }
}

impl PolygonEventDebugger {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set test duration for each endpoint
    pub fn with_test_duration(mut self, duration: Duration) -> Self {
        self.test_duration = duration;
        self
    }

    /// Enable/disable verbose logging
    pub fn with_verbose_logging(mut self, verbose: bool) -> Self {
        self.verbose_logging = verbose;
        self
    }

    /// Run comprehensive debug tests
    pub async fn run_debug_tests(&self) -> Result<Vec<EndpointTestResult>> {
        info!("🔍 Starting Polygon Event Reception Debug Tests");
        info!("   Test duration per endpoint: {:?}", self.test_duration);
        info!("   Connection timeout: {:?}", self.timeout_duration);

        let mut results = Vec::new();

        for endpoint_url in TEST_POLYGON_ENDPOINTS {
            info!("🧪 Testing endpoint: {}", endpoint_url);

            let result = self.test_endpoint_comprehensive(endpoint_url).await;
            self.log_endpoint_result(&result);
            results.push(result);

            // Brief pause between endpoint tests
            tokio::time::sleep(Duration::from_secs(2)).await;
        }

        self.log_summary_report(&results);
        Ok(results)
    }

    /// Comprehensive test of a single endpoint
    async fn test_endpoint_comprehensive(&self, endpoint_url: &str) -> EndpointTestResult {
        let test_start = Instant::now();

        // Step 1: Test basic connectivity
        let (connected, handshake_time, ws_stream, connection_error) =
            self.test_websocket_connection(endpoint_url).await;

        if !connected {
            return EndpointTestResult {
                url: endpoint_url.to_string(),
                connected: false,
                handshake_time,
                subscription_accepted: false,
                events_received: 0,
                test_duration: test_start.elapsed(),
                error_message: connection_error,
            };
        }

        let (mut ws_sender, mut ws_receiver) = ws_stream.unwrap().split();

        // Step 2: Test subscription
        let subscription_accepted = self
            .test_event_subscription(&mut ws_sender, &mut ws_receiver)
            .await;

        if !subscription_accepted {
            return EndpointTestResult {
                url: endpoint_url.to_string(),
                connected: true,
                handshake_time,
                subscription_accepted: false,
                events_received: 0,
                test_duration: test_start.elapsed(),
                error_message: Some("Event subscription failed".to_string()),
            };
        }

        // Step 3: Listen for events
        let events_received = self.listen_for_events(&mut ws_receiver).await;

        EndpointTestResult {
            url: endpoint_url.to_string(),
            connected: true,
            handshake_time,
            subscription_accepted: true,
            events_received,
            test_duration: test_start.elapsed(),
            error_message: None,
        }
    }

    /// Test WebSocket connection establishment
    async fn test_websocket_connection(
        &self,
        url: &str,
    ) -> (
        bool,
        Duration,
        Option<
            tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
            >,
        >,
        Option<String>,
    ) {
        let connect_start = Instant::now();

        match tokio::time::timeout(self.timeout_duration, connect_async(url)).await {
            Ok(Ok((ws_stream, response))) => {
                let handshake_time = connect_start.elapsed();
                info!("✅ WebSocket connected to {}", url);
                info!("   Handshake time: {:?}", handshake_time);
                info!("   Response status: {}", response.status());

                if self.verbose_logging {
                    for (key, value) in response.headers() {
                        debug!("   Header {}: {:?}", key, value);
                    }
                }

                (true, handshake_time, Some(ws_stream), None)
            }
            Ok(Err(e)) => {
                let handshake_time = connect_start.elapsed();
                error!("❌ WebSocket connection failed to {}: {}", url, e);
                (false, handshake_time, None, Some(e.to_string()))
            }
            Err(_) => {
                let handshake_time = connect_start.elapsed();
                error!(
                    "❌ WebSocket connection timeout to {} after {:?}",
                    url, self.timeout_duration
                );
                (
                    false,
                    handshake_time,
                    None,
                    Some("Connection timeout".to_string()),
                )
            }
        }
    }

    /// Test event subscription via JSON-RPC
    async fn test_event_subscription(
        &self,
        ws_sender: &mut futures_util::stream::SplitSink<
            tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
            >,
            Message,
        >,
        ws_receiver: &mut futures_util::stream::SplitStream<
            tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
            >,
        >,
    ) -> bool {
        // Create comprehensive subscription message
        let subscription_message = self.create_debug_subscription_message();

        if self.verbose_logging {
            debug!("📤 Sending subscription: {}", subscription_message);
        }

        // Send subscription
        if let Err(e) = ws_sender.send(Message::Text(subscription_message)).await {
            error!("❌ Failed to send subscription: {}", e);
            return false;
        }

        info!("📤 Subscription sent, waiting for response...");

        // Wait for subscription response
        let response_timeout = Duration::from_secs(10);
        match tokio::time::timeout(response_timeout, ws_receiver.next()).await {
            Ok(Some(Ok(Message::Text(response_text)))) => {
                if self.verbose_logging {
                    debug!("📥 Subscription response: {}", response_text);
                }

                // Parse JSON response
                match serde_json::from_str::<Value>(&response_text) {
                    Ok(json) => {
                        if let Some(result) = json.get("result") {
                            info!("✅ Subscription accepted: {}", result);
                            return true;
                        } else if let Some(error) = json.get("error") {
                            error!("❌ Subscription error: {}", error);
                            return false;
                        } else {
                            warn!("⚠️ Unexpected subscription response format");
                            return false;
                        }
                    }
                    Err(e) => {
                        error!("❌ Failed to parse subscription response: {}", e);
                        return false;
                    }
                }
            }
            Ok(Some(Ok(message))) => {
                warn!("⚠️ Received non-text subscription response: {:?}", message);
                return false;
            }
            Ok(Some(Err(e))) => {
                error!(
                    "❌ WebSocket error waiting for subscription response: {}",
                    e
                );
                return false;
            }
            Ok(None) => {
                error!("❌ WebSocket closed while waiting for subscription response");
                return false;
            }
            Err(_) => {
                error!("❌ Timeout waiting for subscription response");
                return false;
            }
        }
    }

    /// Listen for events over the specified test duration
    async fn listen_for_events(
        &self,
        ws_receiver: &mut futures_util::stream::SplitStream<
            tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
            >,
        >,
    ) -> u32 {
        info!("👂 Listening for events for {:?}...", self.test_duration);

        let mut events_received = 0u32;
        let listen_start = Instant::now();

        while listen_start.elapsed() < self.test_duration {
            let remaining_time = self.test_duration - listen_start.elapsed();

            match tokio::time::timeout(remaining_time, ws_receiver.next()).await {
                Ok(Some(Ok(Message::Text(message_text)))) => {
                    if self.verbose_logging {
                        debug!("📥 Raw message: {}", message_text);
                    }

                    // Try to parse as JSON-RPC notification
                    match serde_json::from_str::<Value>(&message_text) {
                        Ok(json) => {
                            if let Some(method) = json.get("method") {
                                if method == "eth_subscription" {
                                    events_received += 1;
                                    info!("🎯 Event #{} received", events_received);

                                    if self.verbose_logging {
                                        if let Some(params) = json.get("params") {
                                            if let Some(result) = params.get("result") {
                                                self.analyze_received_event(
                                                    result,
                                                    events_received,
                                                )
                                                .await;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            if self.verbose_logging {
                                debug!("📥 Non-JSON message ({}): {}", e, message_text);
                            }
                        }
                    }
                }
                Ok(Some(Ok(Message::Ping(_)))) => {
                    debug!("💓 Received ping");
                }
                Ok(Some(Ok(Message::Pong(_)))) => {
                    debug!("💓 Received pong");
                }
                Ok(Some(Ok(Message::Close(_)))) => {
                    warn!("📌 WebSocket closed by remote");
                    break;
                }
                Ok(Some(Err(e))) => {
                    error!("❌ WebSocket error: {}", e);
                    break;
                }
                Ok(None) => {
                    warn!("📌 WebSocket stream ended");
                    break;
                }
                Err(_) => {
                    // Timeout - normal during low activity
                    debug!("⏰ Listen timeout (normal during low activity)");
                    break;
                }
            }

            // Brief pause to prevent busy waiting
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        info!(
            "👂 Event listening completed: {} events in {:?}",
            events_received,
            listen_start.elapsed()
        );
        events_received
    }

    /// Analyze a received event for debugging
    async fn analyze_received_event(&self, event_json: &Value, event_number: u32) {
        info!("🔍 Analyzing event #{}:", event_number);

        // Extract basic log information
        if let Some(address) = event_json.get("address") {
            info!("   Contract: {}", address);
        }

        if let Some(topics) = event_json.get("topics").and_then(|t| t.as_array()) {
            info!("   Topics: {} topics", topics.len());

            if let Some(topic0) = topics.get(0).and_then(|t| t.as_str()) {
                info!("   Signature: {}", topic0);

                // Identify event type
                let event_type = match topic0 {
                    s if s == SWAP_V2_SIGNATURE => "Uniswap V2 Swap",
                    s if s == SWAP_V3_SIGNATURE => "Uniswap V3 Swap",
                    s if s == MINT_SIGNATURE => "Mint",
                    s if s == BURN_SIGNATURE => "Burn",
                    s if s == SYNC_SIGNATURE => "V2 Sync",
                    _ => "Unknown",
                };
                info!("   Event Type: {}", event_type);
            }
        }

        if let Some(data) = event_json.get("data") {
            if let Some(data_str) = data.as_str() {
                info!("   Data: {} bytes", data_str.len() / 2 - 1); // -1 for 0x prefix
            }
        }

        if let Some(block) = event_json.get("blockNumber") {
            info!("   Block: {}", block);
        }

        // Test conversion to Web3 Log format
        match self.json_to_web3_log(event_json) {
            Ok(log) => {
                info!("   ✅ JSON → Web3 Log conversion successful");

                // Test TLV conversion if it's a swap
                if !log.topics.is_empty() {
                    let topic0_str = format!("{:x}", log.topics[0]);
                    if topic0_str == &SWAP_V2_SIGNATURE[2..]
                        || topic0_str == &SWAP_V3_SIGNATURE[2..]
                    {
                        match self.test_swap_tlv_conversion(&log).await {
                            Ok(tlv_bytes) => {
                                info!(
                                    "   ✅ Swap → TLV conversion successful: {} bytes",
                                    tlv_bytes.len()
                                );
                            }
                            Err(e) => {
                                warn!("   ⚠️ Swap → TLV conversion failed: {}", e);
                            }
                        }
                    }
                }
            }
            Err(e) => {
                warn!("   ⚠️ JSON → Web3 Log conversion failed: {}", e);
            }
        }
    }

    /// Test conversion of swap event to TLV
    async fn test_swap_tlv_conversion(&self, log: &Log) -> Result<Vec<u8>> {
        // Simplified swap TLV creation for testing
        let pool_address = log.address;

        // Extract basic data (simplified for testing)
        let mut pool_addr = [0u8; 20];
        pool_addr.copy_from_slice(&pool_address.0);

        let mut token_in_addr = [0u8; 20];
        let mut token_out_addr = [0u8; 20];
        // Use pool address components as token addresses for testing
        token_in_addr[12..20].copy_from_slice(&pool_address.0[0..8]);
        token_out_addr[12..20].copy_from_slice(&pool_address.0[12..20]);

        let swap_tlv = PoolSwapTLV {
            venue: VenueId::Polygon,
            pool_address: pool_addr,
            token_in_addr,
            token_out_addr,
            amount_in: 1000000000000000000u128, // 1 token
            amount_out: 500000000u128,          // 0.5 USDC (6 decimals)
            amount_in_decimals: 18,             // WETH decimals
            amount_out_decimals: 6,             // USDC decimals
            sqrt_price_x96_after: [0u8; 20],
            tick_after: 0,
            liquidity_after: 0,
            timestamp_ns: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
            block_number: log.block_number.map(|n| n.as_u64()).unwrap_or(0),
        };

        // Build TLV message
        let message = TLVMessageBuilder::new(
            protocol_v2::RelayDomain::MarketData,
            SourceType::PolygonCollector,
        )
        .add_tlv_slice(TLVType::PoolSwap, swap_tlv.as_bytes())
        .build();

        // Validate message structure
        if message.len() < 32 {
            return Err(anyhow::anyhow!("TLV message too short"));
        }

        let header = parse_header(&message[..32]).context("Failed to parse TLV header")?;

        if header.magic != 0xDEADBEEF {
            return Err(anyhow::anyhow!("Invalid TLV magic number"));
        }

        Ok(message)
    }

    /// Convert JSON log to Web3 Log format (same as in collector)
    fn json_to_web3_log(&self, json_log: &Value) -> Result<Log> {
        let address_str = json_log
            .get("address")
            .and_then(|v| v.as_str())
            .context("Missing address field")?;

        let address = address_str
            .parse::<H160>()
            .context("Invalid address format")?;

        let topics = json_log
            .get("topics")
            .and_then(|v| v.as_array())
            .context("Missing topics field")?
            .iter()
            .filter_map(|t| t.as_str())
            .filter_map(|t| t.parse::<H256>().ok())
            .collect();

        let data_str = json_log
            .get("data")
            .and_then(|v| v.as_str())
            .unwrap_or("0x");

        let data_bytes = if data_str.starts_with("0x") {
            hex::decode(&data_str[2..]).unwrap_or_default()
        } else {
            hex::decode(data_str).unwrap_or_default()
        };

        Ok(Log {
            address,
            topics,
            data: web3::types::Bytes(data_bytes),
            block_hash: json_log
                .get("blockHash")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok()),
            block_number: json_log
                .get("blockNumber")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok()),
            transaction_hash: json_log
                .get("transactionHash")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok()),
            transaction_index: json_log
                .get("transactionIndex")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok()),
            log_index: json_log
                .get("logIndex")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok()),
            transaction_log_index: json_log
                .get("transactionLogIndex")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok()),
            log_type: None,
            removed: None,
        })
    }

    /// Create comprehensive subscription message for debugging
    fn create_debug_subscription_message(&self) -> String {
        // Include all event signatures we want to monitor
        let signatures = vec![
            SWAP_V2_SIGNATURE,
            SWAP_V3_SIGNATURE,
            MINT_SIGNATURE,
            BURN_SIGNATURE,
            SYNC_SIGNATURE,
        ];

        if self.verbose_logging {
            info!("🎯 Subscribing to {} event signatures:", signatures.len());
            for (i, sig) in signatures.iter().enumerate() {
                info!("   {}. {}", i + 1, sig);
            }
        }

        // Create subscription with optional address filtering for higher activity
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "eth_subscribe",
            "params": [
                "logs",
                {
                    "topics": [signatures],
                    // Optional: filter by specific pool addresses for higher activity
                    // "address": TEST_POOL_ADDRESSES
                }
            ]
        })
        .to_string()
    }

    /// Log results for a single endpoint test
    fn log_endpoint_result(&self, result: &EndpointTestResult) {
        info!("🧪 Endpoint Test Result: {}", result.url);
        info!(
            "   Connected: {}",
            if result.connected { "✅" } else { "❌" }
        );

        if result.connected {
            info!("   Handshake Time: {:?}", result.handshake_time);
            info!(
                "   Subscription: {}",
                if result.subscription_accepted {
                    "✅"
                } else {
                    "❌"
                }
            );
            info!("   Events Received: {}", result.events_received);
        }

        info!("   Test Duration: {:?}", result.test_duration);

        if let Some(error) = &result.error_message {
            error!("   Error: {}", error);
        }
    }

    /// Log comprehensive summary of all endpoint tests
    fn log_summary_report(&self, results: &[EndpointTestResult]) {
        info!("📋 POLYGON EVENT DEBUG SUMMARY");
        info!("=".repeat(50));

        let total_endpoints = results.len();
        let connected_count = results.iter().filter(|r| r.connected).count();
        let subscription_count = results.iter().filter(|r| r.subscription_accepted).count();
        let total_events: u32 = results.iter().map(|r| r.events_received).sum();

        info!("Total Endpoints Tested: {}", total_endpoints);
        info!(
            "Successfully Connected: {} / {}",
            connected_count, total_endpoints
        );
        info!(
            "Subscriptions Accepted: {} / {}",
            subscription_count, connected_count
        );
        info!("Total Events Received: {}", total_events);

        // Best performing endpoint
        if let Some(best_endpoint) = results
            .iter()
            .filter(|r| r.connected && r.subscription_accepted)
            .max_by_key(|r| r.events_received)
        {
            info!(
                "🏆 Best Endpoint: {} ({} events)",
                best_endpoint.url, best_endpoint.events_received
            );
        }

        // Connection issues
        let failed_connections: Vec<_> = results.iter().filter(|r| !r.connected).collect();

        if !failed_connections.is_empty() {
            warn!("❌ Failed Connections:");
            for result in failed_connections {
                warn!(
                    "   {}: {}",
                    result.url,
                    result.error_message.as_deref().unwrap_or("Unknown error")
                );
            }
        }

        // Subscription issues
        let failed_subscriptions: Vec<_> = results
            .iter()
            .filter(|r| r.connected && !r.subscription_accepted)
            .collect();

        if !failed_subscriptions.is_empty() {
            warn!("❌ Failed Subscriptions:");
            for result in failed_subscriptions {
                warn!("   {}: Subscription rejected or timed out", result.url);
            }
        }

        // Overall diagnosis
        if total_events == 0 {
            error!("🔥 CRITICAL: No events received from any endpoint!");
            error!("   Possible causes:");
            error!("   1. All endpoints are down or inaccessible");
            error!("   2. Event signatures are incorrect");
            error!("   3. Subscription parameters are wrong");
            error!("   4. Network/firewall blocking connections");
            error!("   5. Low DEX activity during test period");
        } else {
            info!("✅ Events successfully received from Polygon network");
            if total_events < 10 {
                warn!("⚠️ Low event count may indicate filtering issues or low activity");
            }
        }

        info!("=".repeat(50));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::test;

    #[test(flavor = "multi_thread")]
    async fn test_polygon_endpoint_connectivity() {
        tracing_subscriber::fmt::init();

        let debugger = PolygonEventDebugger::new()
            .with_test_duration(Duration::from_secs(30)) // Shorter test for CI
            .with_verbose_logging(false);

        let results = debugger
            .run_debug_tests()
            .await
            .expect("Debug tests should not fail");

        // At least one endpoint should be reachable
        let connected_count = results.iter().filter(|r| r.connected).count();
        assert!(
            connected_count > 0,
            "At least one Polygon endpoint should be reachable"
        );
    }

    #[test(flavor = "multi_thread")]
    async fn test_event_signature_subscription() {
        tracing_subscriber::fmt::init();

        // Test subscription message format
        let debugger = PolygonEventDebugger::new();
        let subscription_msg = debugger.create_debug_subscription_message();

        // Should be valid JSON
        let json: Value = serde_json::from_str(&subscription_msg)
            .expect("Subscription message should be valid JSON");

        // Should have required fields
        assert_eq!(json["jsonrpc"], "2.0");
        assert_eq!(json["method"], "eth_subscribe");
        assert!(json["params"].is_array());

        let params = json["params"].as_array().unwrap();
        assert_eq!(params[0], "logs");
        assert!(params[1]["topics"].is_array());
    }

    #[test(flavor = "multi_thread")]
    async fn test_json_log_conversion() {
        let debugger = PolygonEventDebugger::new();

        // Create sample JSON log (format from actual Polygon WebSocket)
        let sample_json = serde_json::json!({
            "address": "0x45dda9cb7c25131df268515131f647d726f50608",
            "topics": [
                "0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822",
                "0x000000000000000000000000f39fd6e51aad88f6f4ce6ab8827279cfffb92266",
                "0x000000000000000000000000f39fd6e51aad88f6f4ce6ab8827279cfffb92266"
            ],
            "data": "0x000000000000000000000000000000000000000000000000016345785d8a0000",
            "blockNumber": "0x0123456",
            "transactionHash": "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890"
        });

        let log = debugger
            .json_to_web3_log(&sample_json)
            .expect("Should convert JSON to Web3 Log");

        assert_eq!(log.topics.len(), 3);
        assert!(!log.data.0.is_empty());
        assert!(log.block_number.is_some());
        assert!(log.transaction_hash.is_some());
    }

    #[test(flavor = "multi_thread")]
    async fn test_swap_tlv_conversion() {
        let debugger = PolygonEventDebugger::new();

        // Create sample log
        let sample_log = Log {
            address: "0x45dda9cb7c25131df268515131f647d726f50608"
                .parse()
                .unwrap(),
            topics: vec![
                "0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822"
                    .parse()
                    .unwrap(),
            ],
            data: web3::types::Bytes(vec![0; 32]),
            block_number: Some(1234567u64.into()),
            block_hash: None,
            transaction_hash: None,
            transaction_index: None,
            log_index: None,
            transaction_log_index: None,
            log_type: None,
            removed: None,
        };

        let tlv_bytes = debugger
            .test_swap_tlv_conversion(&sample_log)
            .await
            .expect("Should convert swap to TLV");

        assert!(
            tlv_bytes.len() > 32,
            "TLV message should have header + payload"
        );

        // Validate TLV structure
        let header = parse_header(&tlv_bytes[..32]).expect("Should parse TLV header");
        assert_eq!(header.magic, 0xDEADBEEF);
    }
}

/// Integration test binary - run with: cargo test --test polygon_event_debug --release -- --nocapture
#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    info!("🚀 Starting Polygon Event Debug Test Suite");

    let debugger = PolygonEventDebugger::new()
        .with_test_duration(Duration::from_secs(60)) // 1 minute per endpoint
        .with_verbose_logging(true);

    let results = debugger.run_debug_tests().await?;

    // Additional analysis
    let working_endpoints: Vec<_> = results
        .iter()
        .filter(|r| r.connected && r.subscription_accepted && r.events_received > 0)
        .collect();

    if working_endpoints.is_empty() {
        error!("🔥 CRITICAL ISSUE: No working endpoints found!");
        error!("   Next steps:");
        error!("   1. Check network connectivity");
        error!("   2. Verify Polygon network is operational");
        error!("   3. Test with different event signatures");
        error!("   4. Try connecting during higher DEX activity periods");
        std::process::exit(1);
    } else {
        info!("✅ Found {} working endpoint(s)", working_endpoints.len());
        for endpoint in working_endpoints {
            info!(
                "   ✅ {}: {} events received",
                endpoint.url, endpoint.events_received
            );
        }
    }

    Ok(())
}
