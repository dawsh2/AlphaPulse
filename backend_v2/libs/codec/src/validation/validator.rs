//! # Unified TLV Message Validator
//!
//! Combines the best features from both validation.rs and validation_enhanced.rs
//! into a single, cohesive validator with configurable features.

use crate::error::{ProtocolError, ProtocolResult};
use crate::parser::{parse_tlv_extensions, TLVExtensionEnum, SimpleTLVExtension, ExtendedTLVExtension};
use crate::tlv_types::{TLVType, TlvTypeRegistry};
use super::config::{ValidationConfig, DomainMessageLimits};
use torq_types::protocol::message::header::MessageHeader;
use torq_types::{RelayDomain, SourceType, MESSAGE_MAGIC};
use std::collections::{HashMap, VecDeque, HashSet};
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;
use tracing::{debug, warn, error};
use tokio::sync::mpsc;

/// Unified validation error types
#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("Protocol error: {0}")]
    Protocol(#[from] ProtocolError),
    
    #[error("Unsupported domain: {0:?}")]
    UnsupportedDomain(RelayDomain),
    
    #[error("TLV type {tlv_type} not valid for domain {domain:?}")]
    InvalidTLVForDomain { tlv_type: u8, domain: RelayDomain },
    
    #[error("Message too large: {size} bytes > {max_size} limit")]
    MessageTooLarge { size: usize, max_size: usize },
    
    #[error("Invalid TLV type range for domain {domain:?}: expected {expected}, got {got}")]
    InvalidTLVRange { domain: RelayDomain, expected: String, got: u8 },
    
    #[error("Checksum mismatch: expected {expected:08x}, calculated {calculated:08x}")]
    ChecksumMismatch { expected: u32, calculated: u32 },
    
    #[error("Strict mode violation: {reason}")]
    StrictModeViolation { reason: String },
    
    #[error("Sequence gap detected for source {source_id}: expected {expected}, got {actual}, gap of {gap}")]
    SequenceGap {
        source_id: u8,
        expected: u64,
        actual: u64,
        gap: u64,
    },
    
    #[error("Duplicate sequence number {sequence} from source {source_id}")]
    DuplicateSequence {
        source_id: u8,
        sequence: u64,
    },
    
    #[error("Timestamp validation failed: {reason}")]
    InvalidTimestamp {
        reason: String,
        timestamp: u64,
        current_time: u64,
    },
    
    #[error("Unknown pool address {pool:?}, queued for discovery")]
    UnknownPool {
        pool: [u8; 20],
    },
}

/// Validation policy configuration
#[derive(Debug, Clone)]
pub struct ValidationPolicy {
    /// Enable CRC32 checksum validation
    pub checksum: bool,
    /// Enable detailed audit logging
    pub audit: bool,
    /// Strict mode - require all validations to pass
    pub strict: bool,
    /// Maximum message size in bytes
    pub max_message_size: Option<usize>,
}

impl Default for ValidationPolicy {
    fn default() -> Self {
        Self {
            checksum: true,
            audit: false,
            strict: false,
            max_message_size: Some(65536), // 64KB default
        }
    }
}

/// Validated message with parsed components
#[derive(Debug)]
pub struct ValidatedMessage<'a> {
    pub header: MessageHeader,
    pub tlv_extensions: Vec<TLVExtensionZeroCopy<'a>>,
    pub validation_policy: String,
}

/// Domain-specific validation rules
#[derive(Debug, Clone)]
pub struct DomainValidationRules {
    /// Allowed TLV type range for this domain
    pub tlv_type_range: (u8, u8),
    /// Domain-specific size limits
    pub max_message_size: Option<usize>,
    /// Required validation level
    pub min_validation_level: ValidationLevel,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ValidationLevel {
    Performance,
    Standard,
    Audit,
}

/// Sequence number tracker for detecting gaps and duplicates
#[derive(Debug, Default)]
pub struct SequenceTracker {
    /// Last seen sequence per source
    sequences: HashMap<u8, u64>,
    /// Recent sequences for duplicate detection
    recent_sequences: HashMap<u8, HashSet<u64>>,
    /// Maximum sequences to track for duplicate detection
    max_tracked: usize,
}

impl SequenceTracker {
    pub fn new(max_tracked: usize) -> Self {
        Self {
            sequences: HashMap::new(),
            recent_sequences: HashMap::new(),
            max_tracked,
        }
    }

