use alphapulse_protocol::*;
use alphapulse_protocol::{MARKET_DATA_RELAY_PATH, SIGNAL_RELAY_PATH};
use zerocopy::FromBytes;
use anyhow::{Context, Result};
use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};
use metrics::{counter, gauge, histogram};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize, Serializer};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::os::unix::net::UnixStream;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, mpsc};
use tokio::task;
use tokio_tungstenite::{accept_async, tungstenite::Message};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

const WS_BIND_ADDR: &str = "127.0.0.1:8765";
const BROADCAST_BUFFER_SIZE: usize = 10000;

// Helper function to serialize u64 as string for JavaScript compatibility
fn serialize_u64_as_string<S>(value: &u64, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&value.to_string())
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "msg_type")]
enum BroadcastMessage {
    #[serde(rename = "trade")]
    Trade {
        #[serde(serialize_with = "serialize_u64_as_string")]
        symbol_hash: u64,
        symbol: Option<String>,
        timestamp: u64,
        price: f64,
        volume: f64,
        side: String,
        // Latency tracking fields (all in microseconds)
        latency_collector_to_relay_us: Option<u64>,
        latency_relay_to_bridge_us: Option<u64>, 
        latency_bridge_to_frontend_us: Option<u64>,
        latency_total_us: Option<u64>,
    },
    #[serde(rename = "orderbook")]
    OrderBook {
        #[serde(serialize_with = "serialize_u64_as_string")]
        symbol_hash: u64,
        symbol: Option<String>,
        timestamp: u64,
        data: OrderBookData,
    },
    #[serde(rename = "l2_snapshot")]
    L2Snapshot {
        #[serde(serialize_with = "serialize_u64_as_string")]
        symbol_hash: u64,
        symbol: Option<String>,
        timestamp: u64,
        sequence: u64,
        bids: Vec<OrderBookLevel>,
        asks: Vec<OrderBookLevel>,
    },
    #[serde(rename = "l2_delta")]
    L2Delta {
        #[serde(serialize_with = "serialize_u64_as_string")]
        symbol_hash: u64,
        symbol: Option<String>,
        timestamp: u64,
        sequence: u64,
        updates: Vec<L2UpdateJson>,
    },
    #[serde(rename = "symbol_mapping")]
    SymbolMapping {
        #[serde(serialize_with = "serialize_u64_as_string")]
        symbol_hash: u64,
        symbol: String,
    },
    #[serde(rename = "arbitrage_opportunity")]
    ArbitrageOpportunity {
        pair: String,
        token_a: String,  // Contract address
        token_b: String,  // Contract address
        dex_buy: String,  // DEX name for buying (cheaper)
        dex_sell: String, // DEX name for selling (expensive)
        dex_buy_router: String,  // Router contract address
        dex_sell_router: String, // Router contract address
        price_buy: f64,
        price_sell: f64,
        estimated_profit: f64,
        profit_percent: f64,
        liquidity_buy: f64,
        liquidity_sell: f64,
        max_trade_size: f64,
        gas_estimate: u64,
        detected_at: u64,
    },
}

#[derive(Debug, Clone, Serialize)]
struct OrderBookLevel {
    price: f64,
    size: f64,
}

#[derive(Debug, Clone, Serialize)]
struct L2UpdateJson {
    side: String,  // "bid" or "ask"
    price: f64,
    size: f64,
    action: String,  // "delete", "update", "insert"
}

#[derive(Debug, Clone, Serialize)]
struct OrderBookData {
    bids: Vec<OrderBookLevel>,
    asks: Vec<OrderBookLevel>,
    timestamp: u64,
}

#[derive(Debug, Deserialize)]
struct ClientMessage {
    msg_type: String,
    #[serde(default)]
    channels: Vec<String>,
    #[serde(default)]
    symbols: Vec<String>,
}

struct BridgeServer {
    broadcast_tx: broadcast::Sender<BroadcastMessage>,
    symbol_cache: Arc<RwLock<std::collections::HashMap<u64, String>>>, // hash -> human-readable
    // Removed l2_snapshots cache - we'll request fresh snapshots on client connect
}

