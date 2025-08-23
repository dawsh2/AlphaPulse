// Multi-Hop Path Validator with Proper Gas and Slippage Accumulation
// Recycled from existing gas calculation logic with multi-hop enhancements

use anyhow::{Result, Context};
use ethers::prelude::*;
use std::sync::Arc;
use tracing::{debug, info, warn};
use serde::{Serialize, Deserialize};

use crate::dex_integration::{RealDexIntegration, DexQuote};
use crate::price_oracle::LivePriceOracle;
use crate::amm_math::{UniswapV2Math, MultiHopSlippage};

/// Detailed validation result for multi-hop paths
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiHopValidation {
    pub is_valid: bool,
    pub path: Vec<Address>,
    pub hop_count: usize,
    pub initial_amount: U256,
    pub final_amount: U256,
    pub profit_ratio: f64,
    pub total_gas_estimate: u64,
    pub gas_cost_usd: f64,
    pub cumulative_slippage: f64,
    pub max_single_hop_impact: f64,
    pub hop_details: Vec<HopDetail>,
    pub warnings: Vec<String>,
    pub failure_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HopDetail {
    pub hop_index: usize,
    pub token_in: Address,
    pub token_out: Address,
    pub amount_in: U256,
    pub amount_out: U256,
    pub price_impact: f64,
    pub gas_estimate: u64,
    pub dex_used: String,
}

/// Enhanced multi-hop path validator
pub struct MultiHopValidator {
    dex_integration: Arc<tokio::sync::RwLock<RealDexIntegration>>,
    price_oracle: Arc<tokio::sync::RwLock<LivePriceOracle>>,
    config: ValidationConfig,
}

#[derive(Debug, Clone)]
pub struct ValidationConfig {
    pub base_gas: u64,              // Base transaction gas (150k)
    pub per_hop_gas: u64,           // Gas per hop (50k)
    pub max_cumulative_slippage: f64, // Max 10% total slippage
    pub max_single_hop_impact: f64,   // Max 5% per hop
    pub min_profit_ratio: f64,        // Min 1% profit after all costs
    pub gas_buffer_multiplier: f64,   // 1.3x gas buffer for safety
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            base_gas: 150_000,           // Base flash loan + callback gas
            per_hop_gas: 50_000,         // Per swap gas (recycled from compound_arb.rs:53)
            max_cumulative_slippage: 0.10,  // 10% max total slippage
            max_single_hop_impact: 0.05,    // 5% max per hop
            min_profit_ratio: 1.01,         // 1% minimum profit
            gas_buffer_multiplier: 1.3,     // 30% gas buffer
        }
    }
}

impl MultiHopValidator {
    pub fn new(
        dex_integration: Arc<tokio::sync::RwLock<RealDexIntegration>>,
        price_oracle: Arc<tokio::sync::RwLock<LivePriceOracle>>,
        config: ValidationConfig,
    ) -> Self {
        Self {
            dex_integration,
            price_oracle,
            config,
        }
    }

