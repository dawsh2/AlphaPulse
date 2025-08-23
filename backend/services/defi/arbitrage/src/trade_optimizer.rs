// Trade Size Optimizer - Maximizes Profit While Respecting Slippage Constraints
// Uses AMM math to find optimal trade sizes for arbitrage opportunities

use anyhow::{Result, anyhow, Context};
use ethers::prelude::*;
use rust_decimal::Decimal;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::amm_math::{UniswapV2Math, MultiHopSlippage};
use crate::dex_integration::{RealDexIntegration, DexType};
use crate::multi_hop_validator::MultiHopValidator;
use crate::liquidity_analyzer::LiquidityAnalyzer;
use crate::config::ArbitrageConfig;
use crate::oracle::PriceOracle;

/// Trade optimization result with detailed analysis
#[derive(Debug, Clone)]
pub struct OptimizationResult {
    pub optimal_amount: U256,
    pub expected_profit_usd: f64,
    pub slippage_impact: f64,
    pub profit_ratio: f64,
    pub gas_cost_usd: f64,
    pub liquidity_limited: bool,
    pub optimization_method: String,
}

/// Advanced trade size optimizer using AMM mathematics
pub struct TradeOptimizer {
    dex_integration: Arc<RwLock<RealDexIntegration>>,
    multi_hop_validator: Arc<MultiHopValidator>,
    liquidity_analyzer: Arc<tokio::sync::RwLock<LiquidityAnalyzer>>,
    config: Arc<ArbitrageConfig>,
    oracle: Arc<PriceOracle>,
}

impl TradeOptimizer {
    pub fn new(
        dex_integration: Arc<RwLock<RealDexIntegration>>,
        multi_hop_validator: Arc<MultiHopValidator>,
        liquidity_analyzer: Arc<tokio::sync::RwLock<LiquidityAnalyzer>>,
        config: Arc<ArbitrageConfig>,
        oracle: Arc<PriceOracle>,
    ) -> Self {
        Self {
            dex_integration,
            multi_hop_validator,
            liquidity_analyzer,
            config,
            oracle,
        }
    }

    /// Find optimal trade size for maximum profit within slippage constraints
    pub async fn optimize_trade_size(
        &self,
        path: Vec<Address>,
        min_amount: U256,
        max_amount: U256,
        max_slippage_pct: f64,
        target_profit_usd: Option<f64>,
    ) -> Result<OptimizationResult> {
        info!("Optimizing trade size for {}-hop path with max {:.2}% slippage", 
              path.len() - 1, max_slippage_pct);

        // Use closed-form solution to find optimal size
        let optimal_amount = self.multi_hop_validator
            .optimize_trade_size(path.clone(), max_slippage_pct, max_amount)
            .await?;

        if optimal_amount < min_amount {
            return Ok(OptimizationResult {
                optimal_amount: U256::zero(),
                expected_profit_usd: 0.0,
                slippage_impact: 100.0,
                profit_ratio: 0.0,
                gas_cost_usd: 0.0,
                liquidity_limited: true,
                optimization_method: "insufficient_liquidity".to_string(),
            });
        }

        // Calculate expected profit and costs
        let (final_amount, slippage_impact) = self.multi_hop_validator
            .verify_path_with_amm_math(path.clone(), optimal_amount)
            .await?;

        // Estimate gas cost (simplified)
        let estimated_gas = 150_000u64 * (path.len() - 1) as u64; // Base gas per hop
        let gas_cost_usd = self.estimate_gas_cost_usd(estimated_gas).await?;

        // Calculate profit
        let amount_in_usd = self.estimate_token_value_usd(path[0], optimal_amount).await?;
        let amount_out_usd = self.estimate_token_value_usd(path[path.len() - 1], final_amount).await?;
        let gross_profit = amount_out_usd - amount_in_usd;
        let net_profit = gross_profit - gas_cost_usd;

        let profit_ratio = if amount_in_usd > 0.0 { net_profit / amount_in_usd } else { 0.0 };

        info!("Optimization result: amount {} -> {}, profit ${:.2}, slippage {:.2}%",
              optimal_amount, final_amount, net_profit, slippage_impact);

        Ok(OptimizationResult {
            optimal_amount,
            expected_profit_usd: net_profit,
            slippage_impact,
            profit_ratio,
            gas_cost_usd,
            liquidity_limited: false,
            optimization_method: "amm_binary_search".to_string(),
        })
    }

