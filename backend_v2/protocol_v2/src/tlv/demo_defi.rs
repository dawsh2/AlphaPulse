//! Demo DeFi Arbitrage TLV Structures
//!
//! Specialized TLV for dashboard demo of arbitrage opportunities.
//! Uses vendor TLV type 200 for experimental/demo purposes.

use super::address::{EthAddress, AddressPadding, ZERO_PADDING};
use crate::VenueId;
#[allow(unused_imports)] // Used in manual trait implementations
use zerocopy::{AsBytes, FromBytes, FromZeroes};

/// Demo DeFi Arbitrage TLV structure - specialized for dashboard display
///
/// Contains exactly the fields needed for arbitrage opportunity display:
/// - Strategy identification (strategy_id, signal_id, confidence)
/// - Financial metrics (expected_profit, required_capital, estimated_gas_cost)
/// - Pool information (pool_a, pool_b with addresses)
/// - Trade execution data (token_in, token_out, optimal_amount)
/// - Risk metrics (slippage_tolerance, max_gas_price)
/// - Timing information (valid_until, timestamp_ns)
///
/// Uses Q64.64 fixed-point encoding for all financial values to maintain precision.
/// Fixed size with proper alignment for zero-copy serialization.
#[repr(C, packed)] // Use packed to avoid alignment padding issues with manual serialization
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DemoDeFiArbitrageTLV {
    // Strategy Identity (12 bytes)
    pub strategy_id: u16, // Flash arbitrage strategy = 21
    pub signal_id: u64,   // Unique signal identifier
    pub confidence: u8,   // Confidence level 0-100
    pub chain_id: u8,     // Chain ID (1=Ethereum, 137=Polygon)

    // Economics in Q64.64 format (48 bytes)
    pub expected_profit_q: i128,    // Expected profit in Q64.64 USD
    pub required_capital_q: u128,   // Required capital in Q64.64 USD
    pub estimated_gas_cost_q: u128, // Estimated gas cost in Q64.64 ETH/MATIC

    // Pool Information (72 bytes total)
    pub venue_a: u16,                // First pool venue as u16
    pub venue_b: u16,                // Second pool venue as u16
    pub pool_a: EthAddress,          // First pool address (20 bytes)
    pub pool_a_padding: AddressPadding, // Explicit padding (12 bytes)
    pub pool_b: EthAddress,          // Second pool address (20 bytes)
    pub pool_b_padding: AddressPadding, // Explicit padding (12 bytes)

    // Trade Execution (32 bytes)
    pub token_in: u64,          // Input token address (truncated to 64-bit)
    pub token_out: u64,         // Output token address (truncated to 64-bit)
    pub optimal_amount_q: u128, // Optimal trade amount in Q64.64

    // Risk Parameters (12 bytes)
    pub slippage_tolerance: u16, // Slippage tolerance in basis points (e.g., 50 = 0.5%)
    pub max_gas_price_gwei: u32, // Maximum gas price in Gwei
    pub valid_until: u32,        // Unix timestamp when opportunity expires
    pub priority: u8,            // Priority level 0-255 (higher = more urgent)
    pub reserved: u8,            // Reserved for alignment

    // Timing (8 bytes)
    pub timestamp_ns: u64, // Nanoseconds since epoch when detected

                           // Total: 12 + 48 + 72 + 32 + 12 + 8 = 184 bytes (packed, no padding)
}

// Manual implementation of zero-copy traits for packed struct
unsafe impl zerocopy::AsBytes for DemoDeFiArbitrageTLV {
    fn only_derive_is_allowed_to_implement_this_trait() {}
}

unsafe impl zerocopy::FromBytes for DemoDeFiArbitrageTLV {
    fn only_derive_is_allowed_to_implement_this_trait() {}
}

unsafe impl zerocopy::FromZeroes for DemoDeFiArbitrageTLV {
    fn only_derive_is_allowed_to_implement_this_trait() {}
}

/// Configuration for creating DemoDeFiArbitrageTLV
#[derive(Debug, Clone)]
pub struct ArbitrageConfig {
    pub strategy_id: u16,
    pub signal_id: u64,
    pub confidence: u8,
    pub chain_id: u8,
    pub expected_profit_q: i128,
    pub required_capital_q: u128,
    pub estimated_gas_cost_q: u128,
    pub venue_a: VenueId,
    pub pool_a: EthAddress,
    pub venue_b: VenueId,
    pub pool_b: EthAddress,
    pub token_in: u64,
    pub token_out: u64,
    pub optimal_amount_q: u128,
    pub slippage_tolerance: u16,
    pub max_gas_price_gwei: u32,
    pub valid_until: u32,
    pub priority: u8,
    pub timestamp_ns: u64,
}