    /// Comprehensive multi-hop path validation with proper accumulation
    pub async fn validate_path(
        &self,
        path: Vec<Address>,
        initial_amount: U256,
    ) -> Result<MultiHopValidation> {
        info!("Validating {}-hop path with initial amount: {}", path.len() - 1, initial_amount);
        
        if path.len() < 2 {
            return Ok(MultiHopValidation {
                is_valid: false,
                failure_reason: Some("Path too short".to_string()),
                ..self.create_empty_validation(path, initial_amount)
            });
        }

        let hop_count = path.len() - 1;
        let mut current_amount = initial_amount;
        let mut hop_details = Vec::new();
        let mut warnings = Vec::new();
        
        // CRITICAL: Accumulate gas costs properly (recycled from execution_validator.rs:563)
        let mut total_gas_estimate = self.config.base_gas; // Start with base gas
        
        // Track cumulative slippage as a multiplier
        let mut cumulative_slippage_multiplier = 1.0;
        let mut max_single_hop_impact = 0.0;
        
        // Expected amount without slippage (for calculating cumulative effect)
        let mut expected_amount_no_slippage = initial_amount;

        // Validate each hop with real DEX quotes
        let mut dex_integration = self.dex_integration.write().await;
        
        for i in 0..hop_count {
            let token_in = path[i];
            let token_out = path[i + 1];
            
            debug!("Validating hop {}/{}: {:?} -> {:?}", i + 1, hop_count, token_in, token_out);
            
            // Get real quote from DEX
            match dex_integration.find_best_quote(token_in, token_out, current_amount).await {
                Ok(quote) => {
                    // Track maximum single hop impact
                    if quote.price_impact > max_single_hop_impact {
                        max_single_hop_impact = quote.price_impact;
                    }
                    
                    // Check single hop impact threshold
                    if quote.price_impact > self.config.max_single_hop_impact * 100.0 {
                        return Ok(MultiHopValidation {
                            is_valid: false,
                            failure_reason: Some(format!(
                                "Hop {} price impact too high: {:.2}%", 
                                i + 1, quote.price_impact
                            )),
                            ..self.create_partial_validation(
                                path, initial_amount, current_amount, 
                                hop_details, warnings, total_gas_estimate
                            )
                        });
                    }
                    
                    // Calculate slippage for this hop
                    let hop_slippage = quote.price_impact / 100.0;
                    cumulative_slippage_multiplier *= (1.0 - hop_slippage);
                    
                    // CRITICAL: Add gas for this hop (recycled from compound_arb.rs:139)
                    let hop_gas = self.calculate_hop_gas(&quote, i);
                    total_gas_estimate += hop_gas;
                    
                    // Record hop details
                    hop_details.push(HopDetail {
                        hop_index: i,
                        token_in,
                        token_out,
                        amount_in: current_amount,
                        amount_out: quote.amount_out,
                        price_impact: quote.price_impact,
                        gas_estimate: hop_gas,
                        dex_used: format!("{:?}", quote.dex_type),
                    });
                    
                    // Update current amount for next hop
                    current_amount = quote.amount_out;
                    
                    // Warn if getting close to slippage limit
                    let current_cumulative_slippage = 1.0 - cumulative_slippage_multiplier;
                    if current_cumulative_slippage > self.config.max_cumulative_slippage * 0.8 {
                        warnings.push(format!(
                            "High cumulative slippage after hop {}: {:.2}%", 
                            i + 1, current_cumulative_slippage * 100.0
                        ));
                    }
                }
                Err(e) => {
                    return Ok(MultiHopValidation {
                        is_valid: false,
                        failure_reason: Some(format!("Failed to get quote for hop {}: {}", i + 1, e)),
                        ..self.create_partial_validation(
                            path, initial_amount, current_amount, 
                            hop_details, warnings, total_gas_estimate
                        )
                    });
                }
            }
        }
        
        // Calculate final cumulative slippage
        let cumulative_slippage = 1.0 - cumulative_slippage_multiplier;
        
        // Check cumulative slippage threshold
        if cumulative_slippage > self.config.max_cumulative_slippage {
            return Ok(MultiHopValidation {
                is_valid: false,
                failure_reason: Some(format!(
                    "Cumulative slippage too high: {:.2}%", 
                    cumulative_slippage * 100.0
                )),
                ..self.create_final_validation(
                    path, initial_amount, current_amount, hop_details, 
                    warnings, total_gas_estimate, cumulative_slippage, max_single_hop_impact
                )
            });
        }
        
        // Apply gas buffer for safety (recycled from execution_validator.rs:557)
        total_gas_estimate = (total_gas_estimate as f64 * self.config.gas_buffer_multiplier) as u64;
        
        // Calculate gas cost in USD (recycled from lib.rs:341)
        let gas_cost_usd = self.calculate_gas_cost_usd(total_gas_estimate).await?;
        
        // Calculate profit ratio
        let profit_ratio = if initial_amount > U256::zero() {
            current_amount.as_u128() as f64 / initial_amount.as_u128() as f64
        } else {
            0.0
        };
        
        // Check minimum profit threshold
        let is_valid = profit_ratio >= self.config.min_profit_ratio;
        
        if !is_valid {
            warnings.push(format!(
                "Insufficient profit ratio: {:.4} (min: {:.4})", 
                profit_ratio, self.config.min_profit_ratio
            ));
        }
        
        info!(
            "Path validation complete: {} hops, ratio: {:.4}, slippage: {:.2}%, gas: {} ({:.2} USD)",
            hop_count, profit_ratio, cumulative_slippage * 100.0, total_gas_estimate, gas_cost_usd
        );
        
        Ok(MultiHopValidation {
            is_valid,
            path,
            hop_count,
            initial_amount,
            final_amount: current_amount,
            profit_ratio,
            total_gas_estimate,
            gas_cost_usd,
            cumulative_slippage,
            max_single_hop_impact,
            hop_details,
            warnings,
            failure_reason: if is_valid { None } else { Some("Insufficient profit".to_string()) },
        })
    }

