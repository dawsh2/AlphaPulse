// Event-driven WebSocket server with dedicated thread shared memory readers
// This is the architecturally correct approach for memory-mapped I/O

use crate::shm_reader::{SharedMemoryReaderPool, MarketDataMessage, OrderBookDelta as ShmOrderBookDelta};
use alphapulse_common::{
    Result, 
    types::Trade,
    orderbook_delta::OrderBookDelta,
    AlphaPulseError,
};
use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::Response,
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::{broadcast, mpsc, RwLock};
use tracing::{info, warn, error, debug};
use uuid::Uuid;

// Client subscription preferences
#[derive(Debug, Clone, Default)]
pub struct ClientPreferences {
    pub exchanges: HashSet<String>,
    pub symbols: HashSet<String>,
    pub subscribe_trades: bool,
    pub subscribe_orderbook: bool,
    pub subscribe_stats: bool,
}

impl ClientPreferences {
    pub fn should_receive_trade(&self, trade: &Trade) -> bool {
        if !self.subscribe_trades {
            return false;
        }
        
        let exchange_match = self.exchanges.is_empty() || 
            self.exchanges.contains(&trade.exchange.to_lowercase());
        let symbol_match = self.symbols.is_empty() || 
            self.symbols.contains(&trade.symbol);
            
        exchange_match && symbol_match
    }
    
    pub fn should_receive_delta(&self, delta: &OrderBookDelta) -> bool {
        if !self.subscribe_orderbook {
            return false;
        }
        
        let exchange_match = self.exchanges.is_empty() || 
            self.exchanges.contains(&delta.exchange.to_lowercase());
        let symbol_match = self.symbols.is_empty() || 
            self.symbols.contains(&delta.symbol);
            
        exchange_match && symbol_match
    }
}

// Performance metrics
#[derive(Debug, Clone, Default, Serialize)]
pub struct PerformanceMetrics {
    pub trades_processed: u64,
    pub deltas_processed: u64,
    pub messages_sent: u64,
    pub avg_latency_us: f64,
    pub active_clients: usize,
    #[serde(skip)]
    pub start_time: Option<Instant>,
}