    /// Validate sequence number and update tracking
    pub fn validate_sequence(
        &mut self,
        source: u8,
        sequence: u64,
        max_gap: u64,
    ) -> Result<(), ValidationError> {
        // Check for duplicates in recent history
        if let Some(recent) = self.recent_sequences.get(&source) {
            if recent.contains(&sequence) {
                return Err(ValidationError::DuplicateSequence { source_id: source, sequence });
            }
        }

        // Check for sequence gaps
        if let Some(&last_seq) = self.sequences.get(&source) {
            if sequence <= last_seq {
                return Err(ValidationError::DuplicateSequence { source_id: source, sequence });
            }
            
            let gap = sequence - last_seq - 1;
            if gap > max_gap {
                return Err(ValidationError::SequenceGap {
                    source_id: source,
                    expected: last_seq + 1,
                    actual: sequence,
                    gap,
                });
            }
            
            if gap > 0 {
                warn!("Sequence gap of {} detected for source {}, but within tolerance", gap, source);
            }
        }

        // Update tracking
        self.sequences.insert(source, sequence);
        
        // Update recent sequences for duplicate detection
        let recent = self.recent_sequences.entry(source).or_insert_with(HashSet::new);
        recent.insert(sequence);
        
        // Trim if too many tracked
        if recent.len() > self.max_tracked {
            let to_remove = recent.len() - self.max_tracked;
            let mut removed = 0;
            recent.retain(|_| {
                if removed < to_remove {
                    removed += 1;
                    false
                } else {
                    true
                }
            });
        }

        Ok(())
    }
}

/// Pool discovery queue for non-blocking RPC calls
#[derive(Debug, Clone)]
pub struct PoolDiscoveryQueue {
    sender: mpsc::UnboundedSender<[u8; 20]>,
}

impl PoolDiscoveryQueue {
    pub fn new() -> (Self, mpsc::UnboundedReceiver<[u8; 20]>) {
        let (sender, receiver) = mpsc::unbounded_channel();
        (Self { sender }, receiver)
    }

    /// Queue a pool for discovery (non-blocking)
    pub fn queue_pool(&self, pool: [u8; 20]) -> Result<(), ValidationError> {
        if let Err(_) = self.sender.send(pool) {
            error!("Pool discovery queue closed, cannot queue pool {:?}", pool);
        } else {
            debug!("Queued pool {:?} for discovery", pool);
        }
        // Always return the UnknownPool error to indicate queuing
        Err(ValidationError::UnknownPool { pool })
    }
}

/// Zero-copy TLV extension that references original buffer
#[derive(Debug)]
pub enum TLVExtensionZeroCopy<'a> {
    Standard {
        tlv_type: u8,
        payload: &'a [u8],
    },
    Extended {
        tlv_type: u8,
        payload: &'a [u8],
    },
}

impl<'a> TLVExtensionZeroCopy<'a> {
    pub fn to_owned(self) -> TLVExtensionEnum {
        match self {
            TLVExtensionZeroCopy::Standard { tlv_type, payload } => {
                TLVExtensionEnum::Standard(SimpleTLVExtension {
                    header: crate::parser::SimpleTLVHeader {
                        tlv_type,
                        tlv_length: payload.len() as u8,
                    },
                    payload: payload.to_vec(),
                })
            }
            TLVExtensionZeroCopy::Extended { tlv_type, payload } => {
                TLVExtensionEnum::Extended(ExtendedTLVExtension {
                    header: crate::parser::ExtendedTLVHeader {
                        marker: 255,
                        reserved: 0,
                        tlv_type,
                        tlv_length: payload.len() as u16,
                    },
                    payload: payload.to_vec(),
                })
            }
        }
    }
}

/// Main TLV validator with all features
pub struct TLVValidator {
    config: ValidationConfig,
    domain_rules: HashMap<RelayDomain, DomainValidationRules>,
    default_policy: ValidationPolicy,
    sequence_tracker: Arc<RwLock<SequenceTracker>>,
    pool_discovery: Option<PoolDiscoveryQueue>,
    known_pools: Arc<RwLock<HashSet<[u8; 20]>>>,
}

impl TLVValidator {
    /// Create a new validator with default configuration
    pub fn new() -> Self {
        Self::with_config(ValidationConfig::default())
    }

