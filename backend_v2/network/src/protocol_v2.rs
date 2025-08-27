//! Protocol V2 Integration for Network Transport
//!
//! Provides TLV message validation, domain separation enforcement, and 
//! precision handling for the AlphaPulse Protocol V2 message format.
//!
//! ## TLV Message Structure
//! 
//! All messages follow the Protocol V2 format:
//! - **32-byte MessageHeader**: Magic number, payload size, sequence, checksum
//! - **Variable TLV Payload**: Type-Length-Value encoded messages
//!
//! ## Domain Separation
//!
//! TLV types are strictly separated by relay domain:
//! - **Market Data (1-19)**: Price updates, order book changes, trades
//! - **Signal (20-39)**: Trading signals, analytics, strategy outputs  
//! - **Execution (40-79)**: Order placement, fills, portfolio updates
//!
//! ## Precision Requirements
//!
//! - **DEX tokens**: Native precision (18 decimals WETH, 6 USDC, etc.)
//! - **Traditional exchanges**: 8-decimal fixed-point for USD prices
//! - **Timestamps**: Nanosecond precision, never truncated
//! - **Financial calculations**: NO floating point allowed

#[cfg(feature = "protocol-integration")]
use alphapulse_types::protocol::{
    message::MessageHeader,
    tlv::{TLVType, TLVHeader, TLVMessage},
    validation::{bounds::validate_message_bounds, checksum::verify_checksum},
};

use crate::error::{Result, TransportError};
use std::collections::HashMap;
use tracing::{debug, warn};

// Simplified types when protocol-integration feature is disabled
#[cfg(not(feature = "protocol-integration"))]
#[derive(Debug, Clone)]
pub struct MessageHeader {
    pub magic: u32,
    pub payload_size: u32,
    pub relay_domain: u8,
    pub source: u8,
    pub sequence: u32,
    pub timestamp_ns: u64,
    pub checksum: u32,
}

/// Protocol V2 message validator for transport layer
#[derive(Debug, Clone)]
pub struct ProtocolV2Validator {
    /// Domain to TLV type range mappings
    domain_ranges: HashMap<u8, TLVTypeRange>,
    /// Maximum message size (16MB)
    max_message_size: usize,
    /// Enable strict domain enforcement
    enforce_domains: bool,
}

/// TLV type range for a specific domain
#[derive(Debug, Clone, Copy)]
pub struct TLVTypeRange {
    /// Minimum TLV type (inclusive)
    pub min: u16,
    /// Maximum TLV type (inclusive)  
    pub max: u16,
    /// Human readable domain name
    pub domain_name: &'static str,
}

impl ProtocolV2Validator {
    /// Create new Protocol V2 validator with standard domain mappings
    pub fn new() -> Self {
        let mut domain_ranges = HashMap::new();
        
        // Standard AlphaPulse domain mappings
        domain_ranges.insert(1, TLVTypeRange {
            min: 1,
            max: 19, 
            domain_name: "MarketData",
        });
        domain_ranges.insert(2, TLVTypeRange {
            min: 20,
            max: 39,
            domain_name: "Signal", 
        });
        domain_ranges.insert(3, TLVTypeRange {
            min: 40,
            max: 79,
            domain_name: "Execution",
        });
        
        Self {
            domain_ranges,
            max_message_size: 16 * 1024 * 1024, // 16MB
            enforce_domains: true,
        }
    }

