#!/usr/bin/env rust-script
//! Test broader subscription to catch ANY logs to see if the endpoint is working

use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use std::time::Duration;
use tokio_tungstenite::{connect_async, tungstenite::Message};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing WebSocket with broad log filter to verify endpoint activity\n");

    let url = "wss://polygon-bor-rpc.publicnode.com";
    let (ws_stream, _) = connect_async(url).await?;
    println!("✅ Connected to {}", url);

    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    // Test 1: Subscribe to ALL logs (no topic filter)
    println!("🌐 Test 1: Subscribing to ALL logs...");
    let all_logs_subscription = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "eth_subscribe",
        "params": ["logs", {}]
    });

    ws_sender
        .send(Message::Text(all_logs_subscription.to_string()))
        .await?;

    let mut event_count = 0u32;
    let test_duration = Duration::from_secs(10);
    let start_time = std::time::Instant::now();

    println!(
        "⏰ Monitoring for 10 seconds (this should get LOTS of events if endpoint is active)..."
    );

    while start_time.elapsed() < test_duration {
        let timeout_duration = Duration::from_secs(1);

        match tokio::time::timeout(timeout_duration, ws_receiver.next()).await {
            Ok(Some(Ok(Message::Text(text)))) => {
                let json_value: Value = serde_json::from_str(&text)?;

                if let Some(method) = json_value.get("method") {
                    if method == "eth_subscription" {
                        event_count += 1;
                        if event_count <= 5 {
                            println!("🎉 Event #{}: {} bytes", event_count, text.len());
                        } else if event_count % 50 == 0 {
                            println!("📈 Event #{} (every 50th)", event_count);
                        }

                        // Stop early if we get too many events
                        if event_count >= 200 {
                            println!("✅ SUCCESS: Endpoint is very active! Stopping early.");
                            break;
                        }
                    }
                } else if let Some(id) = json_value.get("id") {
                    if id == 1 {
                        if let Some(result) = json_value.get("result") {
                            println!("✅ All-logs subscription confirmed: {}", result);
                        } else if let Some(error) = json_value.get("error") {
                            println!("❌ All-logs subscription error: {}", error);
                            break;
                        }
                    }
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

    println!("\n🏁 Results for ALL logs subscription:");
    println!("   Events received: {}", event_count);
    println!("   Events/second: {:.2}", event_count as f64 / 10.0);

    if event_count > 0 {
        println!("✅ SUCCESS: Endpoint is active and receiving events!");
        println!("🔍 This means our specific subscription filter is the problem.");
    } else {
        println!("❌ PROBLEM: Endpoint appears completely inactive or broken");
        println!("🔍 This suggests endpoint/connectivity issues, not filter problems.");

        // Test 2: Try a different endpoint
        println!("\n🔄 Test 2: Trying different endpoint...");
        test_different_endpoint().await?;
    }

    Ok(())
}

async fn test_different_endpoint() -> Result<(), Box<dyn std::error::Error>> {
    let url = "wss://rpc.ankr.com/polygon/ws";
    println!("🔌 Testing {}", url);

    let (ws_stream, _) = connect_async(url).await?;
    println!("✅ Connected to Ankr endpoint");

    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    let all_logs_subscription = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "eth_subscribe",
        "params": ["logs", {}]
    });

    ws_sender
        .send(Message::Text(all_logs_subscription.to_string()))
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
                            println!("🎉 Ankr Event #{}", event_count);
                        }
                    }
                } else if let Some(id) = json_value.get("id") {
                    if id == 2 {
                        if let Some(result) = json_value.get("result") {
                            println!("✅ Ankr subscription confirmed: {}", result);
                        }
                    }
                }
            }
            Err(_) => {
                if event_count == 0 {
                    print!(".");
                }
            }
            _ => {}
        }
    }

    println!("\n🏁 Ankr Results:");
    println!("   Events received: {}", event_count);

    Ok(())
}
