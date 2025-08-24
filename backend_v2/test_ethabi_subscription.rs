#!/usr/bin/env rust-script
//! Test subscription using the exact same ethabi-generated signatures

use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use std::time::Duration;
use tokio_tungstenite::{connect_async, tungstenite::Message};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing WebSocket with ethabi-generated signatures\n");

    let url = "wss://polygon-bor-rpc.publicnode.com";
    let (ws_stream, _) = connect_async(url).await?;
    println!("✅ Connected to {}", url);

    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    // Use the EXACT signatures from our ethabi-based collector
    let signatures = vec![
        "0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822", // V2 Swap
        "0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67", // V3 Swap
        "0x4c209b5fc8ad50758f13e2e1088ba56a560dff690a1c6fef26394f4c03821c4f", // V2 Mint
        "0x7a53080ba414158be7ec69b987b5fb7d07dee101fe85488f0853ae16239d0bde", // V3 Mint
        "0xdccd412f0b1252819cb1fd330b93224ca42612892bb3f4f789976e6d81936496", // V2 Burn
        "0x0c396cd989a39f4459b5fa1aed6a9a8dcdbc45908acfd67e028cd568da98982c", // V3 Burn
        "0x1c411e9a96e071241c2f21f7726b17ae89e3cab4c78be50e062b03a9fffbbad1", // V2 Sync
    ];

    let subscription_message = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "eth_subscribe",
        "params": [
            "logs",
            {
                "topics": [signatures]
            }
        ]
    });

    ws_sender
        .send(Message::Text(subscription_message.to_string()))
        .await?;

    println!("📤 Sent subscription with {} signatures", signatures.len());
    for (i, sig) in signatures.iter().enumerate() {
        println!("  {}. {}", i + 1, sig);
    }

    let mut event_count = 0u32;
    let test_duration = Duration::from_secs(20);
    let start_time = std::time::Instant::now();

    println!("\n⏰ Monitoring for 20 seconds...");

    while start_time.elapsed() < test_duration {
        let timeout_duration = Duration::from_secs(2);

        match tokio::time::timeout(timeout_duration, ws_receiver.next()).await {
            Ok(Some(Ok(Message::Text(text)))) => {
                let json_value: Value = serde_json::from_str(&text)?;

                if let Some(method) = json_value.get("method") {
                    if method == "eth_subscription" {
                        event_count += 1;
                        if event_count <= 3 {
                            println!("🎉 Event #{}: {} bytes", event_count, text.len());
                        } else if event_count % 10 == 0 {
                            println!("📈 Event #{} (every 10th)", event_count);
                        }
                    }
                } else if let Some(id) = json_value.get("id") {
                    if id == 1 {
                        if let Some(result) = json_value.get("result") {
                            println!("✅ Subscription confirmed: {}", result);
                        } else if let Some(error) = json_value.get("error") {
                            println!("❌ Subscription error: {}", error);
                            return Ok(());
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
                println!("❌ WebSocket error: {}", e);
                break;
            }
            Ok(None) => {
                println!("❌ WebSocket stream ended");
                break;
            }
            Err(_) => {
                // Timeout - continue
                if event_count == 0 {
                    print!(".");
                }
            }
        }
    }

    println!("\n🏁 Results:");
    println!("   Events received: {}", event_count);
    println!("   Events/second: {:.2}", event_count as f64 / 20.0);

    if event_count > 0 {
        println!("✅ SUCCESS: Events flowing with ethabi signatures!");
    } else {
        println!("❌ PROBLEM: No events received");
    }

    Ok(())
}
