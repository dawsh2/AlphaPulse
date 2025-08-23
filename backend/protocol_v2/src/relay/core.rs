//! Core Relay Infrastructure
//! 
//! Base relay implementation with common functionality for all domain-specific relays.

use crate::{MessageHeader, parse_header, RelayDomain, SourceType, ProtocolError};
use super::{ConsumerId, RelayConfig, RelayStats, RecoveryRequest, RecoveryRequestType};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH, Instant};
use tokio::sync::{RwLock, mpsc};
use tokio::net::{UnixListener, UnixStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{info, warn, error, debug};

/// Core relay state shared across all relay types
#[derive(Debug)]
pub struct RelayState {
    pub global_sequence: u64,
    pub consumer_sequences: HashMap<ConsumerId, u64>,
    pub domain: RelayDomain,
    pub validate_checksums: bool,
    pub stats: RelayStats,
    pub start_time: Instant,
}

impl RelayState {
    pub fn new(config: &RelayConfig) -> Self {
        Self {
            global_sequence: 1,
            consumer_sequences: HashMap::new(),
            domain: config.domain,
            validate_checksums: config.validate_checksums,
            stats: RelayStats::default(),
            start_time: Instant::now(),
        }
    }
    
    /// Get next global sequence number
    pub fn next_sequence(&mut self) -> u64 {
        let seq = self.global_sequence;
        self.global_sequence += 1;
        seq
    }
    
    /// Update consumer sequence tracking
    pub fn update_consumer_sequence(&mut self, consumer_id: ConsumerId, sequence: u64) -> Option<RecoveryRequest> {
        let expected = self.consumer_sequences.get(&consumer_id).unwrap_or(&0) + 1;
        
        if sequence != expected {
            // Gap detected!
            warn!("Sequence gap detected for {:?}: expected {}, got {}", 
                  consumer_id, expected, sequence);
            
            self.stats.recovery_requests += 1;
            
            // Determine recovery type based on gap size
            let gap_size = sequence - expected;
            let request_type = if gap_size > 100 {
                RecoveryRequestType::Snapshot
            } else {
                RecoveryRequestType::Retransmit
            };
            
            Some(RecoveryRequest {
                consumer_id: consumer_id.clone(),
                start_sequence: expected,
                end_sequence: sequence - 1,
                request_type,
            })
        } else {
            self.consumer_sequences.insert(consumer_id, sequence);
            None
        }
    }
    
    /// Update relay statistics
    pub fn update_stats(&mut self) {
        self.stats.uptime_seconds = self.start_time.elapsed().as_secs();
        self.stats.active_consumers = self.consumer_sequences.len();
        
        // Calculate messages per second (simple moving average)
        if self.stats.uptime_seconds > 0 {
            self.stats.messages_per_second = self.stats.messages_processed as f64 / self.stats.uptime_seconds as f64;
        }
    }
}

/// Base relay implementation with common functionality
pub struct BaseRelay {
    pub config: RelayConfig,
    pub state: Arc<RwLock<RelayState>>,
    pub message_buffer: Vec<(u64, Vec<u8>)>, // (sequence, message) for recovery
    pub connected_clients: Vec<UnixStream>,
}

impl BaseRelay {
    pub fn new(config: RelayConfig) -> Self {
        let state = RelayState::new(&config);
        
        Self {
            config: config.clone(),
            state: Arc::new(RwLock::new(state)),
            message_buffer: Vec::new(),
            connected_clients: Vec::new(),
        }
    }
    
    /// Start the relay server
    pub async fn start(&mut self) -> Result<(), ProtocolError> {
        info!("Starting {:?} relay on {}", self.config.domain, self.config.socket_path);
        
        // Remove existing socket file if it exists
        let _ = std::fs::remove_file(&self.config.socket_path);
        
        let listener = UnixListener::bind(&self.config.socket_path)
            .map_err(|e| ProtocolError::Transport(e))?;
            
        info!("{:?} relay listening on {}", self.config.domain, self.config.socket_path);
        
        loop {
            match listener.accept().await {
                Ok((socket, _addr)) => {
                    info!("New client connected to {:?} relay", self.config.domain);
                    
                    let state = Arc::clone(&self.state);
                    let config = self.config.clone();
                    
                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_client_connection(socket, state, config).await {
                            error!("Client connection error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    error!("Failed to accept connection: {}", e);
                }
            }
        }
    }
    
    /// Handle individual client connection
    async fn handle_client_connection(
        mut socket: UnixStream,
        state: Arc<RwLock<RelayState>>,
        config: RelayConfig,
    ) -> Result<(), ProtocolError> {
        let mut buffer = vec![0u8; config.buffer_size_bytes];
        
        loop {
            match socket.read(&mut buffer).await {
                Ok(0) => {
                    debug!("Client disconnected");
                    break;
                }
                Ok(bytes_read) => {
                    let message_data = &buffer[..bytes_read];
                    
                    // Process the message according to domain policy
                    match Self::process_message(message_data, &state, &config).await {
                        Ok(processed_message) => {
                            // Forward to other subscribers (not implemented yet)
                            // self.broadcast_to_subscribers(&processed_message).await?;
                            debug!("Processed message: {} bytes", processed_message.len());
                        }
                        Err(e) => {
                            warn!("Failed to process message: {}", e);
                            
                            // Increment failure stats
                            let mut state_guard = state.write().await;
                            if matches!(e, ProtocolError::ChecksumFailed) {
                                state_guard.stats.checksum_failures += 1;
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Socket read error: {}", e);
                    break;
                }
            }
        }
        
        Ok(())
    }
    
    /// Process incoming message according to domain-specific validation policy
    async fn process_message(
        message_data: &[u8],
        state: &Arc<RwLock<RelayState>>,
        config: &RelayConfig,
    ) -> Result<Vec<u8>, ProtocolError> {
        // Parse header with or without checksum validation based on config
        let header = if config.validate_checksums {
            // Full validation including checksum
            parse_header(message_data)?
        } else {
            // Skip checksum validation for performance (market data)
            Self::parse_header_unchecked(message_data)?
        };
        
        // Validate domain routing
        if header.relay_domain != config.domain as u8 {
            return Err(ProtocolError::InvalidRelayDomain(header.relay_domain));
        }
        
        // Update statistics and sequence tracking
        let mut state_guard = state.write().await;
        state_guard.stats.messages_processed += 1;
        
        // Assign global sequence number
        let global_seq = state_guard.next_sequence();
        
        // Update the message with global sequence (would modify header in practice)
        // For now, just return the original message
        state_guard.update_stats();
        
        Ok(message_data.to_vec())
    }
    
    /// Parse header without checksum validation (performance optimization)
    fn parse_header_unchecked(data: &[u8]) -> Result<&MessageHeader, ProtocolError> {
        if data.len() < MessageHeader::SIZE {
            return Err(ProtocolError::Parse(crate::ParseError::MessageTooSmall {
                need: MessageHeader::SIZE,
                got: data.len(),
            }));
        }
        
        let header_bytes = &data[..MessageHeader::SIZE];
        let header = zerocopy::Ref::<_, MessageHeader>::new(header_bytes)
            .ok_or(ProtocolError::Parse(crate::ParseError::MessageTooSmall {
                need: MessageHeader::SIZE,
                got: data.len(),
            }))?
            .into_ref();
        
        // Validate magic number only (skip checksum for performance)
        if header.magic != crate::MESSAGE_MAGIC {
            return Err(ProtocolError::Parse(crate::ParseError::InvalidMagic {
                expected: crate::MESSAGE_MAGIC,
                actual: header.magic,
            }));
        }
        
        Ok(header)
    }
    
    /// Get current relay statistics
    pub async fn get_stats(&self) -> RelayStats {
        let mut state = self.state.write().await;
        state.update_stats();
        state.stats.clone()
    }
    
    /// Handle recovery request from consumer
    pub async fn handle_recovery_request(&mut self, request: RecoveryRequest) -> Result<Vec<Vec<u8>>, ProtocolError> {
        info!("Processing recovery request: {:?}", request);
        
        match request.request_type {
            RecoveryRequestType::Retransmit => {
                self.handle_retransmit_request(request).await
            }
            RecoveryRequestType::Snapshot => {
                self.handle_snapshot_request(request).await
            }
        }
    }
    
    async fn handle_retransmit_request(&self, request: RecoveryRequest) -> Result<Vec<Vec<u8>>, ProtocolError> {
        // Find messages in buffer for the requested sequence range
        let messages: Vec<Vec<u8>> = self.message_buffer.iter()
            .filter(|(seq, _)| *seq >= request.start_sequence && *seq <= request.end_sequence)
            .map(|(_, msg)| msg.clone())
            .collect();
        
        info!("Retransmitting {} messages for consumer {:?}", 
              messages.len(), request.consumer_id);
        
        Ok(messages)
    }
    
    async fn handle_snapshot_request(&self, _request: RecoveryRequest) -> Result<Vec<Vec<u8>>, ProtocolError> {
        // Snapshot generation would be implemented here
        // For now, return empty (not implemented)
        warn!("Snapshot recovery not yet implemented");
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tlv::TLVMessageBuilder;
    use crate::tlv::TLVType;
    
    #[test]
    fn test_relay_state_creation() {
        let config = RelayConfig::market_data("/tmp/test_market.sock");
        let state = RelayState::new(&config);
        
        assert_eq!(state.domain, RelayDomain::MarketData);
        assert!(!state.validate_checksums);
        assert_eq!(state.global_sequence, 1);
    }
    
    #[test]
    fn test_consumer_id() {
        let consumer_id = ConsumerId::new("dashboard", 1);
        assert_eq!(consumer_id.to_string(), "dashboard:1");
        
        let parsed = ConsumerId::from_string("dashboard:1").unwrap();
        assert_eq!(parsed.service_name, "dashboard");
        assert_eq!(parsed.instance_id, 1);
    }
    
    #[test]
    fn test_sequence_gap_detection() {
        let config = RelayConfig::signal("/tmp/test_signal.sock");
        let mut state = RelayState::new(&config);
        
        let consumer = ConsumerId::new("test_consumer", 1);
        
        // First message should be fine
        let result = state.update_consumer_sequence(consumer.clone(), 1);
        assert!(result.is_none());
        
        // Gap detected
        let result = state.update_consumer_sequence(consumer.clone(), 5);
        assert!(result.is_some());
        
        let recovery_req = result.unwrap();
        assert_eq!(recovery_req.start_sequence, 2);
        assert_eq!(recovery_req.end_sequence, 4);
    }
}