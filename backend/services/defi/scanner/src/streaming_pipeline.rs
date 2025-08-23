/// Two-layer streaming pipeline: Fast filtering + Selective gas estimation
/// 
/// Layer 1: Stream all swap events via WebSocket, calculate theoretical profit 
/// Layer 2: Only estimate gas for promising opportunities above threshold
use crate::{
    huff_gas_estimator::{HuffGasEstimator, PromisingOpportunity},
    ArbitrageOpportunity, PoolInfo, 
    amm_math::AmmMath
};
use anyhow::Result;
use ethers::types::{Address, U256};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::sync::Arc;
use tokio::time::Duration;
use tracing::{debug, info, warn};

/// Fast in-memory candidate generator (Layer 1)
pub struct StreamingCandidateGenerator {
    profit_threshold_usd: Decimal,  // Only gas-estimate if profit > threshold
    confidence_threshold: f64,      // Only gas-estimate if confidence > threshold
    max_candidates_per_block: usize, // Rate limiting
}

impl StreamingCandidateGenerator {
    pub fn new(profit_threshold_usd: Decimal) -> Self {
        Self {
            profit_threshold_usd,
            confidence_threshold: 0.7,  // 70% confidence minimum
            max_candidates_per_block: 10, // Limit RPC load
        }
    }
    
    /// Fast theoretical profit calculation (no gas costs yet)
    /// This runs on every swap event from WebSocket stream
    pub fn calculate_theoretical_profit(
        &self,
        pool_a: &PoolInfo,
        pool_b: &PoolInfo,
        amount_in: Decimal,
    ) -> Result<Option<PromisingOpportunity>> {
        // Step 1: Fast calculation using pool reserves only (no RPC calls)
        let amount_out_a = if pool_a.exchange.contains("v3") {
            // Use V3 math for V3 pools
            self.calculate_v3_output(pool_a, amount_in)?
        } else {
            // Use V2 math for V2 pools  
            AmmMath::calculate_v2_output(amount_in, pool_a.reserve0, pool_a.reserve1, 30)?
        };
        
        let amount_out_b = if pool_b.exchange.contains("v3") {
            self.calculate_v3_output(pool_b, amount_out_a)?
        } else {
            AmmMath::calculate_v2_output(amount_out_a, pool_b.reserve1, pool_b.reserve0, 30)?
        };
        
        let gross_profit = amount_out_b - amount_in;
        
        // Step 2: Quick profitability filter (before any gas estimation)
        if gross_profit < self.profit_threshold_usd {
            debug!("🚫 Theoretical profit ${:.4} below threshold ${:.4}", gross_profit, self.profit_threshold_usd);
            return Ok(None);
        }
        
        // Step 3: Calculate confidence based on liquidity, slippage, etc.
        let confidence = self.calculate_confidence_score(pool_a, pool_b, amount_in);
        
        if confidence < self.confidence_threshold {
            debug!("🚫 Confidence {:.2} below threshold {:.2}", confidence, self.confidence_threshold);
            return Ok(None);
        }
        
        // Step 4: This is a promising candidate - send to Layer 2
        let opportunity = PromisingOpportunity {
            id: format!("arb_{}_{}", chrono::Utc::now().timestamp(), rand::random::<u32>()),
            amount: U256::from_dec_str(&amount_in.to_string()).unwrap_or_default(),
            token0: pool_a.token0.parse().unwrap_or_default(),
            token1: pool_a.token1.parse().unwrap_or_default(),
            buy_router: self.get_router_address(&pool_a.exchange),
            sell_router: self.get_router_address(&pool_b.exchange),
            theoretical_profit_usd: gross_profit,
            confidence_score: confidence,
        };
        
        info!("⭐ Promising candidate: ${:.4} theoretical profit, {:.1}% confidence", 
              gross_profit, confidence * 100.0);
        
        Ok(Some(opportunity))
    }
    
    fn calculate_v3_output(&self, pool: &PoolInfo, amount_in: Decimal) -> Result<Decimal> {
        // Use V3 math with tick data if available
        if let (Some(sqrt_price), Some(liquidity)) = (pool.v3_sqrt_price_x96, pool.v3_liquidity) {
            let amount_in_u128 = amount_in.to_string().parse::<u128>().unwrap_or(0) * 1_000_000; // 6 decimals
            
            let (_, _, amount_out) = crate::v3_math::swap_within_tick(
                sqrt_price,
                sqrt_price * 95 / 100, // 5% price impact limit
                liquidity,
                amount_in_u128,
                500, // 0.05% fee
                true, // token0 -> token1
            );
            
            Ok(Decimal::new(amount_out as i64, 6))
        } else {
            // Fallback to V2-style calculation
            AmmMath::calculate_v2_output(amount_in, pool.reserve0, pool.reserve1, 30)
        }
    }
    