// WebSocket messages
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum WsMessage {
    Trade(Trade),
    OrderBook(OrderBookDelta),
    Stats(StatsMessage),
    Subscribe(SubscribeMessage),
    SystemStats(SystemStats),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StatsMessage {
    pub trades_per_sec: f64,
    pub deltas_per_sec: f64,
    pub latency_us: f64,
    pub active_clients: usize,
    pub uptime_seconds: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SystemStats {
    pub latency_us: f64,
    pub active_clients: usize,
    pub trades_processed: u64,
    pub deltas_processed: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SubscribeMessage {
    pub exchanges: Option<Vec<String>>,
    pub symbols: Option<Vec<String>>,
    pub trades: Option<bool>,
    pub orderbook: Option<bool>,
    pub stats: Option<bool>,
}

// Main WebSocket server
pub struct RealtimeWebSocketServer {
    trade_broadcaster: broadcast::Sender<Trade>,
    delta_broadcaster: broadcast::Sender<OrderBookDelta>,
    stats_broadcaster: broadcast::Sender<SystemStats>,
    clients: Arc<RwLock<HashMap<Uuid, ClientPreferences>>>,
    metrics: Arc<RwLock<PerformanceMetrics>>,
    reader_pool: Option<SharedMemoryReaderPool>,
}

impl RealtimeWebSocketServer {
    pub fn new() -> Self {
        let (trade_tx, _) = broadcast::channel(10000);
        let (delta_tx, _) = broadcast::channel(10000);
        let (stats_tx, _) = broadcast::channel(100);
        
        Self {
            trade_broadcaster: trade_tx,
            delta_broadcaster: delta_tx,
            stats_broadcaster: stats_tx,
            clients: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(PerformanceMetrics {
                start_time: Some(Instant::now()),
                ..Default::default()
            })),
            reader_pool: None,
        }
    }
    
    pub async fn start(&mut self) -> Result<()> {
        info!("🚀 Starting real-time WebSocket server with dedicated thread readers");
        
        // Initialize the shared memory reader pool
        let reader_pool = SharedMemoryReaderPool::new();
        let mut consolidated_stream = reader_pool.into_stream().await;
        
        // Clone for the processing task
        let trade_broadcaster = self.trade_broadcaster.clone();
        let delta_broadcaster = self.delta_broadcaster.clone();
        let stats_broadcaster = self.stats_broadcaster.clone();
        let metrics = self.metrics.clone();
        
        // Spawn task to process messages from dedicated threads
        tokio::spawn(async move {
            info!("📊 Starting message processor for shared memory readers");
            
            while let Some(msg) = consolidated_stream.recv().await {
                match msg {
                    MarketDataMessage::Trade(trade) => {
                        if let Err(_) = trade_broadcaster.send(trade) {
                            // No receivers, continue
                        }
                        
                        let mut metrics_guard = metrics.write().await;
                        metrics_guard.trades_processed += 1;
                    }
                    
                    MarketDataMessage::OrderBookDelta(shm_delta) => {
                        // Convert from shm_reader format to common format
                        let delta = OrderBookDelta {
                            timestamp: shm_delta.timestamp as f64 / 1_000_000_000.0,
                            symbol: shm_delta.symbol,
                            exchange: shm_delta.exchange,
                            version: shm_delta.version,
                            prev_version: shm_delta.prev_version,
                            changes: shm_delta.changes.into_iter().map(|c| {
                                alphapulse_common::OrderBookChange {
                                    price: c.price,
                                    volume: c.volume,
                                    side: c.side,
                                }
                            }).collect(),
                        };
                        
                        if let Err(_) = delta_broadcaster.send(delta) {
                            // No receivers, continue
                        }
                        
                        let mut metrics_guard = metrics.write().await;
                        metrics_guard.deltas_processed += 1;
                    }
                    
                    MarketDataMessage::Stats(stats) => {
                        debug!("Reader stats: {:?}", stats);
                    }
                }
            }
            
            warn!("Message processor exited");
        });
        
        // Start performance metrics reporter
        let stats_broadcaster = self.stats_broadcaster.clone();
        let metrics = self.metrics.clone();
        let clients = self.clients.clone();
        tokio::spawn(Self::run_stats_reporter(stats_broadcaster, metrics, clients));
        
        info!("✅ Real-time WebSocket server started successfully");
        Ok(())
    }
    
    // Performance statistics reporter
    async fn run_stats_reporter(
        broadcaster: broadcast::Sender<SystemStats>,
        metrics: Arc<RwLock<PerformanceMetrics>>,
        clients: Arc<RwLock<HashMap<Uuid, ClientPreferences>>>,
    ) {
        let mut interval = tokio::time::interval(Duration::from_secs(1));
        
        loop {
            interval.tick().await;
            
            let metrics_guard = metrics.read().await;
            let clients_guard = clients.read().await;
            
            let stats = SystemStats {
                latency_us: metrics_guard.avg_latency_us,
                active_clients: clients_guard.len(),
                trades_processed: metrics_guard.trades_processed,
                deltas_processed: metrics_guard.deltas_processed,
            };
            
            if let Err(_) = broadcaster.send(stats) {
                // No receivers
            }
        }
    }
}

// Global instance
use std::sync::OnceLock;
static WEBSOCKET_SERVER: OnceLock<Arc<RwLock<RealtimeWebSocketServer>>> = OnceLock::new();

pub async fn initialize_realtime_websocket() -> Result<()> {
    let mut server = RealtimeWebSocketServer::new();
    server.start().await?;
    
    WEBSOCKET_SERVER.set(Arc::new(RwLock::new(server)))
        .map_err(|_| alphapulse_common::AlphaPulseError::InitializationError)?;
        
    Ok(())
}

// WebSocket handler
pub async fn realtime_websocket_handler(ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    let client_id = Uuid::new_v4();
    info!("🔌 New WebSocket connection: {}", client_id);
    
    // Get server instance
    let server = match WEBSOCKET_SERVER.get() {
        Some(s) => s.clone(),
        None => {
            error!("WebSocket server not initialized");
            return;
        }
    };
    
    // Add client
    {
        let mut clients = server.read().await.clients.write().await;
        clients.insert(client_id, ClientPreferences::default());
    }
    
    // Subscribe to broadcasts
    let mut trade_rx = server.read().await.trade_broadcaster.subscribe();
    let mut delta_rx = server.read().await.delta_broadcaster.subscribe();
    let mut stats_rx = server.read().await.stats_broadcaster.subscribe();
    
    // Create channels for sending to this specific client
    let (tx, mut rx) = mpsc::channel::<WsMessage>(1000);
    
    // Spawn task to receive broadcasts and filter for this client
    let server_clone = server.clone();
    let tx_clone = tx.clone();
    tokio::spawn(async move {
        loop {
            tokio::select! {
                Ok(trade) = trade_rx.recv() => {
                    let clients = server_clone.read().await.clients.read().await;
                    if let Some(prefs) = clients.get(&client_id) {
                        if prefs.should_receive_trade(&trade) {
                            let _ = tx_clone.send(WsMessage::Trade(trade)).await;
                        }
                    }
                }
                Ok(delta) = delta_rx.recv() => {
                    let clients = server_clone.read().await.clients.read().await;
                    if let Some(prefs) = clients.get(&client_id) {
                        if prefs.should_receive_delta(&delta) {
                            let _ = tx_clone.send(WsMessage::OrderBook(delta)).await;
                        }
                    }
                }
                Ok(stats) = stats_rx.recv() => {
                    let _ = tx_clone.send(WsMessage::SystemStats(stats)).await;
                }
            }
        }
    });
    
    // Handle bidirectional communication
    loop {
        tokio::select! {
            Some(msg) = rx.recv() => {
                let json = serde_json::to_string(&msg).unwrap();
                if socket.send(Message::Text(json)).await.is_err() {
                    break;
                }
                
                // Update metrics
                let mut metrics = server.read().await.metrics.write().await;
                metrics.messages_sent += 1;
            }
            Some(Ok(msg)) = socket.next() => {
                match msg {
                    Message::Text(text) => {
                        // Handle subscription updates
                        if let Ok(subscribe) = serde_json::from_str::<SubscribeMessage>(&text) {
                            let mut clients = server.read().await.clients.write().await;
                            if let Some(prefs) = clients.get_mut(&client_id) {
                                if let Some(exchanges) = subscribe.exchanges {
                                    prefs.exchanges = exchanges.into_iter().collect();
                                }
                                if let Some(symbols) = subscribe.symbols {
                                    prefs.symbols = symbols.into_iter().collect();
                                }
                                if let Some(trades) = subscribe.trades {
                                    prefs.subscribe_trades = trades;
                                }
                                if let Some(orderbook) = subscribe.orderbook {
                                    prefs.subscribe_orderbook = orderbook;
                                }
                                if let Some(stats) = subscribe.stats {
                                    prefs.subscribe_stats = stats;
                                }
                            }
                        }
                    }
                    Message::Close(_) => break,
                    _ => {}
                }
            }
            else => break,
        }
    }
    
    // Remove client on disconnect
    {
        let mut clients = server.read().await.clients.write().await;
        clients.remove(&client_id);
    }
    
    info!("🔌 WebSocket disconnected: {}", client_id);
}