//! WebSocket-based gas price collector for Polygon
//!
//! Subscribes to newHeads events to get real-time base fee updates
//! without RPC rate limiting issues.

use torq_codec::TLVMessageBuilder;
use torq_types::tlv::gas_price::GasPriceTLV;
use torq_types::{RelayDomain, SourceType};
use torq_codec::TLVType;
use anyhow::{Context, Result};
use ethers::prelude::*;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::net::UnixStream;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Default priority fee for Polygon (2 gwei)
/// Will be replaced with dynamic calculation in Phase 2
const DEFAULT_PRIORITY_FEE_GWEI: u32 = 2;

/// Gas price collector configuration
#[derive(Debug, Clone)]
pub struct GasPriceCollectorConfig {
    /// WebSocket RPC endpoint
    pub ws_endpoint: String,
    /// Unix socket path for relay
    pub relay_socket_path: String,
    /// Network ID (137 for Polygon)
    pub network_id: u16,
    /// Static priority fee (until Phase 2)
    pub priority_fee_gwei: u32,
}

impl Default for GasPriceCollectorConfig {
    fn default() -> Self {
        Self {
            ws_endpoint: std::env::var("POLYGON_WS_ENDPOINT")
                .expect("POLYGON_WS_ENDPOINT environment variable must be set"),
            relay_socket_path: std::env::var("MARKET_DATA_RELAY_SOCKET")
                .unwrap_or_else(|_| "/tmp/market_data_relay.sock".to_string()),
            network_id: 137, // Polygon
            priority_fee_gwei: DEFAULT_PRIORITY_FEE_GWEI,
        }
    }
}

/// Gas price statistics for Phase 2
#[derive(Debug, Clone)]
pub struct GasPriceStats {
    pub last_base_fee_gwei: u32,
    pub last_priority_fee_gwei: u32,
    /// Average priority fee in gwei * 1000 (3 decimal precision)
    pub avg_priority_fee_milligwei: u64,
    pub block_number: u64,
    pub timestamp_ns: u64,
}

/// WebSocket-based gas price collector
pub struct GasPriceCollector {
    config: GasPriceCollectorConfig,
    provider: Provider<Ws>,
    relay_socket: Arc<RwLock<Option<UnixStream>>>,
    stats: Arc<RwLock<GasPriceStats>>,
}

impl GasPriceCollector {
    /// Create a new gas price collector
    pub async fn new(config: GasPriceCollectorConfig) -> Result<Self> {
        info!(
            "⛽ Connecting to WebSocket endpoint: {}",
            config.ws_endpoint
        );

        // Connect with automatic reconnection
        let provider = Provider::<Ws>::connect_with_reconnects(
            &config.ws_endpoint,
            10, // max reconnects
        )
        .await
        .context("Failed to connect to WebSocket")?;

        info!("✅ WebSocket connected successfully");

        let stats = Arc::new(RwLock::new(GasPriceStats {
            last_base_fee_gwei: 30,
            last_priority_fee_gwei: config.priority_fee_gwei,
            avg_priority_fee_milligwei: (config.priority_fee_gwei as u64) * 1000,
            block_number: 0,
            timestamp_ns: 0,
        }));

        Ok(Self {
            config,
            provider,
            relay_socket: Arc::new(RwLock::new(None)),
            stats,
        })
    }

    /// Connect to the market data relay
    async fn connect_to_relay(&self) -> Result<()> {
        info!(
            "🌐 Connecting to MarketDataRelay at {}",
            self.config.relay_socket_path
        );

        match UnixStream::connect(&self.config.relay_socket_path).await {
            Ok(socket) => {
                let mut relay = self.relay_socket.write().await;
                *relay = Some(socket);
                info!("✅ Connected to MarketDataRelay");
                Ok(())
            }
            Err(e) => {
                error!("❌ Failed to connect to relay: {}", e);
                Err(e.into())
            }
        }
    }

    /// Start streaming gas prices via WebSocket with automatic reconnection
    pub async fn start_streaming(self: Arc<Self>) -> Result<()> {
        loop {
            if let Err(e) = self.stream_with_reconnect().await {
                error!("⚠️ WebSocket stream failed: {}. Reconnecting in 5s...", e);
                tokio::time::sleep(Duration::from_secs(5)).await;
                continue;
            }
        }
    }