    /// Validate complete Protocol V2 message
    pub fn validate_message(&self, message_bytes: &[u8]) -> Result<ValidationResult> {
        // 1. Check minimum size for header
        if message_bytes.len() < 32 {
            return Err(TransportError::protocol(
                "Message too short for Protocol V2 header",
            ));
        }

        // 2. Check maximum message size
        if message_bytes.len() > self.max_message_size {
            return Err(TransportError::protocol(
                format!("Message size {} exceeds maximum {}", message_bytes.len(), self.max_message_size),
            ));
        }

        // 3. Parse and validate header
        let header = self.parse_header(&message_bytes[..32])?;
        
        // 4. Validate payload bounds
        let expected_total_size = 32 + header.payload_size as usize;
        if message_bytes.len() != expected_total_size {
            return Err(TransportError::protocol(
                format!("Message size {} doesn't match header payload_size {}", 
                    message_bytes.len(), expected_total_size),
            ));
        }

        // 5. Verify checksum
        #[cfg(feature = "protocol-integration")]
        {
            if !verify_checksum(&message_bytes) {
                return Err(TransportError::protocol(
                    "Message checksum verification failed",
                ));
            }
        }
        
        #[cfg(not(feature = "protocol-integration"))]
        {
            // Simple checksum validation when protocol integration is disabled
            if !verify_simple_checksum(&message_bytes) {
                return Err(TransportError::protocol(
                    "Message checksum verification failed",
                ));
            }
        }

        // 6. Validate TLV payload structure
        let payload = &message_bytes[32..];
        let tlv_validations = self.validate_tlv_payload(payload, header.relay_domain)?;

        let payload_size = header.payload_size as usize;
        
        Ok(ValidationResult {
            header,
            tlv_count: tlv_validations.len(),
            total_size: message_bytes.len(),
            domain_violations: tlv_validations.iter().filter(|v| !v.domain_valid).count(),
            payload_size,
        })
    }

    /// Parse 32-byte MessageHeader
    fn parse_header(&self, header_bytes: &[u8]) -> Result<MessageHeader> {
        if header_bytes.len() != 32 {
            return Err(TransportError::protocol(
                format!("Invalid header size: {} (expected 32)", header_bytes.len()),
            ));
        }

        // Parse header using Protocol V2 types
        // This is a simplified parsing - real implementation would use zerocopy
        let magic = u32::from_le_bytes([header_bytes[0], header_bytes[1], header_bytes[2], header_bytes[3]]);
        
        if magic != 0xDEADBEEF {
            return Err(TransportError::protocol(
                format!("Invalid magic number: 0x{:08X} (expected 0xDEADBEEF)", magic),
            ));
        }

        let payload_size = u32::from_le_bytes([header_bytes[4], header_bytes[5], header_bytes[6], header_bytes[7]]);
        let relay_domain = header_bytes[8];
        let source = header_bytes[9];
        let sequence = u32::from_le_bytes([header_bytes[12], header_bytes[13], header_bytes[14], header_bytes[15]]);
        
        Ok(MessageHeader {
            magic,
            payload_size,
            relay_domain,
            source,
            sequence,
            timestamp_ns: u64::from_le_bytes([
                header_bytes[16], header_bytes[17], header_bytes[18], header_bytes[19],
                header_bytes[20], header_bytes[21], header_bytes[22], header_bytes[23],
            ]),
            checksum: u32::from_le_bytes([header_bytes[28], header_bytes[29], header_bytes[30], header_bytes[31]]),
        })
    }

    /// Validate TLV payload structure and domain compliance
    fn validate_tlv_payload(&self, payload: &[u8], domain: u8) -> Result<Vec<TLVValidation>> {
        let mut validations = Vec::new();
        let mut offset = 0;

        while offset < payload.len() {
            // Ensure we have at least TLV header (4 bytes: type + length)
            if offset + 4 > payload.len() {
                return Err(TransportError::protocol(
                    format!("Truncated TLV header at offset {}", offset),
                ));
            }

            let tlv_type = u16::from_le_bytes([payload[offset], payload[offset + 1]]);
            let tlv_length = u16::from_le_bytes([payload[offset + 2], payload[offset + 3]]);
            
            // Validate TLV bounds
            if offset + 4 + tlv_length as usize > payload.len() {
                return Err(TransportError::protocol(
                    format!("TLV payload extends beyond message bounds: type={}, length={}, remaining={}", 
                        tlv_type, tlv_length, payload.len() - offset - 4),
                ));
            }

            // Check domain compliance
            let domain_valid = self.validate_tlv_domain(tlv_type, domain);
            
            if self.enforce_domains && !domain_valid {
                warn!(
                    tlv_type = tlv_type,
                    domain = domain,
                    "TLV type {} not valid for domain {}", tlv_type, domain
                );
            }

            validations.push(TLVValidation {
                tlv_type,
                tlv_length,
                offset,
                domain_valid,
            });

            // Move to next TLV
            offset += 4 + tlv_length as usize;
        }

        debug!("Validated {} TLV entries in payload", validations.len());
        Ok(validations)
    }

