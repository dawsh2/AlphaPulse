//! TLV (Type-Length-Value) Parsing and Processing
//! 
//! This module provides the core TLV functionality including:
//! - Standard TLV parsing (type + u8 length)  
//! - Extended TLV parsing (type 255 with u16 length)
//! - Message building and serialization
//! - Type definitions for all domains

pub mod parser;
pub mod builder;
pub mod types;
pub mod extended;
pub mod relay_parser;

pub use parser::*;
pub use builder::*;
pub use types::*;
pub use extended::*;
pub use relay_parser::*;

use thiserror::Error;

/// TLV parsing errors
#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Message too small: need {need} bytes, got {got}")]
    MessageTooSmall { need: usize, got: usize },
    
    #[error("Invalid magic number: expected {expected:#x}, got {actual:#x}")]
    InvalidMagic { expected: u32, actual: u32 },
    
    #[error("Checksum mismatch: expected {expected:#x}, calculated {calculated:#x}")]
    ChecksumMismatch { expected: u32, calculated: u32 },
    
    #[error("Truncated TLV at offset {offset}")]
    TruncatedTLV { offset: usize },
    
    #[error("Unknown TLV type: {0}")]
    UnknownTLVType(u8),
    
    #[error("Unknown source type: {0}")]
    UnknownSource(u8),
    
    #[error("Invalid extended TLV format")]
    InvalidExtendedTLV,
    
    #[error("TLV payload too large: {size} bytes")]
    PayloadTooLarge { size: usize },
}

/// TLV Header for standard TLVs (types 1-254)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct TLVHeader {
    pub tlv_type: u8,
    pub tlv_length: u8,
}

/// Extended TLV Header for type 255 (large payloads)
#[repr(C, packed)]  
#[derive(Debug, Clone, Copy)]
pub struct ExtendedTLVHeader {
    pub marker: u8,        // Always 255
    pub reserved: u8,      // Always 0
    pub tlv_type: u8,      // Actual TLV type
    pub tlv_length: u16,   // Length as u16 (up to 65KB)
}

/// A parsed TLV extension with payload
#[derive(Debug, Clone)]
pub struct TLVExtension {
    pub header: TLVHeader,
    pub payload: Vec<u8>,
}

/// An extended TLV extension with larger payload
#[derive(Debug, Clone)]
pub struct ExtendedTLVExtension {
    pub header: ExtendedTLVHeader,
    pub payload: Vec<u8>,
}

/// Result type for TLV parsing
pub type ParseResult<T> = std::result::Result<T, ParseError>;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_tlv_header_size() {
        assert_eq!(std::mem::size_of::<TLVHeader>(), 2);
        assert_eq!(std::mem::size_of::<ExtendedTLVHeader>(), 5);
    }
}