use crate::unix_socket::UnixSocketWriter;
use alphapulse_protocol::*;
use alphapulse_protocol::conversion::{parse_price_to_fixed_point, parse_volume_to_fixed_point, parse_trade_side};
use alphapulse_protocol::validation::{validate_trade_data, detect_corruption_patterns};
use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio_tungstenite::{connect_async, tungstenite::Message, tungstenite::http::HeaderValue};
use tracing::{debug, error, info, warn};

const COINBASE_WS_URL: &str = "wss://ws-feed.exchange.coinbase.com";

#[derive(Debug, Serialize)]
struct CoinbaseSubscribe {
    r#type: String,
    product_ids: Vec<String>,
    channels: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct CoinbaseMessage {
    r#type: String,
    #[serde(default)]
    product_id: Option<String>,
    #[serde(default)]
    price: Option<String>,
    #[serde(default)]
    size: Option<String>,
    #[serde(default)]
    side: Option<String>,
    #[serde(default)]
    time: Option<String>,
    #[serde(default)]
    trade_id: Option<u64>,
    #[serde(default)]
    changes: Option<Vec<Vec<String>>>, // [side, price, size] for level2 updates
    #[serde(default)]
    bids: Option<Vec<Vec<String>>>, // [[price, size], ...] for snapshots
    #[serde(default)]
    asks: Option<Vec<Vec<String>>>, // [[price, size], ...] for snapshots
}

pub struct CoinbaseCollector {
    socket_writer: Arc<UnixSocketWriter>,
    symbol_cache: Arc<RwLock<std::collections::HashMap<String, u64>>>, // product_id -> hash
}

impl CoinbaseCollector {
    pub fn new(
        socket_writer: Arc<UnixSocketWriter>,
        _symbol_mapper: Arc<RwLock<std::collections::HashMap<String, u32>>>, // Keep signature for now
    ) -> Self {
        Self {
            socket_writer,
            symbol_cache: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    pub async fn connect_and_stream(&self) -> Result<()> {
        info!("Connecting to Coinbase WebSocket at {}", COINBASE_WS_URL);

        let (ws_stream, _) = connect_async(COINBASE_WS_URL).await
            .map_err(|e| anyhow::anyhow!("Coinbase WebSocket connection failed: {}", e))?;

        info!("Connected to Coinbase WebSocket");

        let (mut write, mut read) = ws_stream.split();

        // Subscribe to trades and level2 order book updates
        let subscribe_msg = CoinbaseSubscribe {
            r#type: "subscribe".to_string(),
            product_ids: vec!["BTC-USD".to_string(), "ETH-USD".to_string()],
            channels: vec!["matches".to_string(), "level2_batch".to_string()],
        };

        let msg = serde_json::to_string(&subscribe_msg)?;
        write.send(Message::Text(msg)).await?;
        info!("Subscribed to Coinbase trade feed");

        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    self.handle_message(&text).await;
                }
                Ok(Message::Close(_)) => {
                    info!("Coinbase WebSocket closed");
                    break;
                }
                Err(e) => {
                    error!("WebSocket error: {}", e);
                    break;
                }
                _ => {}
            }
        }

        Ok(())
    }

    async fn handle_message(&self, text: &str) {
        match serde_json::from_str::<CoinbaseMessage>(text) {
            Ok(msg) => {
                match msg.r#type.as_str() {
                    "match" => self.handle_trade(msg).await,
                    "l2update" => self.handle_l2update(msg).await,
                    "snapshot" => self.handle_snapshot(msg).await,
                    "subscriptions" => info!("Coinbase subscription confirmed"),
                    _ => debug!("Unhandled Coinbase message type: {}", msg.r#type),
                }
            }
            Err(e) => {
                error!("Failed to parse Coinbase message: {} - {}", e, text);
            }
        }
    }

    async fn handle_trade(&self, trade: CoinbaseMessage) {
        if let (Some(product_id), Some(price_str), Some(size_str), Some(side)) = 
            (trade.product_id, trade.price, trade.size, trade.side) {
            
            // Use our precision-preserving conversion module
            match (
                parse_price_to_fixed_point(&price_str),
                parse_volume_to_fixed_point(&size_str),
                parse_trade_side(&side)
            ) {
                (Ok(price_fixed), Ok(volume_fixed), Ok(trade_side)) => {
                    let timestamp_ns = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_nanos() as u64;

                    // Get or create symbol hash
                    let symbol_hash = self.get_or_create_symbol_hash(&product_id);
                    
                    // Validate the trade data before processing
                    if let Err(validation_error) = validate_trade_data(
                        &product_id, 
                        price_fixed, 
                        volume_fixed, 
                        timestamp_ns, 
                        "coinbase"
                    ) {
                        error!("Trade validation failed for {}: {}", product_id, validation_error);
                        return;
                    }
                    
                    // Check for potential data corruption
                    let warnings = detect_corruption_patterns(&product_id, price_fixed, volume_fixed);
                    if !warnings.is_empty() {
                        warn!("Data corruption warnings for {}: {:?}", product_id, warnings);
                    }

                    let trade_message = TradeMessage::new(
                        timestamp_ns,
                        price_fixed as u64,
                        volume_fixed as u64,
                        symbol_hash,
                        trade_side,
                    );

                    if let Err(e) = self.socket_writer.write_trade(&trade_message) {
                        error!("Failed to send trade: {}", e);
                    } else {
                        // Use conversion module for display to show exact precision
                        let display_price = alphapulse_protocol::conversion::fixed_point_to_f64(price_fixed);
                        let display_volume = alphapulse_protocol::conversion::fixed_point_to_f64(volume_fixed);
                        info!("Sent {} trade: ${:.8} ({:.8} {:?})", product_id, display_price, display_volume, trade_side);
                    }
                }
                _ => {
                    error!("Failed to parse trade data for {}: price='{}', volume='{}', side='{}'", 
                           product_id, price_str, size_str, side);
                }
            }
        }
    }