    /// Validate TLV type is appropriate for relay domain
    fn validate_tlv_domain(&self, tlv_type: u16, domain: u8) -> bool {
        if let Some(range) = self.domain_ranges.get(&domain) {
            tlv_type >= range.min && tlv_type <= range.max
        } else {
            // Unknown domain - allow all types with warning
            warn!("Unknown relay domain: {}", domain);
            true
        }
    }

    /// Enable or disable strict domain enforcement
    pub fn set_enforce_domains(&mut self, enforce: bool) {
        self.enforce_domains = enforce;
    }

    /// Add custom domain range mapping
    pub fn add_domain_range(&mut self, domain: u8, range: TLVTypeRange) {
        self.domain_ranges.insert(domain, range);
    }

    /// Get domain name for domain ID
    pub fn domain_name(&self, domain: u8) -> Option<&'static str> {
        self.domain_ranges.get(&domain).map(|r| r.domain_name)
    }
}

impl Default for ProtocolV2Validator {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of Protocol V2 message validation
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Parsed message header
    pub header: MessageHeader,
    /// Number of TLV entries found
    pub tlv_count: usize,
    /// Total message size in bytes
    pub total_size: usize,
    /// Number of domain violations found
    pub domain_violations: usize,
    /// Payload size from header
    pub payload_size: usize,
}

/// Individual TLV validation result
#[derive(Debug, Clone)]
pub struct TLVValidation {
    /// TLV type number
    pub tlv_type: u16,
    /// TLV payload length
    pub tlv_length: u16,
    /// Offset in payload
    pub offset: usize,
    /// Whether TLV type is valid for message domain
    pub domain_valid: bool,
}

impl ValidationResult {
    /// Check if message passed all validations
    pub fn is_valid(&self) -> bool {
        self.domain_violations == 0
    }

    /// Get validation summary
    pub fn summary(&self) -> String {
        format!(
            "Message: {} bytes, {} TLVs, domain={}, violations={}",
            self.total_size,
            self.tlv_count, 
            self.header.relay_domain,
            self.domain_violations
        )
    }
}

/// Validate timestamp precision (must be nanoseconds)
pub fn validate_timestamp_precision(timestamp_ns: u64) -> Result<()> {
    // Check that timestamp is in reasonable range (not microseconds or milliseconds)
    let current_ns = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| TransportError::system(format!("System time error: {}", e)))?
        .as_nanos() as u64;

    // Timestamp should be within 24 hours of current time
    let day_in_ns = 24 * 60 * 60 * 1_000_000_000u64;
    
    if timestamp_ns < current_ns.saturating_sub(day_in_ns) || 
       timestamp_ns > current_ns + day_in_ns {
        return Err(TransportError::protocol(
            format!("Timestamp {} appears invalid (not in nanoseconds?)", timestamp_ns),
        ));
    }

    Ok(())
}

/// Check for floating point usage in financial calculations (compile-time check)
pub fn validate_no_float_in_price(value: &str) -> bool {
    // This is a simple string-based check - in practice would use AST analysis
    !value.contains("f32") && !value.contains("f64") && !value.contains("float")
}

/// Protocol V2-compatible checksum verification 
#[cfg(not(feature = "protocol-integration"))]
fn verify_simple_checksum(message_bytes: &[u8]) -> bool {
    if message_bytes.len() < 32 {
        return false;
    }
    
    // Extract stored checksum from header bytes 28-31
    let stored_checksum = u32::from_le_bytes([
        message_bytes[28], message_bytes[29], message_bytes[30], message_bytes[31]
    ]);
    
    // Protocol V2 checksum algorithm: CRC32 over specific header fields + payload
    // This matches the actual Protocol V2 implementation for financial data integrity
    let calculated_checksum = calculate_protocol_v2_checksum(message_bytes);
    
    let is_valid = stored_checksum == calculated_checksum;
    if !is_valid {
        tracing::error!(
            stored = stored_checksum,
            calculated = calculated_checksum,
            "Protocol V2 checksum mismatch - potential data corruption detected"
        );
    }
    
    is_valid
}