impl BridgeServer {
    fn new() -> Self {
        let (broadcast_tx, _) = broadcast::channel(BROADCAST_BUFFER_SIZE);
        
        // Initialize with empty symbol map - will be populated dynamically by collectors
        let mut symbol_map = HashMap::new();
        
        // Coinbase symbols
        let btc_usd = SymbolDescriptor::spot("coinbase", "BTC", "USD");
        let eth_usd = SymbolDescriptor::spot("coinbase", "ETH", "USD");
        symbol_map.insert(btc_usd.hash(), "coinbase:BTC-USD".to_string());
        symbol_map.insert(eth_usd.hash(), "coinbase:ETH-USD".to_string());
        
        // Polygon DEX symbols - QuickSwap (updated to POL)
        let quickswap_pol_usdc = SymbolDescriptor::spot("quickswap", "POL", "USDC");
        let quickswap_weth_usdc = SymbolDescriptor::spot("quickswap", "WETH", "USDC");
        let quickswap_wbtc_usdc = SymbolDescriptor::spot("quickswap", "WBTC", "USDC");
        let quickswap_dai_usdc = SymbolDescriptor::spot("quickswap", "DAI", "USDC");
        let quickswap_link_usdc = SymbolDescriptor::spot("quickswap", "LINK", "USDC");
        let quickswap_aave_usdc = SymbolDescriptor::spot("quickswap", "AAVE", "USDC");
        let quickswap_usdc_usdt = SymbolDescriptor::spot("quickswap", "USDC", "USDT");
        let quickswap_dai_usdt = SymbolDescriptor::spot("quickswap", "DAI", "USDT");
        let quickswap_pol_weth = SymbolDescriptor::spot("quickswap", "POL", "WETH");
        let quickswap_weth_wbtc = SymbolDescriptor::spot("quickswap", "WETH", "WBTC");
        let quickswap_pol_dai = SymbolDescriptor::spot("quickswap", "POL", "DAI");
        let quickswap_link_weth = SymbolDescriptor::spot("quickswap", "LINK", "WETH");
        let quickswap_aave_weth = SymbolDescriptor::spot("quickswap", "AAVE", "WETH");
        let quickswap_weth_usdt = SymbolDescriptor::spot("quickswap", "WETH", "USDT");
        let quickswap_pol_usdt = SymbolDescriptor::spot("quickswap", "POL", "USDT");
        let quickswap_wbtc_usdt = SymbolDescriptor::spot("quickswap", "WBTC", "USDT");
        let quickswap_link_usdt = SymbolDescriptor::spot("quickswap", "LINK", "USDT");
        let quickswap_aave_usdt = SymbolDescriptor::spot("quickswap", "AAVE", "USDT");
        
        symbol_map.insert(quickswap_pol_usdc.hash(), "quickswap:POL-USDC".to_string());
        symbol_map.insert(quickswap_weth_usdc.hash(), "quickswap:WETH-USDC".to_string());
        symbol_map.insert(quickswap_wbtc_usdc.hash(), "quickswap:WBTC-USDC".to_string());
        symbol_map.insert(quickswap_dai_usdc.hash(), "quickswap:DAI-USDC".to_string());
        symbol_map.insert(quickswap_link_usdc.hash(), "quickswap:LINK-USDC".to_string());
        symbol_map.insert(quickswap_aave_usdc.hash(), "quickswap:AAVE-USDC".to_string());
        symbol_map.insert(quickswap_usdc_usdt.hash(), "quickswap:USDC-USDT".to_string());
        symbol_map.insert(quickswap_dai_usdt.hash(), "quickswap:DAI-USDT".to_string());
        symbol_map.insert(quickswap_pol_weth.hash(), "quickswap:POL-WETH".to_string());
        symbol_map.insert(quickswap_weth_wbtc.hash(), "quickswap:WETH-WBTC".to_string());
        symbol_map.insert(quickswap_pol_dai.hash(), "quickswap:POL-DAI".to_string());
        symbol_map.insert(quickswap_link_weth.hash(), "quickswap:LINK-WETH".to_string());
        symbol_map.insert(quickswap_aave_weth.hash(), "quickswap:AAVE-WETH".to_string());
        symbol_map.insert(quickswap_weth_usdt.hash(), "quickswap:WETH-USDT".to_string());
        symbol_map.insert(quickswap_pol_usdt.hash(), "quickswap:POL-USDT".to_string());
        symbol_map.insert(quickswap_wbtc_usdt.hash(), "quickswap:WBTC-USDT".to_string());
        symbol_map.insert(quickswap_link_usdt.hash(), "quickswap:LINK-USDT".to_string());
        symbol_map.insert(quickswap_aave_usdt.hash(), "quickswap:AAVE-USDT".to_string());
        
        // Polygon DEX symbols - SushiSwap (updated to POL)
        let sushiswap_pol_usdc = SymbolDescriptor::spot("sushiswap", "POL", "USDC");
        let sushiswap_weth_usdc = SymbolDescriptor::spot("sushiswap", "WETH", "USDC");
        let sushiswap_wbtc_usdc = SymbolDescriptor::spot("sushiswap", "WBTC", "USDC");
        let sushiswap_dai_usdc = SymbolDescriptor::spot("sushiswap", "DAI", "USDC");
        let sushiswap_usdc_usdt = SymbolDescriptor::spot("sushiswap", "USDC", "USDT");
        let sushiswap_dai_usdt = SymbolDescriptor::spot("sushiswap", "DAI", "USDT");
        let sushiswap_pol_weth = SymbolDescriptor::spot("sushiswap", "POL", "WETH");
        let sushiswap_weth_wbtc = SymbolDescriptor::spot("sushiswap", "WETH", "WBTC");
        let sushiswap_pol_dai = SymbolDescriptor::spot("sushiswap", "POL", "DAI");
        let sushiswap_weth_usdt = SymbolDescriptor::spot("sushiswap", "WETH", "USDT");
        let sushiswap_pol_usdt = SymbolDescriptor::spot("sushiswap", "POL", "USDT");
        let sushiswap_wbtc_usdt = SymbolDescriptor::spot("sushiswap", "WBTC", "USDT");
        
        symbol_map.insert(sushiswap_pol_usdc.hash(), "sushiswap:POL-USDC".to_string());
        symbol_map.insert(sushiswap_weth_usdc.hash(), "sushiswap:WETH-USDC".to_string());
        symbol_map.insert(sushiswap_wbtc_usdc.hash(), "sushiswap:WBTC-USDC".to_string());
        symbol_map.insert(sushiswap_dai_usdc.hash(), "sushiswap:DAI-USDC".to_string());
        symbol_map.insert(sushiswap_usdc_usdt.hash(), "sushiswap:USDC-USDT".to_string());
        symbol_map.insert(sushiswap_dai_usdt.hash(), "sushiswap:DAI-USDT".to_string());
        symbol_map.insert(sushiswap_pol_weth.hash(), "sushiswap:POL-WETH".to_string());
        symbol_map.insert(sushiswap_weth_wbtc.hash(), "sushiswap:WETH-WBTC".to_string());
        symbol_map.insert(sushiswap_pol_dai.hash(), "sushiswap:POL-DAI".to_string());
        symbol_map.insert(sushiswap_weth_usdt.hash(), "sushiswap:WETH-USDT".to_string());
        symbol_map.insert(sushiswap_pol_usdt.hash(), "sushiswap:POL-USDT".to_string());
        symbol_map.insert(sushiswap_wbtc_usdt.hash(), "sushiswap:WBTC-USDT".to_string());
        
        info!("Initialized symbol cache with {} known symbols", symbol_map.len());
        info!("  BTC-USD hash: {}", btc_usd.hash());
        info!("  ETH-USD hash: {}", eth_usd.hash());
        
        Self {
            broadcast_tx,
            symbol_cache: Arc::new(RwLock::new(symbol_map)),
        }
    }

