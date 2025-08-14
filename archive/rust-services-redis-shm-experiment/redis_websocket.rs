// Redis Streams-based WebSocket server for real-time data
// Uses XREAD BLOCK for event-driven, zero-polling data flow

use alphapulse_common::{Trade, Result, AlphaPulseError};
use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::Response,
};
use futures_util::StreamExt;
use redis::{aio::MultiplexedConnection, RedisResult};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::Arc,
    time::Duration,
};
use tokio::sync::{broadcast, RwLock};
use tracing::{info, warn, error};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum WsMessage {
    Trade(Trade),
    Stats(SystemStats),
    Connected { client_id: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStats {
    pub timestamp: f64,
    pub trades_per_sec: f64,
    pub active_connections: usize,
}

pub struct RedisWebSocketServer {
    trade_broadcaster: broadcast::Sender<Trade>,
    stats_broadcaster: broadcast::Sender<SystemStats>,
    clients: Arc<RwLock<HashMap<Uuid, ()>>>,
    redis_url: String,
}

static WEBSOCKET_SERVER: std::sync::OnceLock<Arc<RwLock<RedisWebSocketServer>>> = std::sync::OnceLock::new();

impl RedisWebSocketServer {
    pub fn new(redis_url: String) -> Result<Self> {
        info!("🚀 Creating Redis WebSocket server");
        
        let (trade_broadcaster, _) = broadcast::channel(10000);
        let (stats_broadcaster, _) = broadcast::channel(100);
        
        Ok(Self {
            trade_broadcaster,
            stats_broadcaster,
            clients: Arc::new(RwLock::new(HashMap::new())),
            redis_url,
        })
    }
    
    pub async fn start(&self) -> Result<()> {
        info!("🔥 Starting Redis WebSocket server...");
        
        // Start Redis stream reader (event-driven with XREAD BLOCK!)
        let trade_broadcaster = self.trade_broadcaster.clone();
        let redis_url = self.redis_url.clone();
        
        tokio::spawn(async move {
            if let Err(e) = run_redis_reader(redis_url, trade_broadcaster).await {
                error!("Redis reader failed: {}", e);
            }
        });
        
        // Start stats reporter
        let stats_broadcaster = self.stats_broadcaster.clone();
        let clients = self.clients.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            let mut trade_count = 0u64;
            let start_time = std::time::Instant::now();
            
            loop {
                interval.tick().await;
                
                let elapsed = start_time.elapsed().as_secs_f64();
                let trades_per_sec = if elapsed > 0.0 {
                    trade_count as f64 / elapsed
                } else {
                    0.0
                };
                
                let stats = SystemStats {
                    timestamp: chrono::Utc::now().timestamp() as f64,
                    trades_per_sec,
                    active_connections: clients.read().await.len(),
                };
                
                if let Err(_) = stats_broadcaster.send(stats) {
                    // No receivers
                }
            }
        });
        
        info!("✅ Redis WebSocket server started successfully");
        Ok(())
    }
}

