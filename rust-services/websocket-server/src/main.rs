// WebSocket server with shared memory reader for ultra-low latency
use alphapulse_common::{
    SharedMemoryReader, SharedTrade, Trade, OrderBookUpdate, MetricsCollector,
    shared_memory::{OrderBookDeltaReader, SharedOrderBookDelta}
};
use axum::{
    extract::ws::{WebSocket, WebSocketUpgrade},
    extract::State,
    response::Response,
    routing::get,
    Router,
};
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::time;
use tower_http::cors::CorsLayer;
use tracing::{info, warn};

#[derive(Clone)]
struct AppState {
    trade_tx: broadcast::Sender<Trade>,
    orderbook_tx: broadcast::Sender<OrderBookUpdate>,
    delta_tx: broadcast::Sender<OrderBookDelta>,
    metrics: Arc<MetricsCollector>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct OrderBookDelta {
    pub symbol: String,
    pub exchange: String,
    pub version: u64,
    pub prev_version: u64,
    pub timestamp_ns: u64,
    pub changes: Vec<PriceChange>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct PriceChange {
    pub price: f64,
    pub volume: f64,
    pub side: String, // "bid" or "ask"
    pub action: String, // "update" or "remove"
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env()
            .add_directive(tracing::Level::INFO.into()))
        .init();
    
    // Create broadcast channels for trades, orderbooks, and deltas
    let (trade_tx, _) = broadcast::channel(10000);
    let (orderbook_tx, _) = broadcast::channel(10000);
    let (delta_tx, _) = broadcast::channel(10000);
    
    let metrics = Arc::new(MetricsCollector::new());
    
    let state = AppState {
        trade_tx: trade_tx.clone(),
        orderbook_tx: orderbook_tx.clone(),
        delta_tx: delta_tx.clone(),
        metrics: metrics.clone(),
    };
    
    // Start shared memory reader tasks
    tokio::spawn(shared_memory_reader(trade_tx.clone(), metrics.clone()));
    
    // Start multi-exchange delta readers
    tokio::spawn(coinbase_delta_reader(delta_tx.clone(), metrics.clone()));
    tokio::spawn(kraken_delta_reader(delta_tx.clone(), metrics.clone()));
    tokio::spawn(binance_delta_reader(delta_tx.clone(), metrics.clone()));
    
    // Start WebSocket server
    let app = Router::new()
        .route("/ws", get(websocket_handler))
        .layer(CorsLayer::permissive())
        .with_state(state);
    
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8765").await?;
    info!("WebSocket server listening on ws://0.0.0.0:8765");
    
    axum::serve(listener, app).await?;
    
    Ok(())
}

async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();
    
    // Subscribe to broadcasts
    let mut trade_rx = state.trade_tx.subscribe();
    let mut orderbook_rx = state.orderbook_tx.subscribe();
    let mut delta_rx = state.delta_tx.subscribe();
    
    info!("New WebSocket client connected");
    state.metrics.record_websocket_connection_status("websocket-server", true);
    
    // Spawn task to send data to client
    let sender_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                Ok(trade) = trade_rx.recv() => {
                    let msg = json!({
                        "type": "trade",
                        "data": trade
                    });
                    
                    if sender.send(axum::extract::ws::Message::Text(msg.to_string())).await.is_err() {
                        break;
                    }
                }
                Ok(orderbook) = orderbook_rx.recv() => {
                    let msg = json!({
                        "type": "orderbook",
                        "data": orderbook
                    });
                    
                    if sender.send(axum::extract::ws::Message::Text(msg.to_string())).await.is_err() {
                        break;
                    }
                }
                Ok(delta) = delta_rx.recv() => {
                    let msg = json!({
                        "type": "orderbook_delta",
                        "data": delta
                    });
                    
                    if sender.send(axum::extract::ws::Message::Text(msg.to_string())).await.is_err() {
                        break;
                    }
                }
            }
        }
    });
    
    // Handle incoming messages (subscriptions, etc.)
    while let Some(msg) = receiver.next().await {
        if let Ok(msg) = msg {
            match msg {
                axum::extract::ws::Message::Text(text) => {
                    // Handle subscription messages
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&text) {
                        if parsed["type"] == "subscribe" {
                            info!("Client subscribed to: {:?}", parsed["channels"]);
                        }
                    }
                }
                axum::extract::ws::Message::Close(_) => break,
                _ => {}
            }
        }
    }
    
    // Cleanup
    sender_task.abort();
    info!("WebSocket client disconnected");
    state.metrics.record_websocket_connection_status("websocket-server", false);
}