    /// Create validator with specific configuration
    pub fn with_config(config: ValidationConfig) -> Self {
        let mut domain_rules = HashMap::new();
        
        // Market Data domain (1-19) - Performance focused
        domain_rules.insert(RelayDomain::MarketData, DomainValidationRules {
            tlv_type_range: (1, 19),
            max_message_size: Some(config.max_message_sizes.market_data),
            min_validation_level: ValidationLevel::Performance,
        });
        
        // Signal domain (20-39) - Standard validation
        domain_rules.insert(RelayDomain::Signal, DomainValidationRules {
            tlv_type_range: (20, 39),
            max_message_size: Some(config.max_message_sizes.signal),
            min_validation_level: ValidationLevel::Standard,
        });
        
        // Execution domain (40-79) - Full audit
        domain_rules.insert(RelayDomain::Execution, DomainValidationRules {
            tlv_type_range: (40, 79),
            max_message_size: Some(config.max_message_sizes.execution),
            min_validation_level: ValidationLevel::Audit,
        });

        let max_tracked = config.sequence.max_tracked_sequences;
        
        Self {
            config,
            domain_rules,
            default_policy: ValidationPolicy::default(),
            sequence_tracker: Arc::new(RwLock::new(SequenceTracker::new(max_tracked))),
            pool_discovery: None,
            known_pools: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    /// Create validator for specific domain with custom policy
    pub fn for_domain(domain: RelayDomain, policy: ValidationPolicy) -> Self {
        let mut validator = Self::new();
        validator.default_policy = policy;
        validator
    }

    /// Add pool discovery queue
    pub fn with_pool_discovery(mut self, queue: PoolDiscoveryQueue) -> Self {
        self.pool_discovery = Some(queue);
        self
    }

    /// Validate complete message with all features
    pub fn validate_message<'a>(&self, full_message: &'a [u8]) -> Result<ValidatedMessage<'a>, ValidationError> {
        // Parse header
        let header = self.parse_header_safe(full_message)?;
        
        // Get domain
        let relay_domain = RelayDomain::try_from(header.relay_domain)
            .map_err(|_| ValidationError::UnsupportedDomain(
                RelayDomain::try_from(header.relay_domain).unwrap_or(RelayDomain::MarketData)
            ))?;

        // Determine validation level based on domain
        let validation_level = self.get_validation_level(relay_domain);
        
        // Validate based on level
        match validation_level {
            ValidationLevel::Performance => {
                // Minimal validation for performance
                self.validate_header(header)?;
                self.validate_message_size(header.payload_size as usize, relay_domain)?;
            },
            ValidationLevel::Standard => {
                // Standard validation with checksum
                self.validate_header(header)?;
                self.validate_checksum(header, full_message)?;
                self.validate_message_size(header.payload_size as usize, relay_domain)?;
                if self.config.sequence.enforce_monotonic {
                    self.validate_sequence(header.source, header.sequence)?;
                }
            },
            ValidationLevel::Audit => {
                // Full validation with all checks
                self.validate_header(header)?;
                self.validate_checksum(header, full_message)?;
                self.validate_timestamp(header.timestamp)?;
                self.validate_sequence(header.source, header.sequence)?;
                self.validate_message_size(header.payload_size as usize, relay_domain)?;
            }
        }
        
        // Extract and validate payload
        let payload = &full_message[32..32 + header.payload_size as usize];
        
        // Parse TLV extensions without allocation
        let tlv_extensions = self.parse_tlv_zero_copy(payload, relay_domain)?;
        
        // Check for pool addresses if configured
        if self.config.pool_discovery.enabled {
            self.check_pool_addresses(&tlv_extensions)?;
        }
        
        Ok(ValidatedMessage {
            header: *header,
            tlv_extensions,
            validation_policy: format!("{:?}", validation_level),
        })
    }

    /// Get validation level for domain
    fn get_validation_level(&self, domain: RelayDomain) -> ValidationLevel {
        self.domain_rules
            .get(&domain)
            .map(|rules| rules.min_validation_level.clone())
            .unwrap_or(ValidationLevel::Standard)
    }

    /// Validate message header fields
    fn validate_header(&self, header: &MessageHeader) -> Result<(), ValidationError> {
        // Magic number validation
        if header.magic != MESSAGE_MAGIC {
            return Err(ValidationError::Protocol(ProtocolError::invalid_magic(
                MESSAGE_MAGIC,
                header.magic,
                0,
            )));
        }

        // Domain validation
        if header.relay_domain == 0 || header.relay_domain > 3 {
            return Err(ValidationError::Protocol(ProtocolError::message_too_small(
                1, 0, &format!("Invalid relay domain: {}", header.relay_domain)
            )));
        }

        // Source validation
        if header.source == 0 || header.source > 100 {
            return Err(ValidationError::Protocol(ProtocolError::message_too_small(
                1, 0, &format!("Invalid source type: {}", header.source)
            )));
        }

        Ok(())
    }