async fn run_redis_reader(
    redis_url: String,
    broadcaster: broadcast::Sender<Trade>,
) -> Result<()> {
    info!("🎯 Starting Redis stream reader (EVENT-DRIVEN with XREAD BLOCK!)");
    
    let client = redis::Client::open(redis_url)?;
    let mut con = client.get_multiplexed_async_connection().await?;
    
    let mut last_id = "$".to_string(); // Start with latest messages
    
    loop {
        // XREAD BLOCK - this blocks until new data arrives (TRUE event-driven!)
        let result: RedisResult<redis::Value> = 
            redis::cmd("XREAD")
                .arg("BLOCK")
                .arg(0) // Block indefinitely until data arrives
                .arg("STREAMS")
                .arg("trades:stream")
                .arg(&last_id)
                .query_async(&mut con)
                .await;
        
        match result {
            Ok(redis::Value::Bulk(streams)) => {
                // Parse XREAD response structure
                for stream in streams {
                    if let redis::Value::Bulk(stream_data) = stream {
                        if stream_data.len() >= 2 {
                            // stream_data[0] is stream name, stream_data[1] is entries
                            if let redis::Value::Bulk(entries) = &stream_data[1] {
                                for entry in entries {
                                    if let redis::Value::Bulk(entry_data) = entry {
                                        if entry_data.len() >= 2 {
                                            // entry_data[0] is ID, entry_data[1] is fields
                                            if let redis::Value::Data(id_bytes) = &entry_data[0] {
                                                last_id = String::from_utf8_lossy(id_bytes).to_string();
                                            }
                                            
                                            if let redis::Value::Bulk(fields) = &entry_data[1] {
                                                // Parse fields into HashMap
                                                let mut data = HashMap::new();
                                                let mut i = 0;
                                                while i + 1 < fields.len() {
                                                    if let (redis::Value::Data(key), redis::Value::Data(val)) = (&fields[i], &fields[i + 1]) {
                                                        data.insert(
                                                            String::from_utf8_lossy(key).to_string(),
                                                            String::from_utf8_lossy(val).to_string()
                                                        );
                                                    }
                                                    i += 2;
                                                }
                                                
                                                // Parse trade from the data
                                                if let Some(trade) = parse_trade_from_redis(data) {
                                                    info!("📊 Broadcasting trade: {} @ ${}", trade.symbol, trade.price);
                                                    
                                                    if let Err(_) = broadcaster.send(trade) {
                                                        // No receivers
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Ok(_) => {
                warn!("Unexpected Redis response format");
            }
            Err(e) => {
                warn!("Redis XREAD error: {}, retrying...", e);
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }
}

fn parse_trade_from_redis(data: HashMap<String, String>) -> Option<Trade> {
    Some(Trade {
        timestamp: data.get("timestamp")?.parse().ok()?,
        symbol: data.get("symbol")?.clone(),
        exchange: data.get("exchange")?.clone(),
        price: data.get("price")?.parse().ok()?,
        volume: data.get("volume")?.parse().ok()?,
        side: data.get("side").cloned(),
        trade_id: data.get("trade_id").cloned(),
    })
}

pub async fn initialize_redis_websocket(redis_url: String) -> Result<()> {
    let server = RedisWebSocketServer::new(redis_url)?;
    server.start().await?;
    
    WEBSOCKET_SERVER.set(Arc::new(RwLock::new(server)))
        .map_err(|_| AlphaPulseError::ConfigError("Failed to initialize WebSocket server".to_string()))?;
        
    Ok(())
}

pub async fn redis_websocket_handler(ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    let client_id = Uuid::new_v4();
    info!("🔌 New WebSocket connection: {}", client_id);
    
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
        server_guard.clients.write().await.insert(client_id, ());
    }
    
    // Subscribe to broadcasts
    let server_guard = server.read().await;
    let mut trade_rx = server_guard.trade_broadcaster.subscribe();
    let mut stats_rx = server_guard.stats_broadcaster.subscribe();
    drop(server_guard);
    
    // Send connection message
    let connect_msg = WsMessage::Connected {
        client_id: client_id.to_string(),
    };
    
    if let Ok(json) = serde_json::to_string(&connect_msg) {
        if let Err(e) = socket.send(Message::Text(json)).await {
            error!("Failed to send connection message: {}", e);
            return;
        }
    }
    
    // Main message loop
    loop {
        tokio::select! {
            // Handle incoming messages from client
            Some(msg) = socket.next() => {
                match msg {
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
                if let Ok(json) = serde_json::to_string(&msg) {
                    if let Err(e) = socket.send(Message::Text(json)).await {
                        error!("Failed to send trade to client {}: {}", client_id, e);
                        break;
                    }
                }
            }
            
            // Handle stats broadcasts
            Ok(stats) = stats_rx.recv() => {
                let msg = WsMessage::Stats(stats);
                if let Ok(json) = serde_json::to_string(&msg) {
                    if let Err(e) = socket.send(Message::Text(json)).await {
                        error!("Failed to send stats to client {}: {}", client_id, e);
                        break;
                    }
                }
            }
        }
    }
    
    // Clean up
    let server_guard = server.read().await;
    server_guard.clients.write().await.remove(&client_id);
    info!("Client {} cleaned up", client_id);
}