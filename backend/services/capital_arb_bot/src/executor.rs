use crate::config::Config;
use crate::dex::{DexManager, IERC20, IUniswapV2Router};
use crate::ArbOpportunity;
use anyhow::{bail, Context, Result};
use ethers::prelude::*;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

pub struct CapitalArbExecutor {
    client: Arc<SignerMiddleware<Provider<Http>, LocalWallet>>,
    config: Config,
    dex_manager: DexManager,
    wallet_address: Address,
}

impl CapitalArbExecutor {
    pub async fn new(
        client: Arc<SignerMiddleware<Provider<Http>, LocalWallet>>,
        config: Config,
    ) -> Result<Self> {
        let wallet_address = client.address();
        let dex_manager = DexManager::new(client.clone());

        Ok(Self {
            client,
            config,
            dex_manager,
            wallet_address,
        })
    }

    pub async fn check_balances(&self) -> Result<()> {
        info!("Checking wallet balances for address: {:?}", self.wallet_address);

        // Check native MATIC balance
        let balance = self.client.get_balance(self.wallet_address, None).await?;
        let matic_balance = ethers::utils::format_ether(balance);
        info!("MATIC balance: {} MATIC", matic_balance);

        // Check common token balances
        let tokens = [
            ("WMATIC", "0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270"),
            ("USDC", "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174"),
            ("USDT", "0xc2132D05D31c914a87C6611C10748AEb04B58e8F"),
            ("WETH", "0x7ceB23fD6bC0adD59E62ac25578270cFf1b9f619"),
            ("DAI", "0x8f3Cf7ad23Cd3CaDbD9735AFf958023239c6A063"),
        ];

        for (symbol, address) in tokens {
            let token_address: Address = address.parse()?;
            let token = IERC20::new(token_address, self.client.clone());
            
            match token.balance_of(self.wallet_address).call().await {
                Ok(balance) => {
                    let decimals = if symbol == "USDC" || symbol == "USDT" { 6 } else { 18 };
                    let formatted = ethers::utils::format_units(balance, decimals)?;
                    info!("{} balance: {}", symbol, formatted);
                }
                Err(e) => {
                    warn!("Failed to check {} balance: {}", symbol, e);
                }
            }
        }

        Ok(())
    }

    pub async fn execute(&self, opportunity: &ArbOpportunity) -> Result<f64> {
        info!("Executing arbitrage for {}", opportunity.pair);

        // Validate opportunity
        self.validate_opportunity(opportunity)?;

        // Calculate trade size
        let trade_size = self.calculate_trade_size(opportunity).await?;
        if trade_size == U256::zero() {
            bail!("Insufficient balance for trade");
        }

        // Check gas price
        let gas_price = self.client.get_gas_price().await?;
        let max_gas = ethers::utils::parse_units(self.config.max_gas_price_gwei, "gwei")?;
        if gas_price > max_gas.into() {
            bail!("Gas price too high: {} > {}", gas_price, max_gas);
        }

        // Execute two-step arbitrage
        let profit = self.execute_two_step_swap(opportunity, trade_size).await?;

        Ok(profit)
    }

    fn validate_opportunity(&self, opp: &ArbOpportunity) -> Result<()> {
        // Check routers are known
        if self.dex_manager.get_router(&opp.dex_buy_router).is_none() {
            bail!("Unknown buy router: {:?}", opp.dex_buy_router);
        }
        if self.dex_manager.get_router(&opp.dex_sell_router).is_none() {
            bail!("Unknown sell router: {:?}", opp.dex_sell_router);
        }

        // Check profit threshold
        if opp.estimated_profit_usd < self.config.min_profit_usd {
            bail!("Profit below threshold: ${}", opp.estimated_profit_usd);
        }

        Ok(())
    }

    async fn calculate_trade_size(&self, opp: &ArbOpportunity) -> Result<U256> {
        let token_a = IERC20::new(opp.token_a, self.client.clone());
        let balance = token_a.balance_of(self.wallet_address).call().await?;

        // Use max_trade_percentage of balance
        let max_trade = balance * U256::from((self.config.max_trade_percentage * 1000.0) as u64) / U256::from(1000);

        // Also consider liquidity constraints
        let token_info = self.dex_manager.get_token(&opp.token_a);
        let decimals = token_info.map(|t| t.decimals).unwrap_or(18);
        
        let liquidity_constraint = ethers::utils::parse_units(
            (opp.liquidity_a.min(opp.liquidity_b) * 0.05).to_string(),
            decimals as u32,
        )?;

        Ok(max_trade.min(liquidity_constraint.into()))
    }

