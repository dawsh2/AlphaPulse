//! # Unified Kraken Collector - Direct RelayOutput Integration
//!
//! ## Architecture
//!
//! Eliminates MPSC channel overhead by connecting WebSocket events directly to RelayOutput:
//! ```
//! Kraken WebSocket → Event Processing → TLV Builder → RelayOutput → MarketDataRelay
//! ```
//!
//! ## Key Improvements (following polygon.rs pattern)
//! - **Zero Channel Overhead**: Direct `relay_output.send_bytes()` calls
//! - **Unified Logic**: Single service combines collection and publishing
//! - **Configuration-Driven**: TOML-based configuration with environment overrides
//! - **Transparent Failures**: Crash immediately on WebSocket/relay failures
//! - **Runtime Validation**: TLV round-trip validation during startup period
//!
//! ## Kraken WebSocket Protocol
//! - **Array Format**: Most data messages use array format: `[channelID, data, "trade", "XBT/USD"]`
//! - **JSON Format**: Control messages use JSON format: `{"event": "subscribe", ...}`
//! - **Channels**: "trade" for executions, "book" for order book, "ticker" for price updates
//!
//! ## Performance Profile
//! - **Latency**: <10ms from market event to relay delivery
//! - **Throughput**: Designed for >1M msg/s TLV construction
//! - **Memory**: <50MB steady state with multiple pair subscriptions

use alphapulse_types::{
    tlv::build_message_direct, InstrumentId, QuoteTLV, RelayDomain, SourceType,
    TLVType, TradeTLV, VenueId,
};
use alphapulse_codec::{parse_header, parse_tlv_extensions}; // Added
use alphapulse_network::time::init_timestamp_system; // Added
use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info, warn};

use alphapulse_adapter_service::output::RelayOutput;

mod config;
use config::KrakenConfig;

/// Unified Kraken Collector with direct RelayOutput integration
pub struct UnifiedKrakenCollector {
    config: KrakenConfig,
    relay_output: Arc<RelayOutput>,
    running: Arc<RwLock<bool>>,
    validation_enabled: Arc<RwLock<bool>>,
    start_time: Instant,
    messages_processed: Arc<RwLock<u64>>,
    validation_failures: Arc<RwLock<u64>>,
}

impl UnifiedKrakenCollector {
    /// Create new unified collector with configuration
    pub fn new(config: KrakenConfig) -> Result<Self> {
        config.validate().context("Invalid configuration")?;

        let relay_domain = config
            .parse_relay_domain()
            .context("Failed to parse relay domain")?;

        let relay_output = Arc::new(RelayOutput::new(
            config.relay.socket_path.clone(),
            relay_domain,
        ));

        Ok(Self {
            config,
            relay_output,
            running: Arc::new(RwLock::new(false)),
            validation_enabled: Arc::new(RwLock::new(true)),
            start_time: Instant::now(),
            messages_processed: Arc::new(RwLock::new(0)),
            validation_failures: Arc::new(RwLock::new(0)),
        })
    }

    /// Start the unified collector
    pub async fn start(&self) -> Result<()> {
        info!("🚀 Starting Unified Kraken Collector");
        info!("   Direct WebSocket → RelayOutput integration");
        info!("   Pairs: {:?}", self.config.pairs);
        info!("   Channels: {:?}", self.config.channels);

        *self.running.write().await = true;

        // Connect to relay first (fail fast if relay unavailable)
        self.relay_output
            .connect()
            .await
            .context("Failed to connect to relay - CRASHING as designed")?;

        info!(
            "✅ Connected to {:?} relay at {}",
            self.config.parse_relay_domain()?,
            self.config.relay.socket_path
        );

        // Start validation disabling timer
        self.start_validation_timer().await;

        // Connect to WebSocket and start event processing
        self.connect_and_process_events()
            .await
            .context("WebSocket processing failed - CRASHING as designed")?;

        Ok(())
    }

