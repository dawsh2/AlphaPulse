//! Market Data domain validator

use super::DomainValidator;
use crate::validation::validator::ValidationError;
use crate::tlv_types::TLVType;
use crate::parser::TLVExtensionEnum;
use crate::error::ProtocolError;
use torq_types::RelayDomain;

/// Market Data domain validator - optimized for performance
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