    fn calculate_confidence_score(&self, pool_a: &PoolInfo, pool_b: &PoolInfo, amount: Decimal) -> f64 {
        let mut confidence: f64 = 1.0;
        
        // Reduce confidence for low liquidity
        let min_liquidity = pool_a.reserve0.min(pool_b.reserve0);
        if min_liquidity < dec!(10000) { confidence *= 0.7; }  // $10k
        if min_liquidity < dec!(1000) { confidence *= 0.5; }   // $1k
        
        // Reduce confidence for high slippage
        let slippage_a = amount / pool_a.reserve0;
        let slippage_b = amount / pool_b.reserve1;
        let max_slippage = slippage_a.max(slippage_b);
        
        if max_slippage > dec!(0.05) { confidence *= 0.8; }    // 5%+
        if max_slippage > dec!(0.10) { confidence *= 0.6; }    // 10%+
        
        // Reduce confidence for stale pool data
        let now = chrono::Utc::now().timestamp();
        let max_age = (now - pool_a.last_updated.max(pool_b.last_updated)) as f64;
        if max_age > 60.0 { confidence *= 0.9; }  // 1 minute
        if max_age > 300.0 { confidence *= 0.7; } // 5 minutes
        
        confidence.max(0.0).min(1.0)
    }
    
    fn get_router_address(&self, exchange: &str) -> Address {
        match exchange {
            "uniswap_v2" => "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D".parse().unwrap(),
            "uniswap_v3" => "0xE592427A0AEce92De3Edee1F18E0157C05861564".parse().unwrap(),
            "sushiswap" => "0xd9e1cE17f2641f24aE83637ab66a2cca9C378B9F".parse().unwrap(),
            _ => Address::zero(),
        }
    }
}

/// Complete two-layer arbitrage pipeline
pub struct ArbitragePipeline {
    candidate_generator: StreamingCandidateGenerator,
    gas_estimator: HuffGasEstimator,
    bot_address: Address,
    matic_price_usd: Decimal,
}

impl ArbitragePipeline {
    pub fn new(
        huff_contract: Address,
        bot_address: Address,
        rpc_url: &str,
        profit_threshold_usd: Decimal,
    ) -> Result<Self> {
        Ok(Self {
            candidate_generator: StreamingCandidateGenerator::new(profit_threshold_usd),
            gas_estimator: HuffGasEstimator::new(rpc_url, huff_contract)?,
            bot_address,
            matic_price_usd: dec!(0.8), // Could fetch from price oracle
        })
    }
    
    /// Complete pipeline: Stream → Filter → Gas Estimate → Execute
    pub async fn process_swap_event(
        &self,
        pool_a: &PoolInfo,
        pool_b: &PoolInfo,
    ) -> Result<Option<ArbitrageOpportunity>> {
        // Layer 1: Fast theoretical profit calculation (no RPC)
        let amount_in = dec!(1000); // $1000 test amount
        let candidate = match self.candidate_generator.calculate_theoretical_profit(pool_a, pool_b, amount_in)? {
            Some(candidate) => candidate,
            None => {
                debug!("🚫 Not promising enough for gas estimation");
                return Ok(None);
            }
        };
        
        // Layer 2: Gas estimation (selective RPC calls)
        info!("⛽ Gas estimating promising candidate: {}", candidate.id);
        
        let gas_units = self.gas_estimator.estimate_arbitrage_with_cache(
            candidate.amount,
            candidate.token0,
            candidate.token1,
            candidate.buy_router,
            candidate.sell_router,
            U256::from(1),
            self.bot_address,
        ).await?;
        
        // Layer 3: Final profitability check with real gas costs
        let net_profit = self.gas_estimator.calculate_net_profitability(
            candidate.theoretical_profit_usd,
            gas_units,
            self.matic_price_usd,
        ).await?;
        
        if net_profit <= dec!(0) {
            warn!("💸 After gas costs: ${:.4} net profit - NOT PROFITABLE", net_profit);
            return Ok(None);
        }
        
        // Create final arbitrage opportunity for execution
        let opportunity = ArbitrageOpportunity {
            id: candidate.id,
            token_in: pool_a.token0.clone(),
            token_out: pool_a.token1.clone(),
            amount_in: amount_in,
            amount_out: amount_in + net_profit,
            profit_usd: candidate.theoretical_profit_usd,
            profit_percentage: (net_profit / amount_in) * dec!(100),
            buy_exchange: pool_a.exchange.clone(),
            sell_exchange: pool_b.exchange.clone(),
            buy_pool: pool_a.address.clone(),
            sell_pool: pool_b.address.clone(),
            gas_cost_estimate: candidate.theoretical_profit_usd - net_profit,
            net_profit_usd: net_profit,
            timestamp: chrono::Utc::now().timestamp(),
            block_number: pool_a.block_number.max(pool_b.block_number),
            confidence_score: candidate.confidence_score,
        };
        
        info!("🎯 EXECUTABLE OPPORTUNITY: ${:.4} net profit after ${:.6} gas", 
              net_profit, opportunity.gas_cost_estimate);
        
        Ok(Some(opportunity))
    }
    
