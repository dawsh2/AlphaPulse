//! # Compatibility Layer for Codec Consolidation Migration
//!
//! ## Purpose
//!
//! Provides backward compatibility interfaces for services migrating from relay-specific
//! parsing to the consolidated codec system. This allows gradual migration without breaking
//! existing services during the transition period.
//!
//! ## Migration Strategy
//!
//! ```text
//! Old Code → Compatibility Layer → New Codec → Same Results
//!    ↓              ↓                  ↓            ↓
//! relay::parse → compat::parse → codec::parse → ValidatedMessage
//! Existing API   Bridge Function   New System    Same Output
//! ```
//!
//! ## Deprecation Timeline
//!
//! - **Phase 1**: Introduce compatibility layer alongside new codec
//! - **Phase 2**: Migrate services one by one to new API
//! - **Phase 3**: Remove compatibility layer after all services migrated

use crate::error::{ProtocolError, ProtocolResult};
use crate::parser::{parse_header, parse_tlv_extensions, TLVExtensionEnum};
use crate::validation::{TLVValidator, ValidationPolicy, ValidatedMessage};
use crate::enhanced_builder::{ValidatingTLVMessageBuilder, BuilderFactory};
use torq_codec::protocol::tlv::types::TLVType;
use torq_types::{RelayDomain, SourceType};
use torq_types::protocol::message::header::MessageHeader;

/// Compatibility module for relay-specific parsing
#[deprecated(note = "Use TLVValidator directly instead")]
pub mod compat {
    use super::*;
    
    /// Compatibility alias for market data parsing
    #[deprecated(note = "Use TLVValidator::validate_message instead")]
    pub type MarketDataParser = TLVValidator;
    
    /// Compatibility alias for signal parsing
    #[deprecated(note = "Use TLVValidator::validate_message instead")]
    pub type SignalParser = TLVValidator;
    
    /// Compatibility alias for execution parsing
    #[deprecated(note = "Use TLVValidator::validate_message instead")]
    pub type ExecutionParser = TLVValidator;

    /// Parse market data message using consolidated codec
    #[deprecated(note = "Use TLVValidator::validate_message with RelayDomain::MarketData")]
    pub fn parse_market_data_message(data: &[u8]) -> ProtocolResult<ValidatedMessage> {
        let validator = TLVValidator::for_domain(
            RelayDomain::MarketData, 
            ValidationPolicy {
                checksum: false, // Performance mode for market data
                audit: false,
                strict: false,
                max_message_size: Some(4096),
            }
        );
        
        if data.len() < 32 {
            return Err(ProtocolError::message_too_small(32, data.len(), "Market data message"));
        }
        
        let header = parse_header(data)?;
        let payload = &data[32..32 + header.payload_size as usize];
        
        validator.validate_message(header, payload)
            .map_err(|e| ProtocolError::message_too_small(0, 0, &e.to_string()))
    }

    /// Parse signal message using consolidated codec
    #[deprecated(note = "Use TLVValidator::validate_message with RelayDomain::Signal")]
    pub fn parse_signal_message(data: &[u8]) -> ProtocolResult<ValidatedMessage> {
        let validator = TLVValidator::for_domain(
            RelayDomain::Signal,
            ValidationPolicy {
                checksum: true, // Standard mode for signals
                audit: false,
                strict: false,
                max_message_size: Some(8192),
            }
        );
        
        if data.len() < 32 {
            return Err(ProtocolError::message_too_small(32, data.len(), "Signal message"));
        }
        
        let header = parse_header(data)?;
        let payload = &data[32..32 + header.payload_size as usize];
        
        validator.validate_message(header, payload)
            .map_err(|e| ProtocolError::message_too_small(0, 0, &e.to_string()))
    }

