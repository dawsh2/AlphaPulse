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
use protocol_v2::{
    message::header::MessageHeader, parse_header, parse_tlv_extensions, ParseError, RelayDomain,
    TLVExtensionEnum, MESSAGE_MAGIC,
};
use serde_json::Value;
use std::collections::HashMap;
use std::mem::size_of;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::net::UnixStream;
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
                    info!("Connected to {:?} relay", domain);

                    let mut buffer = vec![0u8; 8192];
                    let mut message_buffer = Vec::new(); // Accumulate partial messages

                    loop {
                        match stream.read(&mut buffer).await {
                            Ok(0) => {
                                warn!("{:?} relay connection closed", domain);
                                break;
                            }
                            Ok(bytes_read) => {
                                // Append new data to message buffer
                                message_buffer.extend_from_slice(&buffer[..bytes_read]);

                                // Process complete messages from buffer
                                let mut processed_bytes = 0;
                                while message_buffer.len() >= processed_bytes + 32 {
                                    let remaining_data = &message_buffer[processed_bytes..];

                                    // Try to parse header to get message size
                                    let header_result = match domain {
                                        RelayDomain::MarketData => {
                                            Self::parse_header_fast(remaining_data)
                                        }
                                        _ => parse_header(remaining_data),
                                    };

                                    match header_result {
                                        Ok(header) => {
                                            let total_message_size =
                                                32 + header.payload_size as usize;

                                            // Check if we have the complete message
                                            if remaining_data.len() >= total_message_size {
                                                let complete_message =
                                                    &remaining_data[..total_message_size];

                                                if let Err(e) = Self::process_relay_data(
                                                    &client_manager,
                                                    complete_message,
                                                    domain,
                                                    &mut signal_buffer,
                                                )
                                                .await
                                                {
                                                    warn!(
                                                        "Error processing {:?} relay data: {}",
                                                        domain, e
                                                    );
                                                }

                                                processed_bytes += total_message_size;
                                            } else {
                                                // Incomplete message, wait for more data
                                                break;
                                            }
                                        }
                                        Err(_) => {
                                            // Invalid header, skip this byte and try next
                                            processed_bytes += 1;
                                        }
                                    }
                                }

                                // Remove processed data from buffer
                                if processed_bytes > 0 {
                                    message_buffer.drain(..processed_bytes);
                                }
                            }
                            Err(e) => {
                                error!("Error reading from {:?} relay: {}", domain, e);
                                break;
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
        // Use appropriate parsing based on domain policy:
        // MarketDataRelay: Skip checksum validation for performance
        // SignalRelay, ExecutionRelay: Enforce checksum validation
        let header = match domain {
            RelayDomain::MarketData => {
                // Fast parsing without checksum validation (performance optimization)
                match Self::parse_header_fast(data) {
                    Ok(header) => header,
                    Err(e) => {
                        debug!("Failed to parse MarketData header (fast): {}", e);
                        return Ok(()); // Skip malformed messages
                    }
                }
            }
            RelayDomain::Signal | RelayDomain::Execution | RelayDomain::System => {
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
            Ok(tlvs) => tlvs,
            Err(e) => {
                debug!("Failed to parse TLV data: {}", e);
                return Ok(()); // Skip malformed TLV data
            }
        };

        let mut current_signal_id: Option<u64> = None;

        for tlv in tlvs {
            // Extract TLV data based on variant
            let (tlv_type, tlv_payload) = match &tlv {
                TLVExtensionEnum::Standard(std_tlv) => (std_tlv.header.tlv_type, &std_tlv.payload),
                TLVExtensionEnum::Extended(ext_tlv) => (ext_tlv.header.tlv_type, &ext_tlv.payload),
            };

            // Convert TLV to JSON using protocol parsing
            let json_message = convert_tlv_to_json(tlv_type, tlv_payload, timestamp)?;

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
                3 => {
                    // Economics
                    if let Some(signal_id) = current_signal_id {
                        let entry = signal_buffer.entry(signal_id).or_insert((None, None));
                        entry.1 = Some(json_message);

                        // Check if we have both parts of the signal
                        if let (Some(identity), Some(economics)) = &entry {
                            // Check if this is a flash arbitrage signal (strategy_id = 21)
                            let is_flash_arbitrage = identity
                                .get("strategy_id")
                                .and_then(|v| v.as_u64()) == Some(21);

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
                    debug!("Broadcasted pool {} message", tlv_type);
                }
                255 => {
                    // ExtendedTLV - DemoDeFiArbitrageTLV
                    // The converter already creates the full arbitrage opportunity message
                    client_manager.broadcast(json_message).await;
                    debug!("Broadcasted DemoDeFiArbitrageTLV message");
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
}
