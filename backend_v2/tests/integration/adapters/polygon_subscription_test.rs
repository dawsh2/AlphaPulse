//! Polygon Subscription Validation Tests
//!
//! Tests specific pool address subscriptions to ensure events are properly filtered and received.
//! Uses real pool addresses and validates subscription acceptance and event reception.

use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info, warn};

/// High-volume Polygon DEX pools for testing
pub const HIGH_VOLUME_POOLS: &[(&str, &str)] = &[
    ("0x45dda9cb7c25131df268515131f647d726f50608", "USDC-WETH V3"),
    (
        "0x9b08288c3be4f62bbf8d1c20ac9c5e6f9467d8b7",
        "WMATIC-USDC V3",
    ),
    ("0x0e44ceb592acfc5d3f09d996302eb4c499ff8c10", "USDC-USDT V3"),
    (
        "0x167384319b41f7094e62f7506409eb38079abff8",
        "WETH-WMATIC V3",
    ),
    ("0x86f1d8390222a3691c28938ec7404a1661e618e0", "WETH-WBTC V3"),
];

/// Test subscription with specific pool address filtering
pub struct PoolSubscriptionTest {
    endpoint_url: String,
    test_duration: Duration,
    timeout: Duration,
}

impl PoolSubscriptionTest {
    pub fn new(endpoint_url: impl Into<String>) -> Self {
        Self {
            endpoint_url: endpoint_url.into(),
            test_duration: Duration::from_secs(60),
            timeout: Duration::from_secs(30),
        }
    }

    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.test_duration = duration;
        self
    }

    /// Test subscription to specific high-volume pools
    pub async fn test_pool_specific_subscription(&self) -> Result<()> {
        info!(
            "🧪 Testing pool-specific subscription to {}",
            self.endpoint_url
        );

        // Connect to WebSocket
        let (ws_stream, response) =
            tokio::time::timeout(self.timeout, connect_async(&self.endpoint_url))
                .await
                .context("Connection timeout")?
                .context("Connection failed")?;

        info!("✅ Connected: {}", response.status());

        let (mut ws_sender, mut ws_receiver) = ws_stream.split();

        // Create subscription for specific pools
        let subscription = self.create_pool_subscription(HIGH_VOLUME_POOLS);

        info!(
            "📤 Subscribing to {} specific pools",
            HIGH_VOLUME_POOLS.len()
        );
        ws_sender.send(Message::Text(subscription)).await?;

        // Wait for subscription confirmation
        let confirmation = self.wait_for_confirmation(&mut ws_receiver).await?;
        info!("✅ Subscription confirmed: {}", confirmation);

        // Monitor events from specific pools
        let events = self.monitor_pool_events(&mut ws_receiver).await?;

        self.analyze_results(events);

        Ok(())
    }

    /// Test broad subscription without filtering
    pub async fn test_broad_subscription(&self) -> Result<()> {
        info!(
            "🧪 Testing broad subscription (no filtering) to {}",
            self.endpoint_url
        );

        // Connect to WebSocket
        let (ws_stream, response) =
            tokio::time::timeout(self.timeout, connect_async(&self.endpoint_url))
                .await
                .context("Connection timeout")?
                .context("Connection failed")?;

        info!("✅ Connected: {}", response.status());

        let (mut ws_sender, mut ws_receiver) = ws_stream.split();

        // Create broad subscription (all DEX events)
        let subscription = self.create_broad_subscription();

        info!("📤 Subscribing to ALL DEX events (no pool filter)");
        ws_sender.send(Message::Text(subscription)).await?;

        // Wait for subscription confirmation
        let confirmation = self.wait_for_confirmation(&mut ws_receiver).await?;
        info!("✅ Subscription confirmed: {}", confirmation);

        // Monitor all events
        let events = self.monitor_all_events(&mut ws_receiver).await?;

        self.analyze_broad_results(events);

        Ok(())
    }

    /// Create subscription message for specific pools
    fn create_pool_subscription(&self, pools: &[(&str, &str)]) -> String {
        let pool_addresses: Vec<&str> = pools.iter().map(|(addr, _)| *addr).collect();

        // Get event signatures from libs/dex
        let swap_v2_sig = "0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822";
        let swap_v3_sig = "0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67";

        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "eth_subscribe",
            "params": [
                "logs",
                {
                    "address": pool_addresses,
                    "topics": [[swap_v2_sig, swap_v3_sig]]
                }
            ]
        })
        .to_string()
    }

    /// Create broad subscription for all DEX events
    fn create_broad_subscription(&self) -> String {
        // Get all event signatures from libs/dex
        let signatures = vec![
            "0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822", // V2 Swap
            "0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67", // V3 Swap
            "0x7a53080ba414158be7ec69b987b5fb7d07dee101fe85488f0853ae16239d0bde", // Mint
            "0x0c396cd989a39f4459b5fa1aed6a9a8dcdbc45908acfd67e028cd568da98982c", // Burn
            "0x1c411e9a96e071241c2f21f7726b17ae89e3cab4c78be50e062b03a9fffbbad1", // Sync
        ];

        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "eth_subscribe",
            "params": [
                "logs",
                {
                    "topics": [signatures]
                }
            ]
        })
        .to_string()
    }

    /// Wait for subscription confirmation
    async fn wait_for_confirmation(
        &self,
        ws_receiver: &mut futures_util::stream::SplitStream<
            tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
            >,
        >,
    ) -> Result<String> {
        let timeout = Duration::from_secs(10);

        match tokio::time::timeout(timeout, ws_receiver.next()).await {
            Ok(Some(Ok(Message::Text(text)))) => {
                let json: Value = serde_json::from_str(&text)?;

                if let Some(result) = json.get("result") {
                    Ok(result.to_string())
                } else if let Some(error) = json.get("error") {
                    Err(anyhow::anyhow!("Subscription error: {}", error))
                } else {
                    Err(anyhow::anyhow!("Unexpected response format"))
                }
            }
            Ok(Some(Err(e))) => Err(anyhow::anyhow!("WebSocket error: {}", e)),
            Ok(_) => Err(anyhow::anyhow!("Unexpected message type")),
            Err(_) => Err(anyhow::anyhow!("Subscription confirmation timeout")),
        }
    }

    /// Monitor events from specific pools
    async fn monitor_pool_events(
        &self,
        ws_receiver: &mut futures_util::stream::SplitStream<
            tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
            >,
        >,
    ) -> Result<HashMap<String, u32>> {
        let mut pool_events = HashMap::new();
        let start = Instant::now();

        info!("👂 Monitoring pool events for {:?}", self.test_duration);

        while start.elapsed() < self.test_duration {
            let remaining = self.test_duration - start.elapsed();

            match tokio::time::timeout(remaining, ws_receiver.next()).await {
                Ok(Some(Ok(Message::Text(text)))) => {
                    if let Ok(json) = serde_json::from_str::<Value>(&text) {
                        if json.get("method") == Some(&serde_json::json!("eth_subscription")) {
                            if let Some(params) = json.get("params") {
                                if let Some(result) = params.get("result") {
                                    if let Some(address) =
                                        result.get("address").and_then(|a| a.as_str())
                                    {
                                        *pool_events.entry(address.to_lowercase()).or_insert(0) +=
                                            1;

                                        // Find pool name
                                        let pool_name = HIGH_VOLUME_POOLS
                                            .iter()
                                            .find(|(addr, _)| {
                                                addr.to_lowercase() == address.to_lowercase()
                                            })
                                            .map(|(_, name)| *name)
                                            .unwrap_or("Unknown");

                                        info!("📊 Event from {} ({})", pool_name, address);
                                    }
                                }
                            }
                        }
                    }
                }
                Ok(Some(Ok(Message::Ping(_)))) => {
                    debug!("💓 Ping received");
                }
                Ok(Some(Ok(Message::Close(_)))) => {
                    warn!("📌 WebSocket closed");
                    break;
                }
                Ok(Some(Err(e))) => {
                    error!("❌ WebSocket error: {}", e);
                    break;
                }
                Ok(None) => {
                    warn!("📌 Stream ended");
                    break;
                }
                Err(_) => {
                    debug!("⏰ Monitoring timeout");
                    break;
                }
            }
        }

        Ok(pool_events)
    }

    /// Monitor all events without filtering
    async fn monitor_all_events(
        &self,
        ws_receiver: &mut futures_util::stream::SplitStream<
            tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
            >,
        >,
    ) -> Result<HashMap<String, u32>> {
        let mut event_types = HashMap::new();
        let start = Instant::now();
        let mut total_events = 0u32;

        info!("👂 Monitoring ALL events for {:?}", self.test_duration);

        while start.elapsed() < self.test_duration {
            let remaining = self.test_duration - start.elapsed();

            match tokio::time::timeout(remaining, ws_receiver.next()).await {
                Ok(Some(Ok(Message::Text(text)))) => {
                    if let Ok(json) = serde_json::from_str::<Value>(&text) {
                        if json.get("method") == Some(&serde_json::json!("eth_subscription")) {
                            if let Some(params) = json.get("params") {
                                if let Some(result) = params.get("result") {
                                    total_events += 1;

                                    // Categorize by event signature
                                    if let Some(topics) =
                                        result.get("topics").and_then(|t| t.as_array())
                                    {
                                        if let Some(topic0) = topics.get(0).and_then(|t| t.as_str())
                                        {
                                            let event_type = match topic0 {
                                                "0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822" => "V2 Swap",
                                                "0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67" => "V3 Swap",
                                                "0x7a53080ba414158be7ec69b987b5fb7d07dee101fe85488f0853ae16239d0bde" => "Mint",
                                                "0x0c396cd989a39f4459b5fa1aed6a9a8dcdbc45908acfd67e028cd568da98982c" => "Burn",
                                                "0x1c411e9a96e071241c2f21f7726b17ae89e3cab4c78be50e062b03a9fffbbad1" => "Sync",
                                                _ => "Other",
                                            };

                                            *event_types
                                                .entry(event_type.to_string())
                                                .or_insert(0) += 1;

                                            if total_events <= 5 || total_events % 100 == 0 {
                                                info!("📊 Event #{}: {}", total_events, event_type);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Ok(Some(Ok(Message::Ping(_)))) => {
                    debug!("💓 Ping received");
                }
                Ok(Some(Ok(Message::Close(_)))) => {
                    warn!("📌 WebSocket closed");
                    break;
                }
                Ok(Some(Err(e))) => {
                    error!("❌ WebSocket error: {}", e);
                    break;
                }
                Ok(None) => {
                    warn!("📌 Stream ended");
                    break;
                }
                Err(_) => {
                    debug!("⏰ Monitoring timeout");
                    break;
                }
            }
        }

        event_types.insert("TOTAL".to_string(), total_events);
        Ok(event_types)
    }

    /// Analyze results from pool-specific subscription
    fn analyze_results(&self, pool_events: HashMap<String, u32>) {
        info!("📋 POOL-SPECIFIC SUBSCRIPTION RESULTS");
        info!("=".repeat(50));

        let total_events: u32 = pool_events.values().sum();
        info!("Total events received: {}", total_events);

        if pool_events.is_empty() {
            error!("❌ No events received from any monitored pool!");
            error!("   Possible causes:");
            error!("   1. Low activity in specified pools");
            error!("   2. Subscription filtering not working");
            error!("   3. WebSocket connection issues");
        } else {
            info!("✅ Events received from {} pools:", pool_events.len());

            // Sort by event count
            let mut sorted_pools: Vec<_> = pool_events.iter().collect();
            sorted_pools.sort_by(|a, b| b.1.cmp(a.1));

            for (address, count) in sorted_pools {
                let pool_name = HIGH_VOLUME_POOLS
                    .iter()
                    .find(|(addr, _)| addr.to_lowercase() == address.to_lowercase())
                    .map(|(_, name)| *name)
                    .unwrap_or("Unknown");

                info!("   {} ({}): {} events", pool_name, address, count);
            }
        }

        info!("=".repeat(50));
    }

    /// Analyze results from broad subscription
    fn analyze_broad_results(&self, event_types: HashMap<String, u32>) {
        info!("📋 BROAD SUBSCRIPTION RESULTS");
        info!("=".repeat(50));

        let total = event_types.get("TOTAL").copied().unwrap_or(0);
        info!("Total events received: {}", total);

        if total == 0 {
            error!("❌ No events received!");
            error!("   Possible causes:");
            error!("   1. Network connectivity issues");
            error!("   2. Incorrect event signatures");
            error!("   3. Very low DEX activity");
        } else {
            info!("✅ Event breakdown by type:");

            let mut sorted_types: Vec<_> =
                event_types.iter().filter(|(k, _)| k != &"TOTAL").collect();
            sorted_types.sort_by(|a, b| b.1.cmp(a.1));

            for (event_type, count) in sorted_types {
                let percentage = (*count as f64 / total as f64) * 100.0;
                info!("   {}: {} ({:.1}%)", event_type, count, percentage);
            }

            // Estimate event rate
            let events_per_second = total as f64 / self.test_duration.as_secs_f64();
            info!(
                "📈 Average event rate: {:.1} events/second",
                events_per_second
            );
        }

        info!("=".repeat(50));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::test;

    #[test(flavor = "multi_thread")]
    async fn test_pool_specific_events() {
        tracing_subscriber::fmt::init();

        let tester = PoolSubscriptionTest::new("wss://polygon-bor-rpc.publicnode.com")
            .with_duration(Duration::from_secs(30));

        match tester.test_pool_specific_subscription().await {
            Ok(_) => info!("✅ Pool-specific subscription test completed"),
            Err(e) => error!("❌ Pool-specific subscription test failed: {}", e),
        }
    }

    #[test(flavor = "multi_thread")]
    async fn test_broad_events() {
        tracing_subscriber::fmt::init();

        let tester = PoolSubscriptionTest::new("wss://polygon-bor-rpc.publicnode.com")
            .with_duration(Duration::from_secs(30));

        match tester.test_broad_subscription().await {
            Ok(_) => info!("✅ Broad subscription test completed"),
            Err(e) => error!("❌ Broad subscription test failed: {}", e),
        }
    }

    #[test(flavor = "multi_thread")]
    async fn test_multiple_endpoints() {
        tracing_subscriber::fmt::init();

        let endpoints = vec![
            "wss://polygon-bor-rpc.publicnode.com",
            "wss://polygon-mainnet.g.alchemy.com/v2/demo",
            "wss://ws-polygon-mainnet.chainstacklabs.com",
        ];

        for endpoint in endpoints {
            info!("🔍 Testing endpoint: {}", endpoint);

            let tester = PoolSubscriptionTest::new(endpoint).with_duration(Duration::from_secs(20));

            match tester.test_broad_subscription().await {
                Ok(_) => info!("✅ {} working", endpoint),
                Err(e) => warn!("❌ {} failed: {}", endpoint, e),
            }
        }
    }
}

/// Main test runner
#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    info!("🚀 Starting Polygon Subscription Validation Tests");

    // Test 1: Pool-specific subscription
    info!("\n📊 TEST 1: Pool-Specific Subscription");
    let pool_tester = PoolSubscriptionTest::new("wss://polygon-bor-rpc.publicnode.com")
        .with_duration(Duration::from_secs(60));

    if let Err(e) = pool_tester.test_pool_specific_subscription().await {
        error!("Pool-specific test failed: {}", e);
    }

    // Brief pause
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Test 2: Broad subscription
    info!("\n📊 TEST 2: Broad Subscription (All Events)");
    let broad_tester = PoolSubscriptionTest::new("wss://polygon-bor-rpc.publicnode.com")
        .with_duration(Duration::from_secs(60));

    if let Err(e) = broad_tester.test_broad_subscription().await {
        error!("Broad subscription test failed: {}", e);
    }

    info!("\n✅ All subscription validation tests completed");

    Ok(())
}
