//! Mock Kraken WebSocket server for testing

use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tokio_tungstenite::{accept_async, tungstenite::Message};
use tracing::{debug, info, warn};

pub struct MockKrakenServer {
    bind_addr: SocketAddr,
    clients: Arc<Mutex<Vec<tokio_tungstenite::WebSocketStream<TcpStream>>>>,
}

impl MockKrakenServer {
    pub async fn new(port: u16) -> Result<Self> {
        let bind_addr = ([127, 0, 0, 1], port).into();
        Ok(Self {
            bind_addr,
            clients: Arc::new(Mutex::new(Vec::new())),
        })
    }

    pub async fn run(&self) -> Result<()> {
        let listener = TcpListener::bind(self.bind_addr).await?;
        info!("Mock Kraken server listening on {}", self.bind_addr);

        // Start market data broadcaster
        let clients = self.clients.clone();
        tokio::spawn(async move {
            Self::broadcast_market_data(clients).await;
        });

        while let Ok((stream, addr)) = listener.accept().await {
            info!("New connection from {}", addr);

            let clients = self.clients.clone();
            tokio::spawn(async move {
                if let Err(e) = Self::handle_connection(stream, clients).await {
                    warn!("Connection error: {}", e);
                }
            });
        }

        Ok(())
    }

    async fn handle_connection(
        stream: TcpStream,
        clients: Arc<Mutex<Vec<tokio_tungstenite::WebSocketStream<TcpStream>>>>,
    ) -> Result<()> {
        let ws_stream = accept_async(stream).await?;
        debug!("WebSocket connection established");

        let (mut ws_sender, mut ws_receiver) = ws_stream.split();

        // Send initial subscription confirmation
        let subscription_msg = json!({
            "event": "subscriptionStatus",
            "status": "subscribed",
            "pair": "XBT/USD",
            "subscription": {
                "name": "trade"
            }
        });

        ws_sender
            .send(Message::Text(subscription_msg.to_string()))
            .await?;

        // Handle incoming messages
        while let Some(msg) = ws_receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    debug!("Received: {}", text);

                    // Parse subscription requests
                    if let Ok(parsed) = serde_json::from_str::<Value>(&text) {
                        if parsed.get("event") == Some(&Value::String("subscribe".to_string())) {
                            let response = json!({
                                "event": "subscriptionStatus",
                                "status": "subscribed",
                                "pair": parsed.get("pair").unwrap_or(&json!("XBT/USD")),
                                "subscription": parsed.get("subscription").unwrap_or(&json!({"name": "trade"}))
                            });

                            ws_sender.send(Message::Text(response.to_string())).await?;
                        }
                    }
                }
                Ok(Message::Close(_)) => {
                    debug!("WebSocket connection closed");
                    break;
                }
                Err(e) => {
                    warn!("WebSocket error: {}", e);
                    break;
                }
                _ => {}
            }
        }

        Ok(())
    }

    async fn broadcast_market_data(
        clients: Arc<Mutex<Vec<tokio_tungstenite::WebSocketStream<TcpStream>>>>,
    ) {
        let mut interval = tokio::time::interval(Duration::from_millis(100));
        let mut base_price = 45000.0;
        let mut trade_id = 1000000;

        loop {
            interval.tick().await;

            // Generate realistic price movement
            let price_change = (rand::random::<f64>() - 0.5) * 100.0; // ±$50 movement
            base_price = (base_price + price_change).max(40000.0).min(50000.0);

            let volume = rand::random::<f64>() * 0.1 + 0.01; // 0.01 to 0.11 BTC

            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs_f64();

            let side = if rand::random::<bool>() { "b" } else { "s" };

            // Kraken trade format
            let trade_data = json!([
                4, // channel ID
                [[
                    format!("{:.2}", base_price), // price
                    format!("{:.8}", volume),     // volume
                    format!("{:.6}", timestamp),  // time
                    side,                         // side (b=buy, s=sell)
                    "m",                          // order type (m=market, l=limit)
                    ""                            // misc
                ]],
                "trade",
                "XBT/USD"
            ]);

            // Broadcast to all connected clients
            let mut clients_guard = clients.lock().await;
            let mut to_remove = Vec::new();

            for (i, client) in clients_guard.iter_mut().enumerate() {
                use futures_util::SinkExt;

                if let Err(_) = client.send(Message::Text(trade_data.to_string())).await {
                    to_remove.push(i);
                }
            }

            // Remove disconnected clients
            for &i in to_remove.iter().rev() {
                clients_guard.remove(i);
            }

            trade_id += 1;

            // Occasionally send heartbeat
            if trade_id % 100 == 0 {
                let heartbeat = json!({
                    "event": "heartbeat"
                });

                for client in clients_guard.iter_mut() {
                    let _ = client.send(Message::Text(heartbeat.to_string())).await;
                }
            }
        }
    }
}