    async fn handle_l2update(&self, update: CoinbaseMessage) {
        use alphapulse_protocol::{L2DeltaMessage, L2Update, L2Action};
        
        if let (Some(product_id), Some(changes)) = (update.product_id, update.changes) {
            let symbol_hash = self.get_or_create_symbol_hash(&product_id);

            let timestamp_ns = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64;

            let mut updates = Vec::new();
            
            for change in changes {
                if change.len() >= 3 {
                    let side_str = &change[0];
                    let price_str = &change[1];
                    let size_str = &change[2];
                    
                    if let (Ok(price), Ok(size)) = (price_str.parse::<f64>(), size_str.parse::<f64>()) {
                        let side = match side_str.as_str() {
                            "buy" => 0u8,  // bid
                            "sell" => 1u8, // ask
                            _ => continue,
                        };
                        
                        let action = if size == 0.0 {
                            L2Action::Delete
                        } else {
                            L2Action::Update // Coinbase doesn't distinguish insert vs update
                        };
                        
                        let price_fixed = (price * 1e8) as u64;
                        let volume_fixed = (size * 1e8) as u64;
                        
                        updates.push(L2Update::new(side, price_fixed, volume_fixed, action));
                    }
                }
            }
            
            if !updates.is_empty() {
                let delta_msg = L2DeltaMessage {
                    timestamp_ns,
                    symbol_hash,
                    sequence: 0, // TODO: Implement proper sequence tracking
                    updates,
                };
                
                if let Err(e) = self.socket_writer.write_l2_delta(&delta_msg) {
                    error!("Failed to send L2 delta: {}", e);
                } else {
                    debug!("Sent L2 delta for {} with {} updates", product_id, delta_msg.updates.len());
                }
            }
        }
    }

    async fn handle_snapshot(&self, snapshot: CoinbaseMessage) {
        use alphapulse_protocol::{L2SnapshotMessage, PriceLevel};
        
        if let (Some(product_id), Some(bids_data), Some(asks_data)) = 
            (snapshot.product_id, snapshot.bids, snapshot.asks) {
            
            let symbol_hash = self.get_or_create_symbol_hash(&product_id);
            
            let timestamp_ns = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64;
            
            let mut bids = Vec::new();
            let mut asks = Vec::new();
            
            // IMPORTANT: Limit depth to avoid exceeding 64KB protocol limit
            // With 200 levels per side, message size is approximately:
            // Header (28) + 200*16 + 200*16 = 6,428 bytes << 65,535 limit
            const MAX_DEPTH: usize = 200;
            
            // Parse bids - Coinbase sends [[price, size], ...] sorted best to worst
            for (i, bid) in bids_data.iter().take(MAX_DEPTH).enumerate() {
                if bid.len() >= 2 {
                    if let (Ok(price), Ok(size)) = 
                        (bid[0].parse::<f64>(), bid[1].parse::<f64>()) {
                        bids.push(PriceLevel::new(
                            (price * 1e8) as u64,
                            (size * 1e8) as u64
                        ));
                    }
                }
            }
            
            // Parse asks - Coinbase sends [[price, size], ...] sorted best to worst
            for (i, ask) in asks_data.iter().take(MAX_DEPTH).enumerate() {
                if ask.len() >= 2 {
                    if let (Ok(price), Ok(size)) = 
                        (ask[0].parse::<f64>(), ask[1].parse::<f64>()) {
                        asks.push(PriceLevel::new(
                            (price * 1e8) as u64,
                            (size * 1e8) as u64
                        ));
                    }
                }
            }
            
            // Only send if we have data
            if !bids.is_empty() || !asks.is_empty() {
                let snapshot_msg = L2SnapshotMessage {
                    timestamp_ns,
                    symbol_hash,
                    sequence: 0, // Coinbase doesn't provide sequence in snapshot
                    bids,
                    asks,
                };
                
                if let Err(e) = self.socket_writer.write_l2_snapshot(&snapshot_msg) {
                    error!("Failed to send L2 snapshot: {}", e);
                } else {
                    info!("Sent L2 snapshot for {} with {} bids, {} asks", 
                        product_id, snapshot_msg.bids.len(), snapshot_msg.asks.len());
                }
            }
        }
    }

    fn get_or_create_symbol_hash(&self, product_id: &str) -> u64 {
        let mut cache = self.symbol_cache.write();
        
        if let Some(&hash) = cache.get(product_id) {
            return hash;
        }
        
        // Parse Coinbase format: BTC-USD, ETH-USD, etc.
        let parts: Vec<&str> = product_id.split('-').collect();
        let descriptor = if parts.len() == 2 {
            SymbolDescriptor::spot("coinbase", parts[0], parts[1])
        } else {
            // Fallback for unknown formats
            SymbolDescriptor::spot("coinbase", product_id, "USD")
        };
        
        let hash = descriptor.hash();
        cache.insert(product_id.to_string(), hash);
        
        // Send symbol mapping message
        let mapping = SymbolMappingMessage::new(&descriptor);
        if let Err(e) = self.socket_writer.write_symbol_mapping(&mapping) {
            error!("Failed to send symbol mapping: {}", e);
        } else {
            info!("Sent symbol mapping: {} -> {}", product_id, hash);
        }
        
        hash
    }
}