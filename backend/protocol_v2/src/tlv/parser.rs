//! TLV Parser Implementation
//! 
//! Provides zero-copy parsing of TLV messages with support for both standard 
//! and extended TLVs.

use super::{ParseError, ParseResult, TLVHeader, ExtendedTLVHeader, TLVExtension, ExtendedTLVExtension, TLVType};
use crate::header::MessageHeader;
use crate::MESSAGE_MAGIC;
use std::mem::size_of;
use zerocopy::Ref;

/// Parse message header from bytes
pub fn parse_header(data: &[u8]) -> ParseResult<&MessageHeader> {
    if data.len() < size_of::<MessageHeader>() {
        return Err(ParseError::MessageTooSmall {
            need: size_of::<MessageHeader>(),
            got: data.len(),
        });
    }

    let header = Ref::<_, MessageHeader>::new(&data[..size_of::<MessageHeader>()])
        .ok_or(ParseError::MessageTooSmall {
            need: size_of::<MessageHeader>(),
            got: data.len(),
        })?
        .into_ref();

    if header.magic != MESSAGE_MAGIC {
        return Err(ParseError::InvalidMagic {
            expected: MESSAGE_MAGIC,
            actual: header.magic,
        });
    }

    // Validate checksum
    if !header.verify_checksum(data) {
        return Err(ParseError::ChecksumMismatch {
            expected: header.checksum,
            calculated: 0, // We'd need to calculate it again to get the real value
        });
    }

    Ok(header)
}

/// Parse all TLV extensions from payload, handling both standard and extended TLVs
pub fn parse_tlv_extensions(tlv_data: &[u8]) -> ParseResult<Vec<TLVExtensionEnum>> {
    let mut extensions = Vec::new();
    let mut offset = 0;

    while offset < tlv_data.len() {
        if offset + 2 > tlv_data.len() {
            return Err(ParseError::TruncatedTLV { offset });
        }

        let tlv_type = tlv_data[offset];
        
        if tlv_type == TLVType::ExtendedTLV as u8 {
            // Parse extended TLV (Type 255)
            let ext_tlv = parse_extended_tlv(&tlv_data[offset..])?;
            offset += 5 + ext_tlv.header.tlv_length as usize;
            extensions.push(TLVExtensionEnum::Extended(ext_tlv));
        } else {
            // Parse standard TLV
            let std_tlv = parse_standard_tlv(&tlv_data[offset..])?;
            offset += 2 + std_tlv.header.tlv_length as usize;
            extensions.push(TLVExtensionEnum::Standard(std_tlv));
        }
    }

    Ok(extensions)
}

/// Enum to hold either standard or extended TLV extensions
#[derive(Debug, Clone)]
pub enum TLVExtensionEnum {
    Standard(TLVExtension),
    Extended(ExtendedTLVExtension),
}

/// Parse a single standard TLV from data starting at offset 0
fn parse_standard_tlv(data: &[u8]) -> ParseResult<TLVExtension> {
    if data.len() < 2 {
        return Err(ParseError::TruncatedTLV { offset: 0 });
    }

    let tlv_type = data[0];
    let tlv_length = data[1] as usize;

    if data.len() < 2 + tlv_length {
        return Err(ParseError::TruncatedTLV { offset: 0 });
    }

    let header = TLVHeader { tlv_type, tlv_length: tlv_length as u8 };
    let payload = data[2..2 + tlv_length].to_vec();

    // Validate payload size for known fixed-size TLVs
    if let Ok(tlv_type_enum) = TLVType::try_from(tlv_type) {
        if let Some(expected_size) = tlv_type_enum.expected_payload_size() {
            if payload.len() != expected_size {
                return Err(ParseError::PayloadTooLarge { size: payload.len() });
            }
        }
    }

    Ok(TLVExtension { header, payload })
}

/// Parse a single extended TLV (Type 255) from data starting at offset 0  
fn parse_extended_tlv(data: &[u8]) -> ParseResult<ExtendedTLVExtension> {
    if data.len() < 5 {
        return Err(ParseError::TruncatedTLV { offset: 0 });
    }

    if data[0] != 255 {
        return Err(ParseError::InvalidExtendedTLV);
    }

    if data[1] != 0 {
        return Err(ParseError::InvalidExtendedTLV);
    }

    let actual_type = data[2];
    let length = u16::from_le_bytes([data[3], data[4]]) as usize;

    if data.len() < 5 + length {
        return Err(ParseError::TruncatedTLV { offset: 0 });
    }

    let header = ExtendedTLVHeader {
        marker: 255,
        reserved: 0,
        tlv_type: actual_type,
        tlv_length: length as u16,
    };
    
    let payload = data[5..5 + length].to_vec();

    Ok(ExtendedTLVExtension { header, payload })
}

