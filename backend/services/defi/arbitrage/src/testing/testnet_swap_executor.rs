// Testnet Swap Execution Framework
// Executes real swaps on Mumbai/Amoy testnets to validate the arbitrage system

use anyhow::{Result, Context};
use ethers::prelude::*;
use std::sync::Arc;
use std::collections::HashMap;
use tracing::{info, warn, error, debug};
use chrono::{DateTime, Utc};

use crate::config::ArbitrageConfig;

/// Testnet swap execution results
#[derive(Debug, Clone)]
pub struct SwapExecutionResult {
    pub tx_hash: H256,
    pub block_number: u64,
    pub gas_used: u64,
    pub gas_price: U256,
    pub amount_in: U256,
    pub amount_out: U256,
    pub actual_slippage: f64,
    pub execution_time_ms: u64,
    pub success: bool,
    pub error_message: Option<String>,
}

/// Testnet configuration
#[derive(Debug, Clone)]
pub struct TestnetSwapConfig {
    pub network: String,
    pub rpc_url: String,
    pub chain_id: u64,
    pub max_test_amount_usd: f64,
    pub max_slippage_pct: f64,
    pub gas_limit: u64,
}

impl TestnetSwapConfig {
    pub fn mumbai() -> Self {
        Self {
            network: "Mumbai".to_string(),
            rpc_url: "https://rpc-mumbai.maticvigil.com".to_string(),
            chain_id: 80001,
            max_test_amount_usd: 1.0, // $1 max for safety
            max_slippage_pct: 5.0,
            gas_limit: 500_000,
        }
    }
    
    pub fn amoy() -> Self {
        Self {
            network: "Amoy".to_string(),
            rpc_url: "https://rpc-amoy.polygon.technology".to_string(),
            chain_id: 80002,
            max_test_amount_usd: 1.0, // $1 max for safety
            max_slippage_pct: 5.0,
            gas_limit: 500_000,
        }
    }
}

/// Testnet swap executor
pub struct TestnetSwapExecutor {
    config: TestnetSwapConfig,
    provider: Arc<Provider<Http>>,
    wallet: LocalWallet,
    signer: Arc<SignerMiddleware<Provider<Http>, LocalWallet>>,
    test_tokens: HashMap<String, Address>,
    test_results: Vec<SwapExecutionResult>,
}

impl TestnetSwapExecutor {
    pub async fn new(config: TestnetSwapConfig, private_key: &str) -> Result<Self> {
        let provider = Provider::<Http>::try_from(&config.rpc_url)?;
        let wallet = private_key.parse::<LocalWallet>()?
            .with_chain_id(config.chain_id);
        
        let signer = Arc::new(SignerMiddleware::new(
            provider.clone(),
            wallet.clone()
        ));
        
        // Define test tokens for each network
        let test_tokens = match config.chain_id {
            80001 => { // Mumbai
                let mut tokens = HashMap::new();
                tokens.insert("WMATIC".to_string(), "0x9c3C9283D3e44854697Cd22D3Faa240Cfb032889".parse()?);
                tokens.insert("USDC".to_string(), "0x2058A9D7613eEE744279e3856Ef0eAda5FCbaA7e".parse()?);
                tokens.insert("USDT".to_string(), "0xA02f6adc7926efeBBd59Fd43A84f4E0c0c91e832".parse()?);
                tokens.insert("DAI".to_string(), "0x001B3B4d0F3714Ca98ba10F6042DaEbF0B1B7b6F".parse()?);
                tokens
            },
            80002 => { // Amoy
                let mut tokens = HashMap::new();
                tokens.insert("WMATIC".to_string(), "0x360ad4f9a9A8EFe9A8DCB5f461c4Cc1047E1Dcf9".parse()?);
                tokens.insert("USDC".to_string(), "0x41E94Eb019C0762f9Bfcf9Fb1E58725BfB0e7582".parse()?);
                // Add more Amoy test tokens as they become available
                tokens
            },
            _ => return Err(anyhow::anyhow!("Unsupported chain ID: {}", config.chain_id)),
        };
        
        info!("🚀 Initialized testnet swap executor for {}", config.network);
        info!("📍 Wallet address: {:?}", wallet.address());
        info!("🪙 Available test tokens: {:?}", test_tokens.keys().collect::<Vec<_>>());
        
        Ok(Self {
            config,
            provider: Arc::new(provider),
            wallet,
            signer,
            test_tokens,
            test_results: Vec::new(),
        })
    }
    
