//! Address conversion traits for zero-copy TLV serialization
//!
//! Provides traits for converting between 20-byte Ethereum addresses
//! and 32-byte padded arrays required for alignment in zero-copy operations.

use zerocopy::{AsBytes, FromBytes, FromZeroes};

/// Trait for converting 20-byte Ethereum addresses to 32-byte padded arrays
pub trait AddressConversion {
    /// Convert to 32-byte padded representation
    fn to_padded(&self) -> [u8; 32];
}

/// Trait for extracting 20-byte addresses from padded arrays
pub trait AddressExtraction {
    /// Extract the 20-byte Ethereum address
    fn to_eth_address(&self) -> [u8; 20];

    /// Verify padding bytes are zeros (for safety)
    fn validate_padding(&self) -> bool;
}

// Implement for [u8; 20]
impl AddressConversion for [u8; 20] {
    #[inline(always)]
    fn to_padded(&self) -> [u8; 32] {
        let mut padded = [0u8; 32];
        padded[..20].copy_from_slice(self);
        padded
    }
}

// Implement for [u8; 32]
impl AddressExtraction for [u8; 32] {
    #[inline(always)]
    fn to_eth_address(&self) -> [u8; 20] {
        let mut addr = [0u8; 20];
        addr.copy_from_slice(&self[..20]);
        addr
    }

    #[inline(always)]
    fn validate_padding(&self) -> bool {
        self[20..].iter().all(|&b| b == 0)
    }
}

/// Type-safe wrapper for padded Ethereum addresses
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, AsBytes, FromBytes, FromZeroes)]
pub struct PaddedAddress([u8; 32]);

impl PaddedAddress {
    /// Create a zero address
    pub const fn zero() -> Self {
        Self([0u8; 32])
    }

    /// Create from a 20-byte Ethereum address
    #[inline(always)]
    pub fn from_eth(addr: [u8; 20]) -> Self {
        Self(addr.to_padded())
    }

    /// Extract the 20-byte Ethereum address
    #[inline(always)]
    pub fn as_eth(&self) -> [u8; 20] {
        self.0.to_eth_address()
    }

    /// Get the underlying 32-byte array
    #[inline(always)]
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Validate that padding bytes are zeros
    #[inline(always)]
    pub fn is_valid(&self) -> bool {
        self.0.validate_padding()
    }
}

impl From<[u8; 20]> for PaddedAddress {
    fn from(addr: [u8; 20]) -> Self {
        Self::from_eth(addr)
    }
}

impl From<PaddedAddress> for [u8; 32] {
    fn from(padded: PaddedAddress) -> Self {
        padded.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_address_conversion() {
        let eth_addr = [0x42u8; 20];
        let padded = eth_addr.to_padded();

        // First 20 bytes should match
        assert_eq!(&padded[..20], &eth_addr[..]);

        // Last 12 bytes should be zeros
        assert_eq!(&padded[20..], &[0u8; 12]);

        // Round-trip should work
        let extracted = padded.to_eth_address();
        assert_eq!(extracted, eth_addr);
    }

    #[test]
    fn test_padding_validation() {
        let mut padded = [0u8; 32];
        padded[..20].copy_from_slice(&[0x42u8; 20]);

        // Should be valid
        assert!(padded.validate_padding());

        // Add non-zero padding
        padded[25] = 1;

        // Should be invalid
        assert!(!padded.validate_padding());
    }

    #[test]
    fn test_padded_address_wrapper() {
        let eth_addr = [0xAAu8; 20];
        let padded = PaddedAddress::from_eth(eth_addr);

        // Should be valid
        assert!(padded.is_valid());

        // Extraction should work
        assert_eq!(padded.as_eth(), eth_addr);

        // Conversion should work
        let raw: [u8; 32] = padded.into();
        assert_eq!(&raw[..20], &eth_addr[..]);
        assert_eq!(&raw[20..], &[0u8; 12]);
    }
}