    async fn execute_two_step_swap(
        &self,
        opp: &ArbOpportunity,
        trade_size: U256,
    ) -> Result<f64> {
        let start_time = std::time::Instant::now();

        info!(
            "Executing two-step swap: {} -> {}",
            self.dex_manager.get_token_symbol(&opp.token_a),
            self.dex_manager.get_token_symbol(&opp.token_b)
        );

        // Step 1: Buy token_b with token_a on cheaper DEX
        let buy_router = IUniswapV2Router::new(opp.dex_buy_router, self.client.clone());
        let token_a = IERC20::new(opp.token_a, self.client.clone());
        let token_b = IERC20::new(opp.token_b, self.client.clone());

        // Approve token_a for buy router
        debug!("Approving {} for buy router", trade_size);
        let approve_tx = token_a.approve(opp.dex_buy_router, trade_size);
        let pending = approve_tx.send().await?;
        let receipt = pending.await?.ok_or_else(|| anyhow::anyhow!("Approval failed"))?;
        debug!("Approval tx: {:?}", receipt.transaction_hash);

        // Execute buy swap
        let path_buy = vec![opp.token_a, opp.token_b];
        let deadline = U256::from(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs() + 300,
        );

        let min_amount_out_buy = self.calculate_min_amount_out(trade_size, opp.price_a);

        debug!("Executing buy swap on {}", self.dex_manager.get_router_name(&opp.dex_buy_router));
        let buy_tx = buy_router.swap_exact_tokens_for_tokens(
            trade_size,
            min_amount_out_buy,
            path_buy,
            self.wallet_address,
            deadline,
        );

        let buy_tx_with_gas = buy_tx.gas(300000);
        let pending = buy_tx_with_gas.send().await?;
        let receipt = pending.await?.ok_or_else(|| anyhow::anyhow!("Buy swap failed"))?;
        info!("Buy swap tx: {:?}", receipt.transaction_hash);

        // Get amount received from buy
        let token_b_balance = token_b.balance_of(self.wallet_address).call().await?;

        // Step 2: Sell token_b for token_a on expensive DEX
        let sell_router = IUniswapV2Router::new(opp.dex_sell_router, self.client.clone());

        // Approve token_b for sell router
        debug!("Approving {} for sell router", token_b_balance);
        let approve_tx = token_b.approve(opp.dex_sell_router, token_b_balance);
        let pending = approve_tx.send().await?;
        let receipt = pending.await?.ok_or_else(|| anyhow::anyhow!("Approval failed"))?;
        debug!("Approval tx: {:?}", receipt.transaction_hash);

        // Execute sell swap
        let path_sell = vec![opp.token_b, opp.token_a];
        let min_amount_out_sell = self.calculate_min_amount_out(token_b_balance, opp.price_b);

        debug!("Executing sell swap on {}", self.dex_manager.get_router_name(&opp.dex_sell_router));
        let sell_tx = sell_router.swap_exact_tokens_for_tokens(
            token_b_balance,
            min_amount_out_sell,
            path_sell,
            self.wallet_address,
            deadline,
        );

        let sell_tx_with_gas = sell_tx.gas(300000);
        let pending = sell_tx_with_gas.send().await?;
        let receipt = pending.await?.ok_or_else(|| anyhow::anyhow!("Sell swap failed"))?;
        info!("Sell swap tx: {:?}", receipt.transaction_hash);

        // Calculate profit
        let final_balance = token_a.balance_of(self.wallet_address).call().await?;
        let profit_wei = if final_balance > trade_size {
            final_balance - trade_size
        } else {
            U256::zero()
        };

        let token_info = self.dex_manager.get_token(&opp.token_a);
        let decimals = token_info.map(|t| t.decimals).unwrap_or(18);
        let profit_formatted = ethers::utils::format_units(profit_wei, decimals as u32)?;
        let profit_usd = profit_formatted.parse::<f64>()? * opp.price_a;

        let elapsed = start_time.elapsed();
        info!("Execution completed in {:?}", elapsed);
        info!("Profit: {} {} (${:.2})", profit_formatted, 
            self.dex_manager.get_token_symbol(&opp.token_a), profit_usd);

        Ok(profit_usd)
    }

    fn calculate_min_amount_out(&self, amount_in: U256, _price: f64) -> U256 {
        // Apply slippage tolerance
        let expected_out = amount_in; // Simplified - should use actual price calculation
        let slippage_factor = 1.0 - self.config.slippage_tolerance;
        let min_out = expected_out * U256::from((slippage_factor * 1000.0) as u64) / U256::from(1000);
        min_out
    }
}