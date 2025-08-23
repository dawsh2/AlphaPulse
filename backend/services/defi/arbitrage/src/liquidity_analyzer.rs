// Real Liquidity Analyzer - Fetches ACTUAL Pool Liquidity from Blockchain
// CRITICAL: Accurate liquidity is essential for slippage calculations

use anyhow::{Result, anyhow, Context};
use ethers::prelude::*;
use std::sync::Arc;
use std::collections::HashMap;
use tracing::{debug, info, warn, error};
use serde::{Serialize, Deserialize};

use crate::dex_integration::{RealDexIntegration, DexType, PoolData};
use crate::price_oracle::LivePriceOracle;
use crate::secure_registries::SecureRegistryManager;

/// Real liquidity data from blockchain with USD values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidityData {
    pub dex_type: DexType,
    pub pair_address: Address,
    pub token0: Address,
    pub token1: Address,
    pub reserve0_raw: U256,
    pub reserve1_raw: U256,
    pub reserve0_usd: f64,
    pub reserve1_usd: f64,
    pub total_liquidity_usd: f64,
    pub token0_decimals: u8,
    pub token1_decimals: u8,
    pub last_update: u64,
    pub price_token0_usd: f64,
    pub price_token1_usd: f64,
}

/// Aggregated liquidity across all DEXs for a token pair
#[derive(Debug, Clone)]
pub struct AggregatedLiquidity {
    pub token_a: Address,
    pub token_b: Address,
    pub total_liquidity_usd: f64,
    pub best_dex: DexType,
    pub dex_breakdown: HashMap<DexType, f64>,
    pub deepest_pool: Address,
    pub price_impact_1k_usd: f64,  // Price impact for $1K trade
    pub price_impact_10k_usd: f64, // Price impact for $10K trade
    pub price_impact_100k_usd: f64, // Price impact for $100K trade
    pub last_update: u64,  // Add missing timestamp field
}

/// Production liquidity analyzer
pub struct LiquidityAnalyzer {
    dex_integration: Arc<tokio::sync::RwLock<RealDexIntegration>>,
    price_oracle: Arc<tokio::sync::RwLock<LivePriceOracle>>,
    secure_registry: Arc<SecureRegistryManager>,
    liquidity_cache: HashMap<String, LiquidityData>,
    aggregated_cache: HashMap<String, AggregatedLiquidity>,
    chain_id: u64,
}

// ERC20 ABI for getting token decimals and prices
abigen!(
    IERC20Metadata,
    r#"[
        function decimals() external view returns (uint8)
        function symbol() external view returns (string memory)
        function totalSupply() external view returns (uint256)
    ]"#
);

impl LiquidityAnalyzer {
    pub fn new(
        dex_integration: Arc<tokio::sync::RwLock<RealDexIntegration>>,
        price_oracle: Arc<tokio::sync::RwLock<LivePriceOracle>>,
        secure_registry: Arc<SecureRegistryManager>,
        chain_id: u64,
    ) -> Self {
        Self {
            dex_integration,
            price_oracle,
            secure_registry,
            liquidity_cache: HashMap::new(),
            aggregated_cache: HashMap::new(),
            chain_id,
        }
    }

    /// Get REAL liquidity data for a token pair from specific DEX
    pub async fn get_real_liquidity(
        &mut self,
        token0: Address,
        token1: Address,
        dex_type: DexType,
    ) -> Result<LiquidityData> {
        let cache_key = format!("{:?}-{:?}-{:?}", token0, token1, dex_type);
        
        // Check cache (30 second TTL for liquidity data)
        if let Some(cached) = self.liquidity_cache.get(&cache_key) {
            let age = current_timestamp() - cached.last_update;
            if age < 30 {
                debug!("Using cached liquidity for {}", cache_key);
                return Ok(cached.clone());
            }
        }

        // Get fresh pool data
        let mut dex_integration = self.dex_integration.write().await;
        let pool_data = dex_integration.get_real_pool_reserves(token0, token1, dex_type).await?;
        
        // Get token metadata for accurate decimal handling
        let (decimals0, decimals1) = self.get_token_decimals(token0, token1).await?;
        
        // Get real USD prices for accurate liquidity calculation
        let (price0_usd, price1_usd) = self.get_token_prices_usd(token0, token1).await?;
        
        // Calculate actual liquidity in USD with proper decimals
        let reserve0_tokens = pool_data.reserve0.as_u128() as f64 / 10_f64.powi(decimals0 as i32);
        let reserve1_tokens = pool_data.reserve1.as_u128() as f64 / 10_f64.powi(decimals1 as i32);
        
        let reserve0_usd = reserve0_tokens * price0_usd;
        let reserve1_usd = reserve1_tokens * price1_usd;
        let total_liquidity_usd = reserve0_usd + reserve1_usd;

        let liquidity_data = LiquidityData {
            dex_type,
            pair_address: pool_data.pair_address,
            token0,
            token1,
            reserve0_raw: pool_data.reserve0,
            reserve1_raw: pool_data.reserve1,
            reserve0_usd,
            reserve1_usd,
            total_liquidity_usd,
            token0_decimals: decimals0,
            token1_decimals: decimals1,
            last_update: current_timestamp(),
            price_token0_usd: price0_usd,
            price_token1_usd: price1_usd,
        };

        info!("Real liquidity for {:?}/{:?} on {:?}: ${:.0} (R0: {:.0} ${:.0}, R1: {:.0} ${:.0})",
              token0, token1, dex_type, total_liquidity_usd,
              reserve0_tokens, reserve0_usd, reserve1_tokens, reserve1_usd);

        // Cache the result
        self.liquidity_cache.insert(cache_key, liquidity_data.clone());
        
        Ok(liquidity_data)
    }