    /// Optimize for compound arbitrage (10+ token paths)
    pub async fn optimize_compound_arbitrage(
        &self,
        path: Vec<Address>,
        available_capital: U256,
    ) -> Result<OptimizationResult> {
        if path.len() < 10 {
            return Err(anyhow!("Compound arbitrage requires 10+ tokens, got {}", path.len()));
        }

        info!("Optimizing compound arbitrage with {} tokens", path.len());

        // Use higher slippage tolerance for compound paths
        let max_slippage = self.config.compound_strategy.max_slippage * 100.0; // Convert to percentage
        
        // Split capital into smaller chunks for compound paths to reduce slippage
        let chunk_size = available_capital / 4; // Use 25% of capital to minimize impact
        
        self.optimize_trade_size(
            path,
            U256::from(1000) * U256::exp10(18), // Min 1K tokens
            chunk_size,
            max_slippage,
            Some(self.config.compound_strategy.min_profit_usd),
        ).await
    }

    /// Dynamic sizing based on market conditions
    pub async fn dynamic_size_optimization(
        &self,
        path: Vec<Address>,
        market_volatility: f64,
        mev_competition_level: f64,
    ) -> Result<OptimizationResult> {
        // Adjust slippage tolerance based on market conditions
        let base_slippage = self.config.max_slippage_percentage * 100.0;
        let volatility_adjustment = 1.0 + (market_volatility * 0.5); // Higher volatility = more slippage tolerance
        let competition_adjustment = 1.0 - (mev_competition_level * 0.3); // Higher competition = less slippage tolerance
        
        let adjusted_slippage = base_slippage * volatility_adjustment * competition_adjustment;
        let max_slippage = adjusted_slippage.min(10.0).max(0.1); // Cap between 0.1% and 10%

        // Adjust position size based on competition
        let base_capital = U256::from(self.config.position_size_usd as u64) * U256::exp10(18);
        let competition_size_factor = 1.0 - (mev_competition_level * 0.5);
        let adjusted_capital = base_capital * U256::from((competition_size_factor * 100.0) as u64) / U256::from(100);

        info!("Dynamic optimization: {:.2}% slippage tolerance, competition factor {:.2}",
              max_slippage, competition_size_factor);

        self.optimize_trade_size(
            path,
            U256::from(100) * U256::exp10(18), // Min 100 tokens
            adjusted_capital,
            max_slippage,
            None,
        ).await
    }

    /// Batch optimization for multiple opportunities
    pub async fn optimize_batch_trades(
        &self,
        opportunities: Vec<(Vec<Address>, U256)>, // (path, max_amount)
        total_capital: U256,
    ) -> Result<Vec<OptimizationResult>> {
        let mut results = Vec::new();
        let capital_per_trade = total_capital / opportunities.len();

        for (path, max_amount) in opportunities {
            let trade_capital = capital_per_trade.min(max_amount);
            
            match self.optimize_trade_size(
                path.clone(),
                U256::from(100) * U256::exp10(18),
                trade_capital,
                self.config.max_slippage_percentage * 100.0,
                None,
            ).await {
                Ok(result) => results.push(result),
                Err(e) => {
                    warn!("Failed to optimize trade for path {:?}: {}", path, e);
                    results.push(OptimizationResult {
                        optimal_amount: U256::zero(),
                        expected_profit_usd: 0.0,
                        slippage_impact: 100.0,
                        profit_ratio: 0.0,
                        gas_cost_usd: 0.0,
                        liquidity_limited: true,
                        optimization_method: "failed".to_string(),
                    });
                }
            }
        }

        // Sort by profit ratio descending
        results.sort_by(|a, b| b.profit_ratio.partial_cmp(&a.profit_ratio).unwrap_or(std::cmp::Ordering::Equal));
        
        info!("Batch optimization complete: {} trades optimized", results.len());
        Ok(results)
    }