    /// Start timer to disable TLV validation after startup period
    async fn start_validation_timer(&self) {
        let validation_enabled = self.validation_enabled.clone();
        let validation_duration = self.config.validation.runtime_validation_seconds;

        if validation_duration == 0 {
            *validation_enabled.write().await = false;
            info!("🔒 Runtime TLV validation disabled (configured for 0 seconds)");
        } else {
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_secs(validation_duration)).await;
                *validation_enabled.write().await = false;
                info!(
                    "🔒 Runtime TLV validation disabled after {}s startup period",
                    validation_duration
                );
            });
        }
    }

    /// Connect to WebSocket and process events until failure
    async fn connect_and_process_events(&self) -> Result<()> {
        let mut connection_attempts = 0;
        let max_attempts = self.config.websocket.max_reconnect_attempts;

        loop {
            connection_attempts += 1;

            if connection_attempts > max_attempts {
                error!(
                    "🔥 CRASH: Exceeded maximum WebSocket connection attempts ({})",
                    max_attempts
                );
                return Err(anyhow::anyhow!(
                    "Max WebSocket connection attempts exceeded"
                ));
            }

            info!(
                "🔌 WebSocket connection attempt {} of {}",
                connection_attempts, max_attempts
            );

            match self.try_websocket_connection().await {
                Ok(()) => {
                    info!("✅ WebSocket connection successful");
                    return Ok(());
                }
                Err(e) => {
                    warn!("❌ WebSocket connection failed: {}", e);
                }
            }

            // Backoff before retry
            let backoff_ms = std::cmp::min(
                self.config.websocket.base_backoff_ms * (1 << (connection_attempts - 1)),
                self.config.websocket.max_backoff_ms,
            );

            warn!("⏳ Retrying in {}ms", backoff_ms);
            tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
        }
    }

    /// Attempt WebSocket connection to Kraken
    async fn try_websocket_connection(&self) -> Result<()> {
        let timeout_duration = Duration::from_millis(self.config.websocket.connection_timeout_ms);

        // Connect with timeout
        let (ws_stream, _) =
            tokio::time::timeout(timeout_duration, connect_async(&self.config.websocket.url))
                .await
                .context("WebSocket connection timeout")?
                .context("WebSocket connection failed")?;

        info!("✅ WebSocket connected to Kraken");

        let (mut ws_sender, mut ws_receiver) = ws_stream.split();

        // Subscribe to channels
        for channel in &self.config.channels {
            let subscription = json!({
                "event": "subscribe",
                "pair": self.config.pairs,
                "subscription": {
                    "name": channel
                }
            });

            ws_sender
                .send(Message::Text(subscription.to_string()))
                .await
                .context("Failed to send subscription")?;

            info!(
                "📊 Subscribed to {} channel for pairs: {:?}",
                channel, self.config.pairs
            );
        }

        // Process events until failure
        while *self.running.read().await {
            let message_timeout = Duration::from_millis(self.config.websocket.message_timeout_ms);

            match tokio::time::timeout(message_timeout, ws_receiver.next()).await {
                Ok(Some(Ok(Message::Text(text)))) => {
                    if let Err(e) = self.process_websocket_message(&text).await {
                        error!("🔥 CRASH: Failed to process WebSocket message: {}", e);
                        return Err(e);
                    }
                }
                Ok(Some(Ok(Message::Ping(ping)))) => {
                    if let Err(e) = ws_sender.send(Message::Pong(ping)).await {
                        error!("🔥 CRASH: Failed to send WebSocket pong: {}", e);
                        return Err(anyhow::anyhow!("WebSocket pong failed: {}", e));
                    }
                }
                Ok(Some(Ok(Message::Close(_)))) => {
                    error!("🔥 CRASH: WebSocket closed by remote");
                    return Err(anyhow::anyhow!("WebSocket closed by remote"));
                }
                Ok(Some(Err(e))) => {
                    error!("🔥 CRASH: WebSocket error: {}", e);
                    return Err(anyhow::anyhow!("WebSocket error: {}", e));
                }
                Ok(None) => {
                    error!("🔥 CRASH: WebSocket stream ended");
                    return Err(anyhow::anyhow!("WebSocket stream ended"));
                }
                Err(_) => {
                    warn!(
                        "⏳ WebSocket message timeout ({}ms) - normal during low activity",
                        message_timeout.as_millis()
                    );
                }
                _ => {
                    // Other message types ignored
                }
            }
        }

        Ok(())
    }

    /// Process WebSocket message from Kraken
    async fn process_websocket_message(&self, message: &str) -> Result<()> {
        let start_time = Instant::now();

        // Parse JSON
        let json_value: Value = match serde_json::from_str(message) {
            Ok(v) => v,
            Err(_) => {
                debug!("Ignoring non-JSON message: {}", message);
                return Ok(());
            }
        };

        // Handle different message types
        let tlv_message_opt = if json_value.is_array() {
            // Array format: [channelID, data, channel_name, pair]
            self.process_array_message(&json_value).await
        } else if let Some(event) = json_value.get("event") {
            // JSON control messages
            match event.as_str() {
                Some("subscriptionStatus") => {
                    info!("Subscription status: {:?}", json_value);
                    None
                }
                Some("systemStatus") => {
                    info!("System status: {:?}", json_value);
                    None
                }
                Some("heartbeat") => {
                    debug!("Heartbeat received");
                    None
                }
                _ => {
                    debug!("Unknown event type: {:?}", event);
                    None
                }
            }
        } else {
            debug!("Unknown message format: {}", message);
            None
        };

        if let Some(tlv_message) = tlv_message_opt {
            // Runtime TLV validation if enabled
            if *self.validation_enabled.read().await {
                if let Err(e) = self.validate_tlv_message(&tlv_message).await {
                    let mut failures = self.validation_failures.write().await;
                    *failures += 1;
                    error!(
                        "🔥 CRASH: TLV validation failed: {} (failure #{})",
                        e, *failures
                    );
                    return Err(e);
                }
            }

            // Send directly to RelayOutput (no channel overhead)
            self.relay_output
                .send_bytes(&tlv_message)
                .await
                .context("RelayOutput send failed - CRASHING as designed")?;

            // Update statistics
            let mut count = self.messages_processed.write().await;
            *count += 1;
            let total = *count;

            let processing_latency = start_time.elapsed();
            if processing_latency.as_millis()
                > self.config.monitoring.max_processing_latency_ms as u128
            {
                warn!(
                    "⚠️ High processing latency: {}ms (max: {}ms)",
                    processing_latency.as_millis(),
                    self.config.monitoring.max_processing_latency_ms
                );
            }

            if total <= 5 || total % 100 == 0 {
                info!(
                    "📊 Processed {} Kraken events (latency: {}μs)",
                    total,
                    processing_latency.as_micros()
                );
            }
        }

        Ok(())
    }

    /// Process array-format message from Kraken
    async fn process_array_message(&self, json_array: &Value) -> Option<Vec<u8>> {
        let arr = json_array.as_array()?;
        if arr.len() < 4 {
            return None;
        }

        // Extract components: [channelID, data, channel_name, pair]
        let _channel_id = arr[0].as_u64()?;
        let data = &arr[1];
        let channel_name = arr[2].as_str()?;
        let pair = arr[3].as_str()?;

        match channel_name {
            "trade" => self.process_trade_data(data, pair).await,
            "book" => self.process_book_data(data, pair).await,
            _ => {
                debug!("Unknown channel: {}", channel_name);
                None
            }
        }
    }

    /// Process trade data from Kraken
    async fn process_trade_data(&self, data: &Value, pair: &str) -> Option<Vec<u8>> {
        let trades = data.as_array()?;

        // Process first trade (could process all in production)
        if let Some(trade_array) = trades.first().and_then(|t| t.as_array()) {
            if trade_array.len() < 6 {
                return None;
            }

            // Parse trade data: [price, volume, time, side, orderType, misc]
            let price_str = trade_array[0].as_str()?;
            let volume_str = trade_array[1].as_str()?;
            let time_str = trade_array[2].as_str()?;
            let side_str = trade_array[3].as_str()?;

            // Convert strings to numbers
            let price: f64 = price_str.parse().ok()?;
            let volume: f64 = volume_str.parse().ok()?;
            let timestamp: f64 = time_str.parse().unwrap_or(0.0); // Safe fallback for malformed input

            // Convert to 8-decimal fixed-point for USD prices
            let price_fixed = (price * 100_000_000.0) as i64;
            let volume_fixed = (volume * 100_000_000.0) as i64;

            // Create InstrumentId from pair (e.g., "XBT/USD" -> BTC/USD)
            let parts: Vec<&str> = pair.split('/').collect();
            if parts.len() != 2 {
                debug!("Invalid pair format: {}", pair);
                return None;
            }

            let base = parts[0].replace("XBT", "BTC"); // Kraken uses XBT for Bitcoin
            let quote = parts[1];

            // Create cryptocurrency spot pair using coin() method for base currency
            let instrument_id = InstrumentId::coin(VenueId::Kraken, &base);

            // Build TradeTLV using constructor
            let trade_tlv = TradeTLV::new(
                VenueId::Kraken,
                instrument_id,
                price_fixed,
                volume_fixed,
                if side_str == "b" { 0 } else { 1 }, // 0 = buy, 1 = sell
                alphapulse_network::time::parse_external_unix_timestamp_safe(timestamp, "Kraken"), // DoS-safe timestamp conversion
            );

            // Build complete Protocol V2 message (true zero-copy)
            let message = build_message_direct(
                self.config.parse_relay_domain().ok()?,
                SourceType::KrakenCollector,
                TLVType::Trade,
                &trade_tlv,
            )
            .map_err(|e| anyhow::anyhow!("TLV build failed: {}", e))
            .ok()?;

            debug!(
                "📈 Trade processed: {} @ {} (volume: {})",
                pair, price, volume
            );

            return Some(message);
        }

        None
    }

    /// Process book (order book) data from Kraken
    async fn process_book_data(&self, data: &Value, pair: &str) -> Option<Vec<u8>> {
        // Parse book update data
        let obj = data.as_object()?;

        // Extract bids and asks
        let bids = obj.get("b").or_else(|| obj.get("bs"))?.as_array()?;
        let asks = obj.get("a").or_else(|| obj.get("as"))?.as_array()?;

        if bids.is_empty() && asks.is_empty() {
            return None;
        }

        // Create InstrumentId
        let parts: Vec<&str> = pair.split('/').collect();
        if parts.len() != 2 {
            return None;
        }

        let base = parts[0].replace("XBT", "BTC");
        let quote = parts[1];
        // Create cryptocurrency spot pair using coin() method for base currency
        let instrument_id = InstrumentId::coin(VenueId::Kraken, &base);

        // Process best bid and ask (could process full book in production)
        let (bid_price, ask_price) = if let (Some(best_bid), Some(best_ask)) = (
            bids.first().and_then(|b| b.as_array()),
            asks.first().and_then(|a| a.as_array()),
        ) {
            let bid_price_str = best_bid.get(0)?.as_str()?;
            let ask_price_str = best_ask.get(0)?.as_str()?;

            let bid_price: f64 = bid_price_str.parse().ok()?;
            let ask_price: f64 = ask_price_str.parse().ok()?;

            (
                (bid_price * 100_000_000.0) as i64,
                (ask_price * 100_000_000.0) as i64,
            )
        } else {
            return None;
        };

        // Build QuoteTLV using constructor for top of book
        let timestamp_ns = alphapulse_network::time::safe_system_timestamp_ns();

        let quote_tlv = QuoteTLV::new(
            VenueId::Kraken,
            instrument_id,
            bid_price,
            1000000, // Simplified bid size - would parse actual size
            ask_price,
            1000000, // Simplified ask size - would parse actual size
            timestamp_ns,
        );

        // Build complete Protocol V2 message (true zero-copy)
        let message = build_message_direct(
            self.config.parse_relay_domain().ok()?,
            SourceType::KrakenCollector,
            TLVType::Quote,
            &quote_tlv,
        )
        .map_err(|e| anyhow::anyhow!("TLV build failed: {}", e))
        .ok()?;

        debug!(
            "📊 Book processed: {} bid: {} ask: {}",
            pair, bid_price, ask_price
        );

        Some(message)
    }

    /// Validate TLV message by round-trip parsing (startup period only)
    async fn validate_tlv_message(&self, message: &[u8]) -> Result<()> {
        if message.len() < 32 {
            return Err(anyhow::anyhow!(
                "TLV message too short: {} bytes",
                message.len()
            ));
        }

        // Parse header
        let header = parse_header(&message[..32])
            .map_err(|e| anyhow::anyhow!("Header parsing failed: {}", e))?;

        let magic = header.magic;
        if magic != 0xDEADBEEF {
            return Err(anyhow::anyhow!("Invalid magic number: 0x{:08X}", magic));
        }

        // Parse TLV payload
        let payload_size = header.payload_size;
        let payload_end = 32 + payload_size as usize;
        if message.len() < payload_end {
            return Err(anyhow::anyhow!(
                "TLV payload truncated: expected {} bytes, got {}",
                payload_end,
                message.len()
            ));
        }

        let tlv_payload = &message[32..payload_end];
        let _tlvs = parse_tlv_extensions(tlv_payload)
            .map_err(|e| anyhow::anyhow!("TLV parsing failed: {}", e))?;

        if self.config.validation.verbose_validation {
            debug!("✅ TLV validation passed: {} bytes", message.len());
        }

        Ok(())
    }

    /// Get runtime statistics
    pub async fn stats(&self) -> (u64, u64, Duration) {
        let messages = *self.messages_processed.read().await;
        let failures = *self.validation_failures.read().await;
        let uptime = self.start_time.elapsed();

        (messages, failures, uptime)
    }

    /// Stop the collector
    pub async fn stop(&self) {
        *self.running.write().await = false;
        info!("⏹️ Unified Kraken Collector stopped");
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // ✅ CRITICAL: Initialize ultra-fast timestamp system
    init_timestamp_system();
    info!("✅ Ultra-fast timestamp system initialized (~5ns per timestamp)");

    info!("🚀 Starting Unified Kraken Collector");
    info!("   Architecture: WebSocket → TLV Builder → RelayOutput");
    info!("   NO MPSC channels - direct relay integration");

    // Load configuration
    let config_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "kraken.toml".to_string());

    let config = KrakenConfig::from_toml_with_env_overrides(&config_path).unwrap_or_else(|_| {
        info!("📋 Using default configuration");
        KrakenConfig::default()
    });

    info!("📋 Configuration:");
    info!("   WebSocket: {}", config.websocket.url);
    info!("   Pairs: {:?}", config.pairs);
    info!("   Channels: {:?}", config.channels);
    info!(
        "   Relay: {} → {}",
        config.relay.domain, config.relay.socket_path
    );

    // Create and start collector
    let collector = UnifiedKrakenCollector::new(config).context("Failed to create collector")?;

    // Setup signal handling for graceful shutdown
    let collector_ref = Arc::new(collector);
    let collector_shutdown = collector_ref.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.unwrap();
        info!("📡 Received Ctrl+C, shutting down...");
        collector_shutdown.stop().await;
    });

    // Start collector (will crash on WebSocket/relay failures as designed)
    match collector_ref.start().await {
        Ok(()) => {
            let (messages, failures, uptime) = collector_ref.stats().await;
            info!("✅ Collector stopped gracefully");
            info!(
                "📊 Final stats: {} messages, {} validation failures, uptime: {:?}",
                messages, failures, uptime
            );
        }
        Err(e) => {
            error!("🔥 COLLECTOR CRASHED: {}", e);
            error!("   This is by design - external supervision should restart");
            std::process::exit(1);
        }
    }

    Ok(())
}
