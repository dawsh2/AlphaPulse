//! # Domain-specific validators
//!
//! Provides specialized validation logic for each relay domain.

mod market_data;
mod signal;
mod execution;

pub use market_data::MarketDataValidator;
pub use signal::SignalValidator;
pub use execution::ExecutionValidator;

use super::validator::ValidationError;
use crate::tlv_types::TLVType;
use crate::parser::TLVExtensionEnum;
use torq_types::RelayDomain;

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

/// Create domain-specific validator
pub fn create_domain_validator(domain: RelayDomain) -> Box<dyn DomainValidator> {
    match domain {
        RelayDomain::MarketData => Box::new(MarketDataValidator),
        RelayDomain::Signal => Box::new(SignalValidator),
        RelayDomain::Execution => Box::new(ExecutionValidator),
        _ => Box::new(MarketDataValidator), // Default fallback
    }
}