    async fn stream_with_reconnect(&self) -> Result<()> {
        // Connect to relay
        self.connect_to_relay().await?;

        info!("⛽ Subscribing to newHeads events...");

        // Subscribe to new block headers
        let mut stream = self
            .provider
            .subscribe_blocks()
            .await
            .context("Failed to subscribe to blocks")?;

        info!("✅ Subscription active, streaming gas prices...");

        while let Some(block) = stream.next().await {
            // Extract gas price information
            if let Some(base_fee) = block.base_fee_per_gas {
                let base_fee_gwei = (base_fee.as_u64() / 1_000_000_000) as u32;
                let block_number = block.number.unwrap_or_default().as_u64();
                let timestamp_ns = torq_network::time::safe_system_timestamp_ns();

                // Create GasPriceTLV
                let gas_price_tlv = GasPriceTLV::new(
                    self.config.network_id,
                    base_fee_gwei,
                    self.config.priority_fee_gwei, // Static for Phase 1
                    block_number,
                    timestamp_ns,
                );

                // Update stats
                {
                    let mut stats = self.stats.write().await;
                    stats.last_base_fee_gwei = base_fee_gwei;
                    stats.block_number = block_number;
                    stats.timestamp_ns = timestamp_ns;
                }

                // Copy packed field to avoid alignment issues
                let total_gas_price_gwei = gas_price_tlv.gas_price_gwei;
                debug!(
                    "📊 Block {}: base_fee={}gwei, priority={}gwei, total={}gwei",
                    block_number,
                    base_fee_gwei,
                    self.config.priority_fee_gwei,
                    total_gas_price_gwei
                );

                // Send to relay
                if let Err(e) = self.send_to_relay(gas_price_tlv).await {
                    warn!("Failed to send gas price to relay: {}", e);
                    // Try to reconnect
                    if let Err(e) = self.connect_to_relay().await {
                        error!("Failed to reconnect to relay: {}", e);
                    }
                }
            }
        }

        warn!("⚠️ Block stream ended, WebSocket may have disconnected");
        Ok(())
    }

    /// Send gas price TLV to relay via Unix socket
    async fn send_to_relay(&self, tlv: GasPriceTLV) -> Result<()> {
        // Build TLV message
        let builder = TLVMessageBuilder::new(
            RelayDomain::MarketData,
            SourceType::MetricsCollector,
        );

        let builder = builder.add_tlv(TLVType::GasPrice, &tlv);
        let message = builder.build()?;

        // Write to Unix socket (thread-safe)
        {
            let mut relay_guard = self.relay_socket.write().await;
            if let Some(socket) = relay_guard.as_mut() {
                use tokio::io::AsyncWriteExt;
                socket
                    .write_all(&message)
                    .await
                    .context("Failed to write message to relay socket")?;
                socket
                    .flush()
                    .await
                    .context("Failed to flush relay socket")?;

                debug!("✅ Sent {} byte gas price message to relay", message.len());
                Ok(())
            } else {
                Err(anyhow::anyhow!("Not connected to relay"))
            }
        }
    }

    /// Get current gas price statistics
    pub async fn get_stats(&self) -> GasPriceStats {
        self.stats.read().await.clone()
    }
}

/// Phase 2: Priority fee calculator from DEX events
/// This will consume the swap event stream to calculate market-driven priority fees
pub struct PriorityFeeCalculator {
    /// Moving average of observed priority fees in milligwei (gwei * 1000)
    avg_priority_fee_milligwei: RwLock<u64>,
    /// Number of samples in average
    sample_count: RwLock<usize>,
}

impl PriorityFeeCalculator {
    pub fn new() -> Self {
        Self {
            avg_priority_fee_milligwei: RwLock::new(2000), // Start with 2 gwei (2000 milligwei) default
            sample_count: RwLock::new(0),
        }
    }

    /// Update priority fee estimate from observed transaction
    pub async fn update_from_transaction(&self, tx_gas_price: U256, block_base_fee: U256) {
        let priority_fee_wei = tx_gas_price.saturating_sub(block_base_fee);
        // Convert to milligwei (gwei * 1000) for 3 decimal precision
        let priority_fee_milligwei = priority_fee_wei.as_u64() / 1_000_000;

        // Update moving average using integer arithmetic
        let mut avg = self.avg_priority_fee_milligwei.write().await;
        let mut count = self.sample_count.write().await;

        *count += 1;
        *avg = ((*avg * (*count - 1) as u64) + priority_fee_milligwei) / *count as u64;

        debug!(
            "Updated priority fee estimate: {:.2} gwei (from {} samples)",
            *avg, *count
        );
    }

    /// Get current priority fee recommendation
    pub async fn get_priority_fee_gwei(&self) -> u32 {
        let avg = *self.avg_priority_fee.read().await;
        // Add 10% buffer to ensure inclusion
        ((avg * 1.1) as u32).max(1) // At least 1 gwei
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_priority_fee_calculator() {
        let calc = PriorityFeeCalculator::new();

        // Simulate some transactions
        let base_fee = U256::from(30_000_000_000u64); // 30 gwei

        calc.update_from_transaction(
            U256::from(32_000_000_000u64), // 32 gwei total
            base_fee,
        )
        .await;

        calc.update_from_transaction(
            U256::from(33_000_000_000u64), // 33 gwei total
            base_fee,
        )
        .await;

        let recommended = calc.get_priority_fee_gwei().await;
        assert!(recommended >= 2); // Should be at least 2 gwei
        assert!(recommended <= 5); // Should be reasonable
    }
}