impl DemoDeFiArbitrageTLV {
    /// Create new arbitrage opportunity TLV from 20-byte addresses
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_addresses(
        strategy_id: u16,
        signal_id: u64,
        confidence: u8,
        chain_id: u8,
        expected_profit_q: i128,
        required_capital_q: u128,
        estimated_gas_cost_q: u128,
        venue_a: VenueId,
        pool_a: EthAddress,
        venue_b: VenueId,
        pool_b: EthAddress,
        token_in: u64,
        token_out: u64,
        optimal_amount_q: u128,
        slippage_tolerance: u16,
        max_gas_price_gwei: u32,
        valid_until: u32,
        priority: u8,
        timestamp_ns: u64,
    ) -> Self {
        Self {
            strategy_id,
            signal_id,
            confidence,
            chain_id,
            expected_profit_q,
            required_capital_q,
            estimated_gas_cost_q,
            venue_a: venue_a as u16,
            venue_b: venue_b as u16,
            pool_a,
            pool_a_padding: ZERO_PADDING,
            pool_b,
            pool_b_padding: ZERO_PADDING,
            token_in,
            token_out,
            optimal_amount_q,
            slippage_tolerance,
            max_gas_price_gwei,
            valid_until,
            priority,
            reserved: 0,
            timestamp_ns,
        }
    }

    /// Create new arbitrage opportunity TLV from config
    pub fn new(config: ArbitrageConfig) -> Self {
        Self {
            strategy_id: config.strategy_id,
            signal_id: config.signal_id,
            confidence: config.confidence,
            chain_id: config.chain_id,
            expected_profit_q: config.expected_profit_q,
            required_capital_q: config.required_capital_q,
            estimated_gas_cost_q: config.estimated_gas_cost_q,
            venue_a: config.venue_a as u16,
            venue_b: config.venue_b as u16,
            pool_a: config.pool_a,
            pool_a_padding: ZERO_PADDING,
            pool_b: config.pool_b,
            pool_b_padding: ZERO_PADDING,
            token_in: config.token_in,
            token_out: config.token_out,
            optimal_amount_q: config.optimal_amount_q,
            slippage_tolerance: config.slippage_tolerance,
            max_gas_price_gwei: config.max_gas_price_gwei,
            valid_until: config.valid_until,
            priority: config.priority,
            reserved: 0,
            timestamp_ns: config.timestamp_ns,
        }
    }

    /// Convert Q64.64 to human-readable decimal string
    /// Q64.64 means 64 bits integer part, 64 bits fractional part
    /// So we need to divide by 2^64 to get the actual decimal value
    pub fn q64_to_decimal_string(q64_value: u128, _decimals: u8) -> String {
        // For Q64.64 format: divide by 2^64 to get decimal value
        const Q64_DIVISOR: f64 = (1u128 << 64) as f64;
        let decimal_value = q64_value as f64 / Q64_DIVISOR;

        // Format with appropriate precision for financial values
        if decimal_value < 0.01 {
            format!("{:.6}", decimal_value) // Show more precision for small values
        } else if decimal_value < 1.0 {
            format!("{:.4}", decimal_value)
        } else {
            format!("{:.2}", decimal_value) // Standard 2 decimal places for USD amounts
        }
    }

    /// Convert signed Q64.64 to human-readable decimal string
    pub fn signed_q64_to_decimal_string(q64_value: i128, decimals: u8) -> String {
        let is_negative = q64_value < 0;
        let abs_value = q64_value.unsigned_abs();
        let decimal_str = Self::q64_to_decimal_string(abs_value, decimals);
        if is_negative {
            format!("-{}", decimal_str)
        } else {
            decimal_str
        }
    }

    /// Get expected profit as USD string (assuming 8 decimal places)
    pub fn expected_profit_usd(&self) -> String {
        Self::signed_q64_to_decimal_string(self.expected_profit_q, 8)
    }

    /// Get required capital as USD string (assuming 8 decimal places)
    pub fn required_capital_usd(&self) -> String {
        Self::q64_to_decimal_string(self.required_capital_q, 8)
    }

    /// Get estimated gas cost as ETH/MATIC string (assuming 18 decimal places)
    pub fn estimated_gas_cost_native(&self) -> String {
        Self::q64_to_decimal_string(self.estimated_gas_cost_q, 18)
    }

    /// Get optimal amount as token string (assuming token's native decimals)
    pub fn optimal_amount_token(&self, token_decimals: u8) -> String {
        Self::q64_to_decimal_string(self.optimal_amount_q, token_decimals)
    }

    /// Get slippage tolerance as percentage string
    pub fn slippage_percentage(&self) -> String {
        format!("{:.2}%", self.slippage_tolerance as f64 / 100.0)
    }

    /// Check if the opportunity is still valid
    pub fn is_valid(&self, current_timestamp: u32) -> bool {
        current_timestamp <= self.valid_until
    }

