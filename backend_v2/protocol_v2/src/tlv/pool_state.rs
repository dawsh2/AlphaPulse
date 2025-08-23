//! Pool State Management for Arbitrage
//!
//! Handles both V2 (constant product) and V3 (concentrated liquidity) pools

use crate::VenueId; // TLVType removed with legacy TLV system
                    // Legacy TLV types removed - using Protocol V2 MessageHeader + TLV extensions
use super::market_data::PoolSwapTLV;
use std::collections::HashMap;

/// Pool state snapshot - sent on initialization and periodically
/// Contains static pool configuration and current state with full addresses
#[derive(Debug, Clone, PartialEq)]
pub struct PoolStateTLV {
    pub venue: VenueId,
    pub pool_address: [u8; 20], // Full pool contract address
    pub token0_addr: [u8; 20],  // Full token0 address
    pub token1_addr: [u8; 20],  // Full token1 address
    pub pool_type: DEXProtocol,
    pub token0_decimals: u8,  // Native decimals for token0 (e.g., WMATIC=18)
    pub token1_decimals: u8,  // Native decimals for token1 (e.g., USDC=6)
    pub reserve0: u128,       // Native precision reserve0 (no scaling)
    pub reserve1: u128,       // Native precision reserve1 (no scaling)
    pub sqrt_price_x96: u128, // For V3 pools (0 for V2) - u128 to hold uint160
    pub tick: i32,            // Current tick for V3 (0 for V2)
    pub liquidity: u128,      // Active liquidity (native precision)
    pub fee_rate: u32,        // Fee in basis points (30 = 0.3%)
    pub block_number: u64,    // Block when this state was valid
    pub timestamp_ns: u64,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
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

impl PoolStateTLV {
    /// Create from V2 pool reserves with native precision
    pub fn from_v2_reserves(
        venue: VenueId,
        pool_address: [u8; 20],
        token0_addr: [u8; 20],
        token1_addr: [u8; 20],
        token0_decimals: u8,
        token1_decimals: u8,
        reserve0: u128,
        reserve1: u128,
        fee_rate: u32,
        block: u64,
    ) -> Self {
        Self {
            venue,
            pool_address,
            token0_addr,
            token1_addr,
            pool_type: DEXProtocol::UniswapV2,
            token0_decimals,
            token1_decimals,
            reserve0,
            reserve1,
            sqrt_price_x96: 0,
            tick: 0,
            liquidity: 0, // V2 doesn't use this concept
            fee_rate,
            block_number: block,
            timestamp_ns: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
        }
    }

