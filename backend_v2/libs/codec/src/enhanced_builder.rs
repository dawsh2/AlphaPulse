//! # Enhanced TLV Message Builder with Validation
//!
//! ## Purpose
//!
//! Enhanced version of TLVMessageBuilder that includes domain-specific validation
//! during message construction. This ensures messages are valid before sending,
//! preventing invalid messages from being transmitted through the system.
//!
//! ## Integration with Validation System
//!
//! ```text
//! Builder Input → Domain Check → TLV Validation → Message Construction
//!       ↓             ↓             ↓                     ↓
//!   add_tlv()    Domain Rules   Size Checks        Complete Message
//!   Typed Data   Type Range     TLV Structure      Ready for Transport
//! ```
//!
//! ## Performance Considerations
//!
//! - **Optional Validation**: Can be enabled/disabled per builder instance
//! - **Early Validation**: Catches errors during construction, not transmission
//! - **Domain Optimization**: Different validation levels per relay domain

use crate::error::ProtocolResult;
use crate::message_builder::TLVMessageBuilder;
use torq_codec::protocol::tlv::types::TLVType;
use crate::validation::{TLVValidator, ValidationError, ValidationPolicy, DomainValidator, create_domain_validator};
use torq_types::{RelayDomain, SourceType};
use zerocopy::AsBytes;
use tracing::debug;

/// Enhanced message builder with validation capabilities
pub struct ValidatingTLVMessageBuilder {
    inner: TLVMessageBuilder,
    validator: Option<TLVValidator>,
    domain_validator: Option<Box<dyn DomainValidator>>,
    domain: RelayDomain,
    validation_enabled: bool,
}

impl ValidatingTLVMessageBuilder {
    /// Create a new validating builder
    pub fn new(relay_domain: RelayDomain, source: SourceType) -> Self {
        Self {
            inner: TLVMessageBuilder::new(relay_domain, source),
            validator: Some(TLVValidator::new()),
            domain_validator: Some(create_domain_validator(relay_domain)),
            domain: relay_domain,
            validation_enabled: true,
        }
    }

    /// Create builder with custom validation policy
    pub fn with_validation_policy(relay_domain: RelayDomain, source: SourceType, policy: ValidationPolicy) -> Self {
        Self {
            inner: TLVMessageBuilder::new(relay_domain, source),
            validator: Some(TLVValidator::for_domain(relay_domain, policy)),
            domain_validator: Some(create_domain_validator(relay_domain)),
            domain: relay_domain,
            validation_enabled: true,
        }
    }

    /// Create builder without validation (for performance-critical paths)
    pub fn without_validation(relay_domain: RelayDomain, source: SourceType) -> Self {
        Self {
            inner: TLVMessageBuilder::new(relay_domain, source),
            validator: None,
            domain_validator: None,
            domain: relay_domain,
            validation_enabled: false,
        }
    }

    /// Add TLV with validation
    pub fn add_validated_tlv<T: AsBytes>(mut self, tlv_type: TLVType, data: &T) -> Result<Self, ValidationError> {
        if self.validation_enabled {
            // Validate TLV type is appropriate for domain
            if let Some(domain_validator) = &self.domain_validator {
                let bytes = data.as_bytes();
                domain_validator.validate_tlv(tlv_type, bytes)?;
                
                debug!("TLV validation passed for type {:?} in domain {:?}", tlv_type, self.domain);
            }
        }

        // Add to inner builder
        self.inner = self.inner.add_tlv(tlv_type, data);
        Ok(self)
    }

    /// Add TLV slice with validation
    pub fn add_validated_tlv_slice(mut self, tlv_type: TLVType, payload: &[u8]) -> Result<Self, ValidationError> {
        if self.validation_enabled {
            if let Some(domain_validator) = &self.domain_validator {
                domain_validator.validate_tlv(tlv_type, payload)?;
            }
        }

        self.inner = self.inner.add_tlv_slice(tlv_type, payload);
        Ok(self)
    }

    /// Add TLV bytes with validation
    pub fn add_validated_tlv_bytes(mut self, tlv_type: TLVType, payload: Vec<u8>) -> Result<Self, ValidationError> {
        if self.validation_enabled {
            if let Some(domain_validator) = &self.domain_validator {
                domain_validator.validate_tlv(tlv_type, &payload)?;
            }
        }

        self.inner = self.inner.add_tlv_bytes(tlv_type, payload);
        Ok(self)
    }

    /// Add TLV without validation (bypass for trusted data)
    pub fn add_trusted_tlv<T: AsBytes>(mut self, tlv_type: TLVType, data: &T) -> Self {
        self.inner = self.inner.add_tlv(tlv_type, data);
        self
    }

    /// Add TLV slice without validation (bypass for trusted data)
    pub fn add_trusted_tlv_slice(mut self, tlv_type: TLVType, payload: &[u8]) -> Self {
        self.inner = self.inner.add_tlv_slice(tlv_type, payload);
        self
    }

