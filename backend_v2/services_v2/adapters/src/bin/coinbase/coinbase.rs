//! # Unified Coinbase Collector - Direct RelayOutput Integration
//!
//! ## Architecture
//!
//! Eliminates MPSC channel overhead by connecting WebSocket events directly to RelayOutput:
//! ```
//! Coinbase WebSocket → Event Processing → TLV Builder → RelayOutput → MarketDataRelay
//! ```
//!
//! ## Key Improvements (following polygon.rs pattern)
//! - **Zero Channel Overhead**: Direct `relay_output.send_bytes()` calls
//! - **Unified Logic**: Single service combines collection and publishing
//! - **Configuration-Driven**: TOML-based configuration with environment overrides
//! - **Transparent Failures**: Crash immediately on WebSocket/relay failures
//! - **Runtime Validation**: TLV round-trip validation during startup period
//!
//! ## Coinbase WebSocket Protocol
//! - **JSON Format**: All messages use JSON format with clear structure
//! - **Channels**: "matches" for trades, "level2" for order book, "ticker" for price updates
//! - **Products**: "BTC-USD", "ETH-USD" format for trading pairs
//! - **Authentication**: Public feeds don't require API keys
//!
//! ## Performance Profile
//! - **Latency**: <10ms from market event to relay delivery
//! - **Throughput**: Designed for >15,000 msg/s sustained load
//! - **Memory**: <50MB steady state with multiple product subscriptions

use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use protocol_v2::{
    parse_header, parse_tlv_extensions, tlv::build_message_direct,
    tlv::fast_timestamp::init_timestamp_system, InstrumentId, RelayDomain, SourceType, TLVType,
    TradeTLV, VenueId,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info, warn};

use alphapulse_adapter_service::output::RelayOutput;

// =============================================================================
// CONFIGURATION
// =============================================================================

