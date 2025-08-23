//! Live Polygon to Relay publisher
//!
//! Connects to real Polygon blockchain and sends TLV messages directly to MarketDataRelay

use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::net::UnixStream;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

use alphapulse_adapter_service::input::collectors::PolygonDexCollector;
use alphapulse_adapter_service::input::InputAdapter;
use alphapulse_protocol_v2::{MessageHeader, RelayDomain, TLVMessage, VenueId};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    info!("🚀 Starting Live Polygon → MarketDataRelay Publisher");
    info!("   Connecting to REAL blockchain data!");

    // Connect to relay socket
    let socket_path = "/tmp/alphapulse/market_data.sock";
    let mut relay_stream = match UnixStream::connect(socket_path).await {
        Ok(stream) => {
            info!("✅ Connected to MarketDataRelay at {}", socket_path);
            stream
        }
        Err(e) => {
            error!("❌ Failed to connect to relay: {}", e);
            error!("   Make sure MarketDataRelay is running!");
            return;
        }
    };

    // Create channel for TLV messages
    let (tx, mut rx) = mpsc::channel::<TLVMessage>(1000);

    // Start Polygon DEX collector
    let mut collector = PolygonDexCollector::new(tx);

    match collector.start().await {
        Ok(()) => {
            info!("✅ Connected to live Polygon blockchain");
        }
        Err(e) => {
            error!("❌ Failed to connect to Polygon: {}", e);
            return;
        }
    }

    info!("🔄 Publishing live Polygon swaps to relay...");

    let mut messages_sent = 0u64;
    let mut sequence = 0u64;

    // Forward messages from Polygon to relay
    loop {
        match tokio::time::timeout(Duration::from_secs(5), rx.recv()).await {
            Ok(Some(tlv_message)) => {
                messages_sent += 1;
                sequence += 1;

                info!(
                    "📨 Received REAL Polygon TLV #{}: {} bytes",
                    messages_sent,
                    tlv_message.payload.len()
                );

                // Create relay message header (order must match struct definition!)
                let header = MessageHeader {
                    magic: 0xDEADBEEF,
                    relay_domain: RelayDomain::MarketData as u8,
                    version: 1,
                    source: 3, // Polygon source ID
                    flags: 0,
                    payload_size: tlv_message.payload.len() as u32,
                    sequence,
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_nanos() as u64,
                    checksum: 0, // Not validated for performance
                };

                // Serialize header (MessageHeader is repr(C, packed))
                let header_bytes = unsafe {
                    std::slice::from_raw_parts(
                        &header as *const MessageHeader as *const u8,
                        std::mem::size_of::<MessageHeader>(),
                    )
                };

                // Send header + TLV payload
                let mut message =
                    Vec::with_capacity(header_bytes.len() + tlv_message.payload.len());
                message.extend_from_slice(header_bytes);
                message.extend_from_slice(&tlv_message.payload);

                match relay_stream.write_all(&message).await {
                    Ok(()) => {
                        info!(
                            "🚀 Published to relay → Flash Arbitrage (msg #{})",
                            messages_sent
                        );
                    }
                    Err(e) => {
                        error!("Failed to send to relay: {}", e);
                        // Try to reconnect
                        match UnixStream::connect(socket_path).await {
                            Ok(stream) => {
                                info!("Reconnected to relay");
                                relay_stream = stream;
                            }
                            Err(e) => {
                                error!("Failed to reconnect: {}", e);
                                break;
                            }
                        }
                    }
                }
            }
            Ok(None) => {
                info!("Channel closed");
                break;
            }
            Err(_) => {
                // Timeout - normal during low activity
                info!(
                    "⏳ Waiting for Polygon swaps... ({} sent so far)",
                    messages_sent
                );
            }
        }
    }

    // Cleanup
    if let Err(e) = collector.stop().await {
        error!("Error stopping collector: {}", e);
    }

    info!(
        "📊 Final stats: {} messages published to relay",
        messages_sent
    );
}