    // Manual serialization methods removed - use zerocopy AsBytes/FromBytes traits consistently

    // Legacy TLV message methods removed - use Protocol V2 TLVMessageBuilder instead
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::VenueId;
    use crate::tlv::address::AddressConversion;

    fn create_test_arbitrage_tlv() -> DemoDeFiArbitrageTLV {
        // Token IDs (simplified for demo - normally would be proper address mappings)
        let usdc_token_id = 0xa0b86991c431aa73u64; // USDC token (truncated address)
        let weth_token_id = 0xc02aaa39b223fe8du64; // WETH token (truncated address)

        // Create mock pool addresses for demo purposes
        let pool_a = [
            0x45, 0xdd, 0xa9, 0xcb, 0x7c, 0x25, 0x13, 0x1d, 0xf2, 0x68, 0x51, 0x51, 0x31, 0xf6,
            0x47, 0xd7, 0x26, 0xf5, 0x06, 0x08,
        ]; // Mock UniswapV2 pool
        let pool_b = [
            0x88, 0xe6, 0xa0, 0xc2, 0xdd, 0xd2, 0x6f, 0xee, 0xb6, 0x4f, 0x3e, 0x0c, 0x3c, 0x7e,
            0xb1, 0x0e, 0x1f, 0xa0, 0x1d, 0x9b,
        ]; // Mock UniswapV3 pool

        DemoDeFiArbitrageTLV::new(ArbitrageConfig {
            strategy_id: 21,                               // Flash arbitrage strategy
            signal_id: 0x1234567890abcdef,                 // Unique signal ID
            confidence: 85,                                // 85% confidence
            chain_id: 137,                                 // Polygon chain
            expected_profit_q: ((250.0 * (1u128 << 64) as f64) as i128),  // $250.00 profit in Q64.64
            required_capital_q: ((5000.0 * (1u128 << 64) as f64) as u128), // $5000.00 capital in Q64.64
            estimated_gas_cost_q: ((0.0025 * (1u128 << 64) as f64) as u128), // 0.0025 MATIC gas in Q64.64
            venue_a: VenueId::UniswapV2,                   // Pool A venue
            pool_a: pool_a.to_padded(),                    // Pool A address (32-byte padded)
            venue_b: VenueId::UniswapV3,                   // Pool B venue
            pool_b: pool_b.to_padded(),                    // Pool B address (32-byte padded)
            token_in: usdc_token_id,                       // USDC token (truncated address)
            token_out: weth_token_id,                      // WETH token (truncated address)
            optimal_amount_q: ((1000.0 * (1u128 << 64) as f64) as u128), // 1000.00 USDC in Q64.64
            slippage_tolerance: 50,             // 0.5% slippage tolerance
            max_gas_price_gwei: 100,            // 100 Gwei max gas price
            valid_until: 1700000000 + 300,      // Valid for 5 minutes
            priority: 200,                      // High priority
            timestamp_ns: 1700000000000000000u64, // Current timestamp
        })
    }

    #[test]
    fn test_demo_arbitrage_tlv_roundtrip() {
        let original = create_test_arbitrage_tlv();

        let bytes = original.as_bytes();
        let recovered = *DemoDeFiArbitrageTLV::read_from(bytes).unwrap();

        assert_eq!(original, recovered);
    }

    #[test]
    fn test_demo_arbitrage_tlv_message_roundtrip() {
        let original = create_test_arbitrage_tlv();

        // Legacy TLV message test removed - use Protocol V2 TLVMessageBuilder for testing
        let bytes = original.as_bytes();
        let recovered = *DemoDeFiArbitrageTLV::read_from(bytes).unwrap();

        assert_eq!(original, recovered);
    }

    #[test]
    fn test_q64_conversion() {
        let original = create_test_arbitrage_tlv();

        // Test profit conversion (Q64.64 format)
        assert_eq!(original.expected_profit_usd(), "250.00");

        // Test capital conversion (Q64.64 format)
        assert_eq!(original.required_capital_usd(), "5000.00");

        // Test slippage conversion
        assert_eq!(original.slippage_percentage(), "0.50%");
    }

    #[test]
    fn test_validity_check() {
        let original = create_test_arbitrage_tlv();

        // Should be valid before expiry
        assert!(original.is_valid(1700000000 + 200));

        // Should be invalid after expiry
        assert!(!original.is_valid(1700000000 + 400));
    }

    #[test]
    fn test_negative_profit() {
        let mut arbitrage = create_test_arbitrage_tlv();
        arbitrage.expected_profit_q = -(150.0 * (1u128 << 64) as f64) as i128; // -$150.00 in Q64.64

        assert_eq!(arbitrage.expected_profit_usd(), "-150.00");
    }
}
