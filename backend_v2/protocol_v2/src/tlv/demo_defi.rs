//! Demo DeFi Arbitrage TLV Structures
//!
//! Specialized TLV for dashboard demo of arbitrage opportunities.
//! Uses vendor TLV type 200 for experimental/demo purposes.

use crate::VenueId;
// TLVType removed with legacy TLV system
// Legacy TLV types removed - using Protocol V2 MessageHeader + TLV extensions
use std::convert::TryInto;

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
#[derive(Debug, Clone, PartialEq)]
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

    // Pool Information (fixed length)
    pub venue_a: VenueId, // First pool venue (Uniswap V2, V3, etc.)
    pub pool_a: [u8; 20], // First pool address
    pub venue_b: VenueId, // Second pool venue
    pub pool_b: [u8; 20], // Second pool address

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
    pub pool_a: [u8; 20],
    pub venue_b: VenueId,
    pub pool_b: [u8; 20],
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
            venue_a: config.venue_a,
            pool_a: config.pool_a,
            venue_b: config.venue_b,
            pool_b: config.pool_b,
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
    /// Note: These values are already in the final denomination (e.g., $250.00, not wei-style)
    pub fn q64_to_decimal_string(q64_value: u128, decimals: u8) -> String {
        // For Q64.64, we have 64.64 fixed point
        // The decimal point is implied after 64 bits from the right
        // So we divide by 2^64 to get the fractional part, but since we want normal decimal format,
        // we treat the value as already being in the correct scale
        let divisor = 10_u128.pow(decimals as u32);
        let integer_part = q64_value / divisor;
        let fractional_part = q64_value % divisor;
        format!(
            "{}.{:0width$}",
            integer_part,
            fractional_part,
            width = decimals as usize
        )
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

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Strategy Identity (12 bytes)
        bytes.extend_from_slice(&self.strategy_id.to_le_bytes());
        bytes.extend_from_slice(&self.signal_id.to_le_bytes());
        bytes.push(self.confidence);
        bytes.push(self.chain_id);
        bytes.extend_from_slice(&[0, 0]); // 2 bytes padding for alignment

        // Economics (48 bytes) - Q64.64 format
        bytes.extend_from_slice(&self.expected_profit_q.to_le_bytes());
        bytes.extend_from_slice(&self.required_capital_q.to_le_bytes());
        bytes.extend_from_slice(&self.estimated_gas_cost_q.to_le_bytes());

        // Pool A venue (2 bytes)
        bytes.extend_from_slice(&(self.venue_a as u16).to_le_bytes());

        // Pool A address (20 bytes)
        bytes.extend_from_slice(&self.pool_a);

        // Pool B venue (2 bytes)
        bytes.extend_from_slice(&(self.venue_b as u16).to_le_bytes());

        // Pool B address (20 bytes)
        bytes.extend_from_slice(&self.pool_b);

        // Trade Execution (32 bytes)
        bytes.extend_from_slice(&self.token_in.to_le_bytes());
        bytes.extend_from_slice(&self.token_out.to_le_bytes());
        bytes.extend_from_slice(&self.optimal_amount_q.to_le_bytes());

        // Risk Parameters (12 bytes)
        bytes.extend_from_slice(&self.slippage_tolerance.to_le_bytes());
        bytes.extend_from_slice(&self.max_gas_price_gwei.to_le_bytes());
        bytes.extend_from_slice(&self.valid_until.to_le_bytes());
        bytes.push(self.priority);
        bytes.push(self.reserved);

        // Timing (8 bytes)
        bytes.extend_from_slice(&self.timestamp_ns.to_le_bytes());

        bytes
    }

    /// Deserialize from bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self, String> {
        if data.len() < 124 {
            // Minimum size check
            return Err(format!(
                "Invalid DemoDeFiArbitrageTLV size: need at least 124 bytes, got {}",
                data.len()
            ));
        }

        let mut offset = 0;

        // Strategy Identity (12 bytes)
        let strategy_id = u16::from_le_bytes(data[offset..offset + 2].try_into().unwrap());
        offset += 2;
        let signal_id = u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
        offset += 8;
        let confidence = data[offset];
        offset += 1;
        let chain_id = data[offset];
        offset += 1;
        // Skip 2 bytes padding
        offset += 2;

        // Economics (48 bytes)
        let expected_profit_q = i128::from_le_bytes(data[offset..offset + 16].try_into().unwrap());
        offset += 16;
        let required_capital_q = u128::from_le_bytes(data[offset..offset + 16].try_into().unwrap());
        offset += 16;
        let estimated_gas_cost_q =
            u128::from_le_bytes(data[offset..offset + 16].try_into().unwrap());
        offset += 16;

        // Pool A venue (2 bytes)
        let venue_a = VenueId::try_from(u16::from_le_bytes(
            data[offset..offset + 2].try_into().unwrap(),
        ))
        .map_err(|_| "Invalid venue A ID")?;
        offset += 2;

        // Pool A address (20 bytes)
        if offset + 20 > data.len() {
            return Err("Insufficient data for pool A address".to_string());
        }
        let mut pool_a = [0u8; 20];
        pool_a.copy_from_slice(&data[offset..offset + 20]);
        offset += 20;

        // Pool B venue (2 bytes)
        let venue_b = VenueId::try_from(u16::from_le_bytes(
            data[offset..offset + 2].try_into().unwrap(),
        ))
        .map_err(|_| "Invalid venue B ID")?;
        offset += 2;

        // Pool B address (20 bytes)
        if offset + 20 > data.len() {
            return Err("Insufficient data for pool B address".to_string());
        }
        let mut pool_b = [0u8; 20];
        pool_b.copy_from_slice(&data[offset..offset + 20]);
        offset += 20;

        // Trade Execution (32 bytes)
        let token_in = u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
        offset += 8;
        let token_out = u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
        offset += 8;
        let optimal_amount_q = u128::from_le_bytes(data[offset..offset + 16].try_into().unwrap());
        offset += 16;

        // Risk Parameters (12 bytes)
        let slippage_tolerance = u16::from_le_bytes(data[offset..offset + 2].try_into().unwrap());
        offset += 2;
        let max_gas_price_gwei = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap());
        offset += 4;
        let valid_until = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap());
        offset += 4;
        let priority = data[offset];
        offset += 1;
        let reserved = data[offset];
        offset += 1;

        // Timing (8 bytes)
        let timestamp_ns = u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());

        Ok(Self {
            strategy_id,
            signal_id,
            confidence,
            chain_id,
            expected_profit_q,
            required_capital_q,
            estimated_gas_cost_q,
            venue_a,
            pool_a,
            venue_b,
            pool_b,
            token_in,
            token_out,
            optimal_amount_q,
            slippage_tolerance,
            max_gas_price_gwei,
            valid_until,
            priority,
            reserved,
            timestamp_ns,
        })
    }

    // Legacy TLV message methods removed - use Protocol V2 TLVMessageBuilder instead
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{InstrumentId, VenueId};

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

        DemoDeFiArbitrageTLV::new(
            21,                      // Flash arbitrage strategy
            0x1234567890abcdef,      // Unique signal ID
            85,                      // 85% confidence
            137,                     // Polygon chain
            25000000000i128,         // $250.00 expected profit (8 decimals)
            500000000000u128,        // $5000.00 required capital (8 decimals)
            2500000000000000000u128, // 0.0025 MATIC gas cost (18 decimals)
            VenueId::UniswapV2,      // Pool A venue
            pool_a,                  // Pool A address
            VenueId::UniswapV3,      // Pool B venue
            pool_b,                  // Pool B address
            usdc_token_id,           // USDC token (truncated address)
            weth_token_id,           // WETH token (truncated address)
            100000000000u128,        // 1000.00 USDC optimal amount (8 decimals)
            50,                      // 0.5% slippage tolerance
            100,                     // 100 Gwei max gas price
            1700000000 + 300,        // Valid for 5 minutes
            200,                     // High priority
            1700000000000000000u64,  // Current timestamp
        )
    }

    #[test]
    fn test_demo_arbitrage_tlv_roundtrip() {
        let original = create_test_arbitrage_tlv();

        let bytes = original.to_bytes();
        let recovered = DemoDeFiArbitrageTLV::from_bytes(&bytes).unwrap();

        assert_eq!(original, recovered);
    }

    #[test]
    fn test_demo_arbitrage_tlv_message_roundtrip() {
        let original = create_test_arbitrage_tlv();

        // Legacy TLV message test removed - use Protocol V2 TLVMessageBuilder for testing
        let bytes = original.to_bytes();
        let recovered = DemoDeFiArbitrageTLV::from_bytes(&bytes).unwrap();

        assert_eq!(original, recovered);
    }

    #[test]
    fn test_q64_conversion() {
        let original = create_test_arbitrage_tlv();

        // Test profit conversion
        assert_eq!(original.expected_profit_usd(), "250.00000000");

        // Test capital conversion
        assert_eq!(original.required_capital_usd(), "5000.00000000");

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
        arbitrage.expected_profit_q = -15000000000i128; // -$150.00 (8 decimals)

        assert_eq!(arbitrage.expected_profit_usd(), "-150.00000000");
    }
}
