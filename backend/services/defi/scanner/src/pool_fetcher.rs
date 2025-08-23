use anyhow::Result;
use ethers::prelude::*;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::sync::Arc;
use tracing::{info, debug, error};

/// Fetches REAL pool data from blockchain - NO MOCKS
pub struct PoolFetcher {
    provider: Arc<Provider<Http>>,
}

impl PoolFetcher {
    pub fn new(rpc_url: &str) -> Result<Self> {
        let provider = Provider::<Http>::try_from(rpc_url)?;
        Ok(Self {
            provider: Arc::new(provider),
        })
    }
    
    /// Fetch real reserves from Uniswap V2 style pool
    pub async fn fetch_v2_reserves(&self, pool_address: Address) -> Result<V2PoolData> {
        // V2 Pool ABI for getReserves
        abigen!(
            IUniswapV2Pair,
            r#"[
                function getReserves() external view returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast)
                function token0() external view returns (address)
                function token1() external view returns (address)
                function fee() external view returns (uint24)
            ]"#
        );
        
        let pool = IUniswapV2Pair::new(pool_address, self.provider.clone());
        
        // Fetch REAL reserves from blockchain
        let (reserve0, reserve1, _) = pool.get_reserves().call().await?;
        let token0 = pool.token0().call().await?;
        let token1 = pool.token1().call().await?;
        
        // Try to get fee, default to 0.3% if not available
        let fee_bps = pool.fee().call().await.unwrap_or(30);
        
        info!("Fetched REAL V2 pool data from {}: reserves {} / {}", 
              pool_address, reserve0, reserve1);
        
        Ok(V2PoolData {
            address: pool_address,
            token0,
            token1,
            reserve0: Decimal::from_str(&reserve0.to_string())?,
            reserve1: Decimal::from_str(&reserve1.to_string())?,
            fee_bps: fee_bps as u32,
        })
    }
    
    /// Fetch real liquidity and tick data from Uniswap V3 pool
    pub async fn fetch_v3_state(&self, pool_address: Address) -> Result<V3PoolData> {
        // V3 Pool ABI for slot0 and liquidity
        abigen!(
            IUniswapV3Pool,
            r#"[
                {"inputs":[],"name":"slot0","outputs":[{"name":"sqrtPriceX96","type":"uint160"},{"name":"tick","type":"int24"},{"name":"observationIndex","type":"uint16"},{"name":"observationCardinality","type":"uint16"},{"name":"observationCardinalityNext","type":"uint16"},{"name":"feeProtocol","type":"uint8"},{"name":"unlocked","type":"bool"}],"stateMutability":"view","type":"function"},
                {"inputs":[],"name":"liquidity","outputs":[{"name":"","type":"uint128"}],"stateMutability":"view","type":"function"},
                {"inputs":[],"name":"token0","outputs":[{"name":"","type":"address"}],"stateMutability":"view","type":"function"},
                {"inputs":[],"name":"token1","outputs":[{"name":"","type":"address"}],"stateMutability":"view","type":"function"},
                {"inputs":[],"name":"fee","outputs":[{"name":"","type":"uint24"}],"stateMutability":"view","type":"function"},
                {"inputs":[],"name":"tickSpacing","outputs":[{"name":"","type":"int24"}],"stateMutability":"view","type":"function"}
            ]"#
        );
        
        let pool = IUniswapV3Pool::new(pool_address, self.provider.clone());
        
        // Fetch REAL state from blockchain
        let (sqrt_price_x96, tick, _, _, _, _, _) = pool.slot0().call().await?;
        let liquidity = pool.liquidity().call().await?;
        let fee = pool.fee().call().await?;
        let token0 = pool.token0().call().await?;
        let token1 = pool.token1().call().await?;
        let tick_spacing = pool.tick_spacing().call().await?;
        
        info!("Fetched REAL V3 pool data from {}: tick={}, liquidity={}, sqrtPrice={}", 
              pool_address, tick, liquidity, sqrt_price_x96);
        
        Ok(V3PoolData {
            address: pool_address,
            token0,
            token1,
            sqrt_price_x96: Decimal::from_str(&sqrt_price_x96.to_string())?,
            tick,
            liquidity: Decimal::from_str(&liquidity.to_string())?,
            fee: fee as u32,
            tick_spacing,
        })
    }
    
    /// Calculate optimal arbitrage using closed-form solution for V2 pools
    pub fn calculate_v2_optimal_arbitrage(
        &self,
        pool1: &V2PoolData,
        pool2: &V2PoolData,
        gas_cost_usd: Decimal,
    ) -> Result<OptimalArbitrage> {
        // Closed-form solution for V2 arbitrage
        // x* = sqrt(r1_in * r1_out * r2_out * r2_in * f1 * f2) - r1_in * f1
        //      --------------------------------------------------------
        //                              f1
        
        let f1 = dec!(1) - Decimal::from(pool1.fee_bps) / dec!(10000);
        let f2 = dec!(1) - Decimal::from(pool2.fee_bps) / dec!(10000);
        
        // Check if arbitrage exists (price difference)
        let price1 = pool1.reserve1 / pool1.reserve0;
        let price2 = pool2.reserve1 / pool2.reserve0;
        
        if price1 >= price2 {
            return Ok(OptimalArbitrage {
                optimal_amount: dec!(0),
                expected_profit: dec!(0),
                profitable: false,
                reason: "No price advantage".to_string(),
            });
        }
        
        // Calculate optimal amount using closed-form solution
        let sqrt_arg = pool1.reserve0 * pool1.reserve1 * pool2.reserve1 * pool2.reserve0 * f1 * f2;
        let sqrt_value = sqrt_arg.sqrt().unwrap_or(dec!(0));
        
        let optimal_amount = (sqrt_value - pool1.reserve0 * f1) / f1;
        
        if optimal_amount <= dec!(0) {
            return Ok(OptimalArbitrage {
                optimal_amount: dec!(0),
                expected_profit: dec!(0),
                profitable: false,
                reason: "Optimal amount is negative".to_string(),
            });
        }
        
        // Calculate expected profit
        let amount_out_1 = self.calculate_v2_output(optimal_amount, pool1.reserve0, pool1.reserve1, pool1.fee_bps)?;
        let amount_out_2 = self.calculate_v2_output(amount_out_1, pool2.reserve0, pool2.reserve1, pool2.fee_bps)?;
        
        let gross_profit = amount_out_2 - optimal_amount;
        let net_profit = gross_profit - gas_cost_usd;
        
        Ok(OptimalArbitrage {
            optimal_amount,
            expected_profit: net_profit,
            profitable: net_profit > dec!(0),
            reason: if net_profit > dec!(0) {
                "Profitable arbitrage found".to_string()
            } else {
                "Not profitable after gas".to_string()
            },
        })
    }
    
    /// Calculate V2 swap output
    fn calculate_v2_output(
        &self,
        amount_in: Decimal,
        reserve_in: Decimal,
        reserve_out: Decimal,
        fee_bps: u32,
    ) -> Result<Decimal> {
        let fee_multiplier = dec!(10000) - Decimal::from(fee_bps);
        let amount_in_with_fee = amount_in * fee_multiplier;
        let numerator = amount_in_with_fee * reserve_out;
        let denominator = reserve_in * dec!(10000) + amount_in_with_fee;
        
        Ok(numerator / denominator)
    }
    
    /// Calculate optimal V3 arbitrage (within current tick)
    pub fn calculate_v3_optimal_arbitrage(
        &self,
        pool1: &V3PoolData,
        pool2: &V3PoolData,
        gas_cost_usd: Decimal,
    ) -> Result<OptimalArbitrage> {
        // Convert sqrtPriceX96 to actual price
        let q96 = dec!(2).powi(96);
        let sqrt_price1 = pool1.sqrt_price_x96 / q96;
        let sqrt_price2 = pool2.sqrt_price_x96 / q96;
        
        let price1 = sqrt_price1 * sqrt_price1;
        let price2 = sqrt_price2 * sqrt_price2;
        
        if price2 <= price1 {
            return Ok(OptimalArbitrage {
                optimal_amount: dec!(0),
                expected_profit: dec!(0),
                profitable: false,
                reason: "No price advantage in V3 pools".to_string(),
            });
        }
        
        // Closed-form solution for V3 (within current tick)
        // Optimal amount = L * (sqrt(P2/P1) - 1)
        let price_ratio = price2 / price1;
        let sqrt_ratio = price_ratio.sqrt().unwrap_or(dec!(1));
        
        let l_eff = pool1.liquidity.min(pool2.liquidity);
        let optimal_amount = l_eff * (sqrt_ratio - dec!(1));
        
        // Calculate profit
        let expected_output = optimal_amount * sqrt_ratio;
        let gross_profit = expected_output - optimal_amount;
        let net_profit = gross_profit - gas_cost_usd;
        
        Ok(OptimalArbitrage {
            optimal_amount,
            expected_profit: net_profit,
            profitable: net_profit > dec!(0),
            reason: if net_profit > dec!(0) {
                "Profitable V3 arbitrage found".to_string()
            } else {
                "V3 not profitable after gas".to_string()
            },
        })
    }
}

#[derive(Debug, Clone)]
pub struct V2PoolData {
    pub address: Address,
    pub token0: Address,
    pub token1: Address,
    pub reserve0: Decimal,
    pub reserve1: Decimal,
    pub fee_bps: u32,
}

#[derive(Debug, Clone)]
pub struct V3PoolData {
    pub address: Address,
    pub token0: Address,
    pub token1: Address,
    pub sqrt_price_x96: Decimal,
    pub tick: i32,
    pub liquidity: Decimal,
    pub fee: u32,
    pub tick_spacing: i32,
}

#[derive(Debug, Clone)]
pub struct OptimalArbitrage {
    pub optimal_amount: Decimal,
    pub expected_profit: Decimal,
    pub profitable: bool,
    pub reason: String,
}