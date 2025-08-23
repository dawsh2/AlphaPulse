#!/usr/bin/env cargo script

//! Demo script to generate fake arbitrage opportunities for dashboard testing

use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::SinkExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Starting Demo Arbitrage Signal Generator");
    
    // Connect to dashboard WebSocket server
    let (mut ws_stream, _) = connect_async("ws://localhost:8081/ws").await?;
    println!("✅ Connected to dashboard WebSocket");
    
    let mut counter = 1;
    loop {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        
        // Create demo arbitrage opportunity
        let opportunity = json!({
            "msg_type": "arbitrage_opportunity",
            "id": format!("demo-arb-{}", counter),
            "timestamp": timestamp,
            "pair": "WMATIC/USDC",
            "buyExchange": "QuickSwap",
            "sellExchange": "SushiSwap", 
            "buyPrice": 0.45 + (counter as f64 * 0.001),
            "sellPrice": 0.46 + (counter as f64 * 0.001),
            "spread": 2.22,
            "tradeSize": 10000.0,
            "grossProfit": 222.0,
            "gasFee": 15.0,
            "dexFees": 12.0,
            "slippage": 5.0,
            "netProfit": 190.0,
            "netProfitPercent": 1.9,
            "buyPool": "0x6e7a5FAFcec6BB1e78bAE2A1F0B612012bf14827",
            "sellPool": "0xc4e595acDD7d12feC6E29a72390ca4022314c0Ac",
            "confidence": 0.95,
            "executable": true
        });
        
        // Send the opportunity
        let message = Message::Text(opportunity.to_string());
        ws_stream.send(message).await?;
        
        println!("📨 Sent demo arbitrage opportunity #{}: WMATIC/USDC - ${:.2} profit", 
                counter, 190.0);
        
        counter += 1;
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
        
        if counter > 20 {
            break;
        }
    }
    
    println!("✅ Demo complete - sent {} opportunities", counter - 1);
    Ok(())
}