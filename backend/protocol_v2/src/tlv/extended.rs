//! Extended TLV Support (Type 255)
//! 
//! Handles TLVs with payloads larger than 255 bytes using the extended format:
//! ┌─────┬─────┬─────┬─────┬─────────────┐
//! │ 255 │ 0   │ T   │ L   │ Value       │
//! │ 1B  │ 1B  │ 1B  │ 2B  │ L bytes     │
//! └─────┴─────┴─────┴─────┴─────────────┘

use super::{TLVType, ExtendedTLVHeader, ParseError, ParseResult};
use zerocopy::{AsBytes, FromBytes};

/// Maximum payload size for extended TLVs (65KB)
pub const MAX_EXTENDED_PAYLOAD_SIZE: usize = 65535;

/// Helper for creating extended TLV payloads
pub struct ExtendedTLVPayload {
    pub tlv_type: u8,
    pub payload: Vec<u8>,
}

impl ExtendedTLVPayload {
    /// Create a new extended TLV payload
    pub fn new(tlv_type: TLVType, payload: Vec<u8>) -> ParseResult<Self> {
        if payload.len() > MAX_EXTENDED_PAYLOAD_SIZE {
            return Err(ParseError::PayloadTooLarge { size: payload.len() });
        }
        
        Ok(Self {
            tlv_type: tlv_type as u8,
            payload,
        })
    }
    
    /// Create from a struct that implements AsBytes
    pub fn from_struct<T: AsBytes>(tlv_type: TLVType, data: &T) -> ParseResult<Self> {
        Self::new(tlv_type, data.as_bytes().to_vec())
    }
    
    /// Serialize to extended TLV format bytes
    pub fn serialize(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(5 + self.payload.len());
        
        // Extended TLV header
        bytes.push(255); // Marker
        bytes.push(0);   // Reserved
        bytes.push(self.tlv_type);
        bytes.extend_from_slice(&(self.payload.len() as u16).to_le_bytes());
        
        // Payload
        bytes.extend_from_slice(&self.payload);
        
        bytes
    }
    
    /// Get the total serialized size
    pub fn serialized_size(&self) -> usize {
        5 + self.payload.len()
    }
}

/// Parse extended TLV header from bytes
pub fn parse_extended_header(data: &[u8]) -> ParseResult<ExtendedTLVHeader> {
    if data.len() < 5 {
        return Err(ParseError::TruncatedTLV { offset: 0 });
    }
    
    if data[0] != 255 {
        return Err(ParseError::InvalidExtendedTLV);
    }
    
    if data[1] != 0 {
        return Err(ParseError::InvalidExtendedTLV);
    }
    
    let tlv_type = data[2];
    let length = u16::from_le_bytes([data[3], data[4]]);
    
    Ok(ExtendedTLVHeader {
        marker: 255,
        reserved: 0,
        tlv_type,
        tlv_length: length,
    })
}

/// Extract payload from extended TLV bytes
pub fn extract_extended_payload(data: &[u8]) -> ParseResult<&[u8]> {
    let header = parse_extended_header(data)?;
    let payload_start = 5;
    let payload_end = payload_start + header.tlv_length as usize;
    
    if data.len() < payload_end {
        return Err(ParseError::TruncatedTLV { offset: payload_start });
    }
    
    Ok(&data[payload_start..payload_end])
}

/// Extract typed payload from extended TLV bytes
pub fn extract_extended_typed_payload<T>(data: &[u8]) -> ParseResult<T>
where
    T: FromBytes + Copy,
{
    let payload = extract_extended_payload(data)?;
    
    if payload.len() < std::mem::size_of::<T>() {
        return Err(ParseError::MessageTooSmall {
            need: std::mem::size_of::<T>(),
            got: payload.len(),
        });
    }
    
    let layout = zerocopy::Ref::<_, T>::new(&payload[..std::mem::size_of::<T>()])
        .ok_or(ParseError::MessageTooSmall {
            need: std::mem::size_of::<T>(),
            got: payload.len(),
        })?;
    
    Ok(*layout.into_ref())
}

/// Utility for working with large payloads that need extended TLVs
pub struct LargePayloadBuilder {
    chunks: Vec<ExtendedTLVPayload>,
}

impl LargePayloadBuilder {
    /// Create a new large payload builder
    pub fn new() -> Self {
        Self {
            chunks: Vec::new(),
        }
    }
    
