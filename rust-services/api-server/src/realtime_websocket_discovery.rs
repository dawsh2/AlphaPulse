// Service discovery-based WebSocket server for dynamic shared memory feeds
// This implementation discovers available feeds without hardcoded paths or fallbacks

use alphapulse_common::{
    shared_memory_registry::SharedMemoryRegistry,
    types::Trade,
    Result, AlphaPulseError,
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
use tokio::time::interval;
use tracing::{info, warn, error};
use uuid::Uuid;

// OrderBook types for WebSocket messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookDelta {
    pub timestamp: f64,
    pub symbol: String,
    pub exchange: String,
    pub version: u64,
    pub prev_version: u64,
    pub changes: Vec<OrderBookChange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookChange {
    pub price: f64,
    pub volume: f64,
    pub side: String,
}

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

// WebSocket messages
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum WsMessage {
    Trade(Trade),
    OrderBook(OrderBookDelta),
    Stats(StatsMessage),
    Subscribe(SubscribeMessage),
    SystemStats(SystemStats),
    ServiceDiscovery(ServiceDiscoveryMessage),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StatsMessage {
    pub trades_per_sec: f64,
    pub deltas_per_sec: f64,
    pub latency_us: f64,
    pub active_clients: usize,
    pub uptime_seconds: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStats {
    pub latency_us: f64,
    pub active_clients: usize,
    pub trades_processed: u64,
    pub deltas_processed: u64,
    pub active_feeds: usize,
    pub trade_feeds: usize,
    pub delta_feeds: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDiscoveryMessage {
    pub discovered_feeds: usize,
    pub trade_feeds: usize,
    pub delta_feeds: usize,
    pub exchanges: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SubscribeMessage {
    pub exchanges: Option<Vec<String>>,
    pub symbols: Option<Vec<String>>,
    pub trades: Option<bool>,
    pub orderbook: Option<bool>,
    pub stats: Option<bool>,
}

// Main WebSocket server with service discovery
pub struct DiscoveryRealtimeWebSocketServer {
    trade_broadcaster: broadcast::Sender<Trade>,
    delta_broadcaster: broadcast::Sender<OrderBookDelta>,
    stats_broadcaster: broadcast::Sender<SystemStats>,
    discovery_broadcaster: broadcast::Sender<ServiceDiscoveryMessage>,
    clients: Arc<RwLock<HashMap<Uuid, ClientPreferences>>>,
    metrics: Arc<RwLock<PerformanceMetrics>>,
    registry: Arc<RwLock<SharedMemoryRegistry>>,
}

#[derive(Debug, Clone, Default)]
struct PerformanceMetrics {
    pub trades_processed: u64,
    pub deltas_processed: u64,
    pub messages_sent: u64,
    pub avg_latency_us: f64,
    pub start_time: Option<Instant>,
}

impl DiscoveryRealtimeWebSocketServer {
    pub fn new() -> Result<Self> {
        let (trade_tx, _) = broadcast::channel(10000);
        let (delta_tx, _) = broadcast::channel(10000);
        let (stats_tx, _) = broadcast::channel(100);
        let (discovery_tx, _) = broadcast::channel(10);
        
        let registry = SharedMemoryRegistry::new()?;
        
        Ok(Self {
            trade_broadcaster: trade_tx,
            delta_broadcaster: delta_tx,
            stats_broadcaster: stats_tx,
            discovery_broadcaster: discovery_tx,
            clients: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(PerformanceMetrics {
                start_time: Some(Instant::now()),
                ..Default::default()
            })),
            registry: Arc::new(RwLock::new(registry)),
        })
    }
    
    pub async fn start(&mut self) -> Result<()> {
        info!("🚀 Starting service discovery-based WebSocket server");
        
        // Initial feed discovery
        {
            let mut registry = self.registry.write().await;
            let discovered = registry.discover_feeds()?;
            if discovered > 0 {
                registry.initialize_readers()?;
                info!("✅ Discovered and initialized {} feeds", discovered);
            } else {
                info!("⏳ No feeds discovered yet, will retry periodically");
            }
        }
        
        // Clone for the data polling task
        let trade_broadcaster = self.trade_broadcaster.clone();
        let delta_broadcaster = self.delta_broadcaster.clone();
        let metrics = self.metrics.clone();
        let registry = self.registry.clone();
        
        // Spawn event-driven data reading task (eliminates polling!)
        tokio::spawn(async move {
            let mut last_latency_check = Instant::now();
            
            loop {
                let start = Instant::now();
                let mut registry_guard = registry.write().await;
                
                // Use event-driven readers that wait for notifications instead of polling
                let trades = registry_guard.read_all_trades_event_driven().await;
                let shared_deltas = registry_guard.read_all_deltas();
                drop(registry_guard);
                
                let read_latency = start.elapsed().as_nanos() as f64 / 1000.0;
                
                // Broadcast trades
                if !trades.is_empty() {
                    info!("📊 Broadcasting {} trades", trades.len());
                }
                for trade in trades {
                    if let Err(_) = trade_broadcaster.send(trade) {
                        // No receivers
                    }
                    let mut metrics_guard = metrics.write().await;
                    metrics_guard.trades_processed += 1;
                }
                
                // Convert and broadcast deltas
                for shared_delta in shared_deltas {
                    let mut changes = Vec::new();
                    for i in 0..shared_delta.change_count as usize {
                        let change = &shared_delta.changes[i];
                        let is_ask = (change.side_and_action & 0x80) != 0;
                        
                        changes.push(OrderBookChange {
                            price: change.price as f64,
                            volume: change.volume as f64,
                            side: if is_ask { "ask".to_string() } else { "bid".to_string() },
                        });
                    }
                    
                    let delta = OrderBookDelta {
                        timestamp: shared_delta.timestamp_ns as f64 / 1_000_000_000.0,
                        symbol: shared_delta.symbol_str(),
                        exchange: shared_delta.exchange_str(),
                        version: shared_delta.version,
                        prev_version: shared_delta.prev_version,
                        changes,
                    };
                    
                    if let Err(_) = delta_broadcaster.send(delta) {
                        // No receivers
                    }
                    let mut metrics_guard = metrics.write().await;
                    metrics_guard.deltas_processed += 1;
                }
                
                // Update latency metrics
                if last_latency_check.elapsed() > Duration::from_secs(1) {
                    let mut metrics_guard = metrics.write().await;
                    metrics_guard.avg_latency_us = read_latency;
                    last_latency_check = Instant::now();
                }
            }
        });
        
        // Periodic feed discovery task
        let discovery_broadcaster = self.discovery_broadcaster.clone();
        let registry = self.registry.clone();
        tokio::spawn(async move {
            let mut discovery_interval = interval(Duration::from_secs(5)); // Check every 5 seconds
            
            loop {
                discovery_interval.tick().await;
                info!("🔍 Running periodic feed discovery...");
                
                let mut registry_guard = registry.write().await;
                match registry_guard.discover_feeds() {
                    Ok(discovered) => {
                        info!("🔍 Discovery completed: {} feeds found", discovered);
                        if discovered > 0 {
                            if let Err(e) = registry_guard.initialize_readers() {
                                error!("Failed to initialize new readers: {}", e);
                            } else {
                                let (trade_feeds, delta_feeds) = registry_guard.get_feed_counts();
                                let metadata = registry_guard.get_feed_metadata();
                                let exchanges: Vec<String> = metadata.values()
                                    .map(|m| m.exchange.clone())
                                    .collect::<HashSet<_>>()
                                    .into_iter()
                                    .collect();
                                
                                let discovery_msg = ServiceDiscoveryMessage {
                                    discovered_feeds: discovered,
                                    trade_feeds,
                                    delta_feeds,
                                    exchanges: exchanges.clone(),
                                };
                                
                                if let Err(_) = discovery_broadcaster.send(discovery_msg) {
                                    // No receivers
                                }
                                
                                info!("🔄 Feed discovery update: {} trades, {} deltas, exchanges: {:?}", 
                                      trade_feeds, delta_feeds, exchanges);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Feed discovery failed: {}", e);
                    }
                }
            }
        });
        
        // Start performance reporter
        let stats_broadcaster = self.stats_broadcaster.clone();
        let metrics = self.metrics.clone();
        let clients = self.clients.clone();
        let registry = self.registry.clone();
        tokio::spawn(Self::run_stats_reporter(stats_broadcaster, metrics, clients, registry));
        
        info!("✅ Service discovery WebSocket server started successfully");
        Ok(())
    }
    
    async fn run_stats_reporter(
        broadcaster: broadcast::Sender<SystemStats>,
        metrics: Arc<RwLock<PerformanceMetrics>>,
        clients: Arc<RwLock<HashMap<Uuid, ClientPreferences>>>,
        registry: Arc<RwLock<SharedMemoryRegistry>>,
    ) {
        let mut interval = tokio::time::interval(Duration::from_secs(1));
        
        loop {
            interval.tick().await;
            
            let metrics_guard = metrics.read().await;
            let clients_guard = clients.read().await;
            let registry_guard = registry.read().await;
            
            let (trade_feeds, delta_feeds) = registry_guard.get_feed_counts();
            let active_feeds = trade_feeds + delta_feeds;
            
            let stats = SystemStats {
                latency_us: metrics_guard.avg_latency_us,
                active_clients: clients_guard.len(),
                trades_processed: metrics_guard.trades_processed,
                deltas_processed: metrics_guard.deltas_processed,
                active_feeds,
                trade_feeds,
                delta_feeds,
            };
            
            if let Err(_) = broadcaster.send(stats) {
                // No receivers
            }
        }
    }
}

// Global instance
use std::sync::OnceLock;
static WEBSOCKET_SERVER: OnceLock<Arc<RwLock<DiscoveryRealtimeWebSocketServer>>> = OnceLock::new();

pub async fn initialize_discovery_websocket() -> Result<()> {
    let mut server = DiscoveryRealtimeWebSocketServer::new()?;
    server.start().await?;
    
    WEBSOCKET_SERVER.set(Arc::new(RwLock::new(server)))
        .map_err(|_| AlphaPulseError::ConfigError("Failed to initialize WebSocket server".to_string()))?;
        
    Ok(())
}

// WebSocket handler
pub async fn discovery_websocket_handler(ws: WebSocketUpgrade) -> Response {
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
    
    // Add client with default preferences (subscribe to everything)
    {
        let server_guard = server.read().await;
        let mut clients = server_guard.clients.write().await;
        clients.insert(client_id, ClientPreferences {
            subscribe_trades: true,
            subscribe_orderbook: true,
            subscribe_stats: true,
            ..Default::default()
        });
    }
    
    // Subscribe to broadcasts
    let mut trade_rx = server.read().await.trade_broadcaster.subscribe();
    let mut delta_rx = server.read().await.delta_broadcaster.subscribe();
    let mut stats_rx = server.read().await.stats_broadcaster.subscribe();
    let mut discovery_rx = server.read().await.discovery_broadcaster.subscribe();
    
    // Create channel for sending to this client
    let (tx, mut rx) = mpsc::channel::<WsMessage>(1000);
    
    // Spawn task to receive broadcasts and filter for this client
    let server_clone = server.clone();
    let tx_clone = tx.clone();
    tokio::spawn(async move {
        loop {
            tokio::select! {
                Ok(trade) = trade_rx.recv() => {
                    let server_guard = server_clone.read().await;
                    let clients = server_guard.clients.read().await;
                    if let Some(prefs) = clients.get(&client_id) {
                        if prefs.should_receive_trade(&trade) {
                            let _ = tx_clone.send(WsMessage::Trade(trade)).await;
                        }
                    }
                }
                Ok(delta) = delta_rx.recv() => {
                    let server_guard = server_clone.read().await;
                    let clients = server_guard.clients.read().await;
                    if let Some(prefs) = clients.get(&client_id) {
                        if prefs.should_receive_delta(&delta) {
                            let _ = tx_clone.send(WsMessage::OrderBook(delta)).await;
                        }
                    }
                }
                Ok(stats) = stats_rx.recv() => {
                    let _ = tx_clone.send(WsMessage::SystemStats(stats)).await;
                }
                Ok(discovery) = discovery_rx.recv() => {
                    let _ = tx_clone.send(WsMessage::ServiceDiscovery(discovery)).await;
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
                let server_guard = server.read().await;
                let mut metrics = server_guard.metrics.write().await;
                metrics.messages_sent += 1;
            }
            Some(Ok(msg)) = socket.next() => {
                match msg {
                    Message::Text(text) => {
                        // Handle subscription updates
                        if let Ok(subscribe) = serde_json::from_str::<SubscribeMessage>(&text) {
                            let server_guard = server.read().await;
                            let mut clients = server_guard.clients.write().await;
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
        let server_guard = server.read().await;
        let mut clients = server_guard.clients.write().await;
        clients.remove(&client_id);
    }
    
    info!("🔌 WebSocket disconnected: {}", client_id);
}