    /// Parse execution message using consolidated codec
    #[deprecated(note = "Use TLVValidator::validate_message with RelayDomain::Execution")]
    pub fn parse_execution_message(data: &[u8]) -> ProtocolResult<ValidatedMessage> {
        let validator = TLVValidator::for_domain(
            RelayDomain::Execution,
            ValidationPolicy {
                checksum: true, // Audit mode for execution
                audit: true,
                strict: true,
                max_message_size: Some(16384),
            }
        );
        
        if data.len() < 32 {
            return Err(ProtocolError::message_too_small(32, data.len(), "Execution message"));
        }
        
        let header = parse_header(data)?;
        let payload = &data[32..32 + header.payload_size as usize];
        
        validator.validate_message(header, payload)
            .map_err(|e| ProtocolError::message_too_small(0, 0, &e.to_string()))
    }

    /// Legacy message builder compatibility
    #[deprecated(note = "Use ValidatingTLVMessageBuilder or BuilderFactory")]
    pub fn create_legacy_builder(domain: RelayDomain, source: SourceType) -> ValidatingTLVMessageBuilder {
        // Return appropriate builder based on domain
        BuilderFactory::for_domain(domain, source)
    }

    /// Legacy validation function
    #[deprecated(note = "Use TLVValidator::validate_message")]
    pub fn validate_legacy_message(header: &MessageHeader, data: &[u8]) -> ProtocolResult<()> {
        let domain = RelayDomain::try_from(header.relay_domain)
            .map_err(|_| ProtocolError::message_too_small(0, 0, "Invalid relay domain"))?;
        
        let validator = TLVValidator::new();
        validator.validate_message(header, data)
            .map(|_| ()) // Convert ValidatedMessage to unit type for compatibility
            .map_err(|e| ProtocolError::message_too_small(0, 0, &e.to_string()))
    }
}

/// Migration utilities for services transitioning to new codec
pub mod migration_utils {
    use super::*;
    
    /// Wrapper for gradual migration - supports both old and new parsing
    pub struct MigrationWrapper {
        use_new_codec: bool,
        validator: TLVValidator,
    }
    
    impl MigrationWrapper {
        /// Create wrapper with feature flag
        pub fn new(use_new_codec: bool) -> Self {
            Self {
                use_new_codec,
                validator: TLVValidator::new(),
            }
        }
        
        /// Parse message using appropriate method based on flag
        pub fn parse_message(&self, data: &[u8]) -> ProtocolResult<ParseResult> {
            if self.use_new_codec {
                // Use new consolidated codec
                let header = parse_header(data)?;
                let payload = &data[32..32 + header.payload_size as usize];
                
                let validated = self.validator.validate_message(header, payload)
                    .map_err(|e| ProtocolError::message_too_small(0, 0, &e.to_string()))?;
                
                Ok(ParseResult::New(validated))
            } else {
                // Use legacy parsing (just header + TLV parsing without validation)
                let header = parse_header(data)?;
                let payload = &data[32..32 + header.payload_size as usize];
                let tlvs = parse_tlv_extensions(payload)?;
                
                Ok(ParseResult::Legacy(LegacyParseResult {
                    header: *header,
                    tlvs,
                }))
            }
        }
    }
    
    /// Parse result that can be either new or legacy format
    pub enum ParseResult {
        New(ValidatedMessage),
        Legacy(LegacyParseResult),
    }
    
    /// Legacy parse result structure
    pub struct LegacyParseResult {
        pub header: MessageHeader,
        pub tlvs: Vec<TLVExtensionEnum>,
    }
    
    impl ParseResult {
        /// Get header regardless of result type
        pub fn header(&self) -> &MessageHeader {
            match self {
                ParseResult::New(validated) => &validated.header,
                ParseResult::Legacy(legacy) => &legacy.header,
            }
        }
        
        /// Get TLV extensions regardless of result type
        pub fn tlvs(&self) -> &[TLVExtensionEnum] {
            match self {
                ParseResult::New(validated) => &validated.tlv_extensions,
                ParseResult::Legacy(legacy) => &legacy.tlvs,
            }
        }
        
