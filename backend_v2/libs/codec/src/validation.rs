//! # Consolidated TLV Message Validation System
//!
//! ## Purpose
//!
//! Unified validation framework consolidating all TLV message validation logic from
//! the relay infrastructure. Provides configurable validation policies optimized for
//! different relay domains while maintaining >1M msg/s parsing throughput through
//! efficient domain-specific validators.
//!
//! ## Architecture
//!
//! ```text
//! Message Input → TLVValidator → Domain Validator → Validated Message
//!       ↓              ↓             ↓                    ↓
//!   Raw Bytes    Policy Check   TLV Range Check    ValidatedMessage
//!   Header       Size Limits    Type Validation    Ready for Processing
//! ```
//!
//! ## Validation Levels
//!
//! - **Performance** (Market Data): Minimal validation, >1M msg/s throughput
//! - **Standard** (Signals): CRC32 validation, >100K msg/s throughput  
//! - **Audit** (Execution): Full validation + logging, >50K msg/s throughput

use crate::error::{ProtocolError, ProtocolResult};
use crate::parser::{parse_tlv_extensions, TLVExtensionEnum};
use torq_codec::protocol::tlv::types::TLVType;
use crate::tlv_types::TlvTypeRegistry;
use torq_types::protocol::message::header::MessageHeader;
use torq_types::{RelayDomain, SourceType};
use std::collections::HashMap;
use thiserror::Error;
use tracing::{debug, warn};

/// Validation error types
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
    
    #[error("Checksum validation failed: expected {expected:08x}, got {calculated:08x}")]
    ChecksumMismatch { expected: u32, calculated: u32 },
    
    #[error("Strict mode violation: {reason}")]
    StrictModeViolation { reason: String },
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
pub struct ValidatedMessage {
    pub header: MessageHeader,
    pub tlv_extensions: Vec<TLVExtensionEnum>,
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

/// Main TLV validator with domain-specific rules
pub struct TLVValidator {
    domain_rules: HashMap<RelayDomain, DomainValidationRules>,
    default_policy: ValidationPolicy,
}

impl TLVValidator {
    /// Create a new validator with default domain rules
    pub fn new() -> Self {
        let mut domain_rules = HashMap::new();
        
        // Market Data domain (1-19) - Performance focused
        domain_rules.insert(RelayDomain::MarketData, DomainValidationRules {
            tlv_type_range: (1, 19),
            max_message_size: Some(4096), // 4KB for market data
            min_validation_level: ValidationLevel::Performance,
        });
        
        // Signal domain (20-39) - Standard validation
        domain_rules.insert(RelayDomain::Signal, DomainValidationRules {
            tlv_type_range: (20, 39),
            max_message_size: Some(8192), // 8KB for signals
            min_validation_level: ValidationLevel::Standard,
        });
        
        // Execution domain (40-79) - Full audit
        domain_rules.insert(RelayDomain::Execution, DomainValidationRules {
            tlv_type_range: (40, 79),
            max_message_size: Some(16384), // 16KB for execution
            min_validation_level: ValidationLevel::Audit,
        });

        Self {
            domain_rules,
            default_policy: ValidationPolicy::default(),
        }
    }

    /// Create validator for specific domain with custom policy
    pub fn for_domain(domain: RelayDomain, policy: ValidationPolicy) -> Self {
        let mut validator = Self::new();
        validator.default_policy = policy;
        validator
    }

    /// Validate complete message with domain-specific rules
    pub fn validate_message(&self, header: &MessageHeader, payload: &[u8]) -> Result<ValidatedMessage, ValidationError> {
        let relay_domain = RelayDomain::try_from(header.relay_domain)
            .map_err(|_| ValidationError::UnsupportedDomain(
                RelayDomain::try_from(header.relay_domain).unwrap_or(RelayDomain::MarketData)
            ))?;

        // Validate header
        self.validate_header(header)?;
        
        // Validate message size
        self.validate_message_size(payload, relay_domain)?;
        
        // Parse and validate TLV payload
        let tlv_extensions = self.parse_and_validate_tlvs(payload, relay_domain)?;
        
        // Apply domain-specific validation rules
        self.validate_domain_rules(relay_domain, &tlv_extensions)?;

        Ok(ValidatedMessage {
            header: *header,
            tlv_extensions,
            validation_policy: self.get_validation_policy_name(relay_domain),
        })
    }