    /// Liquidity-aware optimization using real pool data
    pub async fn optimize_with_liquidity_analysis(
        &self,
        path: Vec<Address>,
        max_slippage_pct: f64,
    ) -> Result<OptimizationResult> {
        if path.len() < 2 {
            return Err(anyhow!("Path must have at least 2 tokens"));
        }

        // For each hop, analyze real liquidity depth
        let mut total_available_liquidity = f64::MAX;
        let mut liquidity_warnings = Vec::new();

        for i in 0..path.len() - 1 {
            let token_a = path[i];
            let token_b = path[i + 1];

            // Get real liquidity analysis
            let mut analyzer = self.liquidity_analyzer.write().await;
            match analyzer.analyze_liquidity_depth(token_a, token_b).await {
                Ok(depth_analysis) => {
                    info!("Liquidity analysis for {:?}/{:?}: ${:.0} total, max trade @ 1%: ${:.0}",
                          token_a, token_b, depth_analysis.total_liquidity_usd, depth_analysis.max_trade_1_pct_impact);
                    
                    // Use the most restrictive liquidity constraint
                    total_available_liquidity = total_available_liquidity.min(depth_analysis.recommended_max_trade_usd);
                    
                    if depth_analysis.total_liquidity_usd < 100_000.0 {
                        liquidity_warnings.push(format!("Low liquidity pool: ${:.0}", depth_analysis.total_liquidity_usd));
                    }
                }
                Err(e) => {
                    warn!("Failed to analyze liquidity for {:?}/{:?}: {}", token_a, token_b, e);
                    // Use conservative fallback
                    total_available_liquidity = total_available_liquidity.min(10_000.0); // $10K max
                    liquidity_warnings.push("Using conservative liquidity fallback".to_string());
                }
            }
        }

        // Convert USD liquidity limit to token amount
        let max_trade_usd = total_available_liquidity;
        let max_amount_tokens = U256::from((max_trade_usd * 1e18) as u64); // Simplified conversion

        info!("Liquidity-constrained optimization: max trade size ${:.0} ({})",
              max_trade_usd, max_amount_tokens);

        // Use regular optimization with liquidity constraints
        let mut result = self.optimize_trade_size(
            path,
            U256::from(1000) * U256::exp10(18), // Min 1K tokens
            max_amount_tokens,
            max_slippage_pct,
            None,
        ).await?;

        // Add liquidity analysis info
        result.optimization_method = format!("liquidity_aware_{}", result.optimization_method);
        if !liquidity_warnings.is_empty() {
            result.optimization_method = format!("{}_with_warnings", result.optimization_method);
        }

        Ok(result)
    }

    /// Helper: Estimate gas cost in USD using live oracle
    async fn estimate_gas_cost_usd(&self, gas_units: u64) -> Result<f64> {
        // Get live gas price
        let gas_price = self.dex_integration.read().await
            .get_gas_price().await?;
        
        // Calculate gas cost in USD using oracle
        let gas_cost_usd = self.oracle.calculate_gas_cost_usd(gas_units, gas_price).await
            .context("Failed to calculate gas cost via oracle")?;
        
        debug!("⛽ Gas cost: {} units @ {} Gwei = ${:.4}", 
               gas_units, gas_price.as_u128() as f64 / 1e9, gas_cost_usd);
        
        Ok(gas_cost_usd)
    }

    /// Helper: Estimate token value in USD using live oracle
    async fn estimate_token_value_usd(&self, token: Address, amount: U256) -> Result<f64> {
        let token_amount = amount.as_u128() as f64 / 1e18;
        
        // Get live price from oracle
        let price_data = self.oracle.get_price(token).await
            .context("Failed to get token price from oracle")?;
        
        let value_usd = token_amount * price_data.price_usd;
        
        debug!("💎 Token value: {:.6} tokens @ ${:.6} = ${:.4} (source: {:?}, confidence: {:.1}%)", 
               token_amount, price_data.price_usd, value_usd, price_data.source, price_data.confidence * 100.0);
        
        Ok(value_usd)
    }
}

/// Strategy-specific optimization parameters
#[derive(Debug, Clone)]
pub struct OptimizationStrategy {
    pub max_slippage_pct: f64,
    pub min_profit_ratio: f64,
    pub capital_utilization: f64, // 0.0 to 1.0
    pub gas_buffer_multiplier: f64,
}

impl OptimizationStrategy {
    pub fn conservative() -> Self {
        Self {
            max_slippage_pct: 0.5,
            min_profit_ratio: 0.02, // 2% minimum profit
            capital_utilization: 0.25, // Use 25% of available capital
            gas_buffer_multiplier: 1.5,
        }
    }

    pub fn aggressive() -> Self {
        Self {
            max_slippage_pct: 2.0,
            min_profit_ratio: 0.01, // 1% minimum profit
            capital_utilization: 0.75, // Use 75% of available capital
            gas_buffer_multiplier: 1.2,
        }
    }

    pub fn compound() -> Self {
        Self {
            max_slippage_pct: 3.0, // Higher tolerance for complex paths
            min_profit_ratio: 0.05, // 5% minimum for complexity
            capital_utilization: 0.5, // Moderate capital usage
            gas_buffer_multiplier: 2.0, // Higher gas buffer for complex transactions
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_optimization_strategy_selection() {
        let conservative = OptimizationStrategy::conservative();
        let aggressive = OptimizationStrategy::aggressive();
        let compound = OptimizationStrategy::compound();

        assert!(conservative.max_slippage_pct < aggressive.max_slippage_pct);
        assert!(conservative.capital_utilization < aggressive.capital_utilization);
        assert!(compound.max_slippage_pct > conservative.max_slippage_pct);
        assert!(compound.min_profit_ratio > aggressive.min_profit_ratio);
    }
}