    /// Get aggregated liquidity across ALL DEXs for a token pair
    pub async fn get_aggregated_liquidity(
        &mut self,
        token_a: Address,
        token_b: Address,
    ) -> Result<AggregatedLiquidity> {
        let cache_key = format!("{:?}-{:?}", token_a, token_b);
        
        // Check cache (60 second TTL for aggregated data)
        if let Some(cached) = self.aggregated_cache.get(&cache_key) {
            let age = current_timestamp() - cached.last_update;
            if age < 60 {
                return Ok(cached.clone());
            }
        }

        let dex_types = vec![
            DexType::UniswapV2,
            DexType::UniswapV3,
            DexType::SushiSwap,
            DexType::QuickSwap,
        ];

        let mut total_liquidity_usd = 0.0;
        let mut dex_breakdown = HashMap::new();
        let mut best_dex = DexType::UniswapV2;
        let mut best_liquidity = 0.0;
        let mut deepest_pool = Address::zero();

        // Check both token orderings
        for &(token0, token1) in &[(token_a, token_b), (token_b, token_a)] {
            for dex_type in &dex_types {
                match self.get_real_liquidity(token0, token1, *dex_type).await {
                    Ok(liquidity_data) => {
                        total_liquidity_usd += liquidity_data.total_liquidity_usd;
                        dex_breakdown.insert(*dex_type, liquidity_data.total_liquidity_usd);
                        
                        if liquidity_data.total_liquidity_usd > best_liquidity {
                            best_liquidity = liquidity_data.total_liquidity_usd;
                            best_dex = *dex_type;
                            deepest_pool = liquidity_data.pair_address;
                        }
                    }
                    Err(e) => {
                        debug!("No liquidity found for {:?}/{:?} on {:?}: {}", token0, token1, dex_type, e);
                    }
                }
            }
        }

        if total_liquidity_usd == 0.0 {
            return Err(anyhow!("No liquidity found for token pair {:?}/{:?}", token_a, token_b));
        }

        // Calculate price impact at different trade sizes using best DEX
        let (impact_1k, impact_10k, impact_100k) = 
            self.calculate_price_impact_levels(token_a, token_b, best_dex).await?;

        let aggregated = AggregatedLiquidity {
            token_a,
            token_b,
            total_liquidity_usd,
            best_dex,
            dex_breakdown,
            deepest_pool,
            price_impact_1k_usd: impact_1k,
            price_impact_10k_usd: impact_10k,
            price_impact_100k_usd: impact_100k,
            last_update: current_timestamp(),
        };

        info!("Aggregated liquidity for {:?}/{:?}: ${:.0} total, best DEX: {:?} (${:.0})",
              token_a, token_b, total_liquidity_usd, best_dex, best_liquidity);

        // Cache result
        self.aggregated_cache.insert(cache_key, aggregated.clone());
        
        Ok(aggregated)
    }

    /// Calculate maximum trade size for given price impact threshold
    pub async fn max_trade_size_for_impact(
        &mut self,
        token_a: Address,
        token_b: Address,
        max_impact_pct: f64,
    ) -> Result<U256> {
        let aggregated = self.get_aggregated_liquidity(token_a, token_b).await?;
        
        // Use the deepest pool for calculation
        let liquidity_data = self.get_real_liquidity(token_a, token_b, aggregated.best_dex).await?;
        
        // Use AMM math to find max trade size
        crate::amm_math::UniswapV2Math::max_trade_for_impact(
            max_impact_pct,
            liquidity_data.reserve0_raw,
            liquidity_data.reserve1_raw,
        )
    }

    /// Analyze liquidity depth at multiple levels
    pub async fn analyze_liquidity_depth(
        &mut self,
        token_a: Address,
        token_b: Address,
    ) -> Result<LiquidityDepthAnalysis> {
        let aggregated = self.get_aggregated_liquidity(token_a, token_b).await?;
        
        // Calculate how much can be traded at different slippage levels
        let max_1_pct = self.max_trade_size_for_impact(token_a, token_b, 1.0).await?;
        let max_2_pct = self.max_trade_size_for_impact(token_a, token_b, 2.0).await?;
        let max_5_pct = self.max_trade_size_for_impact(token_a, token_b, 5.0).await?;
        
        // Convert to USD values
        let price_a = aggregated.dex_breakdown.values().next().unwrap_or(&1.0);
        let max_1_pct_usd = max_1_pct.as_u128() as f64 / 1e18 * price_a;
        let max_2_pct_usd = max_2_pct.as_u128() as f64 / 1e18 * price_a;
        let max_5_pct_usd = max_5_pct.as_u128() as f64 / 1e18 * price_a;

        Ok(LiquidityDepthAnalysis {
            total_liquidity_usd: aggregated.total_liquidity_usd,
            max_trade_1_pct_impact: max_1_pct_usd,
            max_trade_2_pct_impact: max_2_pct_usd,
            max_trade_5_pct_impact: max_5_pct_usd,
            liquidity_rating: self.calculate_liquidity_rating(aggregated.total_liquidity_usd),
            recommended_max_trade_usd: max_1_pct_usd * 0.8, // 80% of 1% impact for safety
        })
    }

