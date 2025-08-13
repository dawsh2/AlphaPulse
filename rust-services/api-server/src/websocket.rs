// WebSocket server for real-time data streaming to frontend
use alphapulse_common::{Result, Trade};
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
};
use futures_util::{SinkExt, StreamExt};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error, debug};

#[derive(Debug, Serialize, Deserialize)]
pub struct SubscriptionRequest {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub channels: Vec<String>,
    pub symbols: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MarketDataUpdate {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub channel: String,
    pub symbol: String,
    pub data: serde_json::Value,
    pub timestamp: f64,
}

pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<crate::state::AppState>,
) -> Response {
    info!("New WebSocket connection request");
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: crate::state::AppState) {
    let (mut sender, mut receiver) = socket.split();
    
    // Client info
    let client_id = uuid::Uuid::new_v4().to_string();
    info!("WebSocket client connected: {}", client_id);
    
    // Track subscriptions
    let subscriptions = Arc::new(RwLock::new(ClientSubscriptions::default()));
    let subs_clone = subscriptions.clone();
    
    // Start real-time data pusher
    let state_clone = state.clone();
    let send_task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(100));
        
        loop {
            interval.tick().await;
            
            let subs = subs_clone.read().await;
            if subs.symbols.is_empty() {
                continue;
            }
            
            // Send trades if subscribed
            if subs.channels.contains(&"trades".to_string()) {
                for symbol in &subs.symbols {
                    if let Ok(trades) = get_recent_trades(&state_clone, symbol, 5).await {
                        for trade in trades {
                            let update = MarketDataUpdate {
                                msg_type: "trade".to_string(),
                                channel: "trades".to_string(),
                                symbol: symbol.clone(),
                                data: json!(trade),
                                timestamp: trade.timestamp,
                            };
                            
                            if let Ok(json_str) = serde_json::to_string(&update) {
                                if sender.send(Message::Text(json_str)).await.is_err() {
                                    return; // Client disconnected
                                }
                            }
                        }
                    }
                }
            }
            
            // Send orderbook snapshots if subscribed
            if subs.channels.contains(&"orderbook".to_string()) {
                for symbol in &subs.symbols {
                    if let Ok(orderbook) = get_orderbook(&state_clone, symbol).await {
                        let update = MarketDataUpdate {
                            msg_type: "orderbook".to_string(),
                            channel: "orderbook".to_string(),
                            symbol: symbol.clone(),
                            data: orderbook,
                            timestamp: chrono::Utc::now().timestamp() as f64,
                        };
                        
                        if let Ok(json_str) = serde_json::to_string(&update) {
                            if sender.send(Message::Text(json_str)).await.is_err() {
                                return; // Client disconnected
                            }
                        }
                    }
                }
            }
        }
    });
    
    // Handle incoming messages
    while let Some(Ok(msg)) = receiver.next().await {
        if let Err(e) = handle_client_message(msg, &client_id, &subscriptions).await {
            error!("Error handling client message: {}", e);
        }
    }
    
    // Cleanup
    send_task.abort();
    info!("WebSocket client disconnected: {}", client_id);
}

#[derive(Default)]
struct ClientSubscriptions {
    channels: Vec<String>,
    symbols: Vec<String>,
}

async fn handle_client_message(
    msg: Message,
    client_id: &str,
    subscriptions: &Arc<RwLock<ClientSubscriptions>>,
) -> Result<()> {
    match msg {
        Message::Text(text) => {
            debug!("Received from {}: {}", client_id, text);
            
            // Parse subscription request
            if let Ok(sub_request) = serde_json::from_str::<SubscriptionRequest>(&text) {
                match sub_request.msg_type.as_str() {
                    "subscribe" => {
                        let mut subs = subscriptions.write().await;
                        subs.channels = sub_request.channels;
                        subs.symbols = sub_request.symbols;
                        
                        info!("Client {} subscribed to channels: {:?}, symbols: {:?}", 
                            client_id, subs.channels, subs.symbols);
                    }
                    "unsubscribe" => {
                        let mut subs = subscriptions.write().await;
                        subs.channels.clear();
                        subs.symbols.clear();
                        info!("Client {} unsubscribed from all channels", client_id);
                    }
                    _ => {
                        warn!("Unknown message type from {}: {}", client_id, sub_request.msg_type);
                    }
                }
            }
        }
        Message::Binary(_) => {
            debug!("Received binary message from {}", client_id);
        }
        Message::Ping(_) => {
            debug!("Received ping from {}", client_id);
        }
        Message::Pong(_) => {
            debug!("Received pong from {}", client_id);
        }
        Message::Close(_) => {
            info!("Client {} sent close", client_id);
        }
    }
    
    Ok(())
}

async fn get_recent_trades(
    state: &crate::state::AppState,
    symbol: &str,
    limit: usize
) -> Result<Vec<Trade>> {
    // Use the redis's get_recent_trades method
    state.redis.get_recent_trades(symbol, "coinbase", limit).await
}

async fn get_orderbook(
    state: &crate::state::AppState,
    symbol: &str,
) -> Result<serde_json::Value> {
    // Get orderbook from Redis
    let mut conn = state.redis.get_connection();
    let key = format!("orderbook:coinbase:{}", symbol);
    
    match conn.get::<_, Option<String>>(&key).await {
        Ok(Some(orderbook_json)) => {
            match serde_json::from_str::<serde_json::Value>(&orderbook_json) {
                Ok(orderbook) => Ok(orderbook),
                Err(e) => {
                    error!("Failed to parse orderbook JSON: {}", e);
                    Ok(json!({}))
                }
            }
        }
        Ok(None) => {
            debug!("No orderbook found for {}", symbol);
            Ok(json!({}))
        }
        Err(e) => {
            error!("Redis error getting orderbook: {}", e);
            Ok(json!({}))
        }
    }
}