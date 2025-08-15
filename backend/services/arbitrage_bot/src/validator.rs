use alphapulse_protocol::ArbitrageOpportunityMessage;
use anyhow::{Context, Result};
use ethers::prelude::*;
use ethers::providers::{Provider, Http};
use std::sync::Arc;
use tracing::{debug, info, warn};

// Simplified DEX router ABI for price queries
abigen!(
    DexRouter,
    r#"[
        function getAmountsOut(uint256 amountIn, address[] calldata path) external view returns (uint256[] memory amounts)
    ]"#
);

pub struct OpportunityValidator {
    provider: Arc<Provider<Http>>,
}

impl OpportunityValidator {
    pub async fn new() -> Result<Self> {
        let provider = Provider::<Http>::try_from("https://polygon-mainnet.public.blastapi.io")?;
        
        Ok(Self {
            provider: Arc::new(provider),
        })
    }
    
    pub async fn validate(&self, opportunity: &ArbitrageOpportunityMessage) -> Result<bool> {
        debug!("🔍 Validating opportunity: {}", opportunity.pair);
        
        // In production, we would:
        // 1. Query current prices from both DEXs
        // 2. Calculate actual profit considering slippage
        // 3. Check liquidity is still sufficient
        // 4. Verify gas costs haven't spiked
        
        // For now, simplified validation
        let stale_threshold_ms = 5000; // 5 seconds
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis() as u64;
        
        let age_ms = (now_ms * 1_000_000).saturating_sub(opportunity.timestamp_ns) / 1_000_000;
        
        if age_ms > stale_threshold_ms {
            debug!("Opportunity is stale: {}ms old", age_ms);
            return Ok(false);
        }
        
        // Check if estimated profit is still above threshold
        let profit_usd = opportunity.estimated_profit as f64 / 1e8;
        if profit_usd < super::MIN_PROFIT_USD {
            debug!("Profit below threshold: ${:.2}", profit_usd);
            return Ok(false);
        }
        
        // In production, query actual DEX prices here
        if std::env::var("VALIDATE_ONCHAIN").unwrap_or_default() == "true" {
            match self.validate_onchain_prices(opportunity).await {
                Ok(is_profitable) => return Ok(is_profitable),
                Err(e) => {
                    warn!("On-chain validation failed: {}", e);
                    // Fall back to off-chain validation
                }
            }
        }
        
        // Simple validation based on profit percentage
        let profit_percent = opportunity.profit_percent as f64 / 1e10;
        Ok(profit_percent > 0.002) // At least 0.2% profit
    }
    
    async fn validate_onchain_prices(&self, opportunity: &ArbitrageOpportunityMessage) -> Result<bool> {
        // Parse router addresses
        let buy_router_addr = opportunity.dex_buy_router.parse::<Address>()?;
        let sell_router_addr = opportunity.dex_sell_router.parse::<Address>()?;
        
        // Create router contracts
        let buy_router = DexRouter::new(buy_router_addr, self.provider.clone());
        let sell_router = DexRouter::new(sell_router_addr, self.provider.clone());
        
        // Parse token addresses
        let token_a = opportunity.token_a.parse::<Address>()?;
        let token_b = opportunity.token_b.parse::<Address>()?;
        
        // Test trade amount (0.1% of max trade size)
        let test_amount = U256::from((opportunity.max_trade_size / 1000) as u128);
        
        // Query buy price (token_a -> token_b)
        let buy_path = vec![token_a, token_b];
        let buy_amounts = buy_router
            .get_amounts_out(test_amount, buy_path)
            .call()
            .await
            .context("Failed to query buy price")?;
        
        let tokens_received = buy_amounts.get(1)
            .copied()
            .unwrap_or_default();
        
        // Query sell price (token_b -> token_a)
        let sell_path = vec![token_b, token_a];
        let sell_amounts = sell_router
            .get_amounts_out(tokens_received, sell_path)
            .call()
            .await
            .context("Failed to query sell price")?;
        
        let final_amount = sell_amounts.get(1)
            .copied()
            .unwrap_or_default();
        
        // Calculate actual profit
        let profit = final_amount.saturating_sub(test_amount);
        let profit_percent = if test_amount > U256::zero() {
            profit.as_u128() as f64 / test_amount.as_u128() as f64
        } else {
            0.0
        };
        
        info!("📊 On-chain validation: {:.3}% profit", profit_percent * 100.0);
        
        // Account for gas costs and slippage
        Ok(profit_percent > 0.003) // Need at least 0.3% to cover gas and slippage
    }
    
    pub async fn estimate_gas_cost(&self) -> Result<f64> {
        let gas_price = self.provider.get_gas_price().await?;
        let gas_limit = U256::from(500000u64); // Estimated for flash loan + 2 swaps
        
        let gas_cost_wei = gas_price * gas_limit;
        let gas_cost_matic = gas_cost_wei.as_u128() as f64 / 1e18;
        
        // Convert to USD (hardcoded MATIC price for now)
        let matic_price = 0.52;
        Ok(gas_cost_matic * matic_price)
    }
}