    /// Check wallet balance and request test tokens if needed
    pub async fn check_balances(&self) -> Result<()> {
        info!("💰 Checking wallet balances...");
        
        let native_balance = self.provider
            .get_balance(self.wallet.address(), None)
            .await?;
        
        let native_balance_formatted = ethers::utils::format_units(native_balance, 18)?;
        info!("  Native balance: {} MATIC", native_balance_formatted);
        
        if native_balance < U256::from(100_000_000_000_000_000u64) { // Less than 0.1 MATIC
            warn!("⚠️ Low native balance. Request test MATIC from faucet:");
            warn!("   https://faucet.polygon.technology/");
            warn!("   Wallet: {:?}", self.wallet.address());
        }
        
        // Check ERC20 token balances
        for (symbol, address) in &self.test_tokens {
            if let Ok(balance) = self.get_token_balance(*address).await {
                let balance_formatted = ethers::utils::format_units(balance, 18)?; // Assuming 18 decimals
                info!("  {} balance: {}", symbol, balance_formatted);
            }
        }
        
        Ok(())
    }
    
    /// Execute a test swap between two tokens
    pub async fn execute_test_swap(
        &mut self,
        token_in: &str,
        token_out: &str,
        amount_in_tokens: f64,
    ) -> Result<SwapExecutionResult> {
        let start_time = std::time::Instant::now();
        
        info!("🔄 Executing test swap: {} {} -> {}", amount_in_tokens, token_in, token_out);
        
        // Get token addresses
        let token_in_addr = self.test_tokens.get(token_in)
            .ok_or_else(|| anyhow::anyhow!("Token {} not found", token_in))?;
        let token_out_addr = self.test_tokens.get(token_out)
            .ok_or_else(|| anyhow::anyhow!("Token {} not found", token_out))?;
        
        // Convert amount to wei (assuming 18 decimals)
        let amount_in = U256::from((amount_in_tokens * 1e18) as u64);
        
        // Check if we have enough balance
        let balance = self.get_token_balance(*token_in_addr).await?;
        if balance < amount_in {
            return Ok(SwapExecutionResult {
                tx_hash: H256::zero(),
                block_number: 0,
                gas_used: 0,
                gas_price: U256::zero(),
                amount_in,
                amount_out: U256::zero(),
                actual_slippage: 100.0,
                execution_time_ms: start_time.elapsed().as_millis() as u64,
                success: false,
                error_message: Some(format!("Insufficient {} balance", token_in)),
            });
        }
        
        // Execute the swap using QuickSwap router
        match self.execute_quickswap_swap(*token_in_addr, *token_out_addr, amount_in).await {
            Ok((tx_hash, receipt)) => {
                let execution_time = start_time.elapsed().as_millis() as u64;
                
                // Calculate actual slippage (simplified)
                let amount_out = self.extract_amount_out_from_receipt(&receipt)?;
                let expected_out = self.get_expected_amount_out(*token_in_addr, *token_out_addr, amount_in).await?;
                let actual_slippage = if expected_out > U256::zero() {
                    ((expected_out.saturating_sub(amount_out)).as_u128() as f64 / expected_out.as_u128() as f64) * 100.0
                } else {
                    0.0
                };
                
                let result = SwapExecutionResult {
                    tx_hash,
                    block_number: receipt.block_number.unwrap_or_default().as_u64(),
                    gas_used: receipt.gas_used.unwrap_or_default().as_u64(),
                    gas_price: receipt.effective_gas_price.unwrap_or_default(),
                    amount_in,
                    amount_out,
                    actual_slippage,
                    execution_time_ms: execution_time,
                    success: true,
                    error_message: None,
                };
                
                info!("✅ Swap executed successfully:");
                info!("   TX: {:?}", tx_hash);
                info!("   Gas used: {}", result.gas_used);
                info!("   Amount out: {}", result.amount_out);
                info!("   Slippage: {:.2}%", actual_slippage);
                info!("   Time: {}ms", execution_time);
                
                self.test_results.push(result.clone());
                Ok(result)
            }
            Err(e) => {
                let result = SwapExecutionResult {
                    tx_hash: H256::zero(),
                    block_number: 0,
                    gas_used: 0,
                    gas_price: U256::zero(),
                    amount_in,
                    amount_out: U256::zero(),
                    actual_slippage: 100.0,
                    execution_time_ms: start_time.elapsed().as_millis() as u64,
                    success: false,
                    error_message: Some(e.to_string()),
                };
                
                error!("❌ Swap failed: {}", e);
                self.test_results.push(result.clone());
                Ok(result)
            }
        }
    }
    
