use anyhow::Result;
use ethers::prelude::*;
use std::sync::Arc;
use crate::{ArbitrageOpportunity, Config};

// Aave V3 Pool address on Polygon
const AAVE_POOL: &str = "0x794a61358D6845594F94dc1DB02A252b5b4814aD";

// Flash loan contract ABI
const FLASH_LOAN_ABI: &str = r#"[
    {
        "inputs": [
            {"name": "asset", "type": "address"},
            {"name": "amount", "type": "uint256"},
            {"name": "params", "type": "bytes"}
        ],
        "name": "executeFlashLoanArbitrage",
        "outputs": [],
        "type": "function"
    }
]"#;

pub struct FlashLoanExecutor {
    provider: Arc<Provider<Ws>>,
    pub config: Config,
    wallet: Option<LocalWallet>,
    contract_address: Option<Address>,
}

impl FlashLoanExecutor {
    pub fn new(provider: Arc<Provider<Ws>>, config: Config) -> Self {
        let wallet = std::env::var("PRIVATE_KEY")
            .ok()
            .and_then(|key| key.parse::<LocalWallet>().ok())
            .map(|w| w.with_chain_id(config.chain_id));
        
        let contract_address = std::env::var("FLASH_LOAN_CONTRACT")
            .ok()
            .and_then(|addr| addr.parse::<Address>().ok());
        
        Self {
            provider,
            config,
            wallet,
            contract_address,
        }
    }
    
    pub async fn execute_with_flash_loan(&self, opp: &ArbitrageOpportunity) -> Result<H256> {
        let wallet = self.wallet.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No wallet configured"))?;
        
        let contract_address = self.contract_address
            .or_else(|| self.deploy_flash_loan_contract().ok())
            .ok_or_else(|| anyhow::anyhow!("No flash loan contract available"))?;
        
        // Create contract instance
        let client = SignerMiddleware::new(self.provider.clone(), wallet.clone());
        let contract = Contract::new(
            contract_address,
            serde_json::from_str::<Abi>(FLASH_LOAN_ABI)?,
            Arc::new(client)
        );
        
        // Encode arbitrage parameters
        let params = ethers::abi::encode(&[
            Token::Address(opp.buy_router),
            Token::Address(opp.sell_router),
            Token::Address(opp.token0),
            Token::Address(opp.token1),
            Token::Uint(U256::from((opp.size_usd * 1e6) as u128)),
            Token::Uint(U256::from((opp.profit_usd * 0.8 * 1e6) as u128)),
        ]);
        
        // Execute flash loan
        let tx = contract
            .method::<_, ()>(
                "executeFlashLoanArbitrage",
                (
                    opp.token0, // Asset to borrow (USDC)
                    U256::from((opp.size_usd * 1e6) as u128), // Amount
                    Bytes::from(params)
                )
            )?
            .gas(U256::from(500000))
            .gas_price(self.get_competitive_gas_price().await?);
        
        let pending_tx = tx.send().await?;
        
        Ok(pending_tx.tx_hash())
    }
    
    fn deploy_flash_loan_contract(&self) -> Result<Address> {
        // Would deploy actual contract
        // For testing, use a known address or deploy on demand
        Ok("0x0000000000000000000000000000000000000000".parse()?)
    }
    
    async fn get_competitive_gas_price(&self) -> Result<U256> {
        let base_price = self.provider.get_gas_price().await?;
        let priority_fee = U256::from(self.config.max_priority_fee_gwei) * U256::from(1e9);
        let competitive_price = base_price + priority_fee;
        let max_price = U256::from(self.config.max_gas_price_gwei) * U256::from(1e9);
        
        Ok(competitive_price.min(max_price))
    }
}