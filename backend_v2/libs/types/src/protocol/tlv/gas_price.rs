//! Gas price TLV message for real-time gas cost updates
//!
//! Streams gas prices via WebSocket to avoid RPC rate limiting
//! and provide accurate, real-time transaction cost estimates.

use crate::{define_tlv, define_tlv_with_padding};
use zerocopy::AsBytes;

// Gas price update TLV using macro for consistent alignment and validation
define_tlv_with_padding! {
    /// Gas price update TLV structure - 32 bytes for efficient packing
    ///
    /// Provides real-time gas cost information for transaction cost estimation
    /// and optimal gas price selection during arbitrage execution.
    GasPriceTLV {
        size: 32,
        u64: {
            block_number: u64,    // Block number this price applies to
            timestamp_ns: u64     // Timestamp when observed (nanoseconds)
        }
        u32: {
            gas_price_gwei: u32,    // Current gas price in gwei (base + priority)
            base_fee_gwei: u32,     // Base fee per gas in gwei (from block header)
            priority_fee_gwei: u32  // Priority fee (tip) in gwei (from market observation)
        }
        u16: {
            venue: u16,     // Network ID (137 for Polygon)
            reserved: u16   // Reserved for future expansion
        }
        u8: {}
        special: {}
    }
}

impl GasPriceTLV {
    /// Create a new gas price TLV
    pub fn new(
        venue: u16,
        base_fee_gwei: u32,
        priority_fee_gwei: u32,
        block_number: u64,
        timestamp_ns: u64,
    ) -> Self {
        // Use macro-generated constructor with proper field order
        Self::new_raw(
            block_number,
            timestamp_ns,
            base_fee_gwei.saturating_add(priority_fee_gwei), // gas_price_gwei
            base_fee_gwei,
            priority_fee_gwei,
            venue,
            0, // reserved
        )
    }

    /// Get total gas price in wei
    pub fn gas_price_wei(&self) -> u128 {
        self.gas_price_gwei as u128 * 1_000_000_000
    }

    /// Get base fee in wei
    pub fn base_fee_wei(&self) -> u128 {
        self.base_fee_gwei as u128 * 1_000_000_000
    }

    /// Get priority fee in wei
    pub fn priority_fee_wei(&self) -> u128 {
        self.priority_fee_gwei as u128 * 1_000_000_000
    }

    /// Estimate transaction cost in USD for given gas units
    pub fn estimate_cost_usd(&self, gas_units: u64, matic_price_usd: f64) -> f64 {
        let cost_wei = self.gas_price_wei() * gas_units as u128;
        let cost_matic = cost_wei as f64 / 1e18;
        cost_matic * matic_price_usd
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gas_price_tlv_size() {
        assert_eq!(std::mem::size_of::<GasPriceTLV>(), 32);
        assert_eq!(std::mem::align_of::<GasPriceTLV>(), 8);
    }

    #[test]
    fn test_gas_price_calculations() {
        let tlv = GasPriceTLV::new(
            137, // Polygon
            30,  // 30 gwei base fee
            2,   // 2 gwei priority fee
            12345678,
            1234567890000000000,
        );

        // Use local variables to avoid unaligned reference issues with packed structs
        let gas_price_gwei = tlv.gas_price_gwei;
        let base_fee_wei = tlv.base_fee_wei();
        let priority_fee_wei = tlv.priority_fee_wei();

        assert_eq!(gas_price_gwei, 32);
        assert_eq!(tlv.gas_price_wei(), 32_000_000_000);
        assert_eq!(base_fee_wei, 30_000_000_000);
        assert_eq!(priority_fee_wei, 2_000_000_000);

        // Test cost estimation (300k gas at $2 MATIC)
        let cost_usd = tlv.estimate_cost_usd(300_000, 2.0);
        assert!((cost_usd - 0.0192).abs() < 0.0001); // ~$0.0192
    }
}
