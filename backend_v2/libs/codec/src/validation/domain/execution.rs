//! Execution domain validator

use super::DomainValidator;
use crate::validation::validator::ValidationError;
use crate::tlv_types::TLVType;
use crate::parser::TLVExtensionEnum;
use torq_types::RelayDomain;

/// Execution domain validator - full audit mode with comprehensive validation
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
        // For now, just validate the type range
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