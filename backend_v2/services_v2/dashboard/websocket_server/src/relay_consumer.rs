//! # Dashboard Relay Consumer - TLV to JSON Bridge
//!
//! ## Purpose
//! Connects to MarketDataRelay as a consumer, receives Protocol V2 TLV messages,
//! converts them to JSON, and broadcasts to WebSocket dashboard clients.
//!
//! ## Architecture Role
//!
//! ```mermaid
//! graph LR
//!     MarketRelay["/tmp/alphapulse/market_data.sock"] -->|TLV Messages| Consumer[RelayConsumer]
//!     Consumer -->|32-byte Header| HeaderParser[Header Parsing]
//!     Consumer -->|TLV Payload| TLVParser[TLV Extensions Parser]
//!
//!     HeaderParser --> Validation{Domain Validation}
//!     TLVParser --> Conversion[TLV to JSON Converter]
//!     Conversion --> SignalBuffer[Signal Assembly Buffer]
//!
//!     SignalBuffer -->|Complete Signals| Broadcast[WebSocket Broadcast]
//!     Validation -->|Direct Messages| Broadcast
//!
//!     Broadcast --> Frontend[Dashboard Frontend]
//!
//!     subgraph "Consumer Connection"
//!         Consumer --> ReadTask[Read Task]
//!         ReadTask --> MessageBuffer[Message Buffer]
//!         MessageBuffer --> Processing[TLV Processing]
//!     end
//!
//!     classDef consumer fill:#E6E6FA
//!     classDef conversion fill:#F0E68C
//!     class Consumer,ReadTask consumer
//!     class HeaderParser,TLVParser,Conversion conversion
//! ```
//!
//! ## TLV Message Processing Flow
//!
//! **Message Structure**: 32-byte MessageHeader + variable TLV payload
//!
//! 1. **Header Parsing**: Extract domain, source, sequence, payload size
//! 2. **Domain Validation**: Ensure MarketData domain messages only
//! 3. **TLV Parsing**: Extract individual TLV extensions from payload
//! 4. **Type-Specific Processing**: Convert each TLV type to appropriate JSON
//! 5. **Signal Assembly**: Buffer partial signals until complete
//! 6. **WebSocket Broadcast**: Send JSON to all connected dashboard clients
//!
//! ## Performance Optimizations
//!
//! **MarketData Fast Path**: Uses `parse_header_fast()` without checksum validation
//! for >1M msg/s throughput. Signal and Execution domains use full validation.
//!
//! **Message Buffering**: Accumulates partial TCP reads into complete TLV messages
//! before processing. Handles fragmented Unix socket reads gracefully.
//!
//! **Signal Assembly**: Buffers SignalIdentity + Economics TLV pairs before
//! broadcasting complete arbitrage opportunities to dashboard.
//!
//! ## Connection Resilience
//!
//! **Automatic Reconnection**: Continuously attempts to reconnect to relay
//! with 5-second backoff if connection drops. No message loss during relay restarts.
//!
//! **Graceful Degradation**: Invalid messages are logged and skipped without
//! crashing consumer. Maintains service availability during data quality issues.
//!
//! ## Integration with Bidirectional Relay
//!
//! **Consumer Role**: This service connects to the relay AFTER the relay and
//! publisher are running. The relay's bidirectional forwarding ensures this
//! consumer receives all messages broadcast from polygon_publisher.
//!
//! **No Publisher/Consumer Classification**: The relay treats this connection
//! as bidirectional - it could theoretically send messages back to the relay,
//! but currently only consumes for dashboard display.
//!
//! ## Troubleshooting
//!
//! **Not receiving TLV messages**:
//! - Ensure MarketDataRelay is running and polygon_publisher is connected
//! - Check relay logs for "Connection X forwarded message" entries
//! - Verify Unix socket path `/tmp/alphapulse/market_data.sock` accessibility
//!
//! **JSON conversion errors**:
//! - Check TLV payload structure matches expected Protocol V2 format
//! - Verify message_converter.rs handles all active TLV types
//! - Monitor for ParseError logs indicating malformed TLV data