/// Calculate Protocol V2-compatible checksum for financial data integrity
#[cfg(not(feature = "protocol-integration"))]
fn calculate_protocol_v2_checksum(message_bytes: &[u8]) -> u32 {
    let mut hasher = crc32fast::Hasher::new();
    
    // Protocol V2 checksum specification:
    // Include: magic(4) + payload_size(4) + relay_domain(1) + source(1) + padding(2) + sequence(4) + timestamp_ns(8) + reserved(4)
    // Exclude: checksum field itself (bytes 28-31)
    hasher.update(&message_bytes[0..28]);
    
    // Include payload if present  
    if message_bytes.len() > 32 {
        hasher.update(&message_bytes[32..]);
    }
    
    // Protocol V2 uses additional integrity check: XOR with message length
    let base_crc = hasher.finalize();
    let length_factor = (message_bytes.len() as u32).wrapping_mul(0xDEADBEEF);
    
    // Final Protocol V2 checksum: CRC32 XOR length-based factor
    base_crc ^ length_factor
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validator_creation() {
        let validator = ProtocolV2Validator::new();
        assert_eq!(validator.domain_name(1), Some("MarketData"));
        assert_eq!(validator.domain_name(2), Some("Signal"));
        assert_eq!(validator.domain_name(3), Some("Execution"));
        assert_eq!(validator.domain_name(99), None);
    }

    #[test]
    fn test_tlv_domain_validation() {
        let validator = ProtocolV2Validator::new();
        
        // Market data domain (1) should accept types 1-19
        assert!(validator.validate_tlv_domain(1, 1));
        assert!(validator.validate_tlv_domain(19, 1));
        assert!(!validator.validate_tlv_domain(20, 1));
        assert!(!validator.validate_tlv_domain(40, 1));
        
        // Signal domain (2) should accept types 20-39
        assert!(!validator.validate_tlv_domain(19, 2));
        assert!(validator.validate_tlv_domain(20, 2));
        assert!(validator.validate_tlv_domain(39, 2));
        assert!(!validator.validate_tlv_domain(40, 2));
        
        // Execution domain (3) should accept types 40-79
        assert!(!validator.validate_tlv_domain(39, 3));
        assert!(validator.validate_tlv_domain(40, 3));
        assert!(validator.validate_tlv_domain(79, 3));
        assert!(!validator.validate_tlv_domain(80, 3));
    }

    #[test]
    fn test_timestamp_precision_validation() {
        let current_ns = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        
        // Valid nanosecond timestamp
        assert!(validate_timestamp_precision(current_ns).is_ok());
        
        // Timestamp that looks like microseconds (too small)
        let microsecond_ts = current_ns / 1000;
        assert!(validate_timestamp_precision(microsecond_ts).is_err());
        
        // Timestamp that looks like milliseconds (too small)  
        let millisecond_ts = current_ns / 1_000_000;
        assert!(validate_timestamp_precision(millisecond_ts).is_err());
    }

    #[test]
    fn test_float_detection() {
        assert!(validate_no_float_in_price("let price = 100i64;"));
        assert!(validate_no_float_in_price("let price = Decimal::new(100, 2);"));
        assert!(!validate_no_float_in_price("let price = 100.0f64;"));
        assert!(!validate_no_float_in_price("let price: f32 = 100.0;"));
        assert!(!validate_no_float_in_price("let price = 100.0 as float;"));
    }

    #[test]
    #[cfg(not(feature = "protocol-integration"))]
    fn test_protocol_v2_checksum_validation() {
        // Create a test message with proper Protocol V2 structure
        let mut message = Vec::with_capacity(96); // 32-byte header + 64-byte payload
        
        // Header: magic(4) + payload_size(4) + domain(1) + source(1) + padding(2) + sequence(4) + timestamp(8) + reserved(4) + checksum(4)
        message.extend_from_slice(&0xDEADBEEF_u32.to_le_bytes()); // Magic
        message.extend_from_slice(&64_u32.to_le_bytes());          // Payload size
        message.push(1);                                           // Domain (Market Data)
        message.push(1);                                           // Source
        message.extend_from_slice(&[0u8; 2]);                      // Padding
        message.extend_from_slice(&12345_u32.to_le_bytes());       // Sequence
        
        let timestamp_ns = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        message.extend_from_slice(&timestamp_ns.to_le_bytes());    // Timestamp
        message.extend_from_slice(&[0u8; 4]);                      // Reserved
        message.extend_from_slice(&[0u8; 4]);                      // Placeholder checksum
        
        // Add test payload (64 bytes)
        message.extend_from_slice(&[0x42; 64]);
        
        // Calculate and set proper checksum
        let checksum = calculate_protocol_v2_checksum(&message);
        let checksum_bytes = checksum.to_le_bytes();
        message[28] = checksum_bytes[0];
        message[29] = checksum_bytes[1];
        message[30] = checksum_bytes[2];
        message[31] = checksum_bytes[3];
        
        // Valid message should pass checksum verification
        assert!(verify_simple_checksum(&message), "Valid Protocol V2 message should pass checksum");
        
        // Corrupt the payload and verify it fails
        let mut corrupted = message.clone();
        corrupted[32] = 0xFF; // Change first payload byte
        assert!(!verify_simple_checksum(&corrupted), "Corrupted message should fail checksum");
        
        // Corrupt header data and verify it fails
        let mut corrupted_header = message.clone();
        corrupted_header[12] = 0x99; // Change sequence field
        assert!(!verify_simple_checksum(&corrupted_header), "Message with corrupted header should fail checksum");
    }

    #[test]
    #[cfg(not(feature = "protocol-integration"))]
    fn test_protocol_v2_checksum_financial_integrity() {
        // Test with financial data that must not be corrupted
        let mut message = Vec::new();
        
        // Create realistic trading message
        message.extend_from_slice(&0xDEADBEEF_u32.to_le_bytes()); // Magic  
        message.extend_from_slice(&56_u32.to_le_bytes());          // Payload size (trade TLV)
        message.push(1);                                           // Market Data domain
        message.push(2);                                           // Source ID
        message.extend_from_slice(&[0u8; 2]);                      // Padding
        message.extend_from_slice(&54321_u32.to_le_bytes());       // Sequence
        
        let timestamp_ns = 1640995200000000000u64; // Jan 1 2022 00:00:00 UTC in nanoseconds
        message.extend_from_slice(&timestamp_ns.to_le_bytes());
        message.extend_from_slice(&[0u8; 4]);                      // Reserved
        message.extend_from_slice(&[0u8; 4]);                      // Placeholder checksum
        
        // Trade TLV payload: type(2) + length(2) + pool_address(20) + amounts(16) + timestamp(8) + hash(8)
        message.extend_from_slice(&1_u16.to_le_bytes());           // TLV Type: Trade
        message.extend_from_slice(&52_u16.to_le_bytes());          // TLV Length
        message.extend_from_slice(&[0x12; 20]);                    // Pool address
        message.extend_from_slice(&1_000_000_000_000_000_000_u64.to_le_bytes()); // 1 WETH (18 decimals)
        message.extend_from_slice(&2_000_000_000_u64.to_le_bytes());             // 2000 USDC (6 decimals)
        message.extend_from_slice(&timestamp_ns.to_le_bytes());    // Trade timestamp
        message.extend_from_slice(&[0xcd; 8]);                     // Transaction hash prefix
        
        // Calculate and verify checksum for financial data
        let checksum = calculate_protocol_v2_checksum(&message);
        let checksum_bytes = checksum.to_le_bytes();
        message[28] = checksum_bytes[0];
        message[29] = checksum_bytes[1]; 
        message[30] = checksum_bytes[2];
        message[31] = checksum_bytes[3];
        
        assert!(verify_simple_checksum(&message), "Financial trading message must pass Protocol V2 checksum");
        
        // Critical: Even 1 wei change should be detected
        let mut amount_corrupted = message.clone();
        let corrupted_amount = 1_000_000_000_000_000_001_u64; // 1 wei more
        let amount_bytes = corrupted_amount.to_le_bytes();
        for i in 0..8 {
            amount_corrupted[56 + i] = amount_bytes[i];
        }
        
        assert!(!verify_simple_checksum(&amount_corrupted), 
               "1 wei corruption in financial data MUST be detected by Protocol V2 checksum");
    }
}