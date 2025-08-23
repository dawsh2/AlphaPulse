// WebSocket Server for Dashboard Communication
// Provides real-time metrics streaming to frontend dashboard

use anyhow::{Result, Context};
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
    routing::get,
    Router,
};
use serde::{Serialize, Deserialize};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tokio::time::{self, Duration};
use tower_http::cors::CorsLayer;
use tracing::{info, warn, error, debug};

use crate::monitoring::metrics_broadcaster::{MetricsSnapshot, DashboardMessage, DashboardData};

/// WebSocket server configuration
#[derive(Debug, Clone)]
pub struct WebSocketConfig {
    pub host: String,
    pub port: u16,
    pub max_connections: usize,
    pub heartbeat_interval: Duration,
    pub metrics_broadcast_interval: Duration,
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            max_connections: 100,
            heartbeat_interval: Duration::from_secs(30),
            metrics_broadcast_interval: Duration::from_secs(5),
        }
    }
}

/// WebSocket server state
#[derive(Clone)]
struct ServerState {
    metrics_tx: broadcast::Sender<DashboardMessage>,
    connected_clients: Arc<RwLock<usize>>,
    latest_metrics: Arc<RwLock<Option<MetricsSnapshot>>>,
    config: WebSocketConfig,
}

/// WebSocket server for dashboard communication
pub struct WebSocketServer {
    config: WebSocketConfig,
    metrics_tx: broadcast::Sender<DashboardMessage>,
    metrics_rx: broadcast::Receiver<DashboardMessage>,
    state: ServerState,
}

impl WebSocketServer {
    pub fn new(config: WebSocketConfig) -> Self {
        let (metrics_tx, metrics_rx) = broadcast::channel(100);
        
        let state = ServerState {
            metrics_tx: metrics_tx.clone(),
            connected_clients: Arc::new(RwLock::new(0)),
            latest_metrics: Arc::new(RwLock::new(None)),
            config: config.clone(),
        };
        
        Self {
            config,
            metrics_tx,
            metrics_rx,
            state,
        }
    }

    /// Start the WebSocket server
    pub async fn start(self) -> Result<()> {
        let addr = format!("{}:{}", self.config.host, self.config.port)
            .parse::<SocketAddr>()
            .context("Invalid server address")?;
        
        info!("🌐 Starting WebSocket server on {}", addr);
        
        let app = self.create_router();
        
        let listener = tokio::net::TcpListener::bind(&addr).await
            .context("Failed to bind to address")?;
        
        axum::serve(listener, app)
            .await
            .context("Failed to start WebSocket server")?;
        
        Ok(())
    }

    /// Create the Axum router
    fn create_router(self) -> Router {
        Router::new()
            .route("/ws", get(websocket_handler))
            .route("/health", get(health_check))
            .route("/metrics", get(metrics_endpoint))
            .layer(CorsLayer::permissive())
            .with_state(self.state)
    }

    /// Broadcast metrics to all connected clients
    pub async fn broadcast_metrics(&self, metrics: MetricsSnapshot) -> Result<()> {
        let message = DashboardMessage {
            message_type: "metrics_update".to_string(),
            timestamp: chrono::Utc::now().timestamp_millis() as u64,
            data: DashboardData::Metrics(metrics.clone()),
        };
        
        // Update latest metrics
        *self.state.latest_metrics.write().await = Some(metrics);
        
        // Broadcast to all clients
        let subscribers = self.metrics_tx.receiver_count();
        if subscribers > 0 {
            self.metrics_tx.send(message)
                .context("Failed to broadcast metrics")?;
            debug!("📤 Broadcast metrics to {} clients", subscribers);
        }
        
        Ok(())
    }

    /// Get number of connected clients
    pub async fn connected_clients(&self) -> usize {
        *self.state.connected_clients.read().await
    }
}

/// WebSocket upgrade handler
async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<ServerState>,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