    /// Set sequence number
    pub fn with_sequence(mut self, sequence: u64) -> Self {
        self.inner = self.inner.with_sequence(sequence);
        self
    }

    /// Set custom flags
    pub fn with_flags(mut self, flags: u8) -> Self {
        self.inner = self.inner.with_flags(flags);
        self
    }

    /// Set custom timestamp with validation (normally uses current time)
    pub fn with_timestamp(mut self, timestamp_ns: u64) -> Result<Self, ValidationError> {
        // Validate timestamp if validation is enabled
        if self.validation_enabled {
            self.validate_timestamp_bounds(timestamp_ns)?;
        }
        self.inner = self.inner.with_timestamp(timestamp_ns);
        Ok(self)
    }
    
    /// Validate timestamp is within acceptable bounds
    fn validate_timestamp_bounds(&self, timestamp_ns: u64) -> Result<(), ValidationError> {
        use std::time::{SystemTime, UNIX_EPOCH};
        
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| ValidationError::StrictModeViolation {
                reason: "System time before Unix epoch".to_string()
            })?
            .as_nanos() as u64;
        
        // Check if timestamp is too far in the future (>5 seconds)
        let max_future = current_time + 5_000_000_000; // 5 seconds in nanoseconds
        if timestamp_ns > max_future {
            return Err(ValidationError::StrictModeViolation {
                reason: format!("Timestamp {} is too far in future (current: {})", 
                    timestamp_ns, current_time)
            });
        }
        
        // Check if timestamp is too old (>60 seconds)
        let min_time = current_time.saturating_sub(60_000_000_000); // 60 seconds
        if timestamp_ns < min_time {
            return Err(ValidationError::StrictModeViolation {
                reason: format!("Timestamp {} is too old (current: {})", 
                    timestamp_ns, current_time)
            });
        }
        
        Ok(())
    }

    /// Build the final message with optional final validation
    pub fn build(self) -> ProtocolResult<Vec<u8>> {
        // Build the message using inner builder
        let message = self.inner.build()?;

        // Optional final validation
        if self.validation_enabled {
            if let Some(validator) = &self.validator {
                // Parse header for validation
                if message.len() >= 32 {
                    use crate::parser::parse_header;
                    // Preserve original error context
                    let header = parse_header(&message)?;
                    
                    // Validate complete message
                    let payload = &message[32..];
                    if let Err(validation_error) = validator.validate_message(header, payload) {
                        debug!("Final message validation failed: {}", validation_error);
                        // Convert validation error to protocol error
                        return Err(crate::error::ProtocolError::message_too_small(
                            0, 0, &format!("Final validation failed: {}", validation_error)
                        ));
                    }
                }
            }
        }

        Ok(message)
    }

    /// Build into buffer with validation
    pub fn build_into_buffer(self, buffer: &mut [u8]) -> Result<usize, std::io::Error> {
        let message = self.build()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let size = message.len();

        if buffer.len() < size {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Buffer too small: need {}, got {}", size, buffer.len()),
            ));
        }

        buffer[..size].copy_from_slice(&message);
        Ok(size)
    }

    /// Get current payload size
    pub fn payload_size(&self) -> usize {
        self.inner.payload_size()
    }

    /// Get TLV count
    pub fn tlv_count(&self) -> usize {
        self.inner.tlv_count()
    }

    /// Check if would exceed size limit
    pub fn would_exceed_size(&self, max_size: usize) -> bool {
        self.inner.would_exceed_size(max_size)
    }

    /// Enable/disable validation
    pub fn set_validation(mut self, enabled: bool) -> Self {
        self.validation_enabled = enabled;
        self
    }

    /// Check if validation is enabled
    pub fn is_validation_enabled(&self) -> bool {
        self.validation_enabled
    }
}

/// Convenience functions for common message patterns
pub mod patterns {
    use super::*;

    /// Create validated trade message
    pub fn create_trade_message(
        source: SourceType,
        trade_data: Vec<u8>,
    ) -> Result<Vec<u8>, ValidationError> {
        ValidatingTLVMessageBuilder::new(RelayDomain::MarketData, source)
            .add_validated_tlv_bytes(TLVType::Trade, trade_data)?
            .build()
            .map_err(|e| ValidationError::Protocol(e))
    }

    /// Create validated quote message  
    pub fn create_quote_message(
        source: SourceType,
        quote_data: Vec<u8>,
    ) -> Result<Vec<u8>, ValidationError> {
        ValidatingTLVMessageBuilder::new(RelayDomain::MarketData, source)
            .add_validated_tlv_bytes(TLVType::Quote, quote_data)?
            .build()
            .map_err(|e| ValidationError::Protocol(e))
    }

    /// Create validated signal message
    pub fn create_signal_message(
        source: SourceType,
        signal_data: Vec<u8>,
    ) -> Result<Vec<u8>, ValidationError> {
        ValidatingTLVMessageBuilder::new(RelayDomain::Signal, source)
            .add_validated_tlv_bytes(TLVType::SignalIdentity, signal_data)?
            .build()
            .map_err(|e| ValidationError::Protocol(e))
    }