    async fn start(&self) -> Result<()> {
        let unix_reader_handle = self.spawn_unix_reader();
        let ws_server_handle = self.spawn_ws_server().await?;
        
        tokio::select! {
            result = unix_reader_handle => {
                error!("Unix reader exited: {:?}", result);
            }
            result = ws_server_handle => {
                error!("WebSocket server exited: {:?}", result);
            }
        }
        
        Ok(())
    }

    fn spawn_unix_reader(&self) -> task::JoinHandle<()> {
        let broadcast_tx = self.broadcast_tx.clone();
        let symbol_cache = self.symbol_cache.clone();
        
        task::spawn_blocking(move || {
            let mut retry_count = 0u32;
            let max_retry_delay = 30; // Maximum retry delay in seconds
            
            loop {
                // Connect to MarketDataRelay
                match UnixStream::connect(MARKET_DATA_RELAY_PATH) {
                    Ok(mut stream) => {
                        info!("Connected to MarketDataRelay at {}", MARKET_DATA_RELAY_PATH);
                        retry_count = 0; // Reset retry count on successful connection
                        
                        let mut buffer = vec![0u8; 65536];
                        let mut pending_data = Vec::new();
                        let mut total_bytes_read = 0u64;
                        let mut messages_processed = 0u64;
                        
                        loop {
                            use std::io::Read;
                            
                            match stream.read(&mut buffer) {
                                Ok(0) => {
                                    warn!("Unix socket closed");
                                    break;
                                }
                                Ok(n) => {
                                    total_bytes_read += n as u64;
                                    pending_data.extend_from_slice(&buffer[..n]);
                                    debug!("Read {} bytes from relay, total {}, buffer size {}", 
                                        n, total_bytes_read, pending_data.len());
                                    
                                    while pending_data.len() >= MessageHeader::SIZE {
                                        let header = match MessageHeader::read_from_prefix(
                                            &pending_data[..MessageHeader::SIZE]
                                        ) {
                                            Some(h) => h,
                                            None => break,
                                        };
                                        
                                        if let Err(e) = header.validate() {
                                            error!("Invalid header: {}", e);
                                            pending_data.clear();
                                            break;
                                        }
                                        
                                        let total_size = MessageHeader::SIZE + header.get_length() as usize;
                                        
                                        if pending_data.len() < total_size {
                                            break;
                                        }
                                        
                                        let msg_data = &pending_data[MessageHeader::SIZE..total_size];
                                        
                                        match header.get_type() {
                                            Ok(MessageType::Trade) => {
                                                if let Some(trade) = TradeMessage::read_from_prefix(msg_data) {
                                                    Self::handle_trade(
                                                        &trade,
                                                        &broadcast_tx,
                                                        &symbol_cache,
                                                    );
                                                }
                                            }
                                            Ok(MessageType::OrderBook) => {
                                                if let Ok(orderbook) = OrderBookMessage::decode(msg_data) {
                                                    Self::handle_orderbook(
                                                        &orderbook,
                                                        &broadcast_tx,
                                                        &symbol_cache,
                                                    );
                                                }
                                            }
                                            Ok(MessageType::L2Snapshot) => {
                                                info!("Received L2Snapshot message, attempting to decode {} bytes", msg_data.len());
                                                match L2SnapshotMessage::decode(msg_data) {
                                                    Ok(snapshot) => {
                                                        info!("Successfully decoded L2 snapshot for hash {} with {} bids, {} asks", 
                                                            snapshot.symbol_hash, snapshot.bids.len(), snapshot.asks.len());
                                                        Self::handle_l2_snapshot(
                                                            &snapshot,
                                                            &broadcast_tx,
                                                            &symbol_cache,
                                                        );
                                                    }
                                                    Err(e) => {
                                                        error!("Failed to decode L2 snapshot: {}", e);
                                                    }
                                                }
                                            }
                                            Ok(MessageType::L2Delta) => {
                                                if let Ok(delta) = L2DeltaMessage::decode(msg_data) {
                                                    Self::handle_l2_delta(
                                                        &delta,
                                                        &broadcast_tx,
                                                        &symbol_cache,
                                                    );
                                                }
                                            }
                                            Ok(MessageType::SymbolMapping) => {
                                                // Handle dynamic symbol mappings from collectors
                                                if let Ok(mapping) = SymbolMappingMessage::decode(msg_data) {
                                                    let hash = mapping.symbol_hash;
                                                    let display_name = mapping.symbol_string.clone();
                                                    
                                                    // Update the symbol cache
                                                    symbol_cache.write().insert(hash, display_name.clone());
                                                    
                                                    info!("Registered symbol mapping: {} (hash: {})", 
                                                        display_name, hash);
                                                    
                                                    // Broadcast the mapping to all connected clients
                                                    let msg = BroadcastMessage::SymbolMapping {
                                                        symbol_hash: hash,
                                                        symbol: display_name,
                                                    };
                                                    let _ = broadcast_tx.send(msg);
                                                }
                                            }
                                            Ok(MessageType::Heartbeat) => {
                                                debug!("Received heartbeat");
                                            }
                                            _ => {}
                                        }
                                        
                                        pending_data.drain(..total_size);
                                        messages_processed += 1;
                                        if messages_processed % 100 == 0 {
                                            info!("Processed {} messages, {} bytes total", messages_processed, total_bytes_read);
                                        }
                                        counter!("ws_bridge.messages_processed").increment(1);
                                    }
                                }
                                Err(e) => {
                                    error!("Unix socket read error: {}", e);
                                    break;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        retry_count += 1;
                        let delay = std::cmp::min(2u32.pow(retry_count.min(5)), max_retry_delay);
                        warn!("Failed to connect to relay server: {} (attempt {}, retrying in {}s)", e, retry_count, delay);
                        std::thread::sleep(std::time::Duration::from_secs(delay as u64));
                    }
                }
            }
        })
    }

    fn handle_trade(
        trade: &TradeMessage,
        broadcast_tx: &broadcast::Sender<BroadcastMessage>,
        symbol_cache: &RwLock<std::collections::HashMap<u64, String>>,
    ) {
        // Set bridge timestamp before processing
        let mut trade_copy = *trade;
        trade_copy.set_bridge_timestamp();
        
        let symbol_hash = trade_copy.symbol_hash();
        let symbol = symbol_cache.read().get(&symbol_hash).cloned()
            .unwrap_or_else(|| format!("UNKNOWN_SYMBOL_{}", symbol_hash));
        
        let side = match trade_copy.side() {
            TradeSide::Buy => "buy",
            TradeSide::Sell => "sell",
            TradeSide::Unknown => "unknown",
        }.to_string();
        
        // Calculate end-to-end latency
        let now_ns = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        
        let ingestion_ns = trade_copy.ingestion_ns();
        let relay_ns = trade_copy.relay_ns();
        let bridge_ns = trade_copy.bridge_ns();
        
        // Calculate latencies in microseconds
        let latency_collector_to_relay_us = if relay_ns > 0 && ingestion_ns > 0 {
            Some((relay_ns - ingestion_ns) / 1_000)
        } else { None };
        
        let latency_relay_to_bridge_us = if bridge_ns > 0 && relay_ns > 0 {
            Some((bridge_ns - relay_ns) / 1_000)
        } else { None };
        
        let latency_bridge_to_frontend_us = Some((now_ns - bridge_ns) / 1_000);
        
        let latency_total_us = if ingestion_ns > 0 {
            Some((now_ns - ingestion_ns) / 1_000)
        } else { None };
        
        let msg = BroadcastMessage::Trade {
            symbol_hash,
            symbol: Some(symbol),
            timestamp: trade_copy.timestamp_ns() / 1_000_000,
            price: trade_copy.price_f64(),
            volume: trade_copy.volume_f64(),
            side,
            latency_collector_to_relay_us,
            latency_relay_to_bridge_us,
            latency_bridge_to_frontend_us,
            latency_total_us,
        };
        
        // Log latency if we have complete measurements
        if let Some(total_us) = latency_total_us {
            histogram!("ws_bridge.end_to_end_latency_us").record(total_us as f64);
            debug!("Trade E2E latency: {}μs (collector→relay: {:?}μs, relay→bridge: {:?}μs, bridge→frontend: {:?}μs)", 
                total_us, latency_collector_to_relay_us, latency_relay_to_bridge_us, latency_bridge_to_frontend_us);
        }
        
        let _ = broadcast_tx.send(msg);
    }

    fn handle_orderbook(
        orderbook: &OrderBookMessage,
        broadcast_tx: &broadcast::Sender<BroadcastMessage>,
        symbol_cache: &RwLock<std::collections::HashMap<u64, String>>,
    ) {
        let symbol_hash = orderbook.symbol_hash;
        let symbol = symbol_cache.read().get(&symbol_hash).cloned()
            .unwrap_or_else(|| format!("UNKNOWN_SYMBOL_{}", symbol_hash));
        
        let bids: Vec<OrderBookLevel> = orderbook.bids.iter()
            .map(|level| OrderBookLevel {
                price: level.price_f64(),
                size: level.volume_f64(),
            })
            .collect();
        
        let asks: Vec<OrderBookLevel> = orderbook.asks.iter()
            .map(|level| OrderBookLevel {
                price: level.price_f64(),
                size: level.volume_f64(),
            })
            .collect();
        
        let msg = BroadcastMessage::OrderBook {
            symbol_hash,
            symbol: Some(symbol),
            timestamp: orderbook.timestamp_ns / 1_000_000,
            data: OrderBookData {
                bids,
                asks,
                timestamp: orderbook.timestamp_ns / 1_000_000,
            },
        };
        
        let _ = broadcast_tx.send(msg);
    }

    fn handle_l2_snapshot(
        snapshot: &L2SnapshotMessage,
        broadcast_tx: &broadcast::Sender<BroadcastMessage>,
        symbol_cache: &RwLock<std::collections::HashMap<u64, String>>,
    ) {
        let symbol_hash = snapshot.symbol_hash;
        let symbol = symbol_cache.read().get(&symbol_hash).cloned();
        
        // Convert to frontend format - forward raw snapshot
        let bid_levels: Vec<OrderBookLevel> = snapshot.bids.iter()
            .map(|level| OrderBookLevel { 
                price: level.price_f64(), 
                size: level.volume_f64() 
            })
            .collect();
        
        let ask_levels: Vec<OrderBookLevel> = snapshot.asks.iter()
            .map(|level| OrderBookLevel { 
                price: level.price_f64(), 
                size: level.volume_f64() 
            })
            .collect();
        
        let msg = BroadcastMessage::L2Snapshot {
            symbol_hash,
            symbol: symbol.clone(),
            timestamp: snapshot.timestamp_ns / 1_000_000,
            sequence: snapshot.sequence,
            bids: bid_levels,
            asks: ask_levels,
        };
        
        // Don't cache - just broadcast to connected clients
        match broadcast_tx.send(msg) {
            Ok(count) => {
                info!("Broadcast L2 snapshot for hash {} to {} clients ({} bids, {} asks)", 
                    symbol_hash, count, snapshot.bids.len(), snapshot.asks.len());
            }
            Err(e) => {
                debug!("No clients connected to receive L2 snapshot");
            }
        }
    }

    fn handle_l2_delta(
        delta: &L2DeltaMessage,
        broadcast_tx: &broadcast::Sender<BroadcastMessage>,
        symbol_cache: &RwLock<std::collections::HashMap<u64, String>>,
    ) {
        let symbol_hash = delta.symbol_hash;
        // Always try to get the symbol from cache - L2 deltas come after snapshots which include mappings
        let symbol = symbol_cache.read().get(&symbol_hash).cloned()
            .or_else(|| Some(format!("UNKNOWN_{}", symbol_hash)));
        
        // Convert L2 updates to JSON format
        let updates: Vec<L2UpdateJson> = delta.updates.iter()
            .map(|update| L2UpdateJson {
                side: if update.side == 0 { "bid".to_string() } else { "ask".to_string() },
                price: update.price() as f64 / 1e8,
                size: update.volume() as f64 / 1e8,
                action: match update.action() {
                    L2Action::Delete => "delete".to_string(),
                    L2Action::Update => "update".to_string(),
                    L2Action::Insert => "insert".to_string(),
                }
            })
            .collect();
        
        // Forward raw L2 delta to frontend for processing
        let msg = BroadcastMessage::L2Delta {
            symbol_hash,
            symbol,
            timestamp: delta.timestamp_ns / 1_000_000,
            sequence: delta.sequence,
            updates,
        };
        
        let _ = broadcast_tx.send(msg);
        debug!("Sent L2 delta for hash {} with {} updates", symbol_hash, delta.updates.len());
    }
    
    // Symbol mapping handling removed - using static registry instead

    async fn spawn_ws_server(&self) -> Result<task::JoinHandle<()>> {
        let addr: SocketAddr = WS_BIND_ADDR.parse()?;
        let listener = TcpListener::bind(&addr).await?;
        info!("WebSocket server listening on ws://{}/stream", addr);
        
        let broadcast_tx = self.broadcast_tx.clone();
        let symbol_cache = self.symbol_cache.clone();
        
        Ok(task::spawn(async move {
            while let Ok((stream, addr)) = listener.accept().await {
                let broadcast_tx = broadcast_tx.clone();
                let symbol_cache = symbol_cache.clone();
                task::spawn(handle_client(stream, addr, broadcast_tx, symbol_cache));
            }
        }))
    }
}

async fn handle_client(
    stream: TcpStream,
    addr: SocketAddr,
    broadcast_tx: broadcast::Sender<BroadcastMessage>,
    symbol_cache: Arc<RwLock<std::collections::HashMap<u64, String>>>,
) {
    let ws_stream = match accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            error!("WebSocket handshake failed for {}: {}", addr, e);
            return;
        }
    };
    
    let client_id = Uuid::new_v4();
    info!("Client {} connected from {}", client_id, addr);
    
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    let mut broadcast_rx = broadcast_tx.subscribe();
    
    let (tx, mut rx) = mpsc::channel::<Message>(100);
    
    // TODO: Request fresh L2 snapshots from relay for this client
    // For now, client will receive snapshots when relay sends them periodically
    // This ensures we never send stale snapshots that don't match current deltas
    info!("Client {} connected - will receive fresh market data", client_id);
    
    // Spawn task to forward messages to WebSocket
    let send_handle = task::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if ws_sender.send(msg).await.is_err() {
                break;
            }
        }
    });
    
    // Spawn task to receive broadcast messages
    let tx_clone = tx.clone();
    let broadcast_handle = task::spawn(async move {
        while let Ok(msg) = broadcast_rx.recv().await {
            if let Ok(json) = serde_json::to_string(&msg) {
                if tx_clone.send(Message::Text(json)).await.is_err() {
                    break;
                }
            }
        }
    });
    
    // Handle incoming WebSocket messages
    while let Some(Ok(msg)) = ws_receiver.next().await {
        match msg {
            Message::Text(text) => {
                if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
                    match client_msg.msg_type.as_str() {
                        "subscribe" => {
                            info!("Client {} subscribed to {:?}", client_id, client_msg.symbols);
                        }
                        "unsubscribe" => {
                            info!("Client {} unsubscribed", client_id);
                        }
                        _ => {}
                    }
                }
            }
            Message::Close(_) => break,
            Message::Ping(data) => {
                let _ = tx.send(Message::Pong(data)).await;
            }
            _ => {}
        }
    }
    
    broadcast_handle.abort();
    send_handle.abort();
    info!("Client {} disconnected", client_id);
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("ws_bridge=debug".parse()?)
                .add_directive("info".parse()?),
        )
        .init();

    info!("Starting WebSocket bridge service");
    
    let server = BridgeServer::new();
    
    tokio::select! {
        result = server.start() => {
            error!("Server exited: {:?}", result);
        }
        _ = tokio::signal::ctrl_c() => {
            info!("Shutting down WebSocket bridge");
        }
    }
    
    Ok(())
}