/// Handle individual WebSocket connection
async fn handle_socket(socket: WebSocket, state: ServerState) {
    // Increment client counter
    {
        let mut clients = state.connected_clients.write().await;
        *clients += 1;
        info!("👤 Client connected. Total clients: {}", *clients);
    }
    
    // Store socket for bidirectional communication
    let socket = Arc::new(RwLock::new(socket));
    
    // Subscribe to metrics broadcasts
    let mut metrics_rx = state.metrics_tx.subscribe();
    
    // Send initial metrics if available
    if let Some(metrics) = state.latest_metrics.read().await.clone() {
        let initial_msg = DashboardMessage {
            message_type: "initial_metrics".to_string(),
            timestamp: chrono::Utc::now().timestamp_millis() as u64,
            data: DashboardData::Metrics(metrics),
        };
        
        if let Ok(json) = serde_json::to_string(&initial_msg) {
            let socket_clone = socket.clone();
            tokio::spawn(async move {
                let mut socket = socket_clone.write().await;
                let _ = socket.send(Message::Text(json)).await;
            });
        }
    }
    
    // Spawn heartbeat task
    let socket_heartbeat = socket.clone();
    let heartbeat_handle = tokio::spawn(async move {
        let mut interval = time::interval(state.config.heartbeat_interval);
        loop {
            interval.tick().await;
            let mut socket = socket_heartbeat.write().await;
            if socket.send(Message::Ping(vec![])).await.is_err() {
                break;
            }
        }
    });
    
    // Spawn metrics broadcast task
    let socket_metrics = socket.clone();
    let metrics_handle = tokio::spawn(async move {
        loop {
            match metrics_rx.recv().await {
                Ok(message) => {
                    if let Ok(json) = serde_json::to_string(&message) {
                        let mut socket = socket_metrics.write().await;
                        if socket.send(Message::Text(json)).await.is_err() {
                            break;
                        }
                    }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    warn!("Client lagged by {} messages", n);
                }
                Err(broadcast::error::RecvError::Closed) => {
                    break;
                }
            }
        }
    });
    
    // Handle incoming messages
    let mut socket_guard = socket.write().await;
    while let Some(msg) = socket_guard.recv().await {
        match msg {
            Ok(Message::Text(text)) => {
                // Handle client commands
                if let Ok(command) = serde_json::from_str::<ClientCommand>(&text) {
                    handle_client_command(command, &socket, &state).await;
                }
            }
            Ok(Message::Pong(_)) => {
                debug!("Received pong from client");
            }
            Ok(Message::Close(_)) => {
                info!("Client disconnected");
                break;
            }
            Err(e) => {
                error!("WebSocket error: {}", e);
                break;
            }
            _ => {}
        }
    }
    
    // Cleanup
    heartbeat_handle.abort();
    metrics_handle.abort();
    
    // Decrement client counter
    {
        let mut clients = state.connected_clients.write().await;
        *clients = clients.saturating_sub(1);
        info!("👤 Client disconnected. Total clients: {}", *clients);
    }
}

/// Client command structure
#[derive(Debug, Deserialize)]
struct ClientCommand {
    command: String,
    params: Option<serde_json::Value>,
}

/// Handle commands from client
async fn handle_client_command(
    command: ClientCommand,
    socket: &Arc<RwLock<WebSocket>>,
    state: &ServerState,
) {
    match command.command.as_str() {
        "subscribe" => {
            // Client wants to subscribe to specific data streams
            if let Some(params) = command.params {
                debug!("Client subscribed to: {:?}", params);
            }
        }
        "get_metrics" => {
            // Send current metrics
            if let Some(metrics) = state.latest_metrics.read().await.clone() {
                let msg = DashboardMessage {
                    message_type: "metrics_response".to_string(),
                    timestamp: chrono::Utc::now().timestamp_millis() as u64,
                    data: DashboardData::Metrics(metrics),
                };
                
                if let Ok(json) = serde_json::to_string(&msg) {
                    let mut socket = socket.write().await;
                    let _ = socket.send(Message::Text(json)).await;
                }
            }
        }
        "ping" => {
            // Respond with pong
            let response = serde_json::json!({
                "type": "pong",
                "timestamp": chrono::Utc::now().timestamp_millis()
            });
            
            let mut socket = socket.write().await;
            let _ = socket.send(Message::Text(response.to_string())).await;
        }
        _ => {
            warn!("Unknown command: {}", command.command);
        }
    }
}

