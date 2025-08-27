//! Pool State Management for Arbitrage
//!
//! Handles both V2 (constant product) and V3 (concentrated liquidity) pools

// TLVType removed with legacy TLV system
// Legacy TLV types removed - using Protocol V2 MessageHeader + TLV extensions
use super::address::{AddressPadding, EthAddress, ZERO_PADDING};
use super::market_data::PoolSwapTLV;
use crate::define_tlv;
use crate::tlv::fast_timestamp_ns;
use std::collections::HashMap;
use zerocopy::AsBytes;

// Pool state snapshot using macro for consistency
define_tlv! {
    /// Pool state snapshot - sent on initialization and periodically
    /// Contains static pool configuration and current state with full addresses
    PoolStateTLV {
        u128: {
            reserve0: u128,       // Native precision reserve0 (no scaling)
            reserve1: u128,       // Native precision reserve1 (no scaling)
            sqrt_price_x96: u128, // For V3 pools (0 for V2) - u128 to hold uint160
            liquidity: u128       // Active liquidity (native precision)
        }
        u64: {
            block_number: u64, // Block when this state was valid
            timestamp_ns: u64
        }
        u32: {
            tick: i32,     // Current tick for V3 (0 for V2)
            fee_rate: u32  // Fee in basis points (30 = 0.3%)
        }
        u16: { venue: u16 } // VenueId as u16 for zero-copy compatibility
        u8: {
            pool_type: u8,       // DEXProtocol as u8 for zero-copy compatibility
            token0_decimals: u8, // Native decimals for token0 (e.g., WMATIC=18)
            token1_decimals: u8, // Native decimals for token1 (e.g., USDC=6)
            _padding: [u8; 3]    // 3 bytes padding to make struct size 192 bytes
        }
        special: {
            pool_address: EthAddress,           // Pool contract address (20 bytes)
            pool_address_padding: AddressPadding, // Padding for alignment (12 bytes)
            token0_addr: EthAddress,            // Token0 address (20 bytes)
            token0_padding: AddressPadding,     // Padding for alignment (12 bytes)
            token1_addr: EthAddress,            // Token1 address (20 bytes)
            token1_padding: AddressPadding      // Padding for alignment (12 bytes)
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, AsBytes)]
pub enum DEXProtocol {
    UniswapV2 = 0,
    UniswapV3 = 1,
    SushiswapV2 = 2,
    QuickswapV3 = 3,
    Curve = 4,
    Balancer = 5,
}

/// Type alias for backward compatibility
pub type PoolType = DEXProtocol;

/// Configuration for V2 pool state
#[derive(Debug, Clone)]
pub struct V2PoolConfig {
    pub venue: u16,
    pub pool_address: EthAddress,
    pub token0_addr: EthAddress,
    pub token1_addr: EthAddress,
    pub token0_decimals: u8,
    pub token1_decimals: u8,
    pub reserve0: u128,
    pub reserve1: u128,
    pub fee_rate: u32,
    pub block: u64,
}

/// Configuration for V3 pool state
#[derive(Debug, Clone)]
pub struct V3PoolConfig {
    pub venue: u16,
    pub pool_address: EthAddress,
    pub token0_addr: EthAddress,
    pub token1_addr: EthAddress,
    pub token0_decimals: u8,
    pub token1_decimals: u8,
    pub sqrt_price_x96: u128,
    pub tick: i32,
    pub liquidity: u128,
    pub fee_rate: u32,
    pub block: u64,
}

impl PoolStateTLV {
    /// Create from V2 pool reserves with native precision
    pub fn from_v2_reserves(config: V2PoolConfig) -> Self {
        let timestamp_ns = fast_timestamp_ns(); // Ultra-fast ~5ns vs ~200ns

        Self::new_raw(
            config.reserve0,
            config.reserve1,
            0,            // sqrt_price_x96 (0 for V2)
            0,            // liquidity (V2 doesn't use this concept)
            config.block, // block_number
            timestamp_ns,
            0, // tick (0 for V2)
            config.fee_rate,
            config.venue,
            DEXProtocol::UniswapV2 as u8,
            config.token0_decimals,
            config.token1_decimals,
            [0u8; 3], // _padding
            config.pool_address,
            ZERO_PADDING,
            config.token0_addr,
            ZERO_PADDING,
            config.token1_addr,
            ZERO_PADDING,
        )
    }

    /// Create from V3 pool state with native precision
    pub fn from_v3_state(config: V3PoolConfig) -> Self {
        // Calculate virtual reserves from V3 state
        // This is approximate but useful for quick comparisons
        let (reserve0, reserve1) =
            calculate_v3_virtual_reserves(config.sqrt_price_x96, config.liquidity);

        let timestamp_ns = fast_timestamp_ns(); // Ultra-fast ~5ns vs ~200ns

        Self::new_raw(
            reserve0,
            reserve1,
            config.sqrt_price_x96,
            config.liquidity,
            config.block, // block_number
            timestamp_ns,
            config.tick,
            config.fee_rate,
            config.venue,
            DEXProtocol::UniswapV3 as u8,
            config.token0_decimals,
            config.token1_decimals,
            [0u8; 3], // _padding
            config.pool_address,
            ZERO_PADDING,
            config.token0_addr,
            ZERO_PADDING,
            config.token1_addr,
            ZERO_PADDING,
        )
    }

    /// Apply a swap to update state
    /// Note: amount0_delta and amount1_delta represent the net change to pool reserves
    /// Positive values mean tokens flowing INTO the pool, negative means OUT
    pub fn apply_swap(
        &mut self,
        amount0_delta: i128,
        amount1_delta: i128,
        new_sqrt_price: u128,
        new_tick: i32,
    ) {
        match self.pool_type {
            p if p == DEXProtocol::UniswapV2 as u8 || p == DEXProtocol::SushiswapV2 as u8 => {
                // Simple reserve update for V2
                // Apply deltas to u128 reserves with proper bounds checking
                if amount0_delta >= 0 {
                    self.reserve0 = self.reserve0.saturating_add(amount0_delta as u128);
                } else {
                    self.reserve0 = self.reserve0.saturating_sub((-amount0_delta) as u128);
                }

                if amount1_delta >= 0 {
                    self.reserve1 = self.reserve1.saturating_add(amount1_delta as u128);
                } else {
                    self.reserve1 = self.reserve1.saturating_sub((-amount1_delta) as u128);
                }
            }
            p if p == DEXProtocol::UniswapV3 as u8 || p == DEXProtocol::QuickswapV3 as u8 => {
                // V3 updates price and tick, recalculate virtual reserves
                self.sqrt_price_x96 = new_sqrt_price;
                self.tick = new_tick;
                let (new_r0, new_r1) =
                    calculate_v3_virtual_reserves(new_sqrt_price, self.liquidity);
                self.reserve0 = new_r0;
                self.reserve1 = new_r1;
            }
            _ => {
                // Other pool types - basic update
                if amount0_delta >= 0 {
                    self.reserve0 = self.reserve0.saturating_add(amount0_delta as u128);
                } else {
                    self.reserve0 = self.reserve0.saturating_sub((-amount0_delta) as u128);
                }

                if amount1_delta >= 0 {
                    self.reserve1 = self.reserve1.saturating_add(amount1_delta as u128);
                } else {
                    self.reserve1 = self.reserve1.saturating_sub((-amount1_delta) as u128);
                }
            }
        }
    }

    /// Get spot price (token1 per token0)
    pub fn spot_price(&self) -> f64 {
        match self.pool_type {
            p if p == DEXProtocol::UniswapV3 as u8 || p == DEXProtocol::QuickswapV3 as u8 => {
                // Use sqrt price for V3
                let sqrt_price = self.sqrt_price_x96 as f64 / (2_f64.powi(96));
                sqrt_price * sqrt_price
            }
            _ => {
                // Simple ratio for V2
                if self.reserve0 > 0 {
                    self.reserve1 as f64 / self.reserve0 as f64
                } else {
                    0.0
                }
            }
        }
    }

    // from_bytes() method now provided by the macro
    // Legacy to_tlv_message removed - use Protocol V2 TLVMessageBuilder instead
}

/// Calculate approximate virtual reserves from V3 state
fn calculate_v3_virtual_reserves(sqrt_price_x96: u128, liquidity: u128) -> (u128, u128) {
    // This is a simplified calculation
    // In reality, we'd need to consider the tick range
    let sqrt_price = sqrt_price_x96 as f64 / (2_f64.powi(96));
    let _price = sqrt_price * sqrt_price;

    // Virtual reserves based on current liquidity
    let l = liquidity as f64 / 1e18; // Convert from wei to decimal
    let reserve0 = (l / sqrt_price * 1e18) as u128;
    let reserve1 = (l * sqrt_price * 1e18) as u128;

    (reserve0, reserve1)
}

/// Pool state tracker - maintains current state of all pools
pub struct PoolStateTracker {
    states: HashMap<EthAddress, PoolStateTLV>, // Keyed by pool address (20-byte Ethereum address)
}

impl Default for PoolStateTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl PoolStateTracker {
    pub fn new() -> Self {
        Self {
            states: HashMap::new(),
        }
    }

    /// Initialize pool state (called on startup)
    pub async fn initialize_pool(
        &mut self,
        _pool_address: EthAddress,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // In production, we'd use eth_call to get current state:
        // - For V2: call getReserves()
        // - For V3: call slot0() for price/tick, liquidity() for active liquidity

        // Example (would need web3):
        // let contract = Contract::from_json(web3, pool_address, V3_POOL_ABI)?;
        // let slot0: (u160, i24, ...) = contract.query("slot0", (), None, Options::default(), None).await?;
        // let liquidity: u128 = contract.query("liquidity", (), None, Options::default(), None).await?;

        Ok(())
    }

    /// Update state from swap event
    pub fn update_from_swap(&mut self, pool_address: &EthAddress, swap: &PoolSwapTLV) {
        if let Some(state) = self.states.get_mut(pool_address) {
            // Calculate deltas (swap amounts affect reserves oppositely)
            // Compare full addresses to determine which token was swapped in
            // Use i128 for signed arithmetic with u128 values
            let amount0_delta = if swap.token_in_addr == state.token0_addr {
                swap.amount_in as i128 // Pool gains token0
            } else {
                -(swap.amount_out as i128) // Pool loses token0
            };

            let amount1_delta = if swap.token_in_addr == state.token1_addr {
                swap.amount_in as i128 // Pool gains token1
            } else {
                -(swap.amount_out as i128) // Pool loses token1
            };

            state.apply_swap(
                amount0_delta,
                amount1_delta,
                swap.sqrt_price_x96_as_u128(),
                swap.tick_after,
            );
        }
    }

    /// Get current price for a pool
    pub fn get_price(&self, pool_address: &EthAddress) -> Option<f64> {
        self.states.get(pool_address).map(|s| s.spot_price())
    }

    /// Find arbitrage opportunities
    pub fn find_arbitrage(&self) -> Vec<ArbitrageOpportunity> {
        let mut opportunities = Vec::new();

        // Compare prices across pools with same tokens
        // This is simplified - real implementation would consider:
        // - Gas costs
        // - Slippage
        // - MEV protection
        // - Multi-hop paths

        for (pool1_addr, state1) in &self.states {
            for (pool2_addr, state2) in &self.states {
                if pool1_addr != pool2_addr {
                    let price_diff = (state1.spot_price() - state2.spot_price()).abs();
                    let avg_price = (state1.spot_price() + state2.spot_price()) / 2.0;
                    let spread_pct = price_diff / avg_price * 100.0;

                    if spread_pct > 0.5 {
                        // 0.5% spread threshold
                        opportunities.push(ArbitrageOpportunity {
                            pool1: *pool1_addr, // Already an EthAddress
                            pool1_padding: ZERO_PADDING,
                            pool2: *pool2_addr, // Already an EthAddress
                            pool2_padding: ZERO_PADDING,
                            spread_pct,
                            estimated_profit: calculate_profit(state1, state2, spread_pct),
                        });
                    }
                }
            }
        }

        opportunities
    }
}

#[derive(Debug)]
pub struct ArbitrageOpportunity {
    pub pool1: EthAddress,             // Pool 1 address (20 bytes)
    pub pool1_padding: AddressPadding, // Pool 1 padding (12 bytes)
    pub pool2: EthAddress,             // Pool 2 address (20 bytes)
    pub pool2_padding: AddressPadding, // Pool 2 padding (12 bytes)
    pub spread_pct: f64,
    pub estimated_profit: u128,
}

fn calculate_profit(_state1: &PoolStateTLV, _state2: &PoolStateTLV, spread: f64) -> u128 {
    // Simplified profit calculation
    // Real implementation would simulate the actual swap amounts
    let trade_size = 10000_000000000000000000u128; // $10k in 18 decimals (wei)
    let gross_profit = (trade_size as f64 * spread / 100.0) as u128;
    let gas_cost = 50_000000000000000000u128; // $50 gas estimate in wei
    gross_profit.saturating_sub(gas_cost)
}

// Also need to add to TLVType enum:
// PoolState = 15,  // Pool state snapshot
