//! Type-safe wrappers for protocol values
//!
//! These newtype wrappers provide compile-time guarantees about:
//! - Decimal precision
//! - Value semantics (CEX vs DEX amounts)
//! - Price representations
//! - Prevent accidental mixing of incompatible values

use serde::{Deserialize, Serialize};
use std::fmt;

/// Amount in native blockchain precision (wei for Ethereum/Polygon)
/// Always represents the smallest unit of the token
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct NativeAmount(pub u128);

impl NativeAmount {
    pub const ZERO: Self = Self(0);
    pub const MAX: Self = Self(u128::MAX);

    /// Create from raw native units (e.g., wei)
    pub fn from_raw(value: u128) -> Self {
        Self(value)
    }

    /// Get raw value
    pub fn raw(&self) -> u128 {
        self.0
    }

    /// Convert to human-readable format given decimals
    pub fn to_decimal(&self, decimals: u8) -> f64 {
        self.0 as f64 / 10_f64.powi(decimals as i32)
    }

    /// Create from human-readable amount and decimals
    pub fn from_decimal(amount: f64, decimals: u8) -> Self {
        Self((amount * 10_f64.powi(decimals as i32)) as u128)
    }

    /// Checked addition
    pub fn checked_add(&self, other: Self) -> Option<Self> {
        self.0.checked_add(other.0).map(Self)
    }

    /// Checked subtraction
    pub fn checked_sub(&self, other: Self) -> Option<Self> {
        self.0.checked_sub(other.0).map(Self)
    }

    /// Checked multiplication
    pub fn checked_mul(&self, factor: u128) -> Option<Self> {
        self.0.checked_mul(factor).map(Self)
    }

    /// Checked division
    pub fn checked_div(&self, divisor: u128) -> Option<Self> {
        self.0.checked_div(divisor).map(Self)
    }

    /// Serialize to bytes (16 bytes, little-endian)
    pub fn to_bytes(&self) -> [u8; 16] {
        self.0.to_le_bytes()
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(u128::from_le_bytes(bytes))
    }
}

impl fmt::Display for NativeAmount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Fixed-point amount with 8 decimal places (used for CEX data)
/// 1.0 = 100_000_000
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct FixedPoint8(pub i64);

impl FixedPoint8 {
    pub const ZERO: Self = Self(0);
    pub const ONE: Self = Self(100_000_000);
    pub const DECIMALS: u8 = 8;
    pub const SCALE: i64 = 100_000_000;

    /// Create from raw fixed-point value
    pub fn from_raw(value: i64) -> Self {
        Self(value)
    }

    /// Get raw value
    pub fn raw(&self) -> i64 {
        self.0
    }

    /// Convert to f64
    pub fn to_f64(&self) -> f64 {
        self.0 as f64 / Self::SCALE as f64
    }

    /// Create from f64
    pub fn from_f64(value: f64) -> Self {
        Self((value * Self::SCALE as f64) as i64)
    }

    /// Checked addition
    pub fn checked_add(&self, other: Self) -> Option<Self> {
        self.0.checked_add(other.0).map(Self)
    }

    /// Checked subtraction
    pub fn checked_sub(&self, other: Self) -> Option<Self> {
        self.0.checked_sub(other.0).map(Self)
    }

    /// Serialize to bytes (8 bytes, little-endian)
    pub fn to_bytes(&self) -> [u8; 8] {
        self.0.to_le_bytes()
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: [u8; 8]) -> Self {
        Self(i64::from_le_bytes(bytes))
    }
}

impl fmt::Display for FixedPoint8 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.8}", self.to_f64())
    }
}

/// Uniswap V3 sqrt price representation (X96 format)
/// sqrtPriceX96 = sqrt(price) * 2^96
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SqrtPriceX96(pub u128);

impl SqrtPriceX96 {
    pub const ZERO: Self = Self(0);
    pub const Q96: u128 = 1 << 96; // 2^96

    /// Create from raw sqrtPriceX96 value
    pub fn from_raw(value: u128) -> Self {
        Self(value)
    }

    /// Get raw value
    pub fn raw(&self) -> u128 {
        self.0
    }

    /// Convert to actual price (token1/token0)
    pub fn to_price(&self, token0_decimals: u8, token1_decimals: u8) -> f64 {
        let sqrt_price = self.0 as f64 / Self::Q96 as f64;
        let price = sqrt_price * sqrt_price;

        // Adjust for decimal differences
        let decimal_adjustment = 10_f64.powi((token1_decimals as i32) - (token0_decimals as i32));
        price * decimal_adjustment
    }

