//! Real arbitrage signal TLV for production use
//!
//! This replaces the demo TLV with actual arbitrage opportunity data

use zerocopy::{AsBytes, FromBytes, FromZeroes};

/// Real arbitrage signal with actual pool and token data
/// TLV Type: 21 (Signal domain)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, AsBytes, FromBytes, FromZeroes)]
pub struct ArbitrageSignalTLV {
    /// Strategy ID (21 for flash arbitrage)
    pub strategy_id: u16,

    /// Unique signal ID
    pub signal_id: u64,

    /// Chain ID (137 for Polygon)
    pub chain_id: u32,

    /// Source pool address (20 bytes)
    pub source_pool: [u8; 20],

    /// Target pool address (20 bytes)
    pub target_pool: [u8; 20],

    /// Source pool venue/DEX (e.g., UniswapV2 = 300)
    pub source_venue: u16,

    /// Target pool venue/DEX (e.g., UniswapV3 = 301)
    pub target_venue: u16,

    /// Token in address (20 bytes)
    pub token_in: [u8; 20],

    /// Token out address (20 bytes)
    pub token_out: [u8; 20],

    /// Expected profit in USD (8 decimals: $1.23 = 123000000)
    pub expected_profit_usd_q8: i64,

    /// Required capital in USD (8 decimals)
    pub required_capital_usd_q8: i64,

    /// Spread percentage (basis points: 150 = 1.5%)
    pub spread_bps: u16,

    /// DEX fees in USD (8 decimals)
    pub dex_fees_usd_q8: i64,

    /// Gas cost estimate in USD (8 decimals)
    pub gas_cost_usd_q8: i64,

    /// Slippage estimate in USD (8 decimals)
    pub slippage_usd_q8: i64,

    /// Net profit in USD (8 decimals)
    pub net_profit_usd_q8: i64,

    /// Slippage tolerance (basis points)
    pub slippage_tolerance_bps: u16,

    /// Maximum gas price in gwei
    pub max_gas_price_gwei: u32,

    /// Timestamp when opportunity expires (unix seconds)
    pub valid_until: u32,

    /// Priority score (0-65535, higher = more urgent)
    pub priority: u16,

    /// Reserved for future use
    pub reserved: [u8; 2],

    /// Timestamp when signal was created (nanoseconds)
    pub timestamp_ns: u64,
}

impl ArbitrageSignalTLV {
    /// Create from bytes (for parsing)
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.len() != std::mem::size_of::<Self>() {
            return Err("Invalid ArbitrageSignalTLV size");
        }

        // Safety: We've verified the size matches our struct
        let tlv = unsafe { std::ptr::read_unaligned(bytes.as_ptr() as *const Self) };

        Ok(tlv)
    }

    /// Create a new arbitrage signal
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        source_pool: [u8; 20],
        target_pool: [u8; 20],
        source_venue: u16,
        target_venue: u16,
        token_in: [u8; 20],
        token_out: [u8; 20],
        expected_profit_usd: f64,
        required_capital_usd: f64,
        spread_bps: u16,
        dex_fees_usd: f64,
        gas_cost_usd: f64,
        slippage_usd: f64,
        timestamp_ns: u64,
    ) -> Self {
        // Convert USD amounts to 8-decimal fixed point
        let expected_profit_usd_q8 = (expected_profit_usd * 100_000_000.0) as i64;
        let required_capital_usd_q8 = (required_capital_usd * 100_000_000.0) as i64;
        let dex_fees_usd_q8 = (dex_fees_usd * 100_000_000.0) as i64;
        let gas_cost_usd_q8 = (gas_cost_usd * 100_000_000.0) as i64;
        let slippage_usd_q8 = (slippage_usd * 100_000_000.0) as i64;
        let net_profit_usd_q8 =
            expected_profit_usd_q8 - dex_fees_usd_q8 - gas_cost_usd_q8 - slippage_usd_q8;

        Self {
            strategy_id: 21,         // Flash arbitrage strategy
            signal_id: timestamp_ns, // Use timestamp as unique ID for now
            chain_id: 137,           // Polygon
            source_pool,
            target_pool,
            source_venue,
            target_venue,
            token_in,
            token_out,
            expected_profit_usd_q8,
            required_capital_usd_q8,
            spread_bps,
            dex_fees_usd_q8,
            gas_cost_usd_q8,
            slippage_usd_q8,
            net_profit_usd_q8,
            slippage_tolerance_bps: 50, // 0.5% default
            max_gas_price_gwei: 100,    // 100 gwei max
            valid_until: (timestamp_ns / 1_000_000_000) as u32 + 300, // Valid for 5 minutes
            priority: ((spread_bps as f64 * 10.0).min(65535.0)) as u16, // Priority based on spread
            reserved: [0u8; 2],
            timestamp_ns,
        }
    }

    /// Get expected profit in USD
    pub fn expected_profit_usd(&self) -> f64 {
        self.expected_profit_usd_q8 as f64 / 100_000_000.0
    }

    /// Get required capital in USD
    pub fn required_capital_usd(&self) -> f64 {
        self.required_capital_usd_q8 as f64 / 100_000_000.0
    }

    /// Get DEX fees in USD
    pub fn dex_fees_usd(&self) -> f64 {
        self.dex_fees_usd_q8 as f64 / 100_000_000.0
    }

    /// Get gas cost in USD
    pub fn gas_cost_usd(&self) -> f64 {
        self.gas_cost_usd_q8 as f64 / 100_000_000.0
    }

    /// Get slippage in USD
    pub fn slippage_usd(&self) -> f64 {
        self.slippage_usd_q8 as f64 / 100_000_000.0
    }

    /// Get net profit in USD
    pub fn net_profit_usd(&self) -> f64 {
        self.net_profit_usd_q8 as f64 / 100_000_000.0
    }

    /// Get spread as percentage
    pub fn spread_percent(&self) -> f64 {
        self.spread_bps as f64 / 100.0
    }

    /// Check if signal is still valid
    pub fn is_valid(&self, current_time_secs: u32) -> bool {
        current_time_secs <= self.valid_until
    }
}

/// Expected size for ArbitrageSignalTLV
pub const ARBITRAGE_SIGNAL_TLV_SIZE: usize = std::mem::size_of::<ArbitrageSignalTLV>();

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arbitrage_signal_size() {
        // Verify struct size for TLV encoding
        assert_eq!(ARBITRAGE_SIGNAL_TLV_SIZE, 168); // Calculate actual size
    }

    #[test]
    fn test_arbitrage_signal_creation() {
        let source_pool = [1u8; 20];
        let target_pool = [2u8; 20];
        let token_in = [3u8; 20];
        let token_out = [4u8; 20];

        let signal = ArbitrageSignalTLV::new(
            source_pool,
            target_pool,
            300, // UniswapV2
            301, // UniswapV3
            token_in,
            token_out,
            100.50,  // $100.50 profit
            10000.0, // $10k capital
            150,     // 1.5% spread
            60.0,    // $60 DEX fees
            3.0,     // $3 gas
            5.0,     // $5 slippage
            1234567890_000_000_000,
        );

        // Copy packed fields to avoid unaligned references
        let strategy_id = signal.strategy_id;
        let chain_id = signal.chain_id;

        assert_eq!(strategy_id, 21);
        assert_eq!(chain_id, 137);
        assert_eq!(signal.expected_profit_usd(), 100.50);
        assert_eq!(signal.net_profit_usd(), 100.50 - 60.0 - 3.0 - 5.0);
        assert_eq!(signal.spread_percent(), 1.5);
    }
}
