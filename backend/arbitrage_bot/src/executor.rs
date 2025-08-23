use anyhow::Result;
use ethers::{
    prelude::*,
    types::transaction::eip2718::TypedTransaction,
};
use std::sync::Arc;
use crate::{ArbitrageOpportunity, Config};

// Arbitrage contract ABI
const ARBITRAGE_ABI: &str = r#"[
    {
        "inputs": [
            {"name": "buyRouter", "type": "address"},
            {"name": "sellRouter", "type": "address"},
            {"name": "tokenIn", "type": "address"},
            {"name": "tokenOut", "type": "address"},
            {"name": "amountIn", "type": "uint256"},
            {"name": "minProfit", "type": "uint256"}
        ],
        "name": "executeArbitrage",
        "outputs": [{"name": "profit", "type": "uint256"}],
        "type": "function"
    }
]"#;

pub struct ArbitrageExecutor {
    pub provider: Arc<Provider<Ws>>,
    pub config: Config,
    wallet: Option<LocalWallet>,
    contract_address: Option<Address>,
}

impl ArbitrageExecutor {
    pub fn new(provider: Arc<Provider<Ws>>, config: Config) -> Self {
        // Load wallet from environment
        let wallet = std::env::var("PRIVATE_KEY")
            .ok()
            .and_then(|key| key.parse::<LocalWallet>().ok())
            .map(|w| w.with_chain_id(config.chain_id));
        
        let contract_address = std::env::var("ARBITRAGE_CONTRACT")
            .ok()
            .and_then(|addr| addr.parse::<Address>().ok());
        
        Self {
            provider,
            config,
            wallet,
            contract_address,
        }
    }
    
    pub async fn execute_with_capital(&self, opp: &ArbitrageOpportunity) -> Result<H256> {
        let wallet = self.wallet.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No wallet configured"))?;
        
        let contract_address = self.contract_address
            .ok_or_else(|| anyhow::anyhow!("No arbitrage contract deployed"))?;
        
        // Create contract instance
        let client = SignerMiddleware::new(self.provider.clone(), wallet.clone());
        let contract = Contract::new(
            contract_address,
            serde_json::from_str::<Abi>(ARBITRAGE_ABI)?,
            Arc::new(client)
        );
        
        // Prepare transaction parameters
        let amount_in = U256::from((opp.size_usd * 1e6) as u128); // USDC has 6 decimals
        let min_profit = U256::from((opp.profit_usd * 0.8 * 1e6) as u128); // 80% of expected
        
        // Build transaction
        let tx = contract
            .method::<_, U256>(
                "executeArbitrage",
                (
                    opp.buy_router,
                    opp.sell_router,
                    opp.token0,
                    opp.token1,
                    amount_in,
                    min_profit
                )
            )?
            .gas(opp.gas_estimate)
            .gas_price(self.get_competitive_gas_price().await?);
        
        // Send transaction
        let pending_tx = tx.send().await?;
        
        Ok(pending_tx.tx_hash())
    }
    
    async fn get_competitive_gas_price(&self) -> Result<U256> {
        let base_price = self.provider.get_gas_price().await?;
        
        // Add priority fee for faster inclusion
        let priority_fee = U256::from(self.config.max_priority_fee_gwei) * U256::from(1e9);
        let competitive_price = base_price + priority_fee;
        
        // Cap at maximum
        let max_price = U256::from(self.config.max_gas_price_gwei) * U256::from(1e9);
        
        Ok(competitive_price.min(max_price))
    }
    
    pub async fn deploy_arbitrage_contract(&self) -> Result<Address> {
        // In production, would deploy the actual contract
        // For now, return a placeholder
        Ok("0x0000000000000000000000000000000000000000".parse()?)
    }
}