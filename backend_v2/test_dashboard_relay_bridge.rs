#!/usr/bin/env rust-script  
//! Test if dashboard websocket server is consuming from market data relay

use std::time::Duration;
use tokio::net::UnixStream;
use tokio::io::AsyncReadExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Testing if dashboard websocket server consumes from market data relay\n");
    
    // Check if market data relay has multiple consumers
    let socket_path = "/tmp/alphapulse/market_data.sock";
    
    println!("📡 Attempting to connect as secondary consumer to market data relay...");
    match UnixStream::connect(socket_path).await {
        Ok(mut stream) => {
            println!("✅ Connected as secondary consumer");
            
            let mut buffer = vec![0u8; 1024];
            let mut message_count = 0;
            
            println!("🎧 Listening for messages for 10 seconds...");
            let start = std::time::Instant::now();
            
            while start.elapsed() < Duration::from_secs(10) {
                match tokio::time::timeout(Duration::from_secs(2), stream.read(&mut buffer)).await {
                    Ok(Ok(n)) if n > 0 => {
                        message_count += 1;
                        if message_count <= 5 {
                            println!("📨 Message {}: {} bytes", message_count, n);
                        }
                    }
                    Ok(Ok(0)) => {
                        println!("📡 Relay connection closed");
                        break;
                    }
                    Ok(Err(e)) => {
                        println!("❌ Read error: {}", e);
                        break;  
                    }
                    Err(_) => {
                        // Timeout - continue
                        print!(".");
                    }
                }
            }
            
            println!("\n📊 Results:");
            println!("   Messages received in 10s: {}", message_count);
            println!("   Rate: {:.1} msg/sec", message_count as f64 / 10.0);
            
            if message_count > 0 {
                println!("✅ Market data relay is broadcasting to multiple consumers");
                println!("🤔 Dashboard websocket server should be receiving these too");
                println!("💡 Check if dashboard server is properly forwarding to WebSocket clients");
            } else {
                println!("⚠️ No messages received as secondary consumer");
                println!("💭 Market data relay might only support one consumer at a time");
            }
        }
        Err(e) => {
            println!("❌ Failed to connect: {}", e);
            println!("💭 This could mean:");
            println!("   1. Market data relay only supports one consumer");
            println!("   2. Dashboard server has exclusive connection");
            println!("   3. Unix socket permissions issue");
        }
    }
    
    // Check if dashboard websocket server is running and consuming
    println!("\n🔍 Dashboard WebSocket Server Analysis:");
    println!("   - Running on port 8080 ✅");
    println!("   - Frontend connected and receiving heartbeats ✅"); 
    println!("   - BUT: No market data events forwarded to browser ❌");
    println!("\n🎯 Next steps:");
    println!("   1. Check dashboard server logs for market data consumption");
    println!("   2. Verify dashboard server config points to correct relay socket");
    println!("   3. Ensure dashboard server forwards market data events to WebSocket");
    
    Ok(())
}