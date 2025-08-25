//! Main dashboard WebSocket server

use crate::client::ClientManager;
use crate::config::DashboardConfig;
use crate::error::{DashboardError, Result};
use crate::relay_consumer::RelayConsumer;
use futures_util::StreamExt;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::time::{interval, Duration};
use tracing::{error, info, warn};
use warp::Filter;

/// Main dashboard server
pub struct DashboardServer {
    config: DashboardConfig,
    client_manager: Arc<ClientManager>,
}

impl DashboardServer {
    pub fn new(config: DashboardConfig) -> Self {
        let client_manager = Arc::new(ClientManager::new(config.max_connections));

        Self {
            config,
            client_manager,
        }
    }

    /// Start the dashboard server
    pub async fn start(&self) -> Result<()> {
        info!("Starting Dashboard WebSocket Server");
        info!("Configuration: {:?}", self.config);

        // Start relay consumer
        let relay_consumer = RelayConsumer::new(
            self.client_manager.clone(),
            self.config.market_data_relay_path.clone(),
            self.config.signal_relay_path.clone(),
            self.config.execution_relay_path.clone(),
        );

        let relay_handle = tokio::spawn(async move {
            if let Err(e) = relay_consumer.start().await {
                error!("Relay consumer failed: {}", e);
            }
        });

        // Start heartbeat task
        let heartbeat_handle = self.start_heartbeat_task();

        // Start WebSocket server
        let server_handle = self.start_websocket_server().await?;

        info!("Dashboard server started successfully");

        // Wait for all tasks
        tokio::select! {
            result = relay_handle => {
                if let Err(e) = result {
                    error!("Relay consumer task failed: {}", e);
                }
            }
            result = heartbeat_handle => {
                if let Err(e) = result {
                    error!("Heartbeat task failed: {}", e);
                }
            }
            result = server_handle => {
                if let Err(e) = result {
                    error!("WebSocket server failed: {}", e);
                }
            }
        }

        Ok(())
    }

    async fn start_websocket_server(&self) -> Result<tokio::task::JoinHandle<()>> {
        let addr: SocketAddr = format!("{}:{}", self.config.bind_address, self.config.port)
            .parse()
            .map_err(|e| DashboardError::Configuration {
                message: format!("Invalid bind address: {}", e),
            })?;

        info!("Starting WebSocket server on {}", addr);

        let client_manager = self.client_manager.clone();
        let enable_cors = self.config.enable_cors;

        let handle = tokio::spawn(async move {
            // Create WebSocket upgrade endpoint
            let ws_route = warp::path("ws")
                .and(warp::ws())
                .and_then(move |ws: warp::ws::Ws| {
                    let client_manager = client_manager.clone();
                    async move {
                        Ok::<_, warp::Rejection>(ws.on_upgrade(move |socket| {
                            Self::handle_websocket_connection(client_manager, socket)
                        }))
                    }
                });

            // Create health check endpoint
            let health_route = warp::path("health")
                .map(|| warp::reply::with_status("OK", warp::http::StatusCode::OK));

            // Create status endpoint
            let status_route = warp::path("status").map(|| {
                warp::reply::json(&serde_json::json!({
                    "status": "running",
                    "service": "alphapulse-dashboard",
                    "version": env!("CARGO_PKG_VERSION")
                }))
            });

            let routes = ws_route.or(health_route).or(status_route);

            if enable_cors {
                let cors_routes = routes.with(warp::cors().allow_any_origin());
                warp::serve(cors_routes).run(addr).await;
            } else {
                warp::serve(routes).run(addr).await;
            }
        });

        Ok(handle)
    }

    async fn handle_websocket_connection(
        client_manager: Arc<ClientManager>,
        ws: warp::ws::WebSocket,
    ) {
        info!("New WebSocket connection established");

        // Handle the connection directly
        if let Err(e) = Self::handle_client_connection(client_manager, ws).await {
            warn!("WebSocket connection error: {}", e);
        }
    }

    async fn handle_client_connection(
        client_manager: Arc<ClientManager>,
        ws: warp::ws::WebSocket,
    ) -> Result<()> {
        use futures_util::{SinkExt, StreamExt};
        use tokio::sync::mpsc;
        use warp::ws::Message;

        let (tx, mut rx) = mpsc::unbounded_channel::<serde_json::Value>();
        let client = crate::client::Client::new(tx);
        let client_id = client.id;

        // Add client to manager
        client_manager.add_client(client).await?;

        // Split the WebSocket into sender and receiver
        let (mut ws_sender, mut ws_receiver) = ws.split();

        // Handle WebSocket communication using select to avoid Send issues
        loop {
            tokio::select! {
                // Handle outgoing messages from our channel
                msg = rx.recv() => {
                    match msg {
                        Some(message) => {
                            let json_str = match serde_json::to_string(&message) {
                                Ok(s) => s,
                                Err(e) => {
                                    error!("Failed to serialize message: {}", e);
                                    continue;
                                }
                            };

                            if let Err(e) = ws_sender.send(Message::text(json_str)).await {
                                warn!("Failed to send message to client {}: {}", client_id, e);
                                break;
                            }
                        }
                        None => {
                            info!("Message channel closed for client {}", client_id);
                            break;
                        }
                    }
                }

                // Handle incoming WebSocket messages
                ws_msg = ws_receiver.next() => {
                    match ws_msg {
                        Some(Ok(msg)) => {
                            if msg.is_text() {
                                info!("Received message from client {}: {}", client_id, msg.to_str().unwrap_or(""));
                            } else if msg.is_close() {
                                info!("Client {} disconnected", client_id);
                                break;
                            }
                        }
                        Some(Err(e)) => {
                            warn!("WebSocket error for client {}: {}", client_id, e);
                            break;
                        }
                        None => {
                            info!("WebSocket stream closed for client {}", client_id);
                            break;
                        }
                    }
                }
            }
        }

        // Cleanup
        client_manager.remove_client(client_id).await;

        Ok(())
    }

    fn start_heartbeat_task(&self) -> tokio::task::JoinHandle<()> {
        let client_manager = self.client_manager.clone();
        let interval_secs = self.config.heartbeat_interval_secs;

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(interval_secs));

            loop {
                interval.tick().await;

                let heartbeat_msg = serde_json::json!({
                    "msg_type": "heartbeat",
                    "timestamp": std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis(),
                    "client_count": client_manager.client_count().await
                });

                client_manager.broadcast(heartbeat_msg).await;
                info!(
                    "Sent heartbeat to {} clients",
                    client_manager.client_count().await
                );
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_dashboard_server_creation() {
        let config = DashboardConfig::default();
        let server = DashboardServer::new(config);
        assert_eq!(server.client_manager.client_count().await, 0);
    }
}