    /// Create from actual price
    pub fn from_price(price: f64, token0_decimals: u8, token1_decimals: u8) -> Self {
        // Adjust for decimal differences
        let decimal_adjustment = 10_f64.powi((token0_decimals as i32) - (token1_decimals as i32));
        let adjusted_price = price * decimal_adjustment;

        let sqrt_price = adjusted_price.sqrt();
        Self((sqrt_price * Self::Q96 as f64) as u128)
    }

    /// Serialize to bytes (16 bytes, little-endian)
    pub fn to_bytes(&self) -> [u8; 16] {
        self.0.to_le_bytes()
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(u128::from_le_bytes(bytes))
    }
}

impl fmt::Display for SqrtPriceX96 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "sqrtPriceX96:{}", self.0)
    }
}

/// Pool liquidity amount (L in Uniswap V3 math)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Liquidity(pub u128);

impl Liquidity {
    pub const ZERO: Self = Self(0);

    /// Create from raw liquidity value
    pub fn from_raw(value: u128) -> Self {
        Self(value)
    }

    /// Get raw value
    pub fn raw(&self) -> u128 {
        self.0
    }

    /// Checked addition
    pub fn checked_add(&self, other: Self) -> Option<Self> {
        self.0.checked_add(other.0).map(Self)
    }

    /// Checked subtraction
    pub fn checked_sub(&self, other: Self) -> Option<Self> {
        self.0.checked_sub(other.0).map(Self)
    }

    /// Serialize to bytes (16 bytes, little-endian)
    pub fn to_bytes(&self) -> [u8; 16] {
        self.0.to_le_bytes()
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(u128::from_le_bytes(bytes))
    }
}

impl fmt::Display for Liquidity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "L:{}", self.0)
    }
}

/// Token address (20 bytes for Ethereum/Polygon)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TokenAddress(pub [u8; 20]);

impl TokenAddress {
    pub const ZERO: Self = Self([0u8; 20]);

    /// Create from bytes
    pub fn from_bytes(bytes: [u8; 20]) -> Self {
        Self(bytes)
    }

    /// Get bytes
    pub fn as_bytes(&self) -> &[u8; 20] {
        &self.0
    }

    /// Convert to hex string
    pub fn to_hex(&self) -> String {
        format!("0x{}", hex::encode(self.0))
    }

    /// Parse from hex string
    pub fn from_hex(s: &str) -> Result<Self, String> {
        let s = s.strip_prefix("0x").unwrap_or(s);
        let bytes = hex::decode(s).map_err(|e| format!("Invalid hex: {}", e))?;
        if bytes.len() != 20 {
            return Err(format!(
                "Invalid address length: expected 20 bytes, got {}",
                bytes.len()
            ));
        }
        let mut addr = [0u8; 20];
        addr.copy_from_slice(&bytes);
        Ok(Self(addr))
    }
}

impl fmt::Display for TokenAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

/// Block number on the blockchain
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct BlockNumber(pub u64);

impl BlockNumber {
    /// Create from raw block number
    pub fn from_raw(value: u64) -> Self {
        Self(value)
    }

    /// Get raw value
    pub fn raw(&self) -> u64 {
        self.0
    }
}

impl fmt::Display for BlockNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_native_amount() {
        // Test with 1 ETH in wei
        let one_eth = NativeAmount::from_raw(1_000_000_000_000_000_000);
        assert_eq!(one_eth.to_decimal(18), 1.0);

        // Test with USDC (6 decimals)
        let hundred_usdc = NativeAmount::from_raw(100_000_000);
        assert_eq!(hundred_usdc.to_decimal(6), 100.0);

        // Test arithmetic
        let sum = one_eth.checked_add(one_eth).unwrap();
        assert_eq!(sum.raw(), 2_000_000_000_000_000_000);
    }

    #[test]
    fn test_fixed_point8() {
        let one = FixedPoint8::from_f64(1.0);
        assert_eq!(one.raw(), 100_000_000);
        assert_eq!(one.to_f64(), 1.0);

        let half = FixedPoint8::from_f64(0.5);
        let sum = one.checked_add(half).unwrap();
        assert_eq!(sum.to_f64(), 1.5);
    }

    #[test]
    fn test_sqrt_price_x96() {
        // Test price of 1.0 with same decimals
        let price_one = SqrtPriceX96::from_price(1.0, 18, 18);
        let recovered = price_one.to_price(18, 18);
        assert!((recovered - 1.0).abs() < 0.0001);

        // Test with different decimals (USDC/WETH)
        let price = SqrtPriceX96::from_price(2000.0, 18, 6);
        let recovered = price.to_price(18, 6);
        assert!((recovered - 2000.0).abs() < 0.1);
    }

    #[test]
    fn test_token_address() {
        let addr = TokenAddress::from_hex("0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb6").unwrap();
        assert_eq!(addr.to_hex(), "0x742d35cc6634c0532925a3b844bc9e7595f0beb6");
    }
}