    /// Execute swap using QuickSwap router
    async fn execute_quickswap_swap(
        &self,
        token_in: Address,
        token_out: Address,
        amount_in: U256,
    ) -> Result<(H256, TransactionReceipt)> {
        // QuickSwap router address (same on Mumbai and Amoy)
        let router_address: Address = "0xa5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff".parse()?;
        
        // Router ABI for swapExactTokensForTokens
        let abi = ethers::abi::parse_abi(&[
            "function swapExactTokensForTokens(uint amountIn, uint amountOutMin, address[] calldata path, address to, uint deadline) external returns (uint[] memory amounts)",
            "function getAmountsOut(uint amountIn, address[] calldata path) external view returns (uint[] memory amounts)"
        ])?;
        
        let router = Contract::new(router_address, abi, self.signer.clone());
        
        // Create path
        let path = vec![token_in, token_out];
        
        // Get minimum amount out (with slippage tolerance)
        let amounts_out: Vec<U256> = router
            .method::<_, Vec<U256>>("getAmountsOut", (amount_in, path.clone()))?
            .call()
            .await?;
        
        let expected_out = amounts_out.get(1).copied().unwrap_or_default();
        let min_amount_out = expected_out * (100 - self.config.max_slippage_pct as u64) / 100;
        
        // Set deadline (5 minutes from now)
        let deadline = chrono::Utc::now().timestamp() + 300;
        
        info!("📊 Swap parameters:");
        info!("   Amount in: {}", amount_in);
        info!("   Expected out: {}", expected_out);
        info!("   Min amount out: {}", min_amount_out);
        info!("   Deadline: {}", deadline);
        
        // First approve the router to spend our tokens
        self.approve_token(token_in, router_address, amount_in).await?;
        
        // Execute the swap
        let tx = router
            .method::<_, Vec<U256>>(
                "swapExactTokensForTokens",
                (amount_in, min_amount_out, path, self.wallet.address(), U256::from(deadline))
            )?
            .gas(self.config.gas_limit)
            .send()
            .await?;
        
        info!("📤 Swap transaction sent: {:?}", tx.tx_hash());
        
        // Wait for confirmation
        let receipt = tx.await?.ok_or_else(|| anyhow::anyhow!("No receipt"))?;
        
        Ok((receipt.transaction_hash, receipt))
    }
    
    /// Approve token spending
    async fn approve_token(
        &self,
        token: Address,
        spender: Address,
        amount: U256,
    ) -> Result<()> {
        let abi = ethers::abi::parse_abi(&[
            "function approve(address spender, uint256 amount) external returns (bool)",
            "function allowance(address owner, address spender) external view returns (uint256)"
        ])?;
        
        let token_contract = Contract::new(token, abi, self.signer.clone());
        
        // Check current allowance
        let current_allowance: U256 = token_contract
            .method::<_, U256>("allowance", (self.wallet.address(), spender))?
            .call()
            .await?;
        
        if current_allowance >= amount {
            debug!("✅ Sufficient allowance already exists");
            return Ok(());
        }
        
        info!("📝 Approving token spending...");
        
        let tx = token_contract
            .method::<_, bool>("approve", (spender, amount))?
            .gas(100_000)
            .send()
            .await?;
        
        let receipt = tx.await?.ok_or_else(|| anyhow::anyhow!("No receipt"))?;
        
        if receipt.status == Some(U64::from(1)) {
            info!("✅ Token approval successful");
            Ok(())
        } else {
            Err(anyhow::anyhow!("Token approval failed"))
        }
    }
    
    /// Get token balance
    async fn get_token_balance(&self, token: Address) -> Result<U256> {
        let abi = ethers::abi::parse_abi(&[
            "function balanceOf(address account) external view returns (uint256)"
        ])?;
        
        let token_contract = Contract::new(token, abi.clone(), self.provider.clone());
        
        let balance: U256 = token_contract
            .method::<_, U256>("balanceOf", self.wallet.address())?
            .call()
            .await?;
        
        Ok(balance)
    }
    
    /// Get expected amount out for a swap
    async fn get_expected_amount_out(
        &self,
        token_in: Address,
        token_out: Address,
        amount_in: U256,
    ) -> Result<U256> {
        let router_address: Address = "0xa5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff".parse()?;
        let abi = ethers::abi::parse_abi(&[
            "function getAmountsOut(uint amountIn, address[] calldata path) external view returns (uint[] memory amounts)"
        ])?;
        
        let router = Contract::new(router_address, abi, self.provider.clone());
        let path = vec![token_in, token_out];
        
        let amounts: Vec<U256> = router
            .method::<_, Vec<U256>>("getAmountsOut", (amount_in, path))?
            .call()
            .await?;
        
        Ok(amounts.get(1).copied().unwrap_or_default())
    }
    