/// Health check endpoint
async fn health_check(State(state): State<ServerState>) -> String {
    let clients = *state.connected_clients.read().await;
    format!(
        r#"{{"status":"healthy","connected_clients":{}}}"#,
        clients
    )
}

/// Metrics endpoint for HTTP polling
async fn metrics_endpoint(State(state): State<ServerState>) -> String {
    if let Some(metrics) = state.latest_metrics.read().await.clone() {
        serde_json::to_string(&metrics).unwrap_or_else(|_| "{}".to_string())
    } else {
        "{}".to_string()
    }
}

/// Client-side TypeScript interface (for reference)
pub const TYPESCRIPT_INTERFACE: &str = r#"
// TypeScript interface for dashboard WebSocket communication

interface DashboardMessage {
    message_type: string;
    timestamp: number;
    data: MetricsSnapshot | AlertMessage | OpportunityUpdate;
}

interface MetricsSnapshot {
    timestamp: string;
    opportunities_per_minute: number;
    success_rate: number;
    average_profit_usd: number;
    total_profit_usd: number;
    average_latency_ms: number;
    p95_latency_ms: number;
    p99_latency_ms: number;
    gas_efficiency: number;
    cpu_usage_percent: number;
    memory_usage_mb: number;
    active_positions: number;
    pending_transactions: number;
    gas_price_gwei: number;
    network_congestion: 'Low' | 'Medium' | 'High' | 'Critical';
    mev_competition_level: number;
    gas_prediction_accuracy: number;
    slippage_prediction_accuracy: number;
    profit_prediction_accuracy: number;
    var_95: number;
    max_drawdown: number;
    sharpe_ratio: number;
}

// WebSocket connection example
class DashboardWebSocket {
    private ws: WebSocket;
    
    constructor(url: string = 'ws://localhost:8080/ws') {
        this.ws = new WebSocket(url);
        this.setupHandlers();
    }
    
    private setupHandlers() {
        this.ws.onopen = () => {
            console.log('Connected to metrics server');
            this.subscribe(['metrics', 'alerts']);
        };
        
        this.ws.onmessage = (event) => {
            const message: DashboardMessage = JSON.parse(event.data);
            this.handleMessage(message);
        };
        
        this.ws.onerror = (error) => {
            console.error('WebSocket error:', error);
        };
        
        this.ws.onclose = () => {
            console.log('Disconnected from metrics server');
            // Implement reconnection logic
        };
    }
    
    private subscribe(streams: string[]) {
        this.ws.send(JSON.stringify({
            command: 'subscribe',
            params: { streams }
        }));
    }
    
    private handleMessage(message: DashboardMessage) {
        switch (message.message_type) {
            case 'metrics_update':
                this.updateMetrics(message.data as MetricsSnapshot);
                break;
            case 'alert':
                this.showAlert(message.data as AlertMessage);
                break;
            default:
                console.log('Unknown message type:', message.message_type);
        }
    }
    
    private updateMetrics(metrics: MetricsSnapshot) {
        // Update dashboard UI with new metrics
        console.log('Metrics update:', metrics);
    }
    
    private showAlert(alert: AlertMessage) {
        // Display alert in UI
        console.log('Alert:', alert);
    }
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_websocket_config() {
        let config = WebSocketConfig::default();
        assert_eq!(config.port, 8080);
        assert_eq!(config.max_connections, 100);
    }

    #[tokio::test]
    async fn test_server_state() {
        let config = WebSocketConfig::default();
        let server = WebSocketServer::new(config);
        
        assert_eq!(server.connected_clients().await, 0);
    }
}