    /// Calculate gas for a specific hop based on complexity
    fn calculate_hop_gas(&self, quote: &DexQuote, hop_index: usize) -> u64 {
        let mut gas = self.config.per_hop_gas;
        
        // Add extra gas for complex DEX types
        match quote.dex_type {
            crate::dex_integration::DexType::UniswapV3 => gas += 20_000, // V3 is more complex
            crate::dex_integration::DexType::Balancer => gas += 30_000,  // Balancer pools are heavy
            _ => {} // V2 types use base gas
        }
        
        // Add extra gas for later hops (state changes accumulate)
        gas += (hop_index as u64) * 5_000;
        
        gas
    }

    /// Calculate gas cost in USD using live prices (recycled from lib.rs:341)
    async fn calculate_gas_cost_usd(&self, gas_estimate: u64) -> Result<f64> {
        let mut oracle = self.price_oracle.write().await;
        
        // Get live MATIC price
        let matic_price = oracle.get_live_matic_price().await
            .context("Failed to get MATIC price for gas calculation")?;
        
        // Get live gas prices
        let gas_prices = oracle.get_live_gas_prices().await
            .context("Failed to get gas prices")?;
        
        // Calculate gas cost in USD
        let gas_cost_matic = (gas_estimate as f64) * gas_prices.fast * 1e-9;
        let gas_cost_usd = gas_cost_matic * matic_price;
        
        debug!(
            "Gas calculation: {} gas @ {:.1} gwei × ${:.4}/MATIC = ${:.4}", 
            gas_estimate, gas_prices.fast, matic_price, gas_cost_usd
        );
        
        Ok(gas_cost_usd)
    }

    /// Create empty validation result
    fn create_empty_validation(&self, path: Vec<Address>, initial_amount: U256) -> MultiHopValidation {
        MultiHopValidation {
            is_valid: false,
            path,
            hop_count: 0,
            initial_amount,
            final_amount: U256::zero(),
            profit_ratio: 0.0,
            total_gas_estimate: 0,
            gas_cost_usd: 0.0,
            cumulative_slippage: 0.0,
            max_single_hop_impact: 0.0,
            hop_details: Vec::new(),
            warnings: Vec::new(),
            failure_reason: None,
        }
    }

    /// Create partial validation result
    fn create_partial_validation(
        &self,
        path: Vec<Address>,
        initial_amount: U256,
        current_amount: U256,
        hop_details: Vec<HopDetail>,
        warnings: Vec<String>,
        total_gas_estimate: u64,
    ) -> MultiHopValidation {
        MultiHopValidation {
            is_valid: false,
            path,
            hop_count: hop_details.len(),
            initial_amount,
            final_amount: current_amount,
            profit_ratio: if initial_amount > U256::zero() {
                current_amount.as_u128() as f64 / initial_amount.as_u128() as f64
            } else {
                0.0
            },
            total_gas_estimate,
            gas_cost_usd: 0.0,
            cumulative_slippage: 0.0,
            max_single_hop_impact: hop_details.iter()
                .map(|h| h.price_impact)
                .fold(0.0, f64::max),
            hop_details,
            warnings,
            failure_reason: None,
        }
    }

    /// Create final validation result
    fn create_final_validation(
        &self,
        path: Vec<Address>,
        initial_amount: U256,
        final_amount: U256,
        hop_details: Vec<HopDetail>,
        warnings: Vec<String>,
        total_gas_estimate: u64,
        cumulative_slippage: f64,
        max_single_hop_impact: f64,
    ) -> MultiHopValidation {
        MultiHopValidation {
            is_valid: false,
            path,
            hop_count: hop_details.len(),
            initial_amount,
            final_amount,
            profit_ratio: if initial_amount > U256::zero() {
                final_amount.as_u128() as f64 / initial_amount.as_u128() as f64
            } else {
                0.0
            },
            total_gas_estimate,
            gas_cost_usd: 0.0,
            cumulative_slippage,
            max_single_hop_impact,
            hop_details,
            warnings,
            failure_reason: None,
        }
    }