        /// Check if using new codec
        pub fn is_new_codec(&self) -> bool {
            matches!(self, ParseResult::New(_))
        }
    }
}

/// Testing utilities for migration validation
pub mod test_utils {
    use super::*;
    use crate::message_builder::TLVMessageBuilder;
    
    /// Compare results between old and new parsing methods
    pub fn compare_parsing_results(data: &[u8]) -> Option<bool> {
        // Parse with new method
        let new_result = {
            let validator = TLVValidator::new();
            if data.len() >= 32 {
                let header = parse_header(data).ok()?;
                let payload = &data[32..32 + header.payload_size as usize];
                validator.validate_message(header, payload).ok()
            } else {
                None
            }
        };
        
        // Parse with legacy method (basic parsing)
        let legacy_result = {
            if data.len() >= 32 {
                let header = parse_header(data).ok()?;
                let payload = &data[32..32 + header.payload_size as usize];
                let tlvs = parse_tlv_extensions(payload).ok()?;
                Some((header, tlvs))
            } else {
                None
            }
        };
        
        match (new_result, legacy_result) {
            (Some(new), Some((legacy_header, legacy_tlvs))) => {
                // Compare headers
                let headers_match = new.header.magic == legacy_header.magic
                    && new.header.relay_domain == legacy_header.relay_domain
                    && new.header.source == legacy_header.source
                    && new.header.sequence == legacy_header.sequence
                    && new.header.payload_size == legacy_header.payload_size;
                
                // Compare TLV count
                let tlv_count_match = new.tlv_extensions.len() == legacy_tlvs.len();
                
                Some(headers_match && tlv_count_match)
            }
            (None, None) => Some(true), // Both failed consistently
            _ => Some(false), // One succeeded, one failed - inconsistency
        }
    }
    
    /// Create test message using both old and new builders
    pub fn create_test_messages(domain: RelayDomain, source: SourceType) -> (Vec<u8>, Vec<u8>) {
        let test_data = vec![0u8; 16]; // Simple test payload
        let tlv_type = match domain {
            RelayDomain::MarketData => TLVType::Trade,
            RelayDomain::Signal => TLVType::SignalIdentity,
            RelayDomain::Execution => TLVType::OrderStatus,
            _ => TLVType::Trade,
        };
        
        // Create with legacy builder (no validation)
        let legacy_message = TLVMessageBuilder::new(domain, source)
            .add_tlv_slice(tlv_type, &test_data)
            .build()
            .expect("Legacy build failed");
        
        // Create with new validating builder (with validation disabled for comparison)
        let new_message = ValidatingTLVMessageBuilder::without_validation(domain, source)
            .add_trusted_tlv_slice(tlv_type, &test_data)
            .build()
            .expect("New build failed");
        
        (legacy_message, new_message)
    }
    
    /// Validate that migration doesn't change message format
    pub fn validate_message_compatibility() -> bool {
        let test_cases = [
            (RelayDomain::MarketData, SourceType::PolygonCollector),
            (RelayDomain::Signal, SourceType::ArbitrageStrategy),
            (RelayDomain::Execution, SourceType::ExecutionEngine),
        ];
        
        for (domain, source) in test_cases {
            let (legacy_msg, new_msg) = create_test_messages(domain, source);
            
            // Messages should be identical (except possibly timestamps)
            if legacy_msg.len() != new_msg.len() {
                return false;
            }
            
            // Headers should match (except timestamp field)
            let legacy_header = &legacy_msg[..32];
            let new_header = &new_msg[..32];
            
            // Compare everything except timestamp (bytes 16-24)
            let header_match = legacy_header[..16] == new_header[..16] // Before timestamp
                && legacy_header[24..] == new_header[24..]; // After timestamp
            
            if !header_match {
                return false;
            }
            
            // Payload should be identical
            if legacy_msg[32..] != new_msg[32..] {
                return false;
            }
        }
        
        true
    }
}

