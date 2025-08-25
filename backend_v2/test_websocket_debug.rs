use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing Rust WebSocket behavior to isolate the issue");
    
    let url = "wss://polygon.drpc.org";
    
    // DEX event signatures from our collector
    let dex_signatures = vec![
        "0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822", // Uniswap V2 Swap
        "0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67", // Uniswap V3 Swap  
        "0x4c209b5fc8ad50758f13e2e1088ba56a560dff690a1c6fef26394f4c03821c4f", // Uniswap V2 Mint
        "0x7a53080ba414158be7ec69b987b5fb7d07dee101fe85488f0853ae16239d0bde", // Uniswap V3 Mint
        "0xdccd412f0b1252819cb1fd330b93224ca42612892bb3f4f789976e6d81936496", // Uniswap V2 Burn
        "0x0c396cd989a39f4459b5fa1aed6a9a8dcdbc45908acfd67e028cd568da98982c", // Uniswap V3 Burn
        "0x1c411e9a96e071241c2f21f7726b17ae89e3cab4c78be50e062b03a9fffbbad1"  // Uniswap V2 Sync
    ];
    
    println!("🔌 Connecting to {}", url);
    let (ws_stream, _) = connect_async(url).await?;
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    
    println!("✅ Connected!");
    
    // Create subscription request exactly like the collector
    let subscription_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "eth_subscribe",
        "params": [
            "logs",
            {
                "topics": [dex_signatures]
            }
        ]
    });
    
    println!("📡 Sending subscription...");
    println!("📋 Subscription: {}", serde_json::to_string_pretty(&subscription_request)?);
    
    ws_sender
        .send(Message::Text(subscription_request.to_string()))
        .await?;
    
    println!("✅ Subscription sent, waiting for messages...");
    
    let mut message_count = 0;
    let mut timeout_count = 0;
    let start_time = std::time::Instant::now();
    
    // Test for 30 seconds with same timeout as polygon collector
    for iteration in 1..=30 {
        let message_timeout = Duration::from_millis(60000); // Same as polygon collector
        
        println!("🔄 Iteration {}: Waiting for message with 60s timeout...", iteration);
        
        match tokio::time::timeout(message_timeout, ws_receiver.next()).await {
            Ok(Some(Ok(Message::Text(text)))) => {
                message_count += 1;
                println!("📨 Message {} received! ({}s since start)", message_count, start_time.elapsed().as_secs_f64());
                
                // Parse the message
                match serde_json::from_str::<Value>(&text) {
                    Ok(json_value) => {
                        // Check for subscription confirmation
                        if let Some(id) = json_value.get("id") {
                            if id == 1 {
                                if let Some(result) = json_value.get("result") {
                                    println!("✅ Subscription confirmed: {}", result);
                                } else if let Some(error) = json_value.get("error") {
                                    println!("❌ Subscription error: {}", error);
                                    return Err(format!("Subscription failed: {}", error).into());
                                }
                                continue;
                            }
                        }
                        
                        // Check for eth_subscription notifications
                        if let Some(method) = json_value.get("method") {
                            if method == "eth_subscription" {
                                println!("🎯 DEX Event detected!");
                                if let Some(params) = json_value.get("params") {
                                    if let Some(result) = params.get("result") {
                                        let topics = result.get("topics").and_then(|t| t.as_array());
                                        let block_num = result.get("blockNumber").and_then(|b| b.as_str());
                                        let address = result.get("address").and_then(|a| a.as_str());
                                        
                                        println!("   📊 Block: {:?}", block_num);
                                        println!("   📍 Address: {:?}", address);
                                        if let Some(topics_arr) = topics {
                                            if !topics_arr.is_empty() {
                                                println!("   🔑 Topic0: {}", topics_arr[0]);
                                            }
                                        }
                                        
                                        if message_count >= 5 {
                                            println!("✅ Received {} DEX events - test successful!", message_count);
                                            break;
                                        }
                                    }
                                }
                            } else {
                                println!("📨 Non-subscription message: {}", method);
                            }
                        } else {
                            println!("📨 Message without method field");
                        }
                    }
                    Err(e) => {
                        println!("⚠️  JSON parse error: {}", e);
                        println!("   Raw message: {}", &text[..std::cmp::min(200, text.len())]);
                    }
                }
            }
            Ok(Some(Ok(Message::Ping(_)))) => {
                println!("🏓 Ping received");
            }
            Ok(Some(Ok(Message::Close(_)))) => {
                println!("❌ WebSocket closed");
                break;
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
                timeout_count += 1;
                println!("⏳ Timeout {} (60s) - continuing...", timeout_count);
                continue;
            }
        }
    }
    
    let elapsed = start_time.elapsed();
    println!("\n📊 Final Results:");
    println!("   Duration: {:.2}s", elapsed.as_secs_f64());
    println!("   Messages: {}", message_count);
    println!("   Timeouts: {}", timeout_count);
    
    if message_count > 0 {
        println!("✅ Rust WebSocket handling works!");
        println!("❗ Issue must be elsewhere in the collector");
    } else {
        println!("❌ No messages received - confirms Rust WebSocket issue");
    }
    
    Ok(())
}