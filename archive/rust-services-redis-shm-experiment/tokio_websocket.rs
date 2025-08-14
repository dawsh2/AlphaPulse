// Tokio-based WebSocket server for event-driven real-time data streaming
// Uses TokioTransport for zero-polling, high-performance data distribution

use alphapulse_common::{
    tokio_transport::{TokioTransport, get_global_transport},
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
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::{broadcast, RwLock};
use tracing::{info, warn, error};
use uuid::Uuid;

// WebSocket messages
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum WsMessage {
    Trade(Trade),
    Stats(StatsMessage),
    Subscribe(SubscribeMessage),
    SystemStats(SystemStats),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StatsMessage {
    pub trades_per_sec: f64,
    pub latency_us: f64,
    pub active_clients: usize,
    pub uptime_seconds: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SubscribeMessage {
    pub symbols: Vec<String>,
    pub exchanges: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStats {
    pub timestamp: f64,
    pub memory_used_bytes: u64,
    pub cpu_percent: f32,
    pub active_connections: usize,
    pub trades_per_sec: f64,
    pub messages_queued: usize,
}

// Main WebSocket server using TokioTransport
pub struct TokioWebSocketServer {
    trade_broadcaster: broadcast::Sender<Trade>,
    stats_broadcaster: broadcast::Sender<SystemStats>,
    clients: Arc<RwLock<HashMap<Uuid, ClientPreferences>>>,
    metrics: Arc<RwLock<PerformanceMetrics>>,
    transport: Option<TokioTransport>,
}

#[derive(Debug, Clone, Default)]
struct ClientPreferences {
    pub symbols: Vec<String>,
    pub exchanges: Vec<String>,
}

#[derive(Debug, Clone, Default)]
struct PerformanceMetrics {
    pub trades_processed: u64,
    pub messages_sent: u64,
    pub avg_latency_us: f64,
    pub start_time: Option<Instant>,
}

// Global server instance
static WEBSOCKET_SERVER: std::sync::OnceLock<Arc<RwLock<TokioWebSocketServer>>> = std::sync::OnceLock::new();

impl TokioWebSocketServer {
    pub fn new() -> Result<Self> {
        info!("🚀 Creating Tokio WebSocket server");
        
        // Create broadcast channels
        let (trade_broadcaster, _) = broadcast::channel(10000);
        let (stats_broadcaster, _) = broadcast::channel(100);
        
        // Initialize or get global transport
        use alphapulse_common::tokio_transport::init_global_transport;
        
        let transport = if let Some(t) = get_global_transport() {
            info!("✅ Using existing global TokioTransport");
            Some(t.clone())
        } else {
            info!("🚀 Initializing global TokioTransport for API server");
            Some(init_global_transport(10000).clone())
        };
        
        Ok(Self {
            trade_broadcaster,
            stats_broadcaster,
            clients: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(PerformanceMetrics::default())),
            transport,
        })
    }
    
    pub async fn start(&mut self) -> Result<()> {
        info!("🔥 Starting Tokio WebSocket server...");
        
        // Initialize metrics
        {
            let mut metrics = self.metrics.write().await;
            metrics.start_time = Some(Instant::now());
        }
        
        // Start the main reader task if transport is available
        if let Some(transport) = &self.transport {
            let trade_broadcaster = self.trade_broadcaster.clone();
            let metrics = self.metrics.clone();
            let transport_clone = transport.clone();
            
            tokio::spawn(async move {
                info!("🎯 Starting TokioTransport reader task (TRUE event-driven!)");
                
                loop {
                    // This blocks until data is available - TRUE event-driven!
                    let trades = transport_clone.read_batch().await;
                    
                    if !trades.is_empty() {
                        info!("📊 Broadcasting {} trades from TokioTransport", trades.len());
                        
                        for trade in trades {
                            if let Err(_) = trade_broadcaster.send(trade) {
                                // No receivers
                            }
                            
                            let mut metrics_guard = metrics.write().await;
                            metrics_guard.trades_processed += 1;
                        }
                    }
                }
            });
        }
        
        // Start stats reporter
        let stats_broadcaster = self.stats_broadcaster.clone();
        let metrics = self.metrics.clone();
        let clients = self.clients.clone();
        
        tokio::spawn(Self::run_stats_reporter(stats_broadcaster, metrics, clients));
        
        info!("✅ Tokio WebSocket server started successfully");
        Ok(())
    }
    
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
            
            let uptime = metrics_guard.start_time
                .map(|t| t.elapsed().as_secs_f64())
                .unwrap_or(0.0);
            
            let trades_per_sec = if uptime > 0.0 {
                metrics_guard.trades_processed as f64 / uptime
            } else {
                0.0
            };
            
            let stats = SystemStats {
                timestamp: chrono::Utc::now().timestamp() as f64,
                memory_used_bytes: 0, // TODO: Implement memory tracking
                cpu_percent: 0.0, // TODO: Implement CPU tracking
                active_connections: clients_guard.len(),
                trades_per_sec,
                messages_queued: 0, // TODO: Track queue size
            };
            
            if let Err(_) = broadcaster.send(stats) {
                // No receivers
            }
        }
    }
}

// Initialize global WebSocket server
pub async fn initialize_tokio_websocket() -> Result<()> {
    let mut server = TokioWebSocketServer::new()?;
    server.start().await?;
    
    WEBSOCKET_SERVER.set(Arc::new(RwLock::new(server)))
        .map_err(|_| AlphaPulseError::ConfigError("Failed to initialize WebSocket server".to_string()))?;
        
    Ok(())
}

// WebSocket handler
pub async fn tokio_websocket_handler(ws: WebSocketUpgrade) -> Response {
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
        let mut server_guard = server.write().await;
        server_guard.clients.write().await.insert(client_id, ClientPreferences::default());
    }
    
    // Subscribe to broadcasts
    let server_guard = server.read().await;
    let mut trade_rx = server_guard.trade_broadcaster.subscribe();
    let mut stats_rx = server_guard.stats_broadcaster.subscribe();
    drop(server_guard);
    
    // Send initial connection message
    let connect_msg = serde_json::json!({
        "type": "connected",
        "data": {
            "client_id": client_id.to_string(),
            "timestamp": chrono::Utc::now().timestamp(),
        }
    });
    
    if let Err(e) = socket.send(Message::Text(connect_msg.to_string())).await {
        error!("Failed to send connection message: {}", e);
        return;
    }
    
    // Main message loop
    loop {
        tokio::select! {
            // Handle incoming messages from client
            Some(msg) = socket.next() => {
                match msg {
                    Ok(Message::Text(text)) => {
                        if let Ok(ws_msg) = serde_json::from_str::<WsMessage>(&text) {
                            match ws_msg {
                                WsMessage::Subscribe(sub) => {
                                    info!("Client {} subscribing to: {:?} {:?}", 
                                          client_id, sub.symbols, sub.exchanges);
                                    
                                    let server_guard = server.read().await;
                                    let mut clients = server_guard.clients.write().await;
                                    if let Some(prefs) = clients.get_mut(&client_id) {
                                        prefs.symbols = sub.symbols;
                                        prefs.exchanges = sub.exchanges;
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    Ok(Message::Close(_)) => {
                        info!("Client {} disconnected", client_id);
                        break;
                    }
                    Err(e) => {
                        error!("WebSocket error for client {}: {}", client_id, e);
                        break;
                    }
                    _ => {}
                }
            }
            
            // Handle trade broadcasts
            Ok(trade) = trade_rx.recv() => {
                let msg = WsMessage::Trade(trade);
                let json = serde_json::to_string(&msg).unwrap();
                
                if let Err(e) = socket.send(Message::Text(json)).await {
                    error!("Failed to send trade to client {}: {}", client_id, e);
                    break;
                }
            }
            
            // Handle stats broadcasts
            Ok(stats) = stats_rx.recv() => {
                let msg = WsMessage::SystemStats(stats);
                let json = serde_json::to_string(&msg).unwrap();
                
                if let Err(e) = socket.send(Message::Text(json)).await {
                    error!("Failed to send stats to client {}: {}", client_id, e);
                    break;
                }
            }
        }
    }
    
    // Clean up client
    let server_guard = server.read().await;
    server_guard.clients.write().await.remove(&client_id);
    info!("Client {} cleaned up", client_id);
}