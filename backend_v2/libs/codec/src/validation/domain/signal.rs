//! Signal domain validator

use super::DomainValidator;
use crate::validation::validator::ValidationError;
use crate::tlv_types::TLVType;
use crate::parser::TLVExtensionEnum;
use crate::error::ProtocolError;
use torq_types::RelayDomain;

/// Signal domain validator - standard validation with checksums
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