    /// Validate checksum per Protocol V2 specification
    fn validate_checksum(&self, header: &MessageHeader, full_message: &[u8]) -> Result<(), ValidationError> {
        if !self.config.timestamp.enforce_validation {
            return Ok(()); // Skip in development mode
        }

        let calculated = self.calculate_message_checksum(full_message);
        
        if header.checksum != 0 && header.checksum != calculated {
            return Err(ValidationError::ChecksumMismatch {
                expected: header.checksum,
                calculated,
            });
        }
        
        Ok(())
    }

    /// Calculate checksum per Protocol V2 spec
    fn calculate_message_checksum(&self, full_message: &[u8]) -> u32 {
        const HEADER_SIZE: usize = 32;
        const CHECKSUM_OFFSET: usize = 28;
        
        if full_message.len() < HEADER_SIZE {
            return 0;
        }
        
        // Calculate CRC32 over message excluding checksum field
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&full_message[..CHECKSUM_OFFSET]);
        hasher.update(&full_message[CHECKSUM_OFFSET + 4..HEADER_SIZE]);
        hasher.update(&full_message[HEADER_SIZE..]);
        hasher.finalize()
    }

    /// Validate timestamp is within acceptable bounds
    pub fn validate_timestamp(&self, timestamp_ns: u64) -> Result<(), ValidationError> {
        if !self.config.timestamp.enforce_validation {
            return Ok(());
        }

        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        
        // Check if timestamp is too far in the future
        let max_future = current_time + self.config.timestamp.max_future_drift.as_nanos() as u64;
        if timestamp_ns > max_future {
            return Err(ValidationError::InvalidTimestamp {
                reason: format!("Timestamp too far in future (>{} seconds)", 
                    self.config.timestamp.max_future_drift.as_secs()),
                timestamp: timestamp_ns,
                current_time,
            });
        }
        
        // Check if timestamp is too old
        let min_time = current_time.saturating_sub(self.config.timestamp.max_age.as_nanos() as u64);
        if timestamp_ns < min_time {
            return Err(ValidationError::InvalidTimestamp {
                reason: format!("Timestamp too old (>{} seconds)", 
                    self.config.timestamp.max_age.as_secs()),
                timestamp: timestamp_ns,
                current_time,
            });
        }
        
        Ok(())
    }

    /// Validate sequence number
    fn validate_sequence(&self, source: u8, sequence: u64) -> Result<(), ValidationError> {
        let mut tracker = self.sequence_tracker.write().unwrap();
        tracker.validate_sequence(source, sequence, self.config.sequence.max_sequence_gap)
    }

    /// Validate message size against domain limits
    fn validate_message_size(&self, size: usize, domain: RelayDomain) -> Result<(), ValidationError> {
        let max_size = match domain {
            RelayDomain::MarketData => self.config.max_message_sizes.market_data,
            RelayDomain::Signal => self.config.max_message_sizes.signal,
            RelayDomain::Execution => self.config.max_message_sizes.execution,
            _ => self.config.max_message_sizes.system,
        };
        
        if size > max_size {
            return Err(ValidationError::MessageTooLarge { size, max_size });
        }
        
        Ok(())
    }

    /// Parse TLV extensions without allocations (zero-copy)
    pub fn parse_tlv_zero_copy<'a>(
        &self,
        payload: &'a [u8],
        domain: RelayDomain,
    ) -> Result<Vec<TLVExtensionZeroCopy<'a>>, ValidationError> {
        let mut extensions = Vec::new();
        let mut offset = 0;
        
        while offset < payload.len() {
            if offset + 2 > payload.len() {
                return Err(ValidationError::Protocol(
                    ProtocolError::truncated_tlv(payload.len(), offset + 2, 0, offset)
                ));
            }
            
            let tlv_type = payload[offset];
            
            // Validate TLV type for domain
            if !self.is_valid_tlv_for_domain(tlv_type, domain) {
                return Err(ValidationError::InvalidTLVForDomain {
                    tlv_type,
                    domain,
                });
            }
            
            if tlv_type == 255 {
                // Extended TLV
                if offset + 5 > payload.len() {
                    return Err(ValidationError::Protocol(
                        ProtocolError::truncated_tlv(payload.len(), offset + 5, tlv_type as u16, offset)
                    ));
                }
                
                let actual_type = payload[offset + 2];
                let length = u16::from_le_bytes([payload[offset + 3], payload[offset + 4]]) as usize;
                
                if offset + 5 + length > payload.len() {
                    return Err(ValidationError::Protocol(
                        ProtocolError::truncated_tlv(payload.len(), offset + 5 + length, actual_type as u16, offset)
                    ));
                }
                
                let tlv_payload = &payload[offset + 5..offset + 5 + length];
                extensions.push(TLVExtensionZeroCopy::Extended {
                    tlv_type: actual_type,
                    payload: tlv_payload,
                });
                
                offset += 5 + length;
            } else {
                // Standard TLV
                let length = payload[offset + 1] as usize;
                
                if offset + 2 + length > payload.len() {
                    return Err(ValidationError::Protocol(
                        ProtocolError::truncated_tlv(payload.len(), offset + 2 + length, tlv_type as u16, offset)
                    ));
                }
                
                let tlv_payload = &payload[offset + 2..offset + 2 + length];
                extensions.push(TLVExtensionZeroCopy::Standard {
                    tlv_type,
                    payload: tlv_payload,
                });
                
                offset += 2 + length;
            }
        }
        
        Ok(extensions)
    }

    /// Check for unknown pool addresses and queue for discovery
    fn check_pool_addresses(&self, tlvs: &[TLVExtensionZeroCopy]) -> Result<(), ValidationError> {
        for tlv in tlvs {
            let (tlv_type, payload) = match tlv {
                TLVExtensionZeroCopy::Standard { tlv_type, payload } => (*tlv_type, *payload),
                TLVExtensionZeroCopy::Extended { tlv_type, payload } => (*tlv_type, *payload),
            };
            
            // Check if this is a pool-related TLV
            if matches!(tlv_type, 11 | 12 | 13) && payload.len() >= 20 {
                let mut pool_addr = [0u8; 20];
                pool_addr.copy_from_slice(&payload[..20]);
                
                let known_pools = self.known_pools.read().unwrap();
                if !known_pools.contains(&pool_addr) {
                    drop(known_pools);
                    
                    if let Some(ref queue) = self.pool_discovery {
                        queue.queue_pool(pool_addr)?;
                    }
                }
            }
        }
        
        Ok(())
    }

    /// Check if TLV type is valid for domain
    fn is_valid_tlv_for_domain(&self, tlv_type: u8, domain: RelayDomain) -> bool {
        if let Some(rules) = self.domain_rules.get(&domain) {
            tlv_type >= rules.tlv_type_range.0 && tlv_type <= rules.tlv_type_range.1
        } else {
            true // Allow unknown domains for now
        }
    }

    /// Add a known pool address
    pub fn add_known_pool(&self, pool: [u8; 20]) {
        let mut known_pools = self.known_pools.write().unwrap();
        known_pools.insert(pool);
    }

    /// Safe header parsing
    fn parse_header_safe<'a>(&self, data: &'a [u8]) -> Result<&'a MessageHeader, ValidationError> {
        use crate::parser::parse_header;
        parse_header(data).map_err(|e| ValidationError::Protocol(e))
    }
}

impl Default for TLVValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_sequence_validation() {
        let mut tracker = SequenceTracker::new(100);
        
        // First message establishes baseline
        assert!(tracker.validate_sequence(1, 100, 10).is_ok());
        
        // Next message in sequence
        assert!(tracker.validate_sequence(1, 101, 10).is_ok());
        
        // Gap within tolerance
        assert!(tracker.validate_sequence(1, 105, 10).is_ok());
        
        // Gap exceeds tolerance
        assert!(tracker.validate_sequence(1, 120, 10).is_err());
        
        // Duplicate sequence
        assert!(tracker.validate_sequence(1, 105, 10).is_err());
    }
    
    #[test]
    fn test_timestamp_validation() {
        let config = ValidationConfig::default();
        let validator = TLVValidator::with_config(config);
        
        let current_ns = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        
        // Current timestamp should be valid
        assert!(validator.validate_timestamp(current_ns).is_ok());
        
        // 1 second in future should be ok
        assert!(validator.validate_timestamp(current_ns + 1_000_000_000).is_ok());
        
        // 10 seconds in future should fail
        assert!(validator.validate_timestamp(current_ns + 10_000_000_000).is_err());
    }
}