/// Find specific TLV by type in the payload data
pub fn find_tlv_by_type(tlv_data: &[u8], target_type: u8) -> Option<&[u8]> {
    let mut offset = 0;

    while offset + 2 <= tlv_data.len() {
        let tlv_type = tlv_data[offset];
        
        if tlv_type == TLVType::ExtendedTLV as u8 {
            // Handle extended TLV
            if offset + 5 <= tlv_data.len() {
                let actual_type = tlv_data[offset + 2];
                let length = u16::from_le_bytes([tlv_data[offset + 3], tlv_data[offset + 4]]) as usize;
                
                if actual_type == target_type {
                    let start = offset + 5;
                    let end = start + length;
                    if end <= tlv_data.len() {
                        return Some(&tlv_data[start..end]);
                    }
                }
                offset += 5 + length;
            } else {
                break;
            }
        } else {
            // Handle standard TLV
            let tlv_length = tlv_data[offset + 1] as usize;
            
            if tlv_type == target_type {
                let start = offset + 2;
                let end = start + tlv_length;
                if end <= tlv_data.len() {
                    return Some(&tlv_data[start..end]);
                }
            }
            offset += 2 + tlv_length;
        }
    }

    None
}

/// Extract TLV payload for a specific type, handling both standard and extended
pub fn extract_tlv_payload<T>(tlv_data: &[u8], target_type: TLVType) -> ParseResult<Option<T>>
where
    T: zerocopy::FromBytes + Copy,
{
    if let Some(payload_bytes) = find_tlv_by_type(tlv_data, target_type as u8) {
        if payload_bytes.len() >= size_of::<T>() {
            let layout = Ref::<_, T>::new(&payload_bytes[..size_of::<T>()])
                .ok_or(ParseError::MessageTooSmall {
                    need: size_of::<T>(),
                    got: payload_bytes.len(),
                })?;
            Ok(Some(*layout.into_ref()))
        } else {
            Err(ParseError::MessageTooSmall {
                need: size_of::<T>(),
                got: payload_bytes.len(),
            })
        }
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::header::MessageHeader;
    use crate::{RelayDomain, SourceType};

    #[test]
    fn test_parse_standard_tlv() {
        // Create a simple TLV with a vendor-specific type that accepts any size
        // type=200 (vendor-specific), length=4, payload=[0x01, 0x02, 0x03, 0x04]
        let tlv_data = vec![200, 4, 0x01, 0x02, 0x03, 0x04];
        
        let tlv = parse_standard_tlv(&tlv_data).unwrap();
        assert_eq!(tlv.header.tlv_type, 200);
        assert_eq!(tlv.header.tlv_length, 4);
        assert_eq!(tlv.payload, vec![0x01, 0x02, 0x03, 0x04]);
    }

    #[test]
    fn test_parse_extended_tlv() {
        // Create extended TLV: marker=255, reserved=0, type=200, length=300, payload=[0x01; 300]
        let mut tlv_data = vec![255, 0, 200];
        tlv_data.extend_from_slice(&300u16.to_le_bytes());
        tlv_data.extend(vec![0x01; 300]);
        
        let ext_tlv = parse_extended_tlv(&tlv_data).unwrap();
        let marker = ext_tlv.header.marker;
        let reserved = ext_tlv.header.reserved;
        let tlv_type = ext_tlv.header.tlv_type;
        let tlv_length = ext_tlv.header.tlv_length;
        assert_eq!(marker, 255);
        assert_eq!(reserved, 0);
        assert_eq!(tlv_type, 200);
        assert_eq!(tlv_length, 300);
        assert_eq!(ext_tlv.payload.len(), 300);
        assert!(ext_tlv.payload.iter().all(|&b| b == 0x01));
    }

    #[test]
    fn test_find_tlv_by_type() {
        // Create multiple TLVs
        let mut tlv_data = Vec::new();
        // TLV 1: type=1, length=2, payload=[0xAA, 0xBB]
        tlv_data.extend_from_slice(&[1, 2, 0xAA, 0xBB]);
        // TLV 2: type=2, length=3, payload=[0xCC, 0xDD, 0xEE]
        tlv_data.extend_from_slice(&[2, 3, 0xCC, 0xDD, 0xEE]);
        // TLV 3: type=1, length=1, payload=[0xFF]
        tlv_data.extend_from_slice(&[1, 1, 0xFF]);
        
        // Find first TLV of type 1
        let payload = find_tlv_by_type(&tlv_data, 1).unwrap();
        assert_eq!(payload, &[0xAA, 0xBB]);
        
        // Find TLV of type 2
        let payload = find_tlv_by_type(&tlv_data, 2).unwrap();
        assert_eq!(payload, &[0xCC, 0xDD, 0xEE]);
        
        // Try to find non-existent type
        assert!(find_tlv_by_type(&tlv_data, 99).is_none());
    }

    #[test]
    fn test_truncated_tlv_error() {
        // TLV claims length=10 but only has 5 bytes
        let tlv_data = vec![1, 10, 0x01, 0x02, 0x03, 0x04, 0x05];
        
        let result = parse_standard_tlv(&tlv_data);
        assert!(result.is_err());
        matches!(result.unwrap_err(), ParseError::TruncatedTLV { .. });
    }
}