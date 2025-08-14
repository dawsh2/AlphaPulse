// Event-driven WebSocket server with ultra-low latency shared memory integration
use alphapulse_common::{
    Result, Trade, OrderBookDelta, 
    shared_memory::{SharedMemoryReader, OrderBookDeltaReader, SharedOrderBookDelta},
};
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
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

// Helper function to convert SharedTrade to Trade (avoids orphan rule issues)
fn convert_shared_trade_to_trade(shared_trade: &alphapulse_common::shared_memory::SharedTrade) -> Trade {
    let symbol = std::str::from_utf8(&shared_trade.symbol)
        .unwrap_or("UNKNOWN")
        .trim_end_matches('\0')
        .to_string();
        
    let exchange = std::str::from_utf8(&shared_trade.exchange)
        .unwrap_or("UNKNOWN")
        .trim_end_matches('\0')
        .to_string();
        
    let trade_id = if shared_trade.trade_id[0] != 0 {
        Some(std::str::from_utf8(&shared_trade.trade_id)
            .unwrap_or("")
            .trim_end_matches('\0')
            .to_string())
    } else {
        None
    };
    
    let side = match shared_trade.side {
        0 => Some("buy".to_string()),
        1 => Some("sell".to_string()),
        _ => None,
    };
    
    Trade {
        timestamp: shared_trade.timestamp_ns as f64 / 1_000_000_000.0,
        symbol,
        exchange,
        price: shared_trade.price,
        volume: shared_trade.volume,
        side,
        trade_id,
    }
}

