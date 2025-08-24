#!/usr/bin/env rust-script
//! Test Polygon WebSocket connection and eth_subscribe capability
//!
//! This test directly connects to the Polygon WebSocket endpoint
//! and attempts to subscribe to DEX events to verify the connection works.

use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use std::time::Duration;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{error, info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().init();

    // Test different WebSocket endpoints
    let endpoints = vec![
        "wss://polygon-bor-rpc.publicnode.com",
        "wss://polygon-mainnet.g.alchemy.com/v2/demo",
        "wss://ws-polygon-mainnet.chainstacklabs.com",
        "wss://polygon-rpc.com",
    ];

    for endpoint in endpoints {
        println!("\n🧪 Testing WebSocket endpoint: {}", endpoint);

        match test_websocket_endpoint(endpoint).await {
            Ok(()) => {
                println!("✅ {} - WebSocket connection successful!", endpoint);
            }
            Err(e) => {
                println!("❌ {} - WebSocket connection failed: {}", endpoint, e);
            }
        }
    }

    Ok(())
}

async fn test_websocket_endpoint(url: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Try to connect with timeout
    let timeout_duration = Duration::from_secs(10);

    let (ws_stream, response) = tokio::time::timeout(timeout_duration, connect_async(url))
        .await?
        .map_err(|e| format!("WebSocket connection failed: {}", e))?;

    info!("✅ Connected to {} (status: {:?})", url, response.status());

    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    // Test basic subscription
    let subscription_message = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "eth_subscribe",
        "params": [
            "logs",
            {
                "topics": [
                    ["0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822"] // Swap event
                ]
            }
        ]
    });

    // Send subscription
    ws_sender
        .send(Message::Text(subscription_message.to_string()))
        .await?;

    info!("📤 Sent eth_subscribe request");

    // Wait for response or subscription data
    let timeout_duration = Duration::from_secs(5);

    for attempt in 1..=3 {
        match tokio::time::timeout(timeout_duration, ws_receiver.next()).await {
            Ok(Some(Ok(Message::Text(text)))) => {
                info!("📥 Received message: {}", text);

                let json_value: Value = serde_json::from_str(&text)?;

                // Check if this is a subscription response
                if let Some(id) = json_value.get("id") {
                    if id == 1 {
                        if let Some(result) = json_value.get("result") {
                            info!("✅ Subscription successful: {}", result);
                            return Ok(());
                        } else if let Some(error) = json_value.get("error") {
                            return Err(format!("Subscription error: {}", error).into());
                        }
                    }
                }

                // Check if this is a subscription notification
                if let Some(method) = json_value.get("method") {
                    if method == "eth_subscription" {
                        info!("🎉 Received live event notification!");
                        return Ok(());
                    }
                }
            }
            Ok(Some(Ok(Message::Ping(_)))) => {
                info!("📡 Received ping");
            }
            Ok(Some(Ok(other))) => {
                info!("📦 Received other message: {:?}", other);
            }
            Ok(Some(Err(e))) => {
                return Err(format!("WebSocket error: {}", e).into());
            }
            Ok(None) => {
                return Err("WebSocket stream ended".into());
            }
            Err(_) => {
                warn!("⏰ Timeout waiting for response (attempt {})", attempt);
            }
        }
    }

    Err("No valid subscription response received".into())
}