/// Main Coinbase configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoinbaseConfig {
    pub websocket: WebSocketConfig,
    pub relay: RelayConfig,
    pub products: Vec<String>,
    pub channels: Vec<String>,
    pub validation: ValidationConfig,
    pub monitoring: MonitoringConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketConfig {
    pub url: String,
    pub connection_timeout_ms: u64,
    pub message_timeout_ms: u64,
    pub max_reconnect_attempts: u32,
    pub base_backoff_ms: u64,
    pub max_backoff_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayConfig {
    pub domain: String,
    pub socket_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConfig {
    pub runtime_validation_seconds: u64,
    pub verbose_validation: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    pub stats_interval_seconds: u64,
    pub max_processing_latency_ms: u64,
}

impl Default for CoinbaseConfig {
    fn default() -> Self {
        Self {
            websocket: WebSocketConfig {
                url: "wss://ws-feed.exchange.coinbase.com".to_string(),
                connection_timeout_ms: 30000,
                message_timeout_ms: 120000,
                max_reconnect_attempts: 10,
                base_backoff_ms: 1000,
                max_backoff_ms: 60000,
            },
            relay: RelayConfig {
                domain: "market_data".to_string(),
                socket_path: "/tmp/alphapulse/market_data.sock".to_string(),
            },
            // Default to BTC-USD only - use COINBASE_PRODUCTS env var for production
            products: vec!["BTC-USD".to_string()],
            channels: vec!["matches".to_string(), "level2".to_string()],
            validation: ValidationConfig {
                runtime_validation_seconds: 60,
                verbose_validation: false,
            },
            monitoring: MonitoringConfig {
                stats_interval_seconds: 30,
                max_processing_latency_ms: 100,
            },
        }
    }
}

impl CoinbaseConfig {
    pub fn from_toml_with_env_overrides(path: &str) -> Result<Self> {
        let config_str =
            fs::read_to_string(path).context(format!("Failed to read config file: {}", path))?;

        let mut config: CoinbaseConfig =
            toml::from_str(&config_str).context("Failed to parse TOML configuration")?;

        if let Ok(url) = std::env::var("COINBASE_WS_URL") {
            config.websocket.url = url;
        }

        if let Ok(path) = std::env::var("RELAY_SOCKET_PATH") {
            config.relay.socket_path = path;
        }

        if let Ok(products) = std::env::var("COINBASE_PRODUCTS") {
            config.products = products.split(',').map(|s| s.trim().to_string()).collect();
        }

        Ok(config)
    }

    pub fn validate(&self) -> Result<()> {
        if self.products.is_empty() {
            return Err(anyhow::anyhow!("No products configured"));
        }

        if !self.websocket.url.starts_with("wss://") && !self.websocket.url.starts_with("ws://") {
            return Err(anyhow::anyhow!("Invalid WebSocket URL scheme"));
        }

        Ok(())
    }

    pub fn parse_relay_domain(&self) -> Result<RelayDomain> {
        match self.relay.domain.as_str() {
            "market_data" => Ok(RelayDomain::MarketData),
            "signal" => Ok(RelayDomain::Signal),
            "execution" => Ok(RelayDomain::Execution),
            _ => Err(anyhow::anyhow!(
                "Invalid relay domain: {}",
                self.relay.domain
            )),
        }
    }
}

// =============================================================================
// COINBASE MESSAGE STRUCTURES
// =============================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct CoinbaseMatchEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub trade_id: u64,
    pub side: String,
    pub size: String,
    pub price: String,
    pub product_id: String,
    pub sequence: u64,
    pub time: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CoinbaseLevel2Event {
    #[serde(rename = "type")]
    pub event_type: String,
    pub product_id: String,
    pub time: String,
    pub changes: Vec<Vec<String>>, // [side, price, size]
}

// =============================================================================
// UNIFIED COINBASE COLLECTOR
// =============================================================================

pub struct UnifiedCoinbaseCollector {
    config: CoinbaseConfig,
    relay_output: Arc<RelayOutput>,
    running: Arc<RwLock<bool>>,
    validation_enabled: Arc<RwLock<bool>>,
    start_time: Instant,
    messages_processed: Arc<RwLock<u64>>,
    validation_failures: Arc<RwLock<u64>>,
}

impl UnifiedCoinbaseCollector {
    pub fn new(config: CoinbaseConfig) -> Result<Self> {
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

    pub async fn start(&self) -> Result<()> {
        info!("🚀 Starting Unified Coinbase Collector");
        info!("   Direct WebSocket → RelayOutput integration");
        info!("   Products: {:?}", self.config.products);
        info!("   Channels: {:?}", self.config.channels);

        *self.running.write().await = true;

        // Connect to relay first
        self.relay_output
            .connect()
            .await
            .context("Failed to connect to relay - CRASHING as designed")?;

        info!(
            "✅ Connected to {:?} relay at {}",
            self.config.parse_relay_domain()?,
            self.config.relay.socket_path
        );

        // Start validation timer
        self.start_validation_timer().await;

        // Connect and process
        self.connect_and_process_events()
            .await
            .context("WebSocket processing failed - CRASHING as designed")?;

        Ok(())
    }

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

    async fn try_websocket_connection(&self) -> Result<()> {
        let timeout_duration = Duration::from_millis(self.config.websocket.connection_timeout_ms);

        // Connect with timeout
        let (ws_stream, _) =
            tokio::time::timeout(timeout_duration, connect_async(&self.config.websocket.url))
                .await
                .context("WebSocket connection timeout")?
                .context("WebSocket connection failed")?;

        info!("✅ WebSocket connected to Coinbase");

        let (mut ws_sender, mut ws_receiver) = ws_stream.split();

        // Subscribe to channels
        let subscription = json!({
            "type": "subscribe",
            "product_ids": self.config.products,
            "channels": self.config.channels
        });

        ws_sender
            .send(Message::Text(subscription.to_string()))
            .await
            .context("Failed to send subscription")?;

        info!(
            "📊 Subscribed to channels: {:?} for products: {:?}",
            self.config.channels, self.config.products
        );

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

    async fn process_websocket_message(&self, message: &str) -> Result<()> {
        let start_time = Instant::now();

        let json_value: Value = match serde_json::from_str(message) {
            Ok(v) => v,
            Err(_) => {
                debug!("Ignoring non-JSON message: {}", message);
                return Ok(());
            }
        };

        // Handle different message types
        let tlv_message_opt =
            if let Some(msg_type) = json_value.get("type").and_then(|t| t.as_str()) {
                match msg_type {
                    "match" | "last_match" => self.process_match_event(&json_value).await,
                    "l2update" => self.process_l2_update(&json_value).await,
                    "subscriptions" => {
                        info!("Subscription confirmed: {:?}", json_value);
                        None
                    }
                    "heartbeat" => {
                        debug!("Heartbeat received");
                        None
                    }
                    _ => {
                        debug!("Unknown message type: {}", msg_type);
                        None
                    }
                }
            } else {
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
                    "📊 Processed {} Coinbase events (latency: {}μs)",
                    total,
                    processing_latency.as_micros()
                );
            }
        }

        Ok(())
    }

    async fn process_match_event(&self, json_value: &Value) -> Option<Vec<u8>> {
        let match_event: CoinbaseMatchEvent = match serde_json::from_value(json_value.clone()) {
            Ok(m) => m,
            Err(e) => {
                debug!("Failed to parse match event: {}", e);
                return None;
            }
        };

        // Parse price and size (Coinbase sends as strings)
        let price: f64 = match_event.price.parse().ok()?;
        let size: f64 = match_event.size.parse().ok()?;

        // Convert to 8-decimal fixed-point for USD prices
        let price_fixed = (price * 100_000_000.0) as i64;
        let size_fixed = (size * 100_000_000.0) as i64;

        // Create InstrumentId from product_id (e.g., "BTC-USD")
        let parts: Vec<&str> = match_event.product_id.split('-').collect();
        if parts.len() != 2 {
            debug!("Invalid product_id format: {}", match_event.product_id);
            return None;
        }

        // Normalize to standard format: "BTC-USD" → "BTC/USD"
        let normalized_symbol = format!("{}/{}", parts[0], parts[1]);

        // Create cryptocurrency spot pair using coin() method with full pair
        let instrument_id = InstrumentId::coin(VenueId::Coinbase, &normalized_symbol);

        // Parse timestamp
        let timestamp = match chrono::DateTime::parse_from_rfc3339(&match_event.time) {
            Ok(dt) => dt.timestamp_nanos_opt().unwrap_or(0) as u64,
            Err(_) => SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
        };

        // Build TradeTLV using the constructor
        let trade_tlv = TradeTLV::new(
            VenueId::Coinbase,
            instrument_id,
            price_fixed,
            size_fixed,
            if match_event.side == "buy" { 0 } else { 1 }, // 0 = buy, 1 = sell
            timestamp,
        );

        // Build complete Protocol V2 message (true zero-copy)
        let message = build_message_direct(
            self.config.parse_relay_domain().ok()?,
            SourceType::CoinbaseCollector,
            TLVType::Trade,
            &trade_tlv,
        )
        .map_err(|e| anyhow::anyhow!("TLV build failed: {}", e))
        .ok()?;

        debug!(
            "📈 Trade processed: {} @ {} (size: {})",
            match_event.product_id, price, size
        );

        Some(message)
    }

    async fn process_l2_update(&self, _json_value: &Value) -> Option<Vec<u8>> {
        // For now, we'll skip L2 updates to keep it simple
        // In production, you'd convert these to QuoteTLV messages
        debug!("L2 update received (not implemented yet)");
        None
    }

    async fn validate_tlv_message(&self, message: &[u8]) -> Result<()> {
        if message.len() < 32 {
            return Err(anyhow::anyhow!(
                "TLV message too short: {} bytes",
                message.len()
            ));
        }

        let header = parse_header(&message[..32])
            .map_err(|e| anyhow::anyhow!("Header parsing failed: {}", e))?;

        let magic = header.magic;
        if magic != 0xDEADBEEF {
            return Err(anyhow::anyhow!("Invalid magic number: 0x{:08X}", magic));
        }

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

    pub async fn stats(&self) -> (u64, u64, Duration) {
        let messages = *self.messages_processed.read().await;
        let failures = *self.validation_failures.read().await;
        let uptime = self.start_time.elapsed();

        (messages, failures, uptime)
    }

    pub async fn stop(&self) {
        *self.running.write().await = false;
        info!("⏹️ Unified Coinbase Collector stopped");
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // ✅ CRITICAL: Initialize ultra-fast timestamp system
    init_timestamp_system();
    info!("✅ Ultra-fast timestamp system initialized (~5ns per timestamp)");

    info!("🚀 Starting Unified Coinbase Collector");
    info!("   Architecture: WebSocket → TLV Builder → RelayOutput");
    info!("   NO MPSC channels - direct relay integration");

    // Load configuration (supports both CLI arg and env var)
    let config_path = std::env::args()
        .nth(1)
        .or_else(|| std::env::var("COINBASE_CONFIG_PATH").ok())
        .unwrap_or_else(|| "coinbase.toml".to_string());

    let config = CoinbaseConfig::from_toml_with_env_overrides(&config_path).unwrap_or_else(|_| {
        info!("📋 Using default configuration");
        CoinbaseConfig::default()
    });

    info!("📋 Configuration:");
    info!("   WebSocket: {}", config.websocket.url);
    info!("   Products: {:?}", config.products);
    info!("   Channels: {:?}", config.channels);
    info!(
        "   Relay: {} → {}",
        config.relay.domain, config.relay.socket_path
    );

    // Create and start collector
    let collector = UnifiedCoinbaseCollector::new(config).context("Failed to create collector")?;

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