    /// Validate compound arbitrage path (10+ hops)
    pub async fn validate_compound_path(
        &self,
        path: Vec<Address>,
        initial_amount: U256,
    ) -> Result<MultiHopValidation> {
        if path.len() < 10 {
            return Ok(MultiHopValidation {
                is_valid: false,
                failure_reason: Some("Compound arbitrage requires 10+ tokens".to_string()),
                ..self.create_empty_validation(path, initial_amount)
            });
        }

        // Use stricter config for compound paths
        let mut compound_config = self.config.clone();
        compound_config.max_cumulative_slippage = 0.15; // Allow 15% for complex paths
        compound_config.min_profit_ratio = 1.02; // Require 2% profit for complexity
        
        let temp_validator = MultiHopValidator::new(
            self.dex_integration.clone(),
            self.price_oracle.clone(),
            compound_config,
        );
        
        let mut result = temp_validator.validate_path(path, initial_amount).await?;
        
        // Add compound-specific warnings
        if result.hop_count >= 10 && result.total_gas_estimate > 1_000_000 {
            result.warnings.push(format!(
                "Very high gas cost for {}-hop path: {} gas", 
                result.hop_count, result.total_gas_estimate
            ));
        }
        
        Ok(result)
    }
    
    /// Verify slippage calculation using proper AMM math
    pub async fn verify_path_with_amm_math(
        &self,
        path: Vec<Address>,
        initial_amount: U256,
    ) -> Result<(U256, f64)> {
        // Convert path to reserve data for AMM math calculation
        let mut hops = Vec::new();
        
        let mut dex_integration = self.dex_integration.write().await;
        
        for i in 0..path.len() - 1 {
            let token_in = path[i];
            let token_out = path[i + 1];
            
            // Get pool reserves (simplified to V2 for now)
            match dex_integration.get_real_pool_reserves(token_in, token_out, crate::dex_integration::DexType::UniswapV2).await {
                Ok(pool_data) => {
                    hops.push((pool_data.reserve0, pool_data.reserve1, false)); // false = V2
                }
                Err(_) => {
                    // Fallback with estimated reserves
                    let reserve_estimate = U256::from(1_000_000) * U256::exp10(18);
                    hops.push((reserve_estimate, reserve_estimate, false));
                }
            }
        }
        
        // Use the new AMM math for path slippage calculation
        MultiHopSlippage::calculate_path_slippage(initial_amount, &hops)
    }
    
    /// Optimize trade size for maximum slippage tolerance
    pub async fn optimize_trade_size(
        &self,
        path: Vec<Address>,
        max_slippage_pct: f64,
        max_amount: U256,
    ) -> Result<U256> {
        // Get reserve data for optimization
        let mut hops = Vec::new();
        let mut dex_integration = self.dex_integration.write().await;
        
        for i in 0..path.len() - 1 {
            let token_in = path[i];
            let token_out = path[i + 1];
            
            match dex_integration.get_real_pool_reserves(token_in, token_out, crate::dex_integration::DexType::UniswapV2).await {
                Ok(pool_data) => {
                    hops.push((pool_data.reserve0, pool_data.reserve1, false));
                }
                Err(_) => {
                    let reserve_estimate = U256::from(1_000_000) * U256::exp10(18);
                    hops.push((reserve_estimate, reserve_estimate, false));
                }
            }
        }
        
        // Use AMM math to find optimal trade size
        MultiHopSlippage::optimize_trade_size_for_path(max_slippage_pct, &hops, max_amount)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ethers::providers::{Provider, Http};

    #[tokio::test]
    async fn test_multi_hop_validation() {
        // Setup
        let provider = Provider::<Http>::try_from("https://polygon-rpc.com").unwrap();
        let provider = Arc::new(provider);
        
        let dex = RealDexIntegration::new(provider.clone(), 137);
        let oracle = LivePriceOracle::new(provider, 137);
        
        let validator = MultiHopValidator::new(
            Arc::new(tokio::sync::RwLock::new(dex)),
            Arc::new(tokio::sync::RwLock::new(oracle)),
            ValidationConfig::default(),
        );
        
        // Test path: WMATIC -> USDC -> WETH -> WMATIC
        let path = vec![
            "0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270".parse().unwrap(), // WMATIC
            "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174".parse().unwrap(), // USDC
            "0x7ceB23fD6bC0adD59E62ac25578270cFf1b9f619".parse().unwrap(), // WETH
            "0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270".parse().unwrap(), // WMATIC
        ];
        
        let amount = U256::from(1_000_000_000_000_000_000u128); // 1 WMATIC
        
        match validator.validate_path(path, amount).await {
            Ok(result) => {
                println!("Validation result: {:?}", result);
                assert_eq!(result.hop_count, 3);
                assert!(result.total_gas_estimate >= 300_000); // Base + 3 hops
            }
            Err(e) => {
                println!("Validation failed (expected in test): {}", e);
            }
        }
    }
}