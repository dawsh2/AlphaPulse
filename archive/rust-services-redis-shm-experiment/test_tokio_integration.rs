#!/usr/bin/env rust-script
// Test script to verify Tokio transport integration

use alphapulse_common::{
    tokio_transport::{TokioTransport, init_global_transport},
    Trade,
};
use std::time::Duration;
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("debug")
        .init();
    
    println!("🚀 Testing Tokio Transport Integration");
    
    // Initialize global transport
    let transport = init_global_transport(1000);
    println!("✅ Global transport initialized");
    
    // Clone for reader
    let reader_transport = transport.clone();
    
    // Spawn reader task (simulating API server)
    let reader_handle = tokio::spawn(async move {
        println!("📖 Reader waiting for trades...");
        
        for i in 0..3 {
            let trades = reader_transport.read_batch().await;
            println!("📊 Reader received {} trades (batch {})", trades.len(), i + 1);
            
            for trade in &trades {
                println!("  - {} @ ${} ({})", trade.symbol, trade.price, trade.exchange);
            }
        }
    });
    
    // Give reader time to start
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Write some test trades (simulating collector)
    println!("✍️ Writing test trades...");
    
    for i in 0..10 {
        let trade = Trade {
            timestamp: chrono::Utc::now().timestamp() as f64,
            symbol: format!("BTC-USD"),
            exchange: "test".to_string(),
            price: 50000.0 + (i as f64 * 100.0),
            volume: 0.1 * (i + 1) as f64,
            side: Some(if i % 2 == 0 { "buy" } else { "sell" }.to_string()),
            trade_id: Some(format!("test_{}", i)),
        };
        
        transport.write(trade).await?;
        println!("  Written trade #{}", i);
        
        // Batch writes
        if (i + 1) % 3 == 0 {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }
    
    // Wait for reader to finish
    let _ = tokio::time::timeout(Duration::from_secs(2), reader_handle).await;
    
    // Check metrics
    let metrics = transport.metrics();
    println!("\n📈 Transport Metrics:");
    println!("  Writes: {}", metrics.writes_total.load(std::sync::atomic::Ordering::Relaxed));
    println!("  Reads: {}", metrics.reads_total.load(std::sync::atomic::Ordering::Relaxed));
    println!("  Notifications sent: {}", metrics.notifications_sent.load(std::sync::atomic::Ordering::Relaxed));
    println!("  Notifications received: {}", metrics.notifications_received.load(std::sync::atomic::Ordering::Relaxed));
    println!("  Dropped: {}", metrics.dropped_trades.load(std::sync::atomic::Ordering::Relaxed));
    
    println!("\n✅ Integration test complete!");
    
    Ok(())
}