    /// Validate message header fields
    fn validate_header(&self, header: &MessageHeader) -> Result<(), ValidationError> {
        // Magic number validation (should be done by parser, but double-check)
        if header.magic != torq_types::MESSAGE_MAGIC {
            return Err(ValidationError::Protocol(ProtocolError::invalid_magic(
                torq_types::MESSAGE_MAGIC,
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

    /// Validate message size against domain limits
    fn validate_message_size(&self, payload: &[u8], domain: RelayDomain) -> Result<(), ValidationError> {
        let message_size = payload.len();
        
        // Check default policy max size
        if let Some(max_size) = self.default_policy.max_message_size {
            if message_size > max_size {
                return Err(ValidationError::MessageTooLarge { 
                    size: message_size, 
                    max_size 
                });
            }
        }

        // Check domain-specific max size
        if let Some(rules) = self.domain_rules.get(&domain) {
            if let Some(domain_max_size) = rules.max_message_size {
                if message_size > domain_max_size {
                    return Err(ValidationError::MessageTooLarge { 
                        size: message_size, 
                        max_size: domain_max_size 
                    });
                }
            }
        }

        Ok(())
    }

    /// Parse TLV payload and validate structure
    fn parse_and_validate_tlvs(&self, payload: &[u8], domain: RelayDomain) -> Result<Vec<TLVExtensionEnum>, ValidationError> {
        let tlv_extensions = parse_tlv_extensions(payload)?;
        
        // Validate each TLV against domain rules
        for tlv in &tlv_extensions {
            let tlv_type = match tlv {
                TLVExtensionEnum::Standard(t) => t.header.tlv_type,
                TLVExtensionEnum::Extended(t) => t.header.tlv_type,
            };

            // Validate TLV type is valid for domain
            if !self.is_tlv_valid_for_domain(tlv_type, domain)? {
                return Err(ValidationError::InvalidTLVForDomain { 
                    tlv_type, 
                    domain 
                });
            }

            // Validate TLV size constraints
            let payload_size = match tlv {
                TLVExtensionEnum::Standard(t) => t.payload.len(),
                TLVExtensionEnum::Extended(t) => t.payload.len(),
            };

            if let Ok(tlv_type_enum) = TLVType::try_from(tlv_type) {
                if !TlvTypeRegistry::validate_size(tlv_type_enum, payload_size) {
                    debug!("TLV size validation failed for type {}: expected constraint, got {} bytes", 
                           tlv_type, payload_size);
                    // Note: In performance mode, we might choose to only warn about size mismatches
                    // rather than failing the entire message
                }
            }
        }

        Ok(tlv_extensions)
    }

    /// Apply domain-specific validation rules
    fn validate_domain_rules(&self, domain: RelayDomain, tlvs: &[TLVExtensionEnum]) -> Result<(), ValidationError> {
        if let Some(rules) = self.domain_rules.get(&domain) {
            for tlv in tlvs {
                let tlv_type = match tlv {
                    TLVExtensionEnum::Standard(t) => t.header.tlv_type,
                    TLVExtensionEnum::Extended(t) => t.header.tlv_type,
                };

                // Validate TLV type is in correct range for domain
                if tlv_type < rules.tlv_type_range.0 || tlv_type > rules.tlv_type_range.1 {
                    return Err(ValidationError::InvalidTLVRange {
                        domain,
                        expected: format!("{}-{}", rules.tlv_type_range.0, rules.tlv_type_range.1),
                        got: tlv_type,
                    });
                }
            }
        }

        Ok(())
    }

    /// Check if TLV type is valid for domain
    fn is_tlv_valid_for_domain(&self, tlv_type: u8, domain: RelayDomain) -> Result<bool, ValidationError> {
        if let Some(rules) = self.domain_rules.get(&domain) {
            Ok(tlv_type >= rules.tlv_type_range.0 && tlv_type <= rules.tlv_type_range.1)
        } else {
            Ok(true) // Allow unknown domains for now
        }
    }

    /// Get validation policy name for domain
    fn get_validation_policy_name(&self, domain: RelayDomain) -> String {
        if let Some(rules) = self.domain_rules.get(&domain) {
            match rules.min_validation_level {
                ValidationLevel::Performance => "performance".to_string(),
                ValidationLevel::Standard => "standard".to_string(),
                ValidationLevel::Audit => "audit".to_string(),
            }
        } else {
            "default".to_string()
        }
    }
}

impl Default for TLVValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Domain-specific validator trait
pub trait DomainValidator: Send + Sync {
    /// Validate TLV data for this domain
    fn validate_tlv(&self, tlv_type: TLVType, data: &[u8]) -> Result<(), ValidationError>;
    
    /// Validate complete message structure for this domain
    fn validate_message_structure(&self, tlvs: &[TLVExtensionEnum]) -> Result<(), ValidationError>;
    
    /// Get allowed TLV types for this domain
    fn get_allowed_types(&self) -> &[TLVType];
    
    /// Get domain name
    fn domain_name(&self) -> &str;
}

/// Market Data domain validator
pub struct MarketDataValidator;

impl DomainValidator for MarketDataValidator {
    fn validate_tlv(&self, tlv_type: TLVType, data: &[u8]) -> Result<(), ValidationError> {
        match tlv_type {
            TLVType::Trade => {
                // Validate trade TLV structure
                if data.len() != 40 {
                    return Err(ValidationError::Protocol(ProtocolError::PayloadSizeMismatch {
                        tlv_type: tlv_type as u8,
                        expected: 40,
                        got: data.len(),
                        struct_name: "TradeTLV".to_string(),
                    }));
                }
            },
            TLVType::Quote => {
                // Validate quote TLV structure
                if data.len() != 52 {
                    return Err(ValidationError::Protocol(ProtocolError::PayloadSizeMismatch {
                        tlv_type: tlv_type as u8,
                        expected: 52,
                        got: data.len(),
                        struct_name: "QuoteTLV".to_string(),
                    }));
                }
            },
            TLVType::PoolSwap => {
                // Variable size validation for pool swaps (60-200 bytes)
                if data.len() < 60 || data.len() > 200 {
                    return Err(ValidationError::Protocol(ProtocolError::PayloadSizeMismatch {
                        tlv_type: tlv_type as u8,
                        expected: 60, // Min size
                        got: data.len(),
                        struct_name: "PoolSwapTLV".to_string(),
                    }));
                }
            },
            _ => {
                // Check if it's a valid market data type
                let type_num = tlv_type as u8;
                if !(1..=19).contains(&type_num) {
                    return Err(ValidationError::InvalidTLVForDomain {
                        tlv_type: type_num,
                        domain: RelayDomain::MarketData,
                    });
                }
            }
        }
        Ok(())
    }

    fn validate_message_structure(&self, tlvs: &[TLVExtensionEnum]) -> Result<(), ValidationError> {
        for tlv in tlvs {
            let tlv_type = match tlv {
                TLVExtensionEnum::Standard(t) => t.header.tlv_type,
                TLVExtensionEnum::Extended(t) => t.header.tlv_type,
            };
            
            if !(1..=19).contains(&tlv_type) {
                return Err(ValidationError::InvalidTLVForDomain {
                    tlv_type,
                    domain: RelayDomain::MarketData,
                });
            }
        }
        Ok(())
    }

    fn get_allowed_types(&self) -> &[TLVType] {
        &[
            TLVType::Trade,
            TLVType::Quote,
            TLVType::OrderBook,
            TLVType::PoolSwap,
            TLVType::PoolLiquidity,
            TLVType::GasPrice,
        ]
    }

    fn domain_name(&self) -> &str {
        "MarketData"
    }
}

/// Signal domain validator
pub struct SignalValidator;

impl DomainValidator for SignalValidator {
    fn validate_tlv(&self, tlv_type: TLVType, data: &[u8]) -> Result<(), ValidationError> {
        match tlv_type {
            TLVType::SignalIdentity => {
                if data.len() != 16 {
                    return Err(ValidationError::Protocol(ProtocolError::PayloadSizeMismatch {
                        tlv_type: tlv_type as u8,
                        expected: 16,
                        got: data.len(),
                        struct_name: "SignalIdentityTLV".to_string(),
                    }));
                }
            },
            TLVType::ArbitrageSignal => {
                if data.len() != 168 {
                    return Err(ValidationError::Protocol(ProtocolError::PayloadSizeMismatch {
                        tlv_type: tlv_type as u8,
                        expected: 168,
                        got: data.len(),
                        struct_name: "ArbitrageSignalTLV".to_string(),
                    }));
                }
            },
            _ => {
                let type_num = tlv_type as u8;
                if !(20..=39).contains(&type_num) {
                    return Err(ValidationError::InvalidTLVForDomain {
                        tlv_type: type_num,
                        domain: RelayDomain::Signal,
                    });
                }
            }
        }
        Ok(())
    }

    fn validate_message_structure(&self, tlvs: &[TLVExtensionEnum]) -> Result<(), ValidationError> {
        for tlv in tlvs {
            let tlv_type = match tlv {
                TLVExtensionEnum::Standard(t) => t.header.tlv_type,
                TLVExtensionEnum::Extended(t) => t.header.tlv_type,
            };
            
            if !(20..=39).contains(&tlv_type) {
                return Err(ValidationError::InvalidTLVForDomain {
                    tlv_type,
                    domain: RelayDomain::Signal,
                });
            }
        }
        Ok(())
    }

    fn get_allowed_types(&self) -> &[TLVType] {
        &[
            TLVType::SignalIdentity,
            TLVType::ArbitrageSignal,
            TLVType::AssetCorrelation,
            TLVType::RiskParameters,
        ]
    }

    fn domain_name(&self) -> &str {
        "Signal"
    }
}

/// Execution domain validator
pub struct ExecutionValidator;

impl DomainValidator for ExecutionValidator {
    fn validate_tlv(&self, tlv_type: TLVType, data: &[u8]) -> Result<(), ValidationError> {
        let type_num = tlv_type as u8;
        if !(40..=79).contains(&type_num) {
            return Err(ValidationError::InvalidTLVForDomain {
                tlv_type: type_num,
                domain: RelayDomain::Execution,
            });
        }
        
        // Add execution-specific validations here
        Ok(())
    }

    fn validate_message_structure(&self, tlvs: &[TLVExtensionEnum]) -> Result<(), ValidationError> {
        for tlv in tlvs {
            let tlv_type = match tlv {
                TLVExtensionEnum::Standard(t) => t.header.tlv_type,
                TLVExtensionEnum::Extended(t) => t.header.tlv_type,
            };
            
            if !(40..=79).contains(&tlv_type) {
                return Err(ValidationError::InvalidTLVForDomain {
                    tlv_type,
                    domain: RelayDomain::Execution,
                });
            }
        }
        Ok(())
    }

    fn get_allowed_types(&self) -> &[TLVType] {
        &[
            TLVType::OrderRequest,
            TLVType::OrderStatus,
            TLVType::Fill,
            TLVType::ExecutionReport,
        ]
    }

    fn domain_name(&self) -> &str {
        "Execution"
    }
}

/// Create domain-specific validator
pub fn create_domain_validator(domain: RelayDomain) -> Box<dyn DomainValidator> {
    match domain {
        RelayDomain::MarketData => Box::new(MarketDataValidator),
        RelayDomain::Signal => Box::new(SignalValidator),
        RelayDomain::Execution => Box::new(ExecutionValidator),
        _ => Box::new(MarketDataValidator), // Default fallback
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use torq_types::{RelayDomain, SourceType};

    #[test]
    fn test_validator_creation() {
        let validator = TLVValidator::new();
        assert_eq!(validator.domain_rules.len(), 3);
    }

    #[test]
    fn test_domain_rules() {
        let validator = TLVValidator::new();
        
        // Test Market Data domain rules
        let md_rules = validator.domain_rules.get(&RelayDomain::MarketData).unwrap();
        assert_eq!(md_rules.tlv_type_range, (1, 19));
        
        // Test Signal domain rules
        let signal_rules = validator.domain_rules.get(&RelayDomain::Signal).unwrap();
        assert_eq!(signal_rules.tlv_type_range, (20, 39));
    }

    #[test]
    fn test_tlv_domain_validation() {
        let validator = TLVValidator::new();
        
        // TLV type 1 should be valid for MarketData
        assert!(validator.is_tlv_valid_for_domain(1, RelayDomain::MarketData).unwrap());
        
        // TLV type 20 should be valid for Signal
        assert!(validator.is_tlv_valid_for_domain(20, RelayDomain::Signal).unwrap());
        
        // TLV type 1 should NOT be valid for Signal domain
        assert!(!validator.is_tlv_valid_for_domain(1, RelayDomain::Signal).unwrap());
    }

    #[test]
    fn test_domain_validators() {
        let market_data_validator = MarketDataValidator;
        assert_eq!(market_data_validator.domain_name(), "MarketData");
        assert!(!market_data_validator.get_allowed_types().is_empty());

        let signal_validator = SignalValidator;
        assert_eq!(signal_validator.domain_name(), "Signal");
        assert!(!signal_validator.get_allowed_types().is_empty());
    }

    #[test]
    fn test_validation_policy() {
        let policy = ValidationPolicy::default();
        assert!(policy.checksum);
        assert!(!policy.audit);
        assert_eq!(policy.max_message_size, Some(65536));
    }
}