    /// Create from V3 pool state with native precision
    pub fn from_v3_state(
        venue: VenueId,
        pool_address: [u8; 20],
        token0_addr: [u8; 20],
        token1_addr: [u8; 20],
        token0_decimals: u8,
        token1_decimals: u8,
        sqrt_price_x96: u128,
        tick: i32,
        liquidity: u128,
        fee_rate: u32,
        block: u64,
    ) -> Self {
        // Calculate virtual reserves from V3 state
        // This is approximate but useful for quick comparisons
        let (reserve0, reserve1) = calculate_v3_virtual_reserves(sqrt_price_x96, liquidity);

        Self {
            venue,
            pool_address,
            token0_addr,
            token1_addr,
            pool_type: DEXProtocol::UniswapV3,
            token0_decimals,
            token1_decimals,
            reserve0,
            reserve1,
            sqrt_price_x96,
            tick,
            liquidity,
            fee_rate,
            block_number: block,
            timestamp_ns: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
        }
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
            DEXProtocol::UniswapV2 | DEXProtocol::SushiswapV2 => {
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
            DEXProtocol::UniswapV3 | DEXProtocol::QuickswapV3 => {
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
            DEXProtocol::UniswapV3 | DEXProtocol::QuickswapV3 => {
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

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Venue (2 bytes)
        bytes.extend_from_slice(&(self.venue as u16).to_le_bytes());

        // Pool and token addresses (20 bytes each)
        bytes.extend_from_slice(&self.pool_address);
        bytes.extend_from_slice(&self.token0_addr);
        bytes.extend_from_slice(&self.token1_addr);

        // Pool type (1 byte)
        bytes.push(self.pool_type as u8);

        // Token decimals (2 bytes)
        bytes.push(self.token0_decimals);
        bytes.push(self.token1_decimals);

        // State fields (u128 takes 16 bytes each)
        bytes.extend_from_slice(&self.reserve0.to_le_bytes());
        bytes.extend_from_slice(&self.reserve1.to_le_bytes());
        bytes.extend_from_slice(&self.sqrt_price_x96.to_le_bytes());
        bytes.extend_from_slice(&self.tick.to_le_bytes());
        bytes.extend_from_slice(&self.liquidity.to_le_bytes());
        bytes.extend_from_slice(&self.fee_rate.to_le_bytes());
        bytes.extend_from_slice(&self.block_number.to_le_bytes());
        bytes.extend_from_slice(&self.timestamp_ns.to_le_bytes());

        bytes
    }

    /// Deserialize from bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self, String> {
        if data.len() < 153 {
            // 2 + 20*3 + 1 + 2 + 16*3 + 16 + 4 + 16 + 4 + 8 + 8 = 153
            return Err(format!(
                "Invalid PoolStateTLV size: need at least 153 bytes, got {}",
                data.len()
            ));
        }

        let mut offset = 0;

        // Venue (2 bytes)
        let venue = VenueId::try_from(u16::from_le_bytes([data[0], data[1]]))
            .map_err(|_| "Invalid venue ID")?;
        offset += 2;

        // Pool and token addresses (20 bytes each)
        let mut pool_address = [0u8; 20];
        pool_address.copy_from_slice(&data[offset..offset + 20]);
        offset += 20;

        let mut token0_addr = [0u8; 20];
        token0_addr.copy_from_slice(&data[offset..offset + 20]);
        offset += 20;

        let mut token1_addr = [0u8; 20];
        token1_addr.copy_from_slice(&data[offset..offset + 20]);
        offset += 20;

        // Pool type (1 byte)
        if offset >= data.len() {
            return Err("Missing pool type".to_string());
        }
        let pool_type = match data[offset] {
            0 => DEXProtocol::UniswapV2,
            1 => DEXProtocol::UniswapV3,
            2 => DEXProtocol::SushiswapV2,
            3 => DEXProtocol::QuickswapV3,
            4 => DEXProtocol::Curve,
            5 => DEXProtocol::Balancer,
            _ => return Err(format!("Invalid pool type: {}", data[offset])),
        };
        offset += 1;

        // Token decimals (2 bytes)
        if offset + 2 > data.len() {
            return Err("Missing token decimals".to_string());
        }
        let token0_decimals = data[offset];
        let token1_decimals = data[offset + 1];
        offset += 2;

        // Fixed fields (16*3 + 16 + 4 + 16 + 4 + 8 + 8 = 88 bytes)
        if offset + 88 != data.len() {
            return Err(format!(
                "Invalid remaining data size: expected 88 bytes, got {}",
                data.len() - offset
            ));
        }

        let reserve0 = u128::from_le_bytes(data[offset..offset + 16].try_into().unwrap());
        offset += 16;
        let reserve1 = u128::from_le_bytes(data[offset..offset + 16].try_into().unwrap());
        offset += 16;
        let sqrt_price_x96 = u128::from_le_bytes(data[offset..offset + 16].try_into().unwrap());
        offset += 16;
        let tick = i32::from_le_bytes(data[offset..offset + 4].try_into().unwrap());
        offset += 4;
        let liquidity = u128::from_le_bytes(data[offset..offset + 16].try_into().unwrap());
        offset += 16;
        let fee_rate = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap());
        offset += 4;
        let block_number = u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
        offset += 8;
        let timestamp_ns = u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());

        Ok(Self {
            venue,
            pool_address,
            token0_addr,
            token1_addr,
            pool_type,
            token0_decimals,
            token1_decimals,
            reserve0,
            reserve1,
            sqrt_price_x96,
            tick,
            liquidity,
            fee_rate,
            block_number,
            timestamp_ns,
        })
    }

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
    states: HashMap<[u8; 20], PoolStateTLV>, // Keyed by pool address
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
        _pool_address: [u8; 20],
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
    pub fn update_from_swap(&mut self, pool_address: &[u8; 20], swap: &PoolSwapTLV) {
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
    pub fn get_price(&self, pool_address: &[u8; 20]) -> Option<f64> {
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
                            pool1: *pool1_addr,
                            pool2: *pool2_addr,
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
    pub pool1: [u8; 20], // Pool 1 address
    pub pool2: [u8; 20], // Pool 2 address
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
