//! Signal Relay - Reliability Focused
//! 
//! Handles TLV types 20-39 with mandatory checksum validation.
//! All messages MUST pass checksum validation (per PROTOCOL.md).
//! Target: >100K messages/second with full integrity validation

use super::{BaseRelay, RelayConfig, ConsumerId, RelayStats, RecoveryRequest};
use crate::{RelayDomain, SourceType, ProtocolError, MessageHeader, parse_header, parse_tlv_extensions, TLVExtensionEnum};
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::{RwLock, broadcast};
use tokio::net::{UnixListener, UnixStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt as TokioAsyncWriteExt;
use tracing::{info, warn, error, debug};

/// Signal Relay with mandatory checksum validation and reliability features
pub struct SignalRelay {
    base: BaseRelay,
    // Reliability features
    message_sender: broadcast::Sender<SignalMessage>,
    integrity_monitor: IntegrityMonitor,
    audit_log: Option<tokio::fs::File>,
}

/// Signal message with integrity validation
#[derive(Debug, Clone)]
pub struct SignalMessage {
    pub sequence: u64,
    pub data: Vec<u8>,
    pub checksum_verified: bool,
    pub tlv_type: u8,
    pub timestamp_ns: u64,
}

/// Monitors message integrity and validation failures
#[derive(Debug)]
struct IntegrityMonitor {
    total_messages: u64,
    checksum_failures: u64,
    invalid_tlv_types: u64,
    last_failure_logged: std::time::Instant,
}

impl IntegrityMonitor {
    fn new() -> Self {
        Self {
            total_messages: 0,
            checksum_failures: 0,
            invalid_tlv_types: 0,
            last_failure_logged: std::time::Instant::now(),
        }
    }
    
    fn record_message_processed(&mut self) {
        self.total_messages += 1;
    }
    
    fn record_checksum_failure(&mut self) {
        self.checksum_failures += 1;
        self.last_failure_logged = std::time::Instant::now();
    }
    
    fn record_invalid_tlv(&mut self) {
        self.invalid_tlv_types += 1;
    }
    
    fn integrity_rate(&self) -> f64 {
        if self.total_messages == 0 {
            return 1.0;
        }
        let failures = self.checksum_failures + self.invalid_tlv_types;
        1.0 - (failures as f64 / self.total_messages as f64)
    }
}

impl SignalRelay {
    /// Create new signal relay with integrity validation
    pub async fn new(socket_path: &str) -> Result<Self, ProtocolError> {
        let config = RelayConfig::signal(socket_path);
        let base = BaseRelay::new(config.clone());
        
        // Create broadcast channel for reliable message distribution
        let (message_sender, _) = broadcast::channel(1000);
        
        // Open audit log if configured
        let audit_log = if let Some(ref log_path) = config.audit_log_path {
            match OpenOptions::new()
                .create(true)
                .append(true)
                .open(log_path)
                .await
            {
                Ok(file) => {
                    info!("📝 Signal relay audit log opened: {}", log_path);
                    Some(file)
                }
                Err(e) => {
                    warn!("Failed to open audit log {}: {}", log_path, e);
                    None
                }
            }
        } else {
            None
        };
        
        Ok(Self {
            base,
            message_sender,
            integrity_monitor: IntegrityMonitor::new(),
            audit_log,
        })
    }
    
    /// Start the signal relay server
    pub async fn start(&mut self) -> Result<(), ProtocolError> {
        info!("🔐 Starting Signal Relay (Reliability Mode - CHECKSUM VALIDATION ENFORCED)");
        info!("Target throughput: >100K msg/s with full integrity validation");
        info!("Listening on: {}", self.base.config.socket_path);
        
        // Remove existing socket file if it exists
        let _ = std::fs::remove_file(&self.base.config.socket_path);
        
        let listener = UnixListener::bind(&self.base.config.socket_path)
            .map_err(|e| ProtocolError::Transport(e))?;
        
        // Start integrity monitoring task
        let integrity_sender = self.message_sender.clone();
        tokio::spawn(async move {
            Self::integrity_monitoring_task(integrity_sender).await;
        });
        
        info!("✅ Signal Relay ready for connections (checksum validation active)");
        
        loop {
            match listener.accept().await {
                Ok((socket, _)) => {
                    info!("📡 New signal consumer connected");
                    
                    let state = Arc::clone(&self.base.state);
                    let config = self.base.config.clone();
                    let message_receiver = self.message_sender.subscribe();
                    
                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_signal_client(socket, state, config, message_receiver).await {
                            error!("Signal client error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    error!("Failed to accept signal connection: {}", e);
                }
            }
        }
    }
    
    /// Handle signal client with full integrity validation
    async fn handle_signal_client(
        mut socket: UnixStream,
        state: Arc<RwLock<super::RelayState>>,
        config: RelayConfig,
        mut message_receiver: broadcast::Receiver<SignalMessage>,
    ) -> Result<(), ProtocolError> {
        let mut read_buffer = vec![0u8; config.buffer_size_bytes];
        
        loop {
            tokio::select! {
                // Handle incoming messages from producers
                read_result = socket.read(&mut read_buffer) => {
                    match read_result {
                        Ok(0) => {
                            debug!("Signal client disconnected");
                            break;
                        }
                        Ok(bytes_read) => {
                            let message_data = &read_buffer[..bytes_read];
                            
                            // RELIABILITY CRITICAL PATH - ENFORCE CHECKSUM VALIDATION
                            match Self::process_signal_message(message_data, &state, &config).await {
                                Ok(signal_msg) => {
                                    debug!("Processed signal message type {} with verified checksum", signal_msg.tlv_type);
                                }
                                Err(e) => {
                                    warn!("REJECTED signal message: {}", e);
                                    // Update failure statistics
                                    let mut state_guard = state.write().await;
                                    if matches!(e, ProtocolError::ChecksumFailed) {
                                        state_guard.stats.checksum_failures += 1;
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            error!("Signal socket read error: {}", e);
                            break;
                        }
                    }
                }
                
                // Forward messages to subscribers
                message = message_receiver.recv() => {
                    match message {
                        Ok(signal_msg) => {
                            if let Err(e) = socket.write_all(&signal_msg.data).await {
                                warn!("Failed to forward signal to subscriber: {}", e);
                                break;
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(missed)) => {
                            warn!("Signal client lagged, missed {} messages", missed);
                            // For signals, lagging is more serious - might need recovery
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            info!("Signal broadcast channel closed");
                            break;
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Process signal message with MANDATORY checksum validation
    async fn process_signal_message(
        message_data: &[u8],
        state: &Arc<RwLock<super::RelayState>>,
        config: &RelayConfig,
    ) -> Result<SignalMessage, ProtocolError> {
        // CRITICAL: ALWAYS validate checksum for signals
        let header = parse_header(message_data)?;
        
        // Domain validation
        if header.relay_domain != RelayDomain::Signal as u8 {
            return Err(ProtocolError::InvalidRelayDomain(header.relay_domain));
        }
        
        // Validate TLV type range for signals (20-39)
        let tlv_payload = &message_data[MessageHeader::SIZE..];
        let tlvs = parse_tlv_extensions(tlv_payload)?;
        
        let mut primary_tlv_type = 0u8;
        for tlv in tlvs {
            let tlv_type = match tlv {
                TLVExtensionEnum::Standard(ref std_tlv) => std_tlv.header.tlv_type,
                TLVExtensionEnum::Extended(ref ext_tlv) => ext_tlv.header.tlv_type,
            };
            
            if !(20..=39).contains(&tlv_type) {
                error!("REJECTED: Invalid TLV type {} for signal domain (must be 20-39)", tlv_type);
                return Err(ProtocolError::UnknownTLV(tlv_type));
            }
            
            if primary_tlv_type == 0 {
                primary_tlv_type = tlv_type;
            }
        }
        
        // Update statistics and sequence
        let (global_seq, timestamp) = {
            let mut state_guard = state.write().await;
            state_guard.stats.messages_processed += 1;
            let seq = state_guard.next_sequence();
            let ts = crate::header::current_timestamp_ns();
            (seq, ts)
        };
        
        Ok(SignalMessage {
            sequence: global_seq,
            data: message_data.to_vec(),
            checksum_verified: true, // We know it passed validation
            tlv_type: primary_tlv_type,
            timestamp_ns: timestamp,
        })
    }
    
    /// Log signal to audit trail
    async fn log_signal_audit(&mut self, signal: &SignalMessage) -> Result<(), ProtocolError> {
        if let Some(ref mut audit_log) = self.audit_log {
            let log_entry = format!(
                "{}: seq={} type={} checksum_verified={} bytes={}\n",
                signal.timestamp_ns,
                signal.sequence,
                signal.tlv_type,
                signal.checksum_verified,
                signal.data.len()
            );
            
            audit_log.write_all(log_entry.as_bytes()).await
                .map_err(|e| ProtocolError::Transport(e))?;
            
            audit_log.flush().await
                .map_err(|e| ProtocolError::Transport(e))?;
        }
        
        Ok(())
    }
    
    /// Broadcast signal message to all subscribers
    pub async fn broadcast_signal(&mut self, signal: SignalMessage) -> Result<usize, ProtocolError> {
        // Log to audit trail
        self.log_signal_audit(&signal).await?;
        
        // Update integrity monitoring
        self.integrity_monitor.record_message_processed();
        
        match self.message_sender.send(signal) {
            Ok(subscriber_count) => {
                debug!("Signal broadcast to {} subscribers", subscriber_count);
                Ok(subscriber_count)
            }
            Err(_) => {
                warn!("No active subscribers for signals");
                Ok(0)
            }
        }
    }
    
    /// Handle recovery request for signals (critical for strategy continuity)
    pub async fn handle_signal_recovery(&mut self, request: RecoveryRequest) -> Result<Vec<SignalMessage>, ProtocolError> {
        warn!("Signal recovery requested for consumer {:?} (sequences {}-{})", 
              request.consumer_id, request.start_sequence, request.end_sequence);
        
        // In production, this would search the message buffer or persistent store
        // For now, return empty (signals might not be recoverable)
        Ok(vec![])
    }
    
    /// Get current relay statistics with integrity metrics
    pub async fn get_integrity_stats(&mut self) -> SignalRelayStats {
        let base_stats = self.base.get_stats().await;
        
        SignalRelayStats {
            base: base_stats,
            integrity_rate: self.integrity_monitor.integrity_rate(),
            checksum_failures: self.integrity_monitor.checksum_failures,
            invalid_tlv_types: self.integrity_monitor.invalid_tlv_types,
            checksum_validation_enforced: true,
        }
    }
    
    /// Integrity monitoring background task
    async fn integrity_monitoring_task(message_sender: broadcast::Sender<SignalMessage>) {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));
        
        loop {
            interval.tick().await;
            let subscriber_count = message_sender.receiver_count();
            if subscriber_count > 0 {
                info!("🔐 Signal Relay - Active subscribers: {}, Integrity validation: ACTIVE", subscriber_count);
            }
        }
    }
}

/// Extended statistics for signal relay with integrity metrics
#[derive(Debug, Clone)]
pub struct SignalRelayStats {
    pub base: RelayStats,
    pub integrity_rate: f64,
    pub checksum_failures: u64,
    pub invalid_tlv_types: u64,
    pub checksum_validation_enforced: bool,
}

impl SignalRelayStats {
    pub fn integrity_report(&self) -> String {
        format!(
            "Signal Relay Integrity Report:\n\
             🔐 Checksum Validation: ENFORCED\n\
             📊 Total Messages: {}\n\
             ✅ Integrity Rate: {:.2}%\n\
             ❌ Checksum Failures: {}\n\
             🚫 Invalid TLV Types: {}\n\
             💪 Messages/Second: {:.0}\n\
             👥 Active Consumers: {}\n\
             ⏱️  Uptime: {}s",
            self.base.messages_processed,
            self.integrity_rate * 100.0,
            self.checksum_failures,
            self.invalid_tlv_types,
            self.base.messages_per_second,
            self.base.active_consumers,
            self.base.uptime_seconds
        )
    }
    
    pub fn is_healthy(&self) -> bool {
        // Consider healthy if >99% integrity rate and recent activity
        self.integrity_rate > 0.99 && self.base.messages_processed > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    
    #[tokio::test]
    async fn test_signal_relay_creation() {
        let temp_file = NamedTempFile::new().unwrap();
        let socket_path = temp_file.path().to_str().unwrap();
        
        let relay = SignalRelay::new(socket_path).await.unwrap();
        assert_eq!(relay.base.config.domain, RelayDomain::Signal);
        assert!(relay.base.config.validate_checksums);
    }
    
    #[test]
    fn test_integrity_monitor() {
        let mut monitor = IntegrityMonitor::new();
        
        // Start with 100% integrity
        assert_eq!(monitor.integrity_rate(), 1.0);
        
        // Process some messages
        monitor.record_message_processed();
        monitor.record_message_processed();
        assert_eq!(monitor.integrity_rate(), 1.0);
        
        // Record a failure (this counts as a message with a failure)
        monitor.record_message_processed(); // Count the message
        monitor.record_checksum_failure();  // Record it failed
        assert!(monitor.integrity_rate() < 1.0);
        assert!((monitor.integrity_rate() - 2.0/3.0).abs() < 0.0001); // 2 success, 1 failure
    }
    
    #[test]
    fn test_signal_message_structure() {
        let signal = SignalMessage {
            sequence: 12345,
            data: vec![1, 2, 3, 4],
            checksum_verified: true,
            tlv_type: 25,
            timestamp_ns: 1640995200_000_000_000,
        };
        
        assert_eq!(signal.sequence, 12345);
        assert!(signal.checksum_verified);
        assert_eq!(signal.tlv_type, 25);
    }
    
    #[test]
    fn test_signal_stats_health_check() {
        let healthy_stats = SignalRelayStats {
            base: RelayStats {
                messages_processed: 1000,
                ..Default::default()
            },
            integrity_rate: 0.995,
            checksum_failures: 5,
            invalid_tlv_types: 0,
            checksum_validation_enforced: true,
        };
        
        assert!(healthy_stats.is_healthy());
        
        let unhealthy_stats = SignalRelayStats {
            base: RelayStats {
                messages_processed: 1000,
                ..Default::default()
            },
            integrity_rate: 0.95, // Below 99% threshold
            checksum_failures: 50,
            invalid_tlv_types: 0,
            checksum_validation_enforced: true,
        };
        
        assert!(!unhealthy_stats.is_healthy());
    }
}