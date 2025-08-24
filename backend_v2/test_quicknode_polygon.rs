#!/usr/bin/env rust-script
//! Test QuickNode Polygon endpoint for comparison

use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use std::time::Duration;
use tokio_tungstenite::{connect_async, tungstenite::Message};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing QuickNode Polygon endpoint");

    // Known working endpoints - try free tier ones
    let endpoints = vec![
        "wss://rpc-mainnet.matic.network",
        "wss://rpc-mainnet.matic.quiknode.pro", // Often has free tier
        "wss://polygon-rpc.com/ws",
    ];

    for url in endpoints {
        println!("\n🔌 Testing: {}", url);

        match test_endpoint(url).await {
            Ok(events) => {
                if events > 0 {
                    println!("✅ SUCCESS! {} events received from {}", events, url);
                    println!("🎯 This endpoint is active and working");
                    break;
                } else {
                    println!("⚠️ Connected but no events from {}", url);
                }
            }
            Err(e) => {
                println!("❌ Failed to connect to {}: {}", url, e);
            }
        }
    }

    Ok(())
}

async fn test_endpoint(url: &str) -> Result<u32, Box<dyn std::error::Error>> {
    let (ws_stream, _) = connect_async(url).await?;
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    // Subscribe to ALL logs (broad filter)
    let subscription = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "eth_subscribe",
        "params": ["logs", {}]
    });

    ws_sender
        .send(Message::Text(subscription.to_string()))
        .await?;

    let mut event_count = 0u32;
    let test_duration = Duration::from_secs(8);
    let start_time = std::time::Instant::now();

    while start_time.elapsed() < test_duration {
        let timeout_duration = Duration::from_secs(1);

        match tokio::time::timeout(timeout_duration, ws_receiver.next()).await {
            Ok(Some(Ok(Message::Text(text)))) => {
                let json_value: Value = serde_json::from_str(&text)?;

                if let Some(method) = json_value.get("method") {
                    if method == "eth_subscription" {
                        event_count += 1;
                        if event_count <= 3 {
                            println!("  🎉 Event #{}", event_count);
                        }

                        if event_count >= 5 {
                            break; // Found active endpoint
                        }
                    }
                } else if let Some(result) = json_value.get("result") {
                    println!("  ✅ Subscription confirmed: {}", result);
                }
            }
            Ok(Some(Ok(Message::Ping(ping)))) => {
                ws_sender.send(Message::Pong(ping)).await?;
            }
            Err(_) => {
                // Timeout - continue
                if event_count == 0 {
                    print!(".");
                }
            }
            _ => {}
        }
    }

    Ok(event_count)
}