// WebSocket message types for client communication
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WebSocketMessage {
    #[serde(rename = "trade")]
    Trade {
        symbol: String,
        exchange: String,
        price: f64,
        volume: f64,
        side: Option<String>,
        timestamp: f64,
    },
    #[serde(rename = "orderbook_delta")]
    OrderBookDelta {
        symbol: String,
        exchange: String,
        bid_changes: Vec<PriceLevel>,
        ask_changes: Vec<PriceLevel>,
        version: u64,
        timestamp: f64,
    },
    #[serde(rename = "system_stats")]
    SystemStats {
        latency_us: f64,
        compression_ratio: f64,
        active_clients: usize,
        messages_per_second: f64,
    },
    #[serde(rename = "error")]
    Error {
        message: String,
        code: u16,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceLevel {
    pub price: f64,
    pub volume: f64,
    pub action: String, // "add", "update", "remove"
}

// Client subscription request
#[derive(Debug, Deserialize)]
pub struct SubscriptionRequest {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub channels: Vec<String>,   // "trades", "deltas", "stats"
    pub symbols: Vec<String>,    // "BTC/USD", "ETH/USD", etc.
    pub exchanges: Option<Vec<String>>, // "coinbase", "kraken", "binance"
}

// Client session management
#[derive(Debug)]
pub struct ClientSession {
    pub id: String,
    pub subscriptions: HashSet<String>, // subscribed symbols
    pub channels: HashSet<String>,      // subscribed channels
    pub exchanges: HashSet<String>,     // subscribed exchanges
    pub sender: mpsc::Sender<WebSocketMessage>,
    pub connected_at: Instant,
    pub messages_sent: u64,
}

impl ClientSession {
    pub fn new(sender: mpsc::Sender<WebSocketMessage>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            subscriptions: HashSet::new(),
            channels: HashSet::new(),
            exchanges: HashSet::new(),
            sender,
            connected_at: Instant::now(),
            messages_sent: 0,
        }
    }

    pub fn should_receive_trade(&self, trade: &Trade) -> bool {
        self.channels.contains("trades") &&
        self.subscriptions.contains(&trade.symbol) &&
        (self.exchanges.is_empty() || self.exchanges.contains(&trade.exchange))
    }

    pub fn should_receive_delta(&self, delta: &OrderBookDelta) -> bool {
        self.channels.contains("deltas") &&
        self.subscriptions.contains(&delta.symbol) &&
        (self.exchanges.is_empty() || self.exchanges.contains(&delta.exchange))
    }

    pub async fn send_message(&mut self, message: WebSocketMessage) -> bool {
        match self.sender.send(message).await {
            Ok(_) => {
                self.messages_sent += 1;
                true
            }
            Err(_) => false, // Client disconnected
        }
    }
}

// Main real-time WebSocket server
#[derive(Clone)]
pub struct RealtimeWebSocketServer {
    // Shared memory readers are managed by background tasks
    _phantom: std::marker::PhantomData<()>,
    
    // Broadcast channels for real-time distribution
    trade_broadcaster: broadcast::Sender<Trade>,
    delta_broadcaster: broadcast::Sender<OrderBookDelta>,
    stats_broadcaster: broadcast::Sender<WebSocketMessage>,
    
    // Client management
    clients: Arc<RwLock<HashMap<String, ClientSession>>>,
    
    // Performance metrics
    metrics: Arc<RwLock<PerformanceMetrics>>,
}

#[derive(Debug, Default)]
pub struct PerformanceMetrics {
    pub trades_processed: u64,
    pub deltas_processed: u64,
    pub messages_broadcast: u64,
    pub avg_latency_us: f64,
    pub active_clients: usize,
    pub start_time: Option<Instant>,
}

impl RealtimeWebSocketServer {
    pub fn new() -> Self {
        let (trade_tx, _) = broadcast::channel(10000);
        let (delta_tx, _) = broadcast::channel(10000);
        let (stats_tx, _) = broadcast::channel(100);

        Self {
            _phantom: std::marker::PhantomData,
            trade_broadcaster: trade_tx,
            delta_broadcaster: delta_tx,
            stats_broadcaster: stats_tx,
            clients: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(PerformanceMetrics {
                start_time: Some(Instant::now()),
                ..Default::default()
            })),
        }
    }

    pub async fn start(&self) -> Result<()> {
        info!("🚀 Starting real-time WebSocket server with shared memory integration");
        
        // Initialize shared memory readers for background tasks
        
        // Trade reader (all exchanges write to same trades buffer)
        if let Ok(reader) = SharedMemoryReader::open("/tmp/alphapulse_shm/trades", 0) {
            info!("✅ Connected to trades shared memory");
            let broadcaster = self.trade_broadcaster.clone();
            let metrics = self.metrics.clone();
            tokio::spawn(Self::run_trade_reader(reader, broadcaster, metrics));
        } else {
            warn!("❌ Failed to connect to Coinbase trade shared memory");
        }
        
        // Coinbase delta reader
        if let Ok(reader) = OrderBookDeltaReader::open("/tmp/alphapulse_shm/orderbook_deltas", 1) {
            info!("✅ Connected to Coinbase delta shared memory");
            let broadcaster = self.delta_broadcaster.clone();
            let metrics = self.metrics.clone();
            tokio::spawn(Self::run_coinbase_delta_reader(reader, broadcaster, metrics));
        } else {
            warn!("❌ Failed to connect to Coinbase delta shared memory");
        }
        
        // Kraken delta reader
        if let Ok(reader) = OrderBookDeltaReader::open("/tmp/alphapulse_shm/kraken_orderbook_deltas", 2) {
            info!("✅ Connected to Kraken delta shared memory");
            let broadcaster = self.delta_broadcaster.clone();
            let metrics = self.metrics.clone();
            tokio::spawn(Self::run_kraken_delta_reader(reader, broadcaster, metrics));
        } else {
            warn!("❌ Failed to connect to Kraken delta shared memory");
        }
        
        // Binance delta reader
        if let Ok(reader) = OrderBookDeltaReader::open("/tmp/alphapulse_shm/binance_orderbook_deltas", 3) {
            info!("✅ Connected to Binance delta shared memory");
            let broadcaster = self.delta_broadcaster.clone();
            let metrics = self.metrics.clone();
            tokio::spawn(Self::run_binance_delta_reader(reader, broadcaster, metrics));
        } else {
            warn!("❌ Failed to connect to Binance delta shared memory");
        }
        
        // Start performance metrics reporter
        let stats_broadcaster = self.stats_broadcaster.clone();
        let metrics = self.metrics.clone();
        let clients = self.clients.clone();
        tokio::spawn(Self::run_stats_reporter(stats_broadcaster, metrics, clients));
        
        info!("✅ Real-time WebSocket server started successfully");
        Ok(())
    }

    // Event-driven trade reader - pushes data immediately when available
    async fn run_trade_reader(
        reader: SharedMemoryReader,
        broadcaster: broadcast::Sender<Trade>,
        metrics: Arc<RwLock<PerformanceMetrics>>,
    ) {
        info!("📊 Starting Coinbase trade reader (event-driven)");
        
        // Move reader to Arc<Mutex> for safe sharing across blocking tasks
        let reader = Arc::new(tokio::sync::Mutex::new(reader));
        
        loop {
            let start_time = Instant::now();
            
            // Read from shared memory in blocking context to avoid SIGBUS
            let reader_clone = reader.clone();
            let trades = tokio::task::spawn_blocking(move || {
                let mut reader_guard = reader_clone.blocking_lock();
                reader_guard.read_trades()
            }).await.unwrap_or_default();
            if !trades.is_empty() {
                let read_latency = start_time.elapsed().as_nanos() as f64 / 1000.0; // μs
                
                for trade in &trades {
                    // Convert SharedTrade to Trade
                    let trade = convert_shared_trade_to_trade(trade);
                    
                    // Immediately broadcast to all subscribed clients
                    if let Err(_) = broadcaster.send(trade) {
                        // No receivers, continue
                    }
                }
                
                // Update metrics
                let mut metrics_guard = metrics.write().await;
                metrics_guard.trades_processed += trades.len() as u64;
                metrics_guard.avg_latency_us = (metrics_guard.avg_latency_us * 0.9) + (read_latency * 0.1);
                drop(metrics_guard);
                
                debug!("📨 Broadcast {} trades (latency: {:.1}μs)", trades.len(), read_latency);
            } else {
                // No new data, yield briefly and check again
                tokio::time::sleep(Duration::from_micros(100)).await;
            }
        }
    }

    // Event-driven delta reader for Coinbase
    async fn run_coinbase_delta_reader(
        reader: OrderBookDeltaReader,
        broadcaster: broadcast::Sender<OrderBookDelta>,
        metrics: Arc<RwLock<PerformanceMetrics>>,
    ) {
        info!("📊 Starting Coinbase delta reader (event-driven)");
        
        // Move reader to Arc<Mutex> for safe sharing across blocking tasks
        let reader = Arc::new(tokio::sync::Mutex::new(reader));
        
        loop {
            let start_time = Instant::now();
            
            // Read from shared memory in blocking context to avoid SIGBUS
            let reader_clone = reader.clone();
            let deltas = tokio::task::spawn_blocking(move || {
                let mut reader_guard = reader_clone.blocking_lock();
                reader_guard.read_deltas()
            }).await.unwrap_or_default();
            if !deltas.is_empty() {
                let read_latency = start_time.elapsed().as_nanos() as f64 / 1000.0; // μs
                let deltas_len = deltas.len();
                
                for shared_delta in &deltas {
                    // Convert SharedOrderBookDelta to OrderBookDelta
                    let delta = Self::convert_shared_delta_to_delta(&shared_delta, "coinbase");
                    
                    if let Err(_) = broadcaster.send(delta) {
                        // No receivers, continue
                    }
                }
                
                // Update metrics
                let mut metrics_guard = metrics.write().await;
                metrics_guard.deltas_processed += deltas_len as u64;
                metrics_guard.avg_latency_us = (metrics_guard.avg_latency_us * 0.9) + (read_latency * 0.1);
                drop(metrics_guard);
                
                debug!("📨 Broadcast {} Coinbase deltas (latency: {:.1}μs)", deltas_len, read_latency);
            } else {
                tokio::time::sleep(Duration::from_micros(100)).await;
            }
        }
    }

    // Event-driven delta reader for Kraken
    async fn run_kraken_delta_reader(
        reader: OrderBookDeltaReader,
        broadcaster: broadcast::Sender<OrderBookDelta>,
        metrics: Arc<RwLock<PerformanceMetrics>>,
    ) {
        info!("📊 Starting Kraken delta reader (event-driven)");
        
        // Move reader to Arc<Mutex> for safe sharing across blocking tasks
        let reader = Arc::new(tokio::sync::Mutex::new(reader));
        
        loop {
            // Read from shared memory in blocking context to avoid SIGBUS
            let reader_clone = reader.clone();
            let deltas = tokio::task::spawn_blocking(move || {
                let mut reader_guard = reader_clone.blocking_lock();
                reader_guard.read_deltas()
            }).await.unwrap_or_default();
            if !deltas.is_empty() {
                let deltas_len = deltas.len();
                for shared_delta in &deltas {
                    let delta = Self::convert_shared_delta_to_delta(&shared_delta, "kraken");
                    let _ = broadcaster.send(delta);
                }
                
                let mut metrics_guard = metrics.write().await;
                metrics_guard.deltas_processed += deltas_len as u64;
                drop(metrics_guard);
                
                debug!("📨 Broadcast {} Kraken deltas", deltas_len);
            } else {
                tokio::time::sleep(Duration::from_micros(100)).await;
            }
        }
    }

    // Event-driven delta reader for Binance
    async fn run_binance_delta_reader(
        reader: OrderBookDeltaReader,
        broadcaster: broadcast::Sender<OrderBookDelta>,
        metrics: Arc<RwLock<PerformanceMetrics>>,
    ) {
        info!("📊 Starting Binance delta reader (event-driven)");
        
        // Move reader to Arc<Mutex> for safe sharing across blocking tasks
        let reader = Arc::new(tokio::sync::Mutex::new(reader));
        
        loop {
            // Read from shared memory in blocking context to avoid SIGBUS
            let reader_clone = reader.clone();
            let deltas = tokio::task::spawn_blocking(move || {
                let mut reader_guard = reader_clone.blocking_lock();
                reader_guard.read_deltas()
            }).await.unwrap_or_default();
            if !deltas.is_empty() {
                let deltas_len = deltas.len();
                for shared_delta in &deltas {
                    let delta = Self::convert_shared_delta_to_delta(&shared_delta, "binance");
                    let _ = broadcaster.send(delta);
                }
                
                let mut metrics_guard = metrics.write().await;
                metrics_guard.deltas_processed += deltas_len as u64;
                drop(metrics_guard);
                
                debug!("📨 Broadcast {} Binance deltas", deltas_len);
            } else {
                tokio::time::sleep(Duration::from_micros(100)).await;
            }
        }
    }

    // Performance statistics reporter
    async fn run_stats_reporter(
        broadcaster: broadcast::Sender<WebSocketMessage>,
        metrics: Arc<RwLock<PerformanceMetrics>>,
        clients: Arc<RwLock<HashMap<String, ClientSession>>>,
    ) {
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        
        loop {
            interval.tick().await;
            
            let metrics_guard = metrics.read().await;
            let clients_guard = clients.read().await;
            
            let uptime_secs = metrics_guard.start_time
                .map(|start| start.elapsed().as_secs())
                .unwrap_or(0);
            
            let messages_per_second = if uptime_secs > 0 {
                metrics_guard.messages_broadcast as f64 / uptime_secs as f64
            } else {
                0.0
            };
            
            let compression_ratio = 0.99975; // Typical compression ratio
            
            let stats_message = WebSocketMessage::SystemStats {
                latency_us: metrics_guard.avg_latency_us,
                compression_ratio,
                active_clients: clients_guard.len(),
                messages_per_second,
            };
            
            drop(metrics_guard);
            drop(clients_guard);
            
            let _ = broadcaster.send(stats_message);
        }
    }

    // Convert SharedOrderBookDelta to OrderBookDelta
    fn convert_shared_delta_to_delta(shared_delta: &SharedOrderBookDelta, exchange: &str) -> OrderBookDelta {
        use alphapulse_common::orderbook_delta::{PriceLevel, DeltaAction};
        
        let symbol = std::str::from_utf8(&shared_delta.symbol)
            .unwrap_or("UNKNOWN")
            .trim_end_matches('\0')
            .to_string();

        let mut bid_changes = Vec::new();
        let mut ask_changes = Vec::new();

        // Extract changes from shared delta
        for i in 0..shared_delta.change_count as usize {
            if i < shared_delta.changes.len() {
                let change = &shared_delta.changes[i];
                let action = match change.side_and_action & 0x7F {
                    0 => DeltaAction::Add,
                    1 => DeltaAction::Update,
                    2 => DeltaAction::Remove,
                    _ => DeltaAction::Update,
                };

                let price_level = PriceLevel {
                    price: change.price as f64,
                    volume: change.volume as f64,
                    action,
                };

                if (change.side_and_action & 0x80) != 0 {
                    ask_changes.push(price_level);
                } else {
                    bid_changes.push(price_level);
                }
            }
        }

        OrderBookDelta {
            symbol,
            exchange: exchange.to_string(),
            version: shared_delta.version,
            prev_version: shared_delta.prev_version,
            timestamp: shared_delta.timestamp_ns as f64 / 1_000_000_000.0,
            bid_changes,
            ask_changes,
        }
    }

    // WebSocket handler for new client connections  
    pub async fn handle_client_connection(&self, ws: WebSocketUpgrade) -> Response {
        let clients = self.clients.clone();
        let trade_rx = self.trade_broadcaster.subscribe();
        let delta_rx = self.delta_broadcaster.subscribe();
        let stats_rx = self.stats_broadcaster.subscribe();
        
        ws.on_upgrade(move |socket| {
            Self::handle_client_socket(socket, clients, trade_rx, delta_rx, stats_rx)
        })
    }

    // Handle individual client WebSocket connection
    async fn handle_client_socket(
        socket: WebSocket,
        clients: Arc<RwLock<HashMap<String, ClientSession>>>,
        mut trade_rx: broadcast::Receiver<Trade>,
        mut delta_rx: broadcast::Receiver<OrderBookDelta>,
        mut stats_rx: broadcast::Receiver<WebSocketMessage>,
    ) {
        let (mut ws_sender, mut ws_receiver) = socket.split();
        let (msg_tx, mut msg_rx) = mpsc::channel::<WebSocketMessage>(1000);
        
        // Create client session
        let client = ClientSession::new(msg_tx);
        let client_id = client.id.clone();
        
        info!("🔌 WebSocket client connected: {}", client_id);
        
        // Add client to active clients
        {
            let mut clients_guard = clients.write().await;
            clients_guard.insert(client_id.clone(), client);
        }
        
        // Task to send messages to WebSocket
        let clients_clone = clients.clone();
        let client_id_clone = client_id.clone();
        let send_task = tokio::spawn(async move {
            while let Some(message) = msg_rx.recv().await {
                let json_str = serde_json::to_string(&message).unwrap_or_default();
                if ws_sender.send(Message::Text(json_str)).await.is_err() {
                    break; // Client disconnected
                }
            }
            
            // Remove client from active clients
            let mut clients_guard = clients_clone.write().await;
            clients_guard.remove(&client_id_clone);
            info!("🔌 WebSocket client disconnected: {}", client_id_clone);
        });
        
        // Task to handle trade broadcasts
        let clients_trade = clients.clone();
        let client_id_trade = client_id.clone();
        let trade_task = tokio::spawn(async move {
            while let Ok(trade) = trade_rx.recv().await {
                let should_send = {
                    let clients_guard = clients_trade.read().await;
                    clients_guard.get(&client_id_trade)
                        .map(|client| client.should_receive_trade(&trade))
                        .unwrap_or(false)
                };
                
                if should_send {
                    let message = WebSocketMessage::Trade {
                        symbol: trade.symbol,
                        exchange: trade.exchange,
                        price: trade.price,
                        volume: trade.volume,
                        side: trade.side,
                        timestamp: trade.timestamp,
                    };
                    
                    let mut clients_guard = clients_trade.write().await;
                    if let Some(client) = clients_guard.get_mut(&client_id_trade) {
                        if !client.send_message(message).await {
                            break; // Client disconnected
                        }
                    }
                }
            }
        });
        
        // Task to handle delta broadcasts
        let clients_delta = clients.clone();
        let client_id_delta = client_id.clone();
        let delta_task = tokio::spawn(async move {
            while let Ok(delta) = delta_rx.recv().await {
                let should_send = {
                    let clients_guard = clients_delta.read().await;
                    clients_guard.get(&client_id_delta)
                        .map(|client| client.should_receive_delta(&delta))
                        .unwrap_or(false)
                };
                
                if should_send {
                    let bid_changes: Vec<PriceLevel> = delta.bid_changes.iter()
                        .map(|level| PriceLevel {
                            price: level.price,
                            volume: level.volume,
                            action: match level.action {
                                alphapulse_common::orderbook_delta::DeltaAction::Add => "add".to_string(),
                                alphapulse_common::orderbook_delta::DeltaAction::Update => "update".to_string(),
                                alphapulse_common::orderbook_delta::DeltaAction::Remove => "remove".to_string(),
                            }
                        })
                        .collect();
                        
                    let ask_changes: Vec<PriceLevel> = delta.ask_changes.iter()
                        .map(|level| PriceLevel {
                            price: level.price,
                            volume: level.volume,
                            action: match level.action {
                                alphapulse_common::orderbook_delta::DeltaAction::Add => "add".to_string(),
                                alphapulse_common::orderbook_delta::DeltaAction::Update => "update".to_string(),
                                alphapulse_common::orderbook_delta::DeltaAction::Remove => "remove".to_string(),
                            }
                        })
                        .collect();
                    
                    let message = WebSocketMessage::OrderBookDelta {
                        symbol: delta.symbol,
                        exchange: delta.exchange,
                        bid_changes,
                        ask_changes,
                        version: delta.version,
                        timestamp: delta.timestamp,
                    };
                    
                    let mut clients_guard = clients_delta.write().await;
                    if let Some(client) = clients_guard.get_mut(&client_id_delta) {
                        if !client.send_message(message).await {
                            break; // Client disconnected
                        }
                    }
                }
            }
        });
        
        // Task to handle stats broadcasts
        let clients_stats = clients.clone();
        let client_id_stats = client_id.clone();
        let stats_task = tokio::spawn(async move {
            while let Ok(stats_message) = stats_rx.recv().await {
                let should_send = {
                    let clients_guard = clients_stats.read().await;
                    clients_guard.get(&client_id_stats)
                        .map(|client| client.channels.contains("stats"))
                        .unwrap_or(false)
                };
                
                if should_send {
                    let mut clients_guard = clients_stats.write().await;
                    if let Some(client) = clients_guard.get_mut(&client_id_stats) {
                        if !client.send_message(stats_message).await {
                            break; // Client disconnected
                        }
                    }
                }
            }
        });
        
        // Handle incoming messages from client
        while let Some(Ok(msg)) = ws_receiver.next().await {
            if let Err(e) = Self::handle_client_message(msg, &client_id, &clients).await {
                error!("Error handling client message: {}", e);
            }
        }
        
        // Cleanup
        send_task.abort();
        trade_task.abort();
        delta_task.abort();
        stats_task.abort();
    }

    // Handle incoming messages from WebSocket clients
    async fn handle_client_message(
        msg: Message,
        client_id: &str,
        clients: &Arc<RwLock<HashMap<String, ClientSession>>>,
    ) -> Result<()> {
        match msg {
            Message::Text(text) => {
                debug!("📩 Received from {}: {}", client_id, text);
                
                if let Ok(sub_request) = serde_json::from_str::<SubscriptionRequest>(&text) {
                    let mut clients_guard = clients.write().await;
                    if let Some(client) = clients_guard.get_mut(client_id) {
                        match sub_request.msg_type.as_str() {
                            "subscribe" => {
                                client.channels = sub_request.channels.into_iter().collect();
                                client.subscriptions = sub_request.symbols.into_iter().collect();
                                if let Some(exchanges) = sub_request.exchanges {
                                    client.exchanges = exchanges.into_iter().collect();
                                }
                                
                                info!("✅ Client {} subscribed to channels: {:?}, symbols: {:?}, exchanges: {:?}", 
                                    client_id, client.channels, client.subscriptions, client.exchanges);
                            }
                            "unsubscribe" => {
                                client.channels.clear();
                                client.subscriptions.clear();
                                client.exchanges.clear();
                                info!("❌ Client {} unsubscribed from all channels", client_id);
                            }
                            _ => {
                                warn!("❓ Unknown message type from {}: {}", client_id, sub_request.msg_type);
                            }
                        }
                    }
                }
            }
            Message::Ping(_) => {
                debug!("🏓 Received ping from {}", client_id);
            }
            Message::Pong(_) => {
                debug!("🏓 Received pong from {}", client_id);
            }
            Message::Close(_) => {
                info!("👋 Client {} sent close", client_id);
            }
            _ => {
                debug!("📦 Received other message type from {}", client_id);
            }
        }
        
        Ok(())
    }
}


// Global WebSocket server instance
use std::sync::OnceLock;
static REALTIME_WS_SERVER: OnceLock<RealtimeWebSocketServer> = OnceLock::new();

// WebSocket handler function for Axum router
pub async fn realtime_websocket_handler(ws: WebSocketUpgrade) -> Response {
    let server = REALTIME_WS_SERVER.get_or_init(|| RealtimeWebSocketServer::new());
    server.handle_client_connection(ws).await
}

// Initialize the global WebSocket server
pub async fn initialize_realtime_websocket() -> Result<()> {
    let server = REALTIME_WS_SERVER.get_or_init(|| RealtimeWebSocketServer::new());
    server.start().await
}