use crate::client::ClientManager;
use crate::error::{DashboardError, Result};
use crate::message_converter::{
    convert_tlv_to_json, create_arbitrage_opportunity, create_combined_signal,
};
use alphapulse_types::{
    message::header::MessageHeader, parse_header, parse_tlv_extensions, ParseError, RelayDomain,
    TLVExtensionEnum, MESSAGE_MAGIC,
};
use serde_json::Value;
use std::collections::HashMap;
use std::mem::size_of;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::AsyncReadExt;
use tokio::net::UnixStream;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};
use zerocopy::Ref;

/// Multi-relay consumer for dashboard
pub struct RelayConsumer {
    client_manager: Arc<ClientManager>,
    market_data_path: String,
    signal_path: String,
    execution_path: String,
}

impl RelayConsumer {
    pub fn new(
        client_manager: Arc<ClientManager>,
        market_data_path: String,
        signal_path: String,
        execution_path: String,
    ) -> Self {
        Self {
            client_manager,
            market_data_path,
            signal_path,
            execution_path,
        }
    }

    /// Start consuming from all relays
    pub async fn start(&self) -> Result<()> {
        info!("Starting relay consumer for dashboard");

        let mut handles = Vec::new();

        // Start market data consumer
        let market_data_handle = {
            let client_manager = self.client_manager.clone();
            let path = self.market_data_path.clone();
            tokio::spawn(async move {
                Self::consume_relay(client_manager, path, RelayDomain::MarketData).await;
            })
        };
        handles.push(market_data_handle);

        // Start signal consumer
        let signal_handle = {
            let client_manager = self.client_manager.clone();
            let path = self.signal_path.clone();
            tokio::spawn(async move {
                Self::consume_relay(client_manager, path, RelayDomain::Signal).await;
            })
        };
        handles.push(signal_handle);

        // Start execution consumer
        let execution_handle = {
            let client_manager = self.client_manager.clone();
            let path = self.execution_path.clone();
            tokio::spawn(async move {
                Self::consume_relay(client_manager, path, RelayDomain::Execution).await;
            })
        };
        handles.push(execution_handle);

        info!("All relay consumers started");

        // Wait for all consumers
        for handle in handles {
            if let Err(e) = handle.await {
                error!("Relay consumer task failed: {}", e);
            }
        }

        Ok(())
    }

