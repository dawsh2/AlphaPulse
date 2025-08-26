//! Relay Output Adapter - Sends Protocol V2 binary messages directly to relay sockets
//!
//! This adapter allows collectors to send their Protocol V2 messages (built with
//! TLVMessageBuilder) directly to the appropriate relay (MarketData, Signal, or Execution).
//!
//! The collector builds messages using TLVMessageBuilder::build() which returns Vec<u8>,
//! then sends them through this adapter for direct relay delivery.

use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::net::UnixStream;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

use crate::Result;
use alphapulse_types::RelayDomain;

/// Output adapter that sends Protocol V2 binary messages to a relay socket
pub struct RelayOutput {
    socket_path: String,
    stream: Arc<Mutex<Option<UnixStream>>>,
    relay_domain: RelayDomain,
    messages_sent: Arc<Mutex<u64>>,
}

impl RelayOutput {
    /// Create a new relay output adapter
    /// Note: source_id parameter removed - messages should include their own source in the header
    pub fn new(socket_path: String, relay_domain: RelayDomain) -> Self {
        Self {
            socket_path,
            stream: Arc::new(Mutex::new(None)),
            relay_domain,
            messages_sent: Arc::new(Mutex::new(0)),
        }
    }

    /// Connect to the relay socket
    pub async fn connect(&self) -> Result<()> {
        info!("🔌 Connecting to relay at: {}", self.socket_path);

        match UnixStream::connect(&self.socket_path).await {
            Ok(mut stream) => {
                info!("✅ Connected to {:?} relay", self.relay_domain);

                // Send a small identification message immediately to be classified as publisher
                // This is a minimal Protocol V2 header (32 bytes) with zero payload
                // CORRECTED MessageHeader field order: magic(4), relay_domain(1), version(1), source(1), flags(1), sequence(8), timestamp(8), payload_size(4), checksum(4)
                let identification_header = [
                    // magic: 0xDEADBEEF (4 bytes, little endian) - CRITICAL: FIRST 4 BYTES FOR IMMEDIATE PROTOCOL ID
                    0xEF, 0xBE, 0xAD, 0xDE, // relay_domain: MarketData (1 byte)
                    0x01, // version: 1 (1 byte)
                    0x01, // source: PolygonCollector (1 byte)
                    0x02, // flags: 0 (1 byte)
                    0x00, // sequence: 0 (8 bytes)
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    // timestamp: 0 (8 bytes)
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    // payload_size: 0 (4 bytes)
                    0x00, 0x00, 0x00, 0x00, // checksum: 0 (4 bytes)
                    0x00, 0x00, 0x00, 0x00,
                ];

                match stream.write_all(&identification_header).await {
                    Ok(()) => debug!("📡 Sent identification message to establish publisher role"),
                    Err(e) => warn!("Failed to send identification message: {}", e),
                }

                *self.stream.lock().await = Some(stream);
                Ok(())
            }
            Err(e) => {
                error!("❌ Failed to connect to relay: {}", e);
                Err(crate::AdapterError::Io(e))
            }
        }
    }

    /// Send a Protocol V2 binary message to the relay
    /// The message should be built using TLVMessageBuilder::build() which returns Vec<u8>
    /// This message already contains the complete Protocol V2 header and TLV payload
    pub async fn send_bytes(&self, message_bytes: &[u8]) -> Result<()> {
        // Ensure we're connected
        let mut stream_guard = self.stream.lock().await;
        if stream_guard.is_none() {
            drop(stream_guard);
            self.connect().await?;
            stream_guard = self.stream.lock().await;
        }

        if let Some(ref mut stream) = *stream_guard {
            // Send the pre-built Protocol V2 message directly
            match stream.write_all(&message_bytes).await {
                Ok(()) => {
                    let mut count = self.messages_sent.lock().await;
                    *count += 1;
                    let total = *count;
                    drop(count);

                    debug!(
                        "📨 Sent Protocol V2 message #{} to {:?} relay ({} bytes)",
                        total,
                        self.relay_domain,
                        message_bytes.len()
                    );

                    if total <= 5 || total % 1000 == 0 {
                        info!(
                            "📊 RelayOutput stats: {} messages sent to {:?} relay",
                            total, self.relay_domain
                        );
                    }

                    Ok(())
                }
                Err(e) => {
                    warn!("Failed to send to relay: {}", e);
                    // Reset connection on error
                    *stream_guard = None;
                    Err(crate::AdapterError::Io(e))
                }
            }
        } else {
            Err(crate::AdapterError::ConnectionTimeout {
                venue: alphapulse_types::protocol::VenueId::Generic, // Use Generic venue for relay
                timeout_ms: 0,
            })
        }
    }

    /// Get statistics
    pub async fn stats(&self) -> RelayOutputStats {
        RelayOutputStats {
            connected: self.stream.lock().await.is_some(),
            messages_sent: *self.messages_sent.lock().await,
            relay_domain: self.relay_domain,
            socket_path: self.socket_path.clone(),
        }
    }
}

/// Statistics for relay output
#[derive(Debug, Clone)]
pub struct RelayOutputStats {
    /// Whether the relay is currently connected
    pub connected: bool,
    /// Total messages sent to this relay
    pub messages_sent: u64,
    /// Domain this relay serves (MarketData, Signal, or Execution)
    pub relay_domain: RelayDomain,
    /// Unix socket path for relay connection
    pub socket_path: String,
}