    /// Create high-performance market data message (no validation)
    pub fn create_fast_market_message<T: AsBytes>(
        source: SourceType,
        tlv_type: TLVType,
        data: &T,
    ) -> ProtocolResult<Vec<u8>> {
        ValidatingTLVMessageBuilder::without_validation(RelayDomain::MarketData, source)
            .add_trusted_tlv(tlv_type, data)
            .build()
    }

    /// Create batch message with multiple TLVs
    pub fn create_batch_message(
        domain: RelayDomain,
        source: SourceType,
        tlv_data: Vec<(TLVType, Vec<u8>)>,
    ) -> Result<Vec<u8>, ValidationError> {
        let mut builder = ValidatingTLVMessageBuilder::new(domain, source);

        for (tlv_type, payload) in tlv_data {
            builder = builder.add_validated_tlv_bytes(tlv_type, payload)?;
        }

        builder.build().map_err(|e| ValidationError::Protocol(e))
    }
}

/// Builder factory for different validation policies
pub struct BuilderFactory;

impl BuilderFactory {
    /// Create performance-optimized builder (no validation)
    pub fn performance_builder(domain: RelayDomain, source: SourceType) -> ValidatingTLVMessageBuilder {
        ValidatingTLVMessageBuilder::without_validation(domain, source)
    }

    /// Create standard builder with checksum validation
    pub fn standard_builder(domain: RelayDomain, source: SourceType) -> ValidatingTLVMessageBuilder {
        let policy = ValidationPolicy {
            checksum: true,
            audit: false,
            strict: false,
            max_message_size: Some(32768), // 32KB
        };
        ValidatingTLVMessageBuilder::with_validation_policy(domain, source, policy)
    }

    /// Create audit builder with full validation
    pub fn audit_builder(domain: RelayDomain, source: SourceType) -> ValidatingTLVMessageBuilder {
        let policy = ValidationPolicy {
            checksum: true,
            audit: true,
            strict: true,
            max_message_size: Some(16384), // 16KB
        };
        ValidatingTLVMessageBuilder::with_validation_policy(domain, source, policy)
    }

    /// Create builder based on domain requirements
    pub fn for_domain(domain: RelayDomain, source: SourceType) -> ValidatingTLVMessageBuilder {
        match domain {
            RelayDomain::MarketData => Self::performance_builder(domain, source),
            RelayDomain::Signal => Self::standard_builder(domain, source),
            RelayDomain::Execution => Self::audit_builder(domain, source),
            _ => Self::standard_builder(domain, source),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validating_builder() {
        let builder = ValidatingTLVMessageBuilder::new(RelayDomain::MarketData, SourceType::PolygonCollector);
        assert!(builder.is_validation_enabled());
    }

    #[test]
    fn test_validation_bypass() {
        let builder = ValidatingTLVMessageBuilder::without_validation(RelayDomain::MarketData, SourceType::PolygonCollector);
        assert!(!builder.is_validation_enabled());
    }

    #[test]
    fn test_add_validated_tlv() {
        let trade_data = vec![0u8; 40]; // TradeTLV size
        
        let result = ValidatingTLVMessageBuilder::new(RelayDomain::MarketData, SourceType::PolygonCollector)
            .add_validated_tlv_bytes(TLVType::Trade, trade_data);

        assert!(result.is_ok());
    }

    #[test]
    fn test_domain_validation_error() {
        // Try to add Signal TLV to MarketData domain - should fail
        let signal_data = vec![0u8; 16]; // SignalIdentityTLV size
        
        let result = ValidatingTLVMessageBuilder::new(RelayDomain::MarketData, SourceType::PolygonCollector)
            .add_validated_tlv_bytes(TLVType::SignalIdentity, signal_data);

        assert!(result.is_err());
    }

    #[test]
    fn test_builder_factory() {
        let perf_builder = BuilderFactory::performance_builder(RelayDomain::MarketData, SourceType::PolygonCollector);
        assert!(!perf_builder.is_validation_enabled());

        let std_builder = BuilderFactory::standard_builder(RelayDomain::Signal, SourceType::ArbitrageStrategy);
        assert!(std_builder.is_validation_enabled());

        let audit_builder = BuilderFactory::audit_builder(RelayDomain::Execution, SourceType::ExecutionEngine);
        assert!(audit_builder.is_validation_enabled());
    }

    #[test]
    fn test_domain_based_factory() {
        let md_builder = BuilderFactory::for_domain(RelayDomain::MarketData, SourceType::PolygonCollector);
        assert!(!md_builder.is_validation_enabled()); // Performance mode

        let signal_builder = BuilderFactory::for_domain(RelayDomain::Signal, SourceType::ArbitrageStrategy);
        assert!(signal_builder.is_validation_enabled()); // Standard mode

        let exec_builder = BuilderFactory::for_domain(RelayDomain::Execution, SourceType::ExecutionEngine);
        assert!(exec_builder.is_validation_enabled()); // Audit mode
    }
}