    /// Add a chunk of data (automatically creates extended TLV if needed)
    pub fn add_chunk(&mut self, tlv_type: TLVType, data: Vec<u8>) -> ParseResult<()> {
        if data.len() > MAX_EXTENDED_PAYLOAD_SIZE {
            // Split large data into multiple chunks
            let mut offset = 0;
            let mut chunk_index = 0;
            
            while offset < data.len() {
                // Reserve 4 bytes for chunk metadata, so actual data chunk is smaller
                let chunk_size = std::cmp::min(MAX_EXTENDED_PAYLOAD_SIZE - 4, data.len() - offset);
                let mut chunk_data = Vec::with_capacity(chunk_size + 4);
                
                // Add chunk metadata (index as u32)
                chunk_data.extend_from_slice(&(chunk_index as u32).to_le_bytes());
                chunk_data.extend_from_slice(&data[offset..offset + chunk_size]);
                
                self.chunks.push(ExtendedTLVPayload::new(tlv_type, chunk_data)?);
                
                offset += chunk_size;
                chunk_index += 1;
            }
        } else {
            self.chunks.push(ExtendedTLVPayload::new(tlv_type, data)?);
        }
        
        Ok(())
    }
    
    /// Get all chunks
    pub fn chunks(&self) -> &[ExtendedTLVPayload] {
        &self.chunks
    }
    
    /// Calculate total serialized size
    pub fn total_size(&self) -> usize {
        self.chunks.iter().map(|c| c.serialized_size()).sum()
    }
}

impl Default for LargePayloadBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extended_tlv_payload_creation() {
        let payload = vec![0x42; 1000];
        let ext_tlv = ExtendedTLVPayload::new(TLVType::SignalIdentity, payload.clone()).unwrap();
        
        assert_eq!(ext_tlv.tlv_type, TLVType::SignalIdentity as u8);
        assert_eq!(ext_tlv.payload, payload);
    }
    
    #[test]
    fn test_extended_tlv_serialization() {
        let payload = vec![0x01, 0x02, 0x03, 0x04, 0x05];
        let ext_tlv = ExtendedTLVPayload::new(TLVType::OrderBook, payload.clone()).unwrap();
        
        let serialized = ext_tlv.serialize();
        
        // Should be: [255, 0, 3, 5, 0, 1, 2, 3, 4, 5]
        //           marker^  ^type ^len  ^payload
        assert_eq!(serialized.len(), 10);
        assert_eq!(serialized[0], 255); // Marker
        assert_eq!(serialized[1], 0);   // Reserved
        assert_eq!(serialized[2], TLVType::OrderBook as u8);
        assert_eq!(u16::from_le_bytes([serialized[3], serialized[4]]), 5); // Length
        assert_eq!(&serialized[5..], payload);
    }
    
    #[test]
    fn test_extended_header_parsing() {
        let data = [255u8, 0, 42, 0x34, 0x12]; // length = 0x1234 = 4660
        
        let header = parse_extended_header(&data).unwrap();
        let marker = header.marker;
        let reserved = header.reserved;
        let tlv_type = header.tlv_type;
        let tlv_length = header.tlv_length;
        assert_eq!(marker, 255);
        assert_eq!(reserved, 0);
        assert_eq!(tlv_type, 42);
        assert_eq!(tlv_length, 0x1234);
    }
    
    #[test]
    fn test_payload_extraction() {
        let mut data = vec![255u8, 0, 100, 4, 0]; // length = 4
        data.extend_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD]);
        
        let payload = extract_extended_payload(&data).unwrap();
        assert_eq!(payload, &[0xAA, 0xBB, 0xCC, 0xDD]);
    }
    
    #[test]
    fn test_large_payload_builder() {
        let mut builder = LargePayloadBuilder::new();
        
        // Add a normal-sized payload
        builder.add_chunk(TLVType::InstrumentMeta, vec![0x01; 1000]).unwrap();
        assert_eq!(builder.chunks().len(), 1);
        
        // Add a huge payload that should be split
        let large_data = vec![0x02; 200000]; // 200KB
        builder.add_chunk(TLVType::OrderBook, large_data).unwrap();
        
        // Should have created multiple chunks (200000 / 65535 ≈ 4 chunks)
        assert!(builder.chunks().len() >= 4);
    }
    
    #[test]
    fn test_payload_size_limit() {
        let oversized_payload = vec![0xFF; MAX_EXTENDED_PAYLOAD_SIZE + 1];
        let result = ExtendedTLVPayload::new(TLVType::Error, oversized_payload);
        
        assert!(result.is_err());
        // Check that it's a PayloadTooLarge error
        if let Err(ParseError::PayloadTooLarge { size }) = result {
            assert!(size > MAX_EXTENDED_PAYLOAD_SIZE);
        } else {
            panic!("Expected PayloadTooLarge error");
        }
    }
    
    #[test]
    fn test_invalid_extended_header() {
        // Wrong marker
        let bad_data1 = [254u8, 0, 42, 0, 4];
        assert!(parse_extended_header(&bad_data1).is_err());
        
        // Wrong reserved byte
        let bad_data2 = [255u8, 1, 42, 0, 4];
        assert!(parse_extended_header(&bad_data2).is_err());
        
        // Too short
        let bad_data3 = [255u8, 0, 42];
        assert!(parse_extended_header(&bad_data3).is_err());
    }
}