async fn shared_memory_reader(
    trade_tx: broadcast::Sender<Trade>,
    metrics: Arc<MetricsCollector>,
) {
    info!("Starting shared memory reader task");
    
    // Try to connect to shared memory
    let shared_mem_path = if cfg!(target_os = "macos") {
        "/tmp/alphapulse_shm/trades"
    } else {
        "/dev/shm/alphapulse_trades"
    };
    
    let reader_id = 0; // WebSocket server uses reader ID 0
    
    loop {
        match SharedMemoryReader::open(shared_mem_path, reader_id) {
            Ok(mut reader) => {
                info!("Connected to shared memory at {}", shared_mem_path);
                
                // Read from shared memory in a tight loop
                loop {
                    // Read batch of trades
                    let trades = reader.read_trades();
                    
                    if trades.is_empty() {
                        // No new data, yield CPU briefly
                        tokio::time::sleep(Duration::from_micros(100)).await;
                        continue;
                    }
                    
                    // Process all new trades
                    for shared_trade in trades {
                        // Convert SharedTrade to Trade
                        let trade = shared_trade_to_trade(&shared_trade);
                        
                        // Broadcast to WebSocket clients
                        if let Err(e) = trade_tx.send(trade.clone()) {
                            if trade_tx.receiver_count() == 0 {
                                // No subscribers, this is fine
                                tokio::time::sleep(Duration::from_millis(10)).await;
                            } else {
                                warn!("Failed to broadcast trade: {}", e);
                            }
                        }
                        
                        // Record metrics
                        metrics.record_trade(&trade.exchange, &trade.symbol);
                        metrics.record_latency(
                            (chrono::Utc::now().timestamp_nanos() as u64 - shared_trade.timestamp_ns) as f64 / 1_000_000.0,
                            "shared_memory_read"
                        );
                    }
                    
                    // Check lag periodically
                    let lag = reader.get_lag();
                    if lag > 1000 {
                        warn!("Shared memory reader lagging: {} messages behind", lag);
                    }
                }
            }
            Err(e) => {
                warn!("Failed to open shared memory at {}: {}. Retrying in 5 seconds...", shared_mem_path, e);
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }
}

fn shared_trade_to_trade(shared_trade: &SharedTrade) -> Trade {
    // Convert fixed-size arrays to strings
    let symbol = String::from_utf8_lossy(&shared_trade.symbol)
        .trim_end_matches('\0')
        .to_string();
    let exchange = String::from_utf8_lossy(&shared_trade.exchange)
        .trim_end_matches('\0')
        .to_string();
    let trade_id = String::from_utf8_lossy(&shared_trade.trade_id)
        .trim_end_matches('\0')
        .to_string();
    
    Trade {
        timestamp: shared_trade.timestamp_ns as f64 / 1_000_000_000.0,
        symbol,
        exchange,
        price: shared_trade.price,
        volume: shared_trade.volume,
        side: match shared_trade.side {
            0 => Some("buy".to_string()),
            1 => Some("sell".to_string()),
            _ => None,
        },
        trade_id: if trade_id.is_empty() { None } else { Some(trade_id) },
    }
}

async fn coinbase_delta_reader(
    delta_tx: broadcast::Sender<OrderBookDelta>,
    metrics: Arc<MetricsCollector>,
) {
    info!("Starting Coinbase orderbook delta shared memory reader task");
    
    // Try to connect to Coinbase delta shared memory
    let shared_mem_path = if cfg!(target_os = "macos") {
        "/tmp/alphapulse_shm/orderbook_deltas"
    } else {
        "/dev/shm/alphapulse_orderbook_deltas"
    };
    
    let reader_id = 1; // WebSocket server uses reader ID 1 for Coinbase deltas
    
    loop {
        match OrderBookDeltaReader::open(shared_mem_path, reader_id) {
            Ok(mut reader) => {
                info!("Connected to Coinbase delta shared memory at {}", shared_mem_path);
                
                // Read from shared memory in a tight loop
                loop {
                    // Read batch of deltas
                    let deltas = reader.read_deltas();
                    
                    if deltas.is_empty() {
                        // No new data, yield CPU briefly
                        tokio::time::sleep(Duration::from_micros(100)).await;
                        continue;
                    }
                    
                    // Process all new deltas
                    for shared_delta in deltas {
                        // Convert SharedOrderBookDelta to OrderBookDelta
                        let delta = shared_delta_to_delta(&shared_delta);
                        
                        // Broadcast to WebSocket clients
                        if let Err(e) = delta_tx.send(delta.clone()) {
                            if delta_tx.receiver_count() == 0 {
                                // No subscribers, this is fine
                                tokio::time::sleep(Duration::from_millis(10)).await;
                            } else {
                                warn!("Failed to broadcast delta: {}", e);
                            }
                        }
                        
                        // Record metrics
                        metrics.record_latency(
                            (chrono::Utc::now().timestamp_nanos() as u64 - shared_delta.timestamp_ns) as f64 / 1_000_000.0,
                            "shared_memory_delta_read"
                        );
                    }
                    
                    // Check lag periodically
                    let lag = reader.get_lag();
                    if lag > 1000 {
                        warn!("Delta shared memory reader lagging: {} messages behind", lag);
                    }
                }
            }
            Err(e) => {
                warn!("Failed to open delta shared memory at {}: {}. Retrying in 5 seconds...", shared_mem_path, e);
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }
}

fn shared_delta_to_delta(shared_delta: &SharedOrderBookDelta) -> OrderBookDelta {
    // Convert fixed-size arrays to strings
    let symbol = String::from_utf8_lossy(&shared_delta.symbol)
        .trim_end_matches('\0')
        .to_string();
    let exchange = String::from_utf8_lossy(&shared_delta.exchange)
        .trim_end_matches('\0')
        .to_string();
    
    // Convert price level changes
    let mut changes = Vec::new();
    for i in 0..shared_delta.change_count as usize {
        if i >= shared_delta.changes.len() {
            break;
        }
        
        let change = &shared_delta.changes[i];
        let is_ask = (change.side_and_action & 0x80) != 0;
        let side = if is_ask { "ask" } else { "bid" };
        let action = if change.volume == 0.0 { "remove" } else { "update" };
        
        changes.push(PriceChange {
            price: change.price as f64,
            volume: change.volume as f64,
            side: side.to_string(),
            action: action.to_string(),
        });
    }
    
    OrderBookDelta {
        symbol,
        exchange,
        version: shared_delta.version,
        prev_version: shared_delta.prev_version,
        timestamp_ns: shared_delta.timestamp_ns,
        changes,
    }
}

async fn kraken_delta_reader(
    delta_tx: broadcast::Sender<OrderBookDelta>,
    metrics: Arc<MetricsCollector>,
) {
    info!("Starting Kraken orderbook delta shared memory reader task");
    
    // Try to connect to Kraken delta shared memory
    let shared_mem_path = if cfg!(target_os = "macos") {
        "/tmp/alphapulse_shm/kraken_orderbook_deltas"
    } else {
        "/dev/shm/alphapulse_kraken_orderbook_deltas"
    };
    
    let reader_id = 2; // WebSocket server uses reader ID 2 for Kraken deltas
    
    loop {
        match OrderBookDeltaReader::open(shared_mem_path, reader_id) {
            Ok(mut reader) => {
                info!("Connected to Kraken delta shared memory at {}", shared_mem_path);
                
                // Read from shared memory in a tight loop
                loop {
                    // Read batch of deltas
                    let deltas = reader.read_deltas();
                    
                    if deltas.is_empty() {
                        // No new data, yield CPU briefly
                        tokio::time::sleep(Duration::from_micros(100)).await;
                        continue;
                    }
                    
                    // Process all new deltas
                    for shared_delta in deltas {
                        // Convert SharedOrderBookDelta to OrderBookDelta
                        let delta = shared_delta_to_delta(&shared_delta);
                        
                        // Broadcast to WebSocket clients
                        if let Err(e) = delta_tx.send(delta.clone()) {
                            if delta_tx.receiver_count() == 0 {
                                // No subscribers, this is fine
                                tokio::time::sleep(Duration::from_millis(10)).await;
                            } else {
                                warn!("Failed to broadcast Kraken delta: {}", e);
                            }
                        }
                        
                        // Record metrics
                        metrics.record_latency(
                            (chrono::Utc::now().timestamp_nanos() as u64 - shared_delta.timestamp_ns) as f64 / 1_000_000.0,
                            "kraken_delta_read"
                        );
                    }
                    
                    // Check lag periodically
                    let lag = reader.get_lag();
                    if lag > 1000 {
                        warn!("Kraken delta reader lagging: {} messages behind", lag);
                    }
                }
            }
            Err(e) => {
                warn!("Failed to open Kraken delta shared memory at {}: {}. Retrying in 5 seconds...", shared_mem_path, e);
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }
}

async fn binance_delta_reader(
    delta_tx: broadcast::Sender<OrderBookDelta>,
    metrics: Arc<MetricsCollector>,
) {
    info!("Starting Binance.US orderbook delta shared memory reader task");
    
    // Try to connect to Binance delta shared memory
    let shared_mem_path = if cfg!(target_os = "macos") {
        "/tmp/alphapulse_shm/binance_orderbook_deltas"
    } else {
        "/dev/shm/alphapulse_binance_orderbook_deltas"
    };
    
    let reader_id = 3; // WebSocket server uses reader ID 3 for Binance deltas
    
    loop {
        match OrderBookDeltaReader::open(shared_mem_path, reader_id) {
            Ok(mut reader) => {
                info!("Connected to Binance.US delta shared memory at {}", shared_mem_path);
                
                // Read from shared memory in a tight loop
                loop {
                    // Read batch of deltas
                    let deltas = reader.read_deltas();
                    
                    if deltas.is_empty() {
                        // No new data, yield CPU briefly
                        tokio::time::sleep(Duration::from_micros(100)).await;
                        continue;
                    }
                    
                    // Process all new deltas
                    for shared_delta in deltas {
                        // Convert SharedOrderBookDelta to OrderBookDelta
                        let delta = shared_delta_to_delta(&shared_delta);
                        
                        // Broadcast to WebSocket clients
                        if let Err(e) = delta_tx.send(delta.clone()) {
                            if delta_tx.receiver_count() == 0 {
                                // No subscribers, this is fine
                                tokio::time::sleep(Duration::from_millis(10)).await;
                            } else {
                                warn!("Failed to broadcast Binance.US delta: {}", e);
                            }
                        }
                        
                        // Record metrics
                        metrics.record_latency(
                            (chrono::Utc::now().timestamp_nanos() as u64 - shared_delta.timestamp_ns) as f64 / 1_000_000.0,
                            "binance_delta_read"
                        );
                    }
                    
                    // Check lag periodically
                    let lag = reader.get_lag();
                    if lag > 1000 {
                        warn!("Binance.US delta reader lagging: {} messages behind", lag);
                    }
                }
            }
            Err(e) => {
                warn!("Failed to open Binance.US delta shared memory at {}: {}. Retrying in 5 seconds...", shared_mem_path, e);
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }
}