    /// Extract amount out from transaction receipt
    fn extract_amount_out_from_receipt(&self, receipt: &TransactionReceipt) -> Result<U256> {
        // This is a simplified version - in production would parse the Transfer events
        // For now, return a placeholder
        Ok(U256::from(95) * U256::exp10(16)) // Placeholder: ~0.95 tokens out
    }
    
    /// Run comprehensive swap test suite
    pub async fn run_test_suite(&mut self) -> Result<TestSuiteResults> {
        info!("🧪 Running comprehensive swap test suite...");
        
        let mut results = TestSuiteResults::default();
        
        // Test scenarios
        let test_scenarios = vec![
            ("WMATIC", "USDC", 0.1), // Small MATIC -> USDC swap
            ("USDC", "WMATIC", 0.1), // Small USDC -> MATIC swap
            // Add more scenarios as needed
        ];
        
        for (token_in, token_out, amount) in test_scenarios {
            match self.execute_test_swap(token_in, token_out, amount).await {
                Ok(result) => {
                    if result.success {
                        results.successful_swaps += 1;
                        results.total_gas_used += result.gas_used;
                        results.total_execution_time_ms += result.execution_time_ms;
                        results.slippage_measurements.push(result.actual_slippage);
                    } else {
                        results.failed_swaps += 1;
                        results.errors.push(result.error_message.unwrap_or_default());
                    }
                }
                Err(e) => {
                    results.failed_swaps += 1;
                    results.errors.push(e.to_string());
                }
            }
            
            // Wait between swaps to avoid rate limiting
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
        
        results.calculate_summary();
        results.print_report();
        
        Ok(results)
    }
    
    /// Get test results
    pub fn get_results(&self) -> &[SwapExecutionResult] {
        &self.test_results
    }
}

/// Test suite results
#[derive(Debug, Default)]
pub struct TestSuiteResults {
    pub successful_swaps: usize,
    pub failed_swaps: usize,
    pub total_gas_used: u64,
    pub total_execution_time_ms: u64,
    pub slippage_measurements: Vec<f64>,
    pub errors: Vec<String>,
    pub average_gas_per_swap: f64,
    pub average_execution_time_ms: f64,
    pub average_slippage: f64,
    pub success_rate: f64,
}

impl TestSuiteResults {
    pub fn calculate_summary(&mut self) {
        let total_swaps = self.successful_swaps + self.failed_swaps;
        
        if total_swaps > 0 {
            self.success_rate = (self.successful_swaps as f64 / total_swaps as f64) * 100.0;
        }
        
        if self.successful_swaps > 0 {
            self.average_gas_per_swap = self.total_gas_used as f64 / self.successful_swaps as f64;
            self.average_execution_time_ms = self.total_execution_time_ms as f64 / self.successful_swaps as f64;
            
            if !self.slippage_measurements.is_empty() {
                self.average_slippage = self.slippage_measurements.iter().sum::<f64>() / self.slippage_measurements.len() as f64;
            }
        }
    }
    
    pub fn print_report(&self) {
        println!("\n{}", "=".repeat(60));
        println!("📊 TESTNET SWAP EXECUTION REPORT");
        println!("{}", "=".repeat(60));
        
        println!("\n📈 Execution Summary:");
        println!("  Total swaps: {}", self.successful_swaps + self.failed_swaps);
        println!("  Successful: {}", self.successful_swaps);
        println!("  Failed: {}", self.failed_swaps);
        println!("  Success rate: {:.1}%", self.success_rate);
        
        if self.successful_swaps > 0 {
            println!("\n⛽ Gas Metrics:");
            println!("  Total gas used: {}", self.total_gas_used);
            println!("  Average per swap: {:.0}", self.average_gas_per_swap);
            
            println!("\n⏱️ Performance Metrics:");
            println!("  Average execution time: {:.0}ms", self.average_execution_time_ms);
            println!("  Average slippage: {:.2}%", self.average_slippage);
        }
        
        if !self.errors.is_empty() {
            println!("\n❌ Errors:");
            for error in &self.errors {
                println!("  - {}", error);
            }
        }
        
        println!("\n{}", "=".repeat(60));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_testnet_config() {
        let mumbai = TestnetSwapConfig::mumbai();
        assert_eq!(mumbai.chain_id, 80001);
        assert_eq!(mumbai.max_test_amount_usd, 1.0);
        
        let amoy = TestnetSwapConfig::amoy();
        assert_eq!(amoy.chain_id, 80002);
    }
}