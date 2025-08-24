#!/usr/bin/env rust-script
//! Test alternative Polygon WebSocket endpoints for higher activity

use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use std::time::Duration;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().init();

    // Try different high-activity Polygon endpoints
    let endpoints = vec![
        ("Ankr", "wss://rpc.ankr.com/polygon/ws"),
        ("Blast", "wss://polygon-mainnet.blastapi.io/{API_KEY}/ws"),
        ("GetBlock", "wss://pol.getblock.io/mainnet/"),
        (
            "QuickNode",
            "wss://old-weathered-river.matic.quiknode.pro/{API_KEY}/ws",
        ),
        (
            "Infura",
            "wss://polygon-mainnet.infura.io/ws/v3/{PROJECT_ID}",
        ),
        (
            "Moralis",
            "wss://speedy-nodes-nyc.moralis.io/{API_KEY}/polygon/mainnet/ws",
        ),
    ];

    let free_endpoints = vec![
        ("Ankr Public", "wss://rpc.ankr.com/polygon/ws"),
        ("Public Node", "wss://polygon-bor-rpc.publicnode.com"),
    ];

    println!("🧪 Testing free public Polygon WebSocket endpoints...\n");

    for (name, endpoint) in free_endpoints {
        println!("Testing {} ({})", name, endpoint);

        match test_live_events(endpoint).await {
            Ok(event_count) => {
                println!(
                    "✅ {} - Received {} live events in 15 seconds!",
                    name, event_count
                );
            }
            Err(e) => {
                println!("❌ {} - Failed: {}", name, e);
            }
        }
        println!();
    }

    println!("\n📋 Premium endpoint options (require API keys):");
    for (name, endpoint) in endpoints {
        println!("  {} - {}", name, endpoint);
    }

    Ok(())
}

async fn test_live_events(url: &str) -> Result<u32, Box<dyn std::error::Error>> {
    let timeout_duration = Duration::from_secs(10);

    let (ws_stream, _) = tokio::time::timeout(timeout_duration, connect_async(url))
        .await?
        .map_err(|e| format!("WebSocket connection failed: {}", e))?;

    info!("✅ Connected to {}", url);

    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    // Subscribe to high-activity DEX events (all major pools)
    let subscription_message = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "eth_subscribe",
        "params": [
            "logs",
            {
                "topics": [
                    [
                        "0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822", // V3 Swap
                        "0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67", // V2 Swap
                        "0x1c411e9a96e071241c2f21f7726b17ae89e3cab4c78be50e062b03a9fffbbad1"  // Sync
                    ]
                ]
            }
        ]
    });

    ws_sender
        .send(Message::Text(subscription_message.to_string()))
        .await?;

    info!("📤 Subscribed to DEX events, monitoring for 15 seconds...");

    let mut event_count = 0u32;
    let test_duration = Duration::from_secs(15);
    let start_time = std::time::Instant::now();

    while start_time.elapsed() < test_duration {
        let timeout_duration = Duration::from_secs(2);

        match tokio::time::timeout(timeout_duration, ws_receiver.next()).await {
            Ok(Some(Ok(Message::Text(text)))) => {
                let json_value: Value = serde_json::from_str(&text)?;

                // Check if this is a subscription notification (live event)
                if let Some(method) = json_value.get("method") {
                    if method == "eth_subscription" {
                        event_count += 1;
                        if event_count <= 3 {
                            info!(
                                "🎉 Event #{}: {}",
                                event_count,
                                json_value
                                    .get("params")
                                    .and_then(|p| p.get("result"))
                                    .and_then(|r| r.get("address"))
                                    .map(|v| v.as_str().unwrap_or("unknown"))
                                    .unwrap_or("unknown")
                            );
                        }
                        if event_count == 1 {
                            info!("✅ Live events confirmed! Continuing to count...");
                        }
                    }
                } else if let Some(id) = json_value.get("id") {
                    if id == 1 {
                        if let Some(result) = json_value.get("result") {
                            info!("✅ Subscription ID: {}", result);
                        } else if let Some(error) = json_value.get("error") {
                            return Err(format!("Subscription error: {}", error).into());
                        }
                    }
                }
            }
            Ok(Some(Ok(Message::Ping(ping)))) => {
                ws_sender.send(Message::Pong(ping)).await?;
            }
            Ok(Some(Ok(_))) => {
                // Other message types
            }
            Ok(Some(Err(e))) => {
                return Err(format!("WebSocket error: {}", e).into());
            }
            Ok(None) => {
                return Err("WebSocket stream ended".into());
            }
            Err(_) => {
                // Timeout - continue
            }
        }
    }

    info!("🏁 Test completed: {} events in 15 seconds", event_count);
    Ok(event_count)
}
