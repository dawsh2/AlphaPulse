//! Address conversion traits for zero-copy TLV serialization
//!
//! Provides traits for converting between 20-byte Ethereum addresses
//! and 32-byte padded arrays required for alignment in zero-copy operations.
//!
//! # Standardized Types
//!
//! - `EthAddress`: Type alias for 20-byte Ethereum addresses
//! - `AddressPadding`: Type alias for 12-byte padding arrays
//! - `PaddedAddress`: Type-safe wrapper for padded addresses
//!
//! # Usage Examples
//!
//! ```rust
//! use torq_types::tlv::address::{EthAddress, AddressPadding, PaddedAddress};
//!
//! // TLV structure with explicit padding
//! #[repr(C)]
//! pub struct MyTLV {
//!     pub pool_address: EthAddress,
//!     pub pool_address_padding: AddressPadding,
//!     // other fields...
//! }
//!
//! // Safe construction
//! let addr: EthAddress = [0x42; 20];
//! let tlv = MyTLV {
//!     pool_address: addr,
//!     pool_address_padding: [0u8; 12], // Always zero
//!     // ...
//! };
//! ```

use zerocopy::{AsBytes, FromBytes, FromZeroes};

/// Standardized 20-byte Ethereum address type
pub type EthAddress = [u8; 20];

/// Standardized 12-byte padding array for address alignment
pub type AddressPadding = [u8; 12];

/// Zero padding constant for safe construction
pub const ZERO_PADDING: AddressPadding = [0u8; 12];

/// Trait for converting 20-byte Ethereum addresses to 32-byte padded arrays
pub trait AddressConversion {
    /// Convert to 32-byte padded representation
    fn to_padded(&self) -> [u8; 32];
}

/// Trait for extracting 20-byte addresses from padded arrays
pub trait AddressExtraction {
    /// Extract the 20-byte Ethereum address
    fn to_eth_address(&self) -> EthAddress;

    /// Verify padding bytes are zeros (for safety)
    fn validate_padding(&self) -> bool;
}

// Implement for EthAddress
impl AddressConversion for EthAddress {
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
    fn to_eth_address(&self) -> EthAddress {
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
    pub fn from_eth(addr: EthAddress) -> Self {
        Self(addr.to_padded())
    }

    /// Create from explicit address and padding (preferred for TLV construction)
    #[inline(always)]
    pub fn from_parts(addr: EthAddress, padding: AddressPadding) -> Self {
        let mut result = [0u8; 32];
        result[..20].copy_from_slice(&addr);
        result[20..].copy_from_slice(&padding);
        Self(result)
    }

    /// Extract address and padding as separate components
    #[inline(always)]
    pub fn to_parts(&self) -> (EthAddress, AddressPadding) {
        let mut addr = [0u8; 20];
        let mut padding = [0u8; 12];
        addr.copy_from_slice(&self.0[..20]);
        padding.copy_from_slice(&self.0[20..]);
        (addr, padding)
    }

    /// Extract the 20-byte Ethereum address
    #[inline(always)]
    pub fn as_eth(&self) -> EthAddress {
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

impl From<EthAddress> for PaddedAddress {
    fn from(addr: EthAddress) -> Self {
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
