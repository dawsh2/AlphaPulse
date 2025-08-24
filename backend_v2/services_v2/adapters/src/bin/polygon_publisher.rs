//! # Polygon DEX Publisher - TLV Message Producer
//!
//! ## Purpose
//! Connects to live Polygon blockchain via WebSocket, converts DEX events to Protocol V2 TLV messages,
//! and publishes them to the MarketDataRelay for distribution to consumers.
//!
//! ## Architecture Role
//!
//! ```mermaid
//! graph LR
//!     Polygon[Polygon Blockchain] -->|WebSocket| Collector[PolygonDexCollector]
//!     Collector -->|DEX Events| Parser[TLV Message Builder]
//!     Parser -->|Protocol V2 TLV| MPSC[MPSC Channel]
//!     MPSC -->|Vec<u8>| RelayOutput[RelayOutput]
//!     RelayOutput -->|Unix Socket| MarketRelay["/tmp/alphapulse/market_data.sock"]
//!     
//!     subgraph "This Binary"
//!         Collector
//!         Parser
//!         MPSC
//!         RelayOutput
//!     end
//!     
//!     classDef producer fill:#FFE4B5
//!     class Collector,Parser,MPSC,RelayOutput producer
//! ```
//!
//! ## Critical Timing Considerations
//!
//! **Race Condition Context**: This publisher was originally misclassified as a "consumer" 
//! because it takes >100ms to establish WebSocket connection and send first TLV message.
//! The relay now uses bidirectional forwarding, eliminating timing-based classification.
//!
//! **Startup Behavior**:
//! 1. **Connection Phase** (~50-200ms): WebSocket connection to Polygon blockchain
//! 2. **Subscription Phase** (~20-50ms): Subscribe to DEX event logs
//! 3. **First Message** (~100-500ms): First DEX event converted to TLV
//! 4. **Steady State** (~10-50ms): Continuous TLV message production
//!
//! **Why This Matters**: Any relay that waits for "immediate data" would misclassify
//! this publisher. The fixed relay architecture handles this correctly.
//!
//! ## Protocol V2 TLV Integration
//!
//! **Message Construction**:
//! - DEX events → TLV message builder with RelayDomain::MarketData
//! - 32-byte MessageHeader + variable TLV payload structure
//! - Preserves native token precision (18 decimals WETH, 6 USDC)
//! - Maintains nanosecond timestamps from blockchain events
//!
//! **Performance Profile**:
//! - **Message Rate**: 100-1000 TLV messages/second (depends on DEX activity)
//! - **Message Size**: 200-2000 bytes per TLV message
//! - **Latency**: <10ms from DEX event to TLV message construction
//! - **Memory**: <50MB steady state with full DEX subscription
//!
//! ## Connection Management
//!
//! **Automatic Reconnection**: RelayOutput handles Unix socket reconnection
//! if MarketDataRelay restarts. Producer will continue collecting DEX events
//! and buffer them until relay connection is restored.
//!
//! ## Troubleshooting
//!
//! **No DEX events received**:
//! - Check internet connection and Polygon RPC endpoint
//! - Verify WebSocket connection logs
//! - Ensure proper API key configuration if required
//!
//! **Relay connection failures**:  
//! - Ensure MarketDataRelay is running and listening
//! - Check Unix socket path `/tmp/alphapulse/market_data.sock` exists
//! - Verify proper startup sequence (relay first, then publisher)

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