    /// Batch process multiple swap events (parallel Layer 2)
    pub async fn batch_process_candidates(
        &self,
        candidates: Vec<PromisingOpportunity>
    ) -> Vec<ArbitrageOpportunity> {
        if candidates.is_empty() {
            return vec![];
        }
        
        info!("⚡ Batch processing {} candidates", candidates.len());
        
        // Parallel gas estimation for all promising candidates
        let gas_results = self.gas_estimator.batch_estimate_promising_opportunities(
            candidates.clone(), 
            self.bot_address
        ).await;
        
        let mut executable_opportunities = Vec::new();
        
        for (candidate, gas_result) in gas_results {
            match gas_result {
                Ok(gas_units) => {
                    // Check final profitability
                    if let Ok(net_profit) = self.gas_estimator.calculate_net_profitability(
                        candidate.theoretical_profit_usd,
                        gas_units,
                        self.matic_price_usd,
                    ).await {
                        if net_profit > dec!(0) {
                            // Convert to executable opportunity
                            let opportunity = ArbitrageOpportunity {
                                id: candidate.id,
                                token_in: format!("{:?}", candidate.token0),
                                token_out: format!("{:?}", candidate.token1),
                                amount_in: Decimal::from(candidate.amount.as_u128()),
                                amount_out: Decimal::from(candidate.amount.as_u128()) + net_profit,
                                profit_usd: candidate.theoretical_profit_usd,
                                profit_percentage: (net_profit / Decimal::from(candidate.amount.as_u128())) * dec!(100),
                                buy_exchange: "exchange_a".to_string(),
                                sell_exchange: "exchange_b".to_string(),
                                buy_pool: format!("{:?}", candidate.buy_router),
                                sell_pool: format!("{:?}", candidate.sell_router),
                                gas_cost_estimate: candidate.theoretical_profit_usd - net_profit,
                                net_profit_usd: net_profit,
                                timestamp: chrono::Utc::now().timestamp(),
                                block_number: 1000000,
                                confidence_score: candidate.confidence_score,
                            };
                            
                            executable_opportunities.push(opportunity);
                        }
                    }
                },
                Err(e) => {
                    warn!("⛽ Gas estimation failed for {}: {}", candidate.id, e);
                }
            }
        }
        
        info!("✅ Found {} executable opportunities from {} candidates", 
              executable_opportunities.len(), candidates.len());
        
        executable_opportunities
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_streaming_pipeline() {
        tracing_subscriber::fmt::try_init().ok();
        
        // This would be your deployed Huff contract
        let huff_contract = "0x1234567890123456789012345678901234567890".parse().unwrap();
        let bot_address = "0x9876543210987654321098765432109876543210".parse().unwrap();
        
        let pipeline = ArbitragePipeline::new(
            huff_contract,
            bot_address,
            "https://polygon-rpc.com",
            dec!(5.0), // $5 minimum theoretical profit
        ).unwrap();
        
        // Simulate pool data that would come from WebSocket stream
        let pool_a = PoolInfo {
            address: "0xAAA".to_string(),
            exchange: "uniswap_v2".to_string(),
            token0: "USDC".to_string(),
            token1: "WETH".to_string(),
            reserve0: dec!(1000000), // 1M USDC
            reserve1: dec!(400),     // 400 WETH
            fee: dec!(0.003),
            last_updated: chrono::Utc::now().timestamp(),
            block_number: 1000000,
            v3_tick: None,
            v3_sqrt_price_x96: None,
            v3_liquidity: None,
        };
        
        let pool_b = PoolInfo {
            address: "0xBBB".to_string(),
            exchange: "sushiswap".to_string(),
            token0: "USDC".to_string(),
            token1: "WETH".to_string(),
            reserve0: dec!(800000),  // 800K USDC
            reserve1: dec!(320),     // 320 WETH (price difference!)
            fee: dec!(0.003),
            last_updated: chrono::Utc::now().timestamp(),
            block_number: 1000000,
            v3_tick: None,
            v3_sqrt_price_x96: None,
            v3_liquidity: None,
        };
        
        // Test the complete pipeline
        let result = pipeline.process_swap_event(&pool_a, &pool_b).await;
        
        match result {
            Ok(Some(opportunity)) => {
                info!("🎉 Pipeline found executable opportunity: ${:.4}", opportunity.net_profit_usd);
            },
            Ok(None) => {
                info!("✅ Pipeline correctly filtered out unprofitable opportunity");
            },
            Err(e) => {
                warn!("⚠️ Pipeline error: {}", e);
            }
        }
    }
}