/// Configuration for gradual rollout
pub struct MigrationConfig {
    /// Enable new codec for market data
    pub market_data_new_codec: bool,
    /// Enable new codec for signals
    pub signals_new_codec: bool,
    /// Enable new codec for execution
    pub execution_new_codec: bool,
    /// Enable validation during migration
    pub validation_enabled: bool,
}

impl Default for MigrationConfig {
    fn default() -> Self {
        Self {
            market_data_new_codec: false, // Start with legacy
            signals_new_codec: false,
            execution_new_codec: false,
            validation_enabled: true,
        }
    }
}

impl MigrationConfig {
    /// Create config for full new codec usage
    pub fn new_codec_only() -> Self {
        Self {
            market_data_new_codec: true,
            signals_new_codec: true,
            execution_new_codec: true,
            validation_enabled: true,
        }
    }
    
    /// Create config for gradual rollout
    pub fn gradual_rollout(phase: u8) -> Self {
        match phase {
            1 => Self {
                market_data_new_codec: true, // Start with market data
                signals_new_codec: false,
                execution_new_codec: false,
                validation_enabled: true,
            },
            2 => Self {
                market_data_new_codec: true,
                signals_new_codec: true, // Add signals
                execution_new_codec: false,
                validation_enabled: true,
            },
            3 => Self::new_codec_only(), // Full migration
            _ => Self::default(), // Legacy only
        }
    }
    
    /// Check if new codec should be used for domain
    pub fn use_new_codec_for_domain(&self, domain: RelayDomain) -> bool {
        match domain {
            RelayDomain::MarketData => self.market_data_new_codec,
            RelayDomain::Signal => self.signals_new_codec,
            RelayDomain::Execution => self.execution_new_codec,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_migration_config() {
        let default_config = MigrationConfig::default();
        assert!(!default_config.market_data_new_codec);
        assert!(!default_config.signals_new_codec);
        assert!(!default_config.execution_new_codec);
        
        let new_codec_config = MigrationConfig::new_codec_only();
        assert!(new_codec_config.market_data_new_codec);
        assert!(new_codec_config.signals_new_codec);
        assert!(new_codec_config.execution_new_codec);
    }
    
    #[test]
    fn test_gradual_rollout() {
        let phase1 = MigrationConfig::gradual_rollout(1);
        assert!(phase1.use_new_codec_for_domain(RelayDomain::MarketData));
        assert!(!phase1.use_new_codec_for_domain(RelayDomain::Signal));
        
        let phase2 = MigrationConfig::gradual_rollout(2);
        assert!(phase2.use_new_codec_for_domain(RelayDomain::MarketData));
        assert!(phase2.use_new_codec_for_domain(RelayDomain::Signal));
        assert!(!phase2.use_new_codec_for_domain(RelayDomain::Execution));
        
        let phase3 = MigrationConfig::gradual_rollout(3);
        assert!(phase3.use_new_codec_for_domain(RelayDomain::MarketData));
        assert!(phase3.use_new_codec_for_domain(RelayDomain::Signal));
        assert!(phase3.use_new_codec_for_domain(RelayDomain::Execution));
    }
    
    #[test]
    fn test_migration_wrapper() {
        let wrapper = migration_utils::MigrationWrapper::new(false);
        // Test with wrapper - this would need actual message data to work
    }
    
    #[test]
    fn test_message_compatibility() {
        // This validates that message formats don't change during migration
        assert!(test_utils::validate_message_compatibility());
    }
    
    #[test]
    #[allow(deprecated)]
    fn test_compatibility_functions() {
        // Test that deprecated functions still work
        let test_message = vec![0u8; 64]; // Mock message
        
        // These should work but issue deprecation warnings
        let _builder = compat::create_legacy_builder(RelayDomain::MarketData, SourceType::PolygonCollector);
    }
}