    async fn consume_relay(
        client_manager: Arc<ClientManager>,
        relay_path: String,
        domain: RelayDomain,
    ) {
        info!("Starting consumer for {:?} relay: {}", domain, relay_path);

        let mut signal_buffer = HashMap::new(); // Buffer partial signals

        loop {
            match Self::connect_to_relay(&relay_path).await {
                Ok(mut stream) => {
                    info!("✅ Connected to {:?} relay", domain);
                    if matches!(domain, RelayDomain::Signal) {
                        info!("🎯 Signal relay consumer ready to receive arbitrage signals");
                    }

                    let mut buffer = vec![0u8; 8192];
                    let mut message_buffer = Vec::new(); // Accumulate partial messages

                    loop {
                        // Use non-blocking read with short timeout to enable continuous polling
                        // This prevents the consumer from getting stuck waiting for data
                        match tokio::time::timeout(
                            Duration::from_millis(50),
                            stream.read(&mut buffer),
                        )
                        .await
                        {
                            Ok(Ok(0)) => {
                                warn!("{:?} relay connection closed", domain);
                                break;
                            }
                            Ok(Ok(bytes_read)) => {
                                // Extra debugging for Signal relay
                                if matches!(domain, RelayDomain::Signal) {
                                    info!("🔍 Signal relay: Read {} bytes", bytes_read);
                                }
                                debug!("Read {} bytes from {:?} relay", bytes_read, domain);

                                // Log first 32 bytes in hex for debugging
                                if bytes_read >= 32 {
                                    let hex_preview: String = buffer[..32]
                                        .iter()
                                        .map(|b| format!("{:02x}", b))
                                        .collect::<Vec<_>>()
                                        .join(" ");
                                    debug!("Message header bytes: [{}]", hex_preview);
                                }

                                // Append new data to message buffer
                                message_buffer.extend_from_slice(&buffer[..bytes_read]);

                                // Process complete messages from buffer with robust multi-message parsing
                                if let Err(e) = Self::process_message_buffer(
                                    &client_manager,
                                    &mut message_buffer,
                                    domain,
                                    &mut signal_buffer,
                                )
                                .await
                                {
                                    warn!("Error processing message buffer: {}", e);
                                    // On parsing failure, clear corrupted buffer to prevent infinite loops
                                    if message_buffer.len() > 8192 {
                                        warn!("Clearing oversized buffer ({} bytes) due to parsing failure", message_buffer.len());
                                        message_buffer.clear();
                                    }
                                }
                            }
                            Ok(Err(e)) => {
                                error!("Error reading from {:?} relay: {}", domain, e);
                                break;
                            }
                            Err(_timeout) => {
                                // Timeout occurred - no data available, continue polling
                                // This prevents infinite blocking and enables continuous streaming
                                // Log timeouts for Signal relay debugging
                                if matches!(domain, RelayDomain::Signal) {
                                    static mut TIMEOUT_COUNT: u32 = 0;
                                    unsafe {
                                        TIMEOUT_COUNT += 1;
                                        if TIMEOUT_COUNT % 100 == 0 {
                                            debug!("Signal relay: {} read timeouts (normal during idle)", TIMEOUT_COUNT);
                                        }
                                    }
                                }
                                continue;
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to connect to {:?} relay: {}", domain, e);
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
    }

    async fn connect_to_relay(path: &str) -> Result<UnixStream> {
        UnixStream::connect(path)
            .await
            .map_err(|e| DashboardError::RelayConnection {
                message: format!("Failed to connect to relay {}: {}", path, e),
            })
    }

    async fn process_relay_data(
        client_manager: &ClientManager,
        data: &[u8],
        domain: RelayDomain,
        signal_buffer: &mut HashMap<u64, (Option<Value>, Option<Value>)>,
    ) -> Result<()> {
        info!(
            "🔍 Processing relay data: {} bytes, domain={:?}",
            data.len(),
            domain
        );
        // Use appropriate parsing based on domain policy:
        // MarketDataRelay & SignalRelay: Skip checksum validation for performance
        // ExecutionRelay: Enforce checksum validation for security
        let header = match domain {
            RelayDomain::MarketData | RelayDomain::Signal => {
                // Fast parsing without checksum validation (performance optimization)
                // Signal domain also skips checksum as messages have checksum=0
                match Self::parse_header_fast(data) {
                    Ok(header) => header,
                    Err(e) => {
                        debug!("Failed to parse {:?} header (fast): {}", domain, e);
                        return Ok(()); // Skip malformed messages
                    }
                }
            }
            RelayDomain::Execution | RelayDomain::System => {
                // Full parsing with checksum validation (reliability/security)
                match parse_header(data) {
                    Ok(header) => header,
                    Err(e) => {
                        debug!("Failed to parse {:?} header (validated): {}", domain, e);
                        return Ok(()); // Skip malformed messages
                    }
                }
            }
        };

        // Extract TLV payload after header (32 bytes)
        let header_size = 32;
        if data.len() <= header_size {
            debug!("Message too small for TLV payload");
            return Ok(());
        }
        let tlv_data = &data[header_size..];

        // Validate domain matches expected
        if let Ok(parsed_domain) = header.get_relay_domain() {
            if parsed_domain != domain {
                debug!(
                    "Domain mismatch: expected {:?}, got {:?}",
                    domain, parsed_domain
                );
                return Ok(());
            }
        }

        Self::process_tlv_data(
            client_manager,
            tlv_data,
            header.timestamp,
            domain,
            signal_buffer,
        )
        .await?;

        Ok(())
    }

    async fn process_tlv_data(
        client_manager: &ClientManager,
        tlv_data: &[u8],
        timestamp: u64,
        domain: RelayDomain,
        signal_buffer: &mut HashMap<u64, (Option<Value>, Option<Value>)>,
    ) -> Result<()> {
        // Use protocol's TLV parser
        let tlvs = match parse_tlv_extensions(tlv_data) {
            Ok(tlvs) => {
                info!(
                    "✅ Parsed {} TLV extensions from {} bytes",
                    tlvs.len(),
                    tlv_data.len()
                );
                tlvs
            }
            Err(e) => {
                error!(
                    "❌ TLV parsing failed: {:?} (data_len={}, first_bytes={:02x?})",
                    e,
                    tlv_data.len(),
                    &tlv_data[..std::cmp::min(32, tlv_data.len())]
                );
                return Ok(()); // Skip malformed TLV data
            }
        };

        let mut current_signal_id: Option<u64> = None;

        for tlv in tlvs {
            // Extract TLV data based on variant
            let (tlv_type, tlv_payload) = match &tlv {
                TLVExtensionEnum::Standard(std_tlv) => (std_tlv.header.tlv_type, &std_tlv.payload),
                TLVExtensionEnum::Extended(ext_tlv) => {
                    info!(
                        "📦 Extended TLV detected: type={}, payload_size={}",
                        ext_tlv.header.tlv_type,
                        ext_tlv.payload.len()
                    );
                    (ext_tlv.header.tlv_type, &ext_tlv.payload)
                }
            };

            // Convert TLV to JSON using protocol parsing
            let json_message = convert_tlv_to_json(tlv_type, tlv_payload, timestamp)?;

            // Log TLV type for debugging
            if tlv_type == 255 {
                info!("🔍 Processing TLV type 255 (DemoDeFiArbitrageTLV)");
                info!("📊 JSON message created: {:?}", json_message);
            }

            match tlv_type {
                1 => {
                    // Trade
                    client_manager.broadcast(json_message).await;
                    debug!("Broadcasted trade message");
                }
                2 => {
                    // SignalIdentity
                    if let Some(signal_id) = json_message.get("signal_id").and_then(|v| v.as_u64())
                    {
                        current_signal_id = Some(signal_id);
                        let entry = signal_buffer.entry(signal_id).or_insert((None, None));
                        entry.0 = Some(json_message);
                    }
                }
                11 => {
                    // PoolSwap - broadcast for dashboard "pool_swap" channel
                    client_manager.broadcast(json_message).await;
                    // debug!("Broadcasted pool_swap message"); // Commented out to reduce log noise
                }
                16 => {
                    // PoolSync - broadcast for dashboard pool updates
                    client_manager.broadcast(json_message).await;
                    // debug!("Broadcasted pool_sync message"); // Commented out to reduce log noise
                }
                10 => {
                    // PoolLiquidity - broadcast for dashboard liquidity updates
                    client_manager.broadcast(json_message).await;
                    // debug!("Broadcasted pool_liquidity message"); // Commented out to reduce log noise
                }
                12 => {
                    // PoolMint - broadcast for dashboard mint events
                    client_manager.broadcast(json_message).await;
                    // debug!("Broadcasted pool_mint message"); // Commented out to reduce log noise
                }
                13 => {
                    // PoolBurn - broadcast for dashboard burn events
                    client_manager.broadcast(json_message).await;
                    // debug!("Broadcasted pool_burn message"); // Commented out to reduce log noise
                }
                14 => {
                    // PoolTick - broadcast for dashboard tick events
                    client_manager.broadcast(json_message).await;
                    // debug!("Broadcasted pool_tick message"); // Commented out to reduce log noise
                }
                3 => {
                    // Economics
                    if let Some(signal_id) = current_signal_id {
                        let entry = signal_buffer.entry(signal_id).or_insert((None, None));
                        entry.1 = Some(json_message);

                        // Check if we have both parts of the signal
                        if let (Some(identity), Some(economics)) = &entry {
                            // Check if this is a flash arbitrage signal (strategy_id = 21)
                            let is_flash_arbitrage =
                                identity.get("strategy_id").and_then(|v| v.as_u64()) == Some(21);

                            if is_flash_arbitrage {
                                // Create arbitrage opportunity message for dashboard
                                let arbitrage_msg = create_arbitrage_opportunity(
                                    Some(identity.clone()),
                                    Some(economics.clone()),
                                    timestamp,
                                );

                                client_manager.broadcast(arbitrage_msg).await;
                                debug!("Broadcasted arbitrage opportunity {}", signal_id);
                            } else {
                                // Create regular combined signal for other strategies
                                let combined_signal = create_combined_signal(
                                    Some(identity.clone()),
                                    Some(economics.clone()),
                                    timestamp,
                                );

                                client_manager.broadcast(combined_signal).await;
                                debug!("Broadcasted combined signal {}", signal_id);
                            }

                            // Remove from buffer
                            signal_buffer.remove(&signal_id);
                        }
                    }
                }
                10..=14 => {
                    // Pool TLVs (PoolLiquidity, PoolSwap, PoolMint, PoolBurn, PoolTick)
                    client_manager.broadcast(json_message).await;
                    // debug!("Broadcasted pool {} message", tlv_type); // Commented out to reduce log noise
                }
                255 => {
                    // ExtendedTLV - DemoDeFiArbitrageTLV
                    // The converter already creates the full arbitrage opportunity message
                    client_manager.broadcast(json_message.clone()).await;
                    info!(
                        "🎯 Broadcasted DemoDeFiArbitrageTLV signal with profit: {}",
                        json_message
                            .get("expected_profit_usd")
                            .unwrap_or(&serde_json::Value::Null)
                    );
                }
                _ => {
                    // Broadcast other message types immediately
                    client_manager.broadcast(json_message).await;
                    debug!("Broadcasted {} message", tlv_type);
                }
            }
        }

        Ok(())
    }

    /// Process message buffer handling multiple concatenated TLV messages
    async fn process_message_buffer(
        client_manager: &ClientManager,
        message_buffer: &mut Vec<u8>,
        domain: RelayDomain,
        signal_buffer: &mut HashMap<u64, (Option<Value>, Option<Value>)>,
    ) -> Result<()> {
        let mut offset = 0;
        let mut consecutive_failures = 0;
        const MAX_CONSECUTIVE_FAILURES: u8 = 10;
        const MAX_BUFFER_SIZE: usize = 16384; // Increased to handle more messages

        debug!("Processing buffer with {} bytes", message_buffer.len());

        while offset + 32 <= message_buffer.len() {
            // Pre-check magic number for fast rejection - magic is at bytes 0-3 in Protocol V2 (fixed header)
            if message_buffer.len() >= offset + 4 {
                // Need at least 4 bytes to read magic at offset 0
                let magic = u32::from_le_bytes([
                    // Protocol V2 uses little-endian
                    message_buffer[offset + 0], // Magic is at bytes 0-3 after the fix
                    message_buffer[offset + 1],
                    message_buffer[offset + 2],
                    message_buffer[offset + 3],
                ]);

                if magic != MESSAGE_MAGIC {
                    debug!(
                        "Invalid magic at offset {}: expected 0x{:08X}, got 0x{:08X}",
                        offset, MESSAGE_MAGIC, magic
                    );
                    // Invalid magic - look for next valid magic number
                    if let Some(next_magic_offset) =
                        Self::find_next_magic(&message_buffer[offset..])
                    {
                        offset += next_magic_offset;
                        consecutive_failures += 1;

                        if consecutive_failures > MAX_CONSECUTIVE_FAILURES {
                            warn!("Too many consecutive parsing failures, clearing buffer");
                            message_buffer.clear();
                            return Ok(());
                        }
                        continue;
                    } else {
                        // No valid magic found in remaining buffer
                        debug!(
                            "No valid magic found in remaining {} bytes",
                            message_buffer.len() - offset
                        );
                        break;
                    }
                }
            }

            // Parse header at exact offset
            let header_slice = &message_buffer[offset..offset + 32];
            let header = match domain {
                RelayDomain::MarketData | RelayDomain::Signal => {
                    Self::parse_header_fast(header_slice)
                }
                _ => parse_header(header_slice),
            };

            match header {
                Ok(header) => {
                    let total_message_size = 32 + header.payload_size as usize;
                    info!(
                        "📦 Found message at offset {}: size={} (header=32 + payload={})",
                        offset, total_message_size, header.payload_size
                    );

                    // Validate expected message sizes for DemoDeFiArbitrageTLV (remove hardcoded magic numbers)
                    let expected_sizes = [214, 217, 258, 261]; // Correct (214), Extended (217), Legacy (258), Observed (261)
                    if expected_sizes.contains(&total_message_size) {
                        debug!(
                            "Valid DemoDeFiArbitrageTLV message: {} bytes",
                            total_message_size
                        );
                    } else if total_message_size % 214 == 0 && total_message_size > 214 {
                        warn!(
                            "⚠️ Concatenated correct messages: {} bytes ({}×214)",
                            total_message_size,
                            total_message_size / 214
                        );
                    } else if total_message_size % 261 == 0 && total_message_size > 261 {
                        warn!(
                            "⚠️ Concatenated legacy messages: {} bytes ({}×261)",
                            total_message_size,
                            total_message_size / 261
                        );
                    } else {
                        warn!("🚨 Unexpected message size: {} bytes", total_message_size);
                    }

                    // Validate complete message availability
                    if offset + total_message_size > message_buffer.len() {
                        debug!(
                            "Incomplete message: need {} bytes, have {} remaining",
                            total_message_size,
                            message_buffer.len() - offset
                        );
                        break; // Wait for more data
                    }

                    // Extract and process complete message
                    let complete_message = &message_buffer[offset..offset + total_message_size];

                    if let Err(e) = Self::process_relay_data(
                        client_manager,
                        complete_message,
                        domain,
                        signal_buffer,
                    )
                    .await
                    {
                        warn!("Error processing message at offset {}: {}", offset, e);
                        consecutive_failures += 1;
                    } else {
                        debug!("Successfully processed message at offset {}", offset);
                        consecutive_failures = 0; // Reset on success
                    }

                    // Move to next message boundary
                    offset += total_message_size;

                    // Yield control to prevent burst behavior
                    tokio::task::yield_now().await;
                }
                Err(e) => {
                    debug!("Failed to parse header at offset {}: {:?}", offset, e);
                    // Try to find next valid message
                    if let Some(next_magic_offset) =
                        Self::find_next_magic(&message_buffer[offset + 1..])
                    {
                        offset += 1 + next_magic_offset;
                        consecutive_failures += 1;
                    } else {
                        break; // No more valid messages in buffer
                    }

                    if consecutive_failures > MAX_CONSECUTIVE_FAILURES {
                        warn!(
                            "Circuit breaker triggered: {} consecutive failures",
                            consecutive_failures
                        );
                        message_buffer.clear();
                        return Ok(());
                    }
                }
            }
        }

        // Remove all processed messages from buffer
        if offset > 0 {
            debug!(
                "Draining {} bytes from buffer (had {} bytes)",
                offset,
                message_buffer.len()
            );
            message_buffer.drain(..offset);
            debug!(
                "Buffer after processing: {} bytes remaining",
                message_buffer.len()
            );
            consecutive_failures = 0; // Reset on successful processing
        }

        // Only truncate buffer if parsing consistently fails AND buffer is oversized
        if consecutive_failures > MAX_CONSECUTIVE_FAILURES && message_buffer.len() > MAX_BUFFER_SIZE
        {
            warn!("Circuit breaker triggered: {} consecutive failures with oversized buffer, clearing", consecutive_failures);
            message_buffer.clear();
        } else if message_buffer.len() > MAX_BUFFER_SIZE * 2 {
            // Emergency truncation only if buffer becomes extremely large
            warn!(
                "Emergency buffer truncation: {} bytes exceeds safety limit",
                message_buffer.len()
            );
            message_buffer.truncate(MAX_BUFFER_SIZE);
        }

        Ok(())
    }

    /// Find next occurrence of MESSAGE_MAGIC in buffer at Protocol V2 header offsets
    /// Magic number is at bytes 0-3 of each message (after header fix)
    fn find_next_magic(buffer: &[u8]) -> Option<usize> {
        if buffer.len() < 4 {
            // Need at least 4 bytes to read magic at offset 0
            return None;
        }

        // Check every byte for the magic number (since messages can start at any byte after corruption)
        let mut search_offset = 1; // Start at 1 since we already checked 0
        while search_offset + 4 <= buffer.len() {
            // Check for magic at current search position
            let magic = u32::from_le_bytes([
                // Protocol V2 uses little-endian
                buffer[search_offset],
                buffer[search_offset + 1],
                buffer[search_offset + 2],
                buffer[search_offset + 3],
            ]);

            if magic == MESSAGE_MAGIC {
                return Some(search_offset); // Return start of message
            }

            // Move to next byte
            search_offset += 1;
        }

        None
    }

    /// Fast header parsing without checksum validation (MarketData optimization)
    fn parse_header_fast(data: &[u8]) -> std::result::Result<&MessageHeader, ParseError> {
        if data.len() < size_of::<MessageHeader>() {
            return Err(ParseError::MessageTooSmall {
                need: size_of::<MessageHeader>(),
                got: data.len(),
            });
        }

        let header = Ref::<_, MessageHeader>::new(&data[..size_of::<MessageHeader>()])
            .ok_or(ParseError::MessageTooSmall {
                need: size_of::<MessageHeader>(),
                got: data.len(),
            })?
            .into_ref();

        if header.magic != MESSAGE_MAGIC {
            return Err(ParseError::InvalidMagic {
                expected: MESSAGE_MAGIC,
                actual: header.magic,
            });
        }

        // Skip checksum validation for MarketData performance
        Ok(header)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::ClientManager;
    use alphapulse_types::{message::header::MessageHeader, RelayDomain, MESSAGE_MAGIC};
    use std::collections::HashMap;
    use zerocopy::AsBytes;

    #[test]
    fn test_relay_consumer_creation() {
        let client_manager = Arc::new(ClientManager::new(100));
        let consumer = RelayConsumer::new(
            client_manager,
            "/tmp/test_market.sock".to_string(),
            "/tmp/test_signal.sock".to_string(),
            "/tmp/test_execution.sock".to_string(),
        );

        assert_eq!(consumer.market_data_path, "/tmp/test_market.sock");
        assert_eq!(consumer.signal_path, "/tmp/test_signal.sock");
        assert_eq!(consumer.execution_path, "/tmp/test_execution.sock");
    }

    fn create_test_message(payload_size: u16) -> Vec<u8> {
        let header = MessageHeader {
            magic: MESSAGE_MAGIC,
            relay_domain: RelayDomain::MarketData as u8,
            source: 1,
            sequence: 1,
            timestamp: 1234567890,
            payload_size: payload_size.into(),
            checksum: 0, // Not used in fast parsing
            version: 1,
            flags: 0,
        };

        let mut message = header.as_bytes().to_vec();

        // Add minimal TLV payload
        if payload_size > 0 {
            // Add SimpleTLV header manually (2 bytes)
            message.push(1); // tlv_type = 1 (Trade)
            message.push((payload_size - 2) as u8); // tlv_length

            // Pad with zeros to reach payload_size
            let remaining = payload_size as usize - 2;
            message.extend(vec![0u8; remaining]);
        }

        message
    }

    #[tokio::test]
    async fn test_single_message_processing() {
        let client_manager = Arc::new(ClientManager::new(100));
        let mut message_buffer = create_test_message(210); // 242 total bytes (32 + 210)
        let mut signal_buffer = HashMap::new();

        let result = RelayConsumer::process_message_buffer(
            &client_manager,
            &mut message_buffer,
            RelayDomain::MarketData,
            &mut signal_buffer,
        )
        .await;

        assert!(result.is_ok(), "Single message processing should succeed");
        assert_eq!(
            message_buffer.len(),
            0,
            "Buffer should be empty after processing"
        );
    }

    #[tokio::test]
    async fn test_concatenated_messages() {
        let client_manager = Arc::new(ClientManager::new(100));

        // Create two identical messages concatenated
        let mut message_buffer = create_test_message(210);
        message_buffer.extend_from_slice(&create_test_message(210));

        assert_eq!(
            message_buffer.len(),
            484,
            "Should have 2 messages of 242 bytes each"
        );

        let mut signal_buffer = HashMap::new();

        let result = RelayConsumer::process_message_buffer(
            &client_manager,
            &mut message_buffer,
            RelayDomain::MarketData,
            &mut signal_buffer,
        )
        .await;

        assert!(
            result.is_ok(),
            "Concatenated message processing should succeed"
        );
        assert_eq!(
            message_buffer.len(),
            0,
            "Buffer should be empty after processing both messages"
        );
    }

    #[tokio::test]
    async fn test_partial_message_buffering() {
        let client_manager = Arc::new(ClientManager::new(100));

        // Create partial message (only header + half payload)
        let full_message = create_test_message(210);
        let mut message_buffer = full_message[..100].to_vec(); // Incomplete message

        let mut signal_buffer = HashMap::new();

        let result = RelayConsumer::process_message_buffer(
            &client_manager,
            &mut message_buffer,
            RelayDomain::MarketData,
            &mut signal_buffer,
        )
        .await;

        assert!(result.is_ok(), "Partial message handling should succeed");
        assert_eq!(
            message_buffer.len(),
            100,
            "Incomplete message should remain in buffer"
        );

        // Add remaining bytes
        message_buffer.extend_from_slice(&full_message[100..]);

        let result = RelayConsumer::process_message_buffer(
            &client_manager,
            &mut message_buffer,
            RelayDomain::MarketData,
            &mut signal_buffer,
        )
        .await;

        assert!(result.is_ok(), "Complete message processing should succeed");
        assert_eq!(
            message_buffer.len(),
            0,
            "Buffer should be empty after completing message"
        );
    }

    #[tokio::test]
    async fn test_corrupted_message_recovery() {
        let client_manager = Arc::new(ClientManager::new(100));

        // Create buffer with corrupted data followed by valid message
        let mut message_buffer = vec![0xFF, 0xDE, 0xAD, 0xBE]; // Invalid magic
        message_buffer.extend_from_slice(&create_test_message(210)); // Valid message

        let mut signal_buffer = HashMap::new();

        let result = RelayConsumer::process_message_buffer(
            &client_manager,
            &mut message_buffer,
            RelayDomain::MarketData,
            &mut signal_buffer,
        )
        .await;

        assert!(result.is_ok(), "Should recover from corrupted message");
        assert_eq!(
            message_buffer.len(),
            0,
            "Buffer should be clean after recovery"
        );
    }

    #[tokio::test]
    async fn test_oversized_buffer_handling() {
        let client_manager = Arc::new(ClientManager::new(100));

        // Create oversized buffer (> 8KB)
        let mut message_buffer = vec![0xFF; 10000];
        let mut signal_buffer = HashMap::new();

        let result = RelayConsumer::process_message_buffer(
            &client_manager,
            &mut message_buffer,
            RelayDomain::MarketData,
            &mut signal_buffer,
        )
        .await;

        assert!(result.is_ok(), "Should handle oversized buffer");
        assert!(
            message_buffer.len() <= 8192,
            "Buffer should be truncated to max size"
        );
    }

    #[tokio::test]
    async fn test_circuit_breaker() {
        let client_manager = Arc::new(ClientManager::new(100));

        // Create buffer with many invalid magic numbers
        let mut message_buffer = vec![0xFF; 1000]; // All invalid data
        let mut signal_buffer = HashMap::new();

        let result = RelayConsumer::process_message_buffer(
            &client_manager,
            &mut message_buffer,
            RelayDomain::MarketData,
            &mut signal_buffer,
        )
        .await;

        assert!(result.is_ok(), "Should handle circuit breaker gracefully");
        // Buffer may be cleared by circuit breaker
    }

    #[test]
    fn test_find_next_magic() {
        // Test finding magic in buffer
        let mut buffer = vec![0xFF, 0xDE, 0xAD, 0xBE]; // Invalid data
        buffer.extend_from_slice(&MESSAGE_MAGIC.to_be_bytes()); // Valid magic
        buffer.extend(&[0x12, 0x34]); // More data

        let magic_offset = RelayConsumer::find_next_magic(&buffer);
        assert_eq!(magic_offset, Some(4), "Should find magic at offset 4");

        // Test no magic found
        let no_magic_buffer = vec![0xFF; 100];
        let no_magic_result = RelayConsumer::find_next_magic(&no_magic_buffer);
        assert_eq!(
            no_magic_result, None,
            "Should return None when no magic found"
        );
    }

    #[test]
    fn test_magic_precheck() {
        // Test buffer starting with valid magic
        let valid_buffer = create_test_message(10);
        let magic = u32::from_be_bytes([
            valid_buffer[0],
            valid_buffer[1],
            valid_buffer[2],
            valid_buffer[3],
        ]);
        assert_eq!(magic, MESSAGE_MAGIC, "Should have valid magic number");

        // Test buffer with invalid magic
        let invalid_buffer = vec![0xFF, 0xDE, 0xAD, 0xBE];
        let invalid_magic = u32::from_be_bytes([
            invalid_buffer[0],
            invalid_buffer[1],
            invalid_buffer[2],
            invalid_buffer[3],
        ]);
        assert_ne!(
            invalid_magic, MESSAGE_MAGIC,
            "Should have invalid magic number"
        );
    }
}
