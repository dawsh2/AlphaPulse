//! Polygon DEX Publisher - Host process with relay bridging
//!
//! This binary instantiates the PolygonDexCollector with an MPSC channel,
//! then bridges messages to RelayOutput. The collector sends Protocol V2
//! messages (Vec<u8>) which are forwarded directly to the MarketDataRelay.

use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

use alphapulse_adapter_service::{
    input::{collectors::PolygonDexCollector, InputAdapter},
    output::RelayOutput,
};
use protocol_v2::RelayDomain;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    info!("🚀 Starting Polygon DEX Publisher");
    info!("   Bridges collector MPSC to RelayOutput");
    info!("   Connects to LIVE Polygon blockchain via WebSocket");

    // Create MPSC channel for collector
    let (tx, mut rx) = mpsc::channel::<Vec<u8>>(1000);

    // Create RelayOutput that connects directly to MarketDataRelay
    let socket_path = "/tmp/alphapulse/market_data.sock".to_string();
    let relay_output = Arc::new(RelayOutput::new(socket_path, RelayDomain::MarketData));

    // Create Polygon DEX collector with MPSC channel
    info!("🔌 Creating Polygon DEX collector...");
    let collector = Arc::new(tokio::sync::Mutex::new(PolygonDexCollector::new(tx.clone())));

    // Start the collector first to begin receiving events
    {
        let mut coll = collector.lock().await;
        match coll.start().await {
            Ok(()) => {
                info!("✅ Connected to live Polygon DEX WebSocket");
                info!("📡 Subscribed to DEX event logs (Swap, Mint, Burn, Sync, etc.)");
            }
            Err(e) => {
                error!("❌ Failed to connect to Polygon DEX: {}", e);
                error!("   Check internet connection and RPC endpoints");
                return Err(e.into());
            }
        }
    }

    // Connect to relay (RelayOutput will send identification message)
    info!("🔌 Connecting to MarketDataRelay...");
    match relay_output.connect().await {
        Ok(()) => info!("✅ Connected to MarketDataRelay"),
        Err(e) => {
            error!("❌ Failed to connect to MarketDataRelay: {}", e);
            error!("   Make sure MarketDataRelay service is running!");
            return Err(e.into());
        }
    }

    info!("🚀 Data flow: Polygon → Collector → MPSC → RelayOutput → MarketDataRelay");

    // Spawn collector event processing task
    let collector_handle = tokio::spawn(async move {
        let mut events_processed = 0;
        loop {
            let mut coll = collector.lock().await;
            match coll.process_next_websocket_event().await {
                Ok(_) => {
                    events_processed += 1;
                    if events_processed % 100 == 0 {
                        info!("📊 Processed {} DEX events from Polygon", events_processed);
                    }
                }
                Err(e) => {
                    error!("❌ WebSocket event error: {}", e);
                    // WebSocket will reconnect automatically via the collector's logic
                    drop(coll); // Release lock before sleeping
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                }
            }
        }
    });

    info!("🔄 Bridging messages from collector to relay...");

    // Bridge messages from MPSC to RelayOutput
    let mut messages_sent = 0;
    while let Some(message_bytes) = rx.recv().await {
        messages_sent += 1;

        // Forward Protocol V2 message to relay
        match relay_output.send_bytes(message_bytes).await {
            Ok(()) => {
                if messages_sent <= 5 || messages_sent % 100 == 0 {
                    info!("🚀 Forwarded {} messages to MarketDataRelay", messages_sent);
                }
            }
            Err(e) => {
                warn!("⚠️ Failed to send message to relay: {}", e);
                // Relay will reconnect on next send attempt
            }
        }
    }

    // If we get here, the channel was closed
    warn!("📴 Collector channel closed");
    collector_handle.abort();

    info!(
        "🎯 Polygon Publisher stopped after {} messages",
        messages_sent
    );
    Ok(())
}
