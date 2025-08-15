use crate::dex::IUniswapV2Router;
use crate::ArbOpportunity;
use anyhow::{Context, Result};
use ethers::prelude::*;
use std::sync::Arc;
use tracing::{debug, info};

pub struct ArbSimulator {
    client: Arc<SignerMiddleware<Provider<Http>, LocalWallet>>,
}

impl ArbSimulator {
    pub fn new(client: Arc<SignerMiddleware<Provider<Http>, LocalWallet>>) -> Self {
        Self { client }
    }

    pub async fn simulate(&self, opportunity: &ArbOpportunity) -> Result<f64> {
        info!("Simulating arbitrage for {}", opportunity.pair);

        // Simulate with a standard trade size (e.g., $1000 worth)
        let trade_size_usd = 1000.0;
        let trade_size_tokens = trade_size_usd / opportunity.price_a;

        // Get decimals for token_a
        let decimals = 18; // Default, should fetch from token contract
        let trade_size = ethers::utils::parse_units(trade_size_tokens.to_string(), decimals)?;

        // Query buy router for expected output
        let buy_router = IUniswapV2Router::new(opportunity.dex_buy_router, self.client.clone());
        let path_buy = vec![opportunity.token_a, opportunity.token_b];

        let amounts_out_buy = buy_router
            .get_amounts_out(trade_size.into(), path_buy)
            .call()
            .await
            .context("Failed to get buy amounts")?;

        let token_b_received = amounts_out_buy
            .last()
            .cloned()
            .unwrap_or(U256::zero());

        debug!("Simulated buy: {} -> {}", trade_size, token_b_received);

        // Query sell router for expected output
        let sell_router = IUniswapV2Router::new(opportunity.dex_sell_router, self.client.clone());
        let path_sell = vec![opportunity.token_b, opportunity.token_a];

        let amounts_out_sell = sell_router
            .get_amounts_out(token_b_received, path_sell)
            .call()
            .await
            .context("Failed to get sell amounts")?;

        let token_a_received = amounts_out_sell
            .last()
            .cloned()
            .unwrap_or(U256::zero());

        debug!("Simulated sell: {} -> {}", token_b_received, token_a_received);

        // Calculate profit
        let trade_size_u256: U256 = trade_size.into();
        let profit = if token_a_received > trade_size_u256 {
            token_a_received - trade_size_u256
        } else {
            U256::zero()
        };

        let profit_formatted = ethers::utils::format_units(profit, decimals)?;
        let profit_usd = profit_formatted.parse::<f64>()? * opportunity.price_a;

        // Estimate gas costs
        let gas_estimate = U256::from(600000); // ~600k gas for two swaps
        let gas_price = self.client.get_gas_price().await?;
        let gas_cost = gas_estimate * gas_price;
        let gas_cost_matic = ethers::utils::format_ether(gas_cost);
        let gas_cost_usd = gas_cost_matic.parse::<f64>()? * 0.8; // Approximate MATIC price

        let net_profit = profit_usd - gas_cost_usd;

        info!(
            "Simulation result: Gross profit ${:.2}, Gas cost ${:.2}, Net profit ${:.2}",
            profit_usd, gas_cost_usd, net_profit
        );

        Ok(net_profit)
    }

    pub async fn simulate_with_custom_amount(
        &self,
        opportunity: &ArbOpportunity,
        trade_size: U256,
    ) -> Result<f64> {
        info!("Simulating arbitrage with custom amount");

        // Query buy router
        let buy_router = IUniswapV2Router::new(opportunity.dex_buy_router, self.client.clone());
        let path_buy = vec![opportunity.token_a, opportunity.token_b];

        let amounts_out_buy = buy_router
            .get_amounts_out(trade_size.into(), path_buy)
            .call()
            .await?;

        let token_b_received = amounts_out_buy.last().cloned().unwrap_or(U256::zero());

        // Query sell router
        let sell_router = IUniswapV2Router::new(opportunity.dex_sell_router, self.client.clone());
        let path_sell = vec![opportunity.token_b, opportunity.token_a];

        let amounts_out_sell = sell_router
            .get_amounts_out(token_b_received, path_sell)
            .call()
            .await?;

        let token_a_received = amounts_out_sell.last().cloned().unwrap_or(U256::zero());

        // Calculate profit
        let trade_size_u256: U256 = trade_size.into();
        let profit = if token_a_received > trade_size_u256 {
            token_a_received - trade_size_u256
        } else {
            U256::zero()
        };

        let decimals = 18; // Should fetch from token
        let profit_formatted = ethers::utils::format_units(profit, decimals)?;
        let profit_usd = profit_formatted.parse::<f64>()? * opportunity.price_a;

        Ok(profit_usd)
    }
}