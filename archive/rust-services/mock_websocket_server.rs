// Simple mock WebSocket server to test the dashboard
use tokio::net::TcpListener;
use tokio_tungstenite::{accept_async, tungstenite::Message};
use futures_util::{StreamExt, SinkExt};
use serde_json::json;
use std::time::Duration;

#[tokio::main]
async fn main() {
    println!("🚀 Starting mock WebSocket server on ws://localhost:3002/ws");
    
    let listener = TcpListener::bind("127.0.0.1:3002").await.unwrap();
    println!("✅ Server listening on port 3002");
    
    while let Ok((stream, addr)) = listener.accept().await {
        println!("🔌 New connection from: {}", addr);
        tokio::spawn(handle_connection(stream));
    }
}

async fn handle_connection(stream: tokio::net::TcpStream) {
    let ws_stream = accept_async(stream).await.unwrap();
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    
    // Spawn task to handle incoming messages
    let incoming = tokio::spawn(async move {
        while let Some(msg) = ws_receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    println!("📨 Received: {}", text);
                }
                Ok(Message::Close(_)) => {
                    println!("🔌 Client disconnected");
                    break;
                }
                _ => {}
            }
        }
    });
    
    // Send mock data periodically
    let mut trade_id = 0u64;
    let mut interval = tokio::time::interval(Duration::from_millis(100));
    
    loop {
        interval.tick().await;
        
        // Generate batch of mock trades
        for _ in 0..5 {
            trade_id += 1;
            
            let trade = json!({
                "type": "Trade",
                "data": {
                    "timestamp": chrono::Utc::now().timestamp(),
                    "symbol": if trade_id % 2 == 0 { "BTC-USD" } else { "ETH-USD" },
                    "exchange": ["coinbase", "kraken", "binance"][trade_id as usize % 3],
                    "price": if trade_id % 2 == 0 { 
                        50000.0 + (trade_id as f64 % 1000.0) 
                    } else { 
                        3000.0 + (trade_id as f64 % 100.0) 
                    },
                    "volume": 0.1 + (trade_id as f64 % 10.0) * 0.01,
                    "side": if trade_id % 3 == 0 { "buy" } else { "sell" },
                    "trade_id": format!("mock_{}", trade_id)
                }
            });
            
            if ws_sender.send(Message::Text(trade.to_string())).await.is_err() {
                println!("❌ Failed to send, client disconnected");
                incoming.abort();
                return;
            }
        }
        
        // Send system stats periodically
        if trade_id % 50 == 0 {
            let stats = json!({
                "type": "SystemStats",
                "data": {
                    "latency_us": 250,
                    "active_clients": 1,
                    "trades_processed": trade_id,
                    "deltas_processed": 0,
                    "active_feeds": 1,
                    "trade_feeds": 1,
                    "delta_feeds": 0
                }
            });
            
            if ws_sender.send(Message::Text(stats.to_string())).await.is_err() {
                break;
            }
            
            println!("📊 Sent {} trades, latest stats sent", trade_id);
        }
    }
}