    // Helper methods

    async fn get_token_decimals(&self, token0: Address, token1: Address) -> Result<(u8, u8)> {
        // Use SECURE registry for token decimals - NO hardcoded values!
        let token0_info = self.secure_registry.get_secure_token_info(token0).await?;
        let token1_info = self.secure_registry.get_secure_token_info(token1).await?;
        
        debug!("Token decimals from SECURE registry: {:?}={}, {:?}={}", 
               token0, token0_info.decimals, token1, token1_info.decimals);
               
        Ok((token0_info.decimals, token1_info.decimals))
    }

    async fn get_token_prices_usd(&self, token0: Address, token1: Address) -> Result<(f64, f64)> {
        let oracle = self.price_oracle.read().await;
        
        // NO MORE HARDCODED ADDRESSES!
        
        // Get REAL prices from live oracle - NO STABLECOIN ASSUMPTIONS!
        let price0 = oracle.get_token_price_usd(token0).await
            .context(format!("Failed to get real price for token0 {:?}", token0))?;
        
        let price1 = oracle.get_token_price_usd(token1).await
            .context(format!("Failed to get real price for token1 {:?}", token1))?;
        
        debug!("Real prices: token0={:?} @ ${:.4}, token1={:?} @ ${:.4}", token0, price0, token1, price1);
        Ok((price0, price1))
    }

    async fn calculate_price_impact_levels(
        &mut self,
        token_a: Address,
        token_b: Address,
        dex_type: DexType,
    ) -> Result<(f64, f64, f64)> {
        let liquidity_data = self.get_real_liquidity(token_a, token_b, dex_type).await?;
        
        // Calculate impact for $1K, $10K, $100K trades
        let amount_1k = U256::from(1000) * U256::exp10(18) / U256::from(liquidity_data.price_token0_usd as u64);
        let amount_10k = amount_1k * 10;
        let amount_100k = amount_1k * 100;
        
        let impact_1k = crate::amm_math::UniswapV2Math::calculate_price_impact(
            amount_1k, liquidity_data.reserve0_raw, liquidity_data.reserve1_raw
        ).unwrap_or(100.0);
        
        let impact_10k = crate::amm_math::UniswapV2Math::calculate_price_impact(
            amount_10k, liquidity_data.reserve0_raw, liquidity_data.reserve1_raw
        ).unwrap_or(100.0);
        
        let impact_100k = crate::amm_math::UniswapV2Math::calculate_price_impact(
            amount_100k, liquidity_data.reserve0_raw, liquidity_data.reserve1_raw
        ).unwrap_or(100.0);
        
        Ok((impact_1k, impact_10k, impact_100k))
    }

    fn calculate_liquidity_rating(&self, total_liquidity_usd: f64) -> LiquidityRating {
        match total_liquidity_usd {
            x if x >= 10_000_000.0 => LiquidityRating::Excellent, // $10M+
            x if x >= 1_000_000.0 => LiquidityRating::Good,       // $1M+
            x if x >= 100_000.0 => LiquidityRating::Fair,         // $100K+
            x if x >= 10_000.0 => LiquidityRating::Poor,          // $10K+
            _ => LiquidityRating::VeryPoor,                       // <$10K
        }
    }
}

#[derive(Debug, Clone)]
pub struct LiquidityDepthAnalysis {
    pub total_liquidity_usd: f64,
    pub max_trade_1_pct_impact: f64,
    pub max_trade_2_pct_impact: f64,
    pub max_trade_5_pct_impact: f64,
    pub liquidity_rating: LiquidityRating,
    pub recommended_max_trade_usd: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LiquidityRating {
    Excellent, // $10M+ liquidity
    Good,      // $1M+ liquidity
    Fair,      // $100K+ liquidity
    Poor,      // $10K+ liquidity
    VeryPoor,  // <$10K liquidity
}

fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_liquidity_rating() {
        let analyzer = LiquidityAnalyzer::new(
            Arc::new(tokio::sync::RwLock::new(todo!())),
            Arc::new(tokio::sync::RwLock::new(todo!())),
            137,
        );
        
        assert_eq!(analyzer.calculate_liquidity_rating(15_000_000.0), LiquidityRating::Excellent);
        assert_eq!(analyzer.calculate_liquidity_rating(500_000.0), LiquidityRating::Fair);
        assert_eq!(analyzer.calculate_liquidity_rating(5_000.0), LiquidityRating::VeryPoor);
    }
}