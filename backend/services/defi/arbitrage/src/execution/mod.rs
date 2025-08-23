pub mod aave_client;
pub mod execution_validator;

pub use aave_client::{AaveClient, FlashLoanRequest};
pub use execution_validator::{ExecutionValidator, ValidationResult, ExecutionValidation};

use anyhow::Result;
use ethers::prelude::*;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

use crate::config::ArbitrageConfig;
use crate::mev_protection::FlashbotsClient;
use crate::strategies::{StrategyResult, compound_arb};
use crate::dex_integration::{RealDexIntegration, DexType};
use crate::secure_registries::SecureRegistryManager;

#[derive(Debug, Clone)]
pub enum ExecutionMode {
    FlashLoan,
    Capital,
}

#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub success: bool,
    pub transaction_hash: Option<H256>,
    pub profit_usd: f64,
    pub gas_cost_usd: f64,
    pub execution_time_ms: u64,
    pub error: Option<String>,
}

/// Main execution engine that handles both flash loan and capital-based arbitrage
pub struct ExecutionEngine {
    config: Arc<ArbitrageConfig>,
    aave_client: Arc<AaveClient>,
    flashbots_client: Arc<FlashbotsClient>,
    execution_validator: Arc<tokio::sync::RwLock<ExecutionValidator>>,
    dex_integration: Arc<tokio::sync::RwLock<RealDexIntegration>>,
    provider: Arc<Provider<Http>>,
}

impl ExecutionEngine {
    pub async fn new(config: Arc<ArbitrageConfig>) -> Result<Self> {
        let provider = Arc::new(Provider::<Http>::try_from(&config.rpc_url)?);
        
        let aave_client = Arc::new(AaveClient::new(config.clone()).await?);
        
        let private_key = config.private_key.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Private key required for execution"))?;
        
        let flashbots_client = Arc::new(FlashbotsClient::new(
            &config.rpc_url,
            private_key,
            config.flashbots_url.clone(),
            config.chain_id,
        )?);
        
        let execution_validator = Arc::new(tokio::sync::RwLock::new(ExecutionValidator::new(
            &config.rpc_url,
            private_key,
            config.chain_id,
        ).await?));
        
        // Create secure registry for DEX integration
        let secure_registry = Arc::new(
            SecureRegistryManager::new(config.chain_id, config.rpc_url.clone()).await?
        );
        
        // Initialize REAL DEX integration with wallet signer - NO MORE MOCKS!
        let dex_integration = Arc::new(tokio::sync::RwLock::new(
            RealDexIntegration::new_with_signer(provider.clone(), secure_registry.clone(), private_key)?
        ));
        
        info!("ExecutionEngine initialized with REAL DEX integration, MEV protection and validation");
        
        Ok(Self {
            config,
            aave_client,
            flashbots_client,
            execution_validator,
            dex_integration,
            provider,
        })
    }
    
    /// Execute a strategy using the specified mode
    pub async fn execute_strategy(
        &self,
        strategy_result: &StrategyResult,
        execution_mode: ExecutionMode,
    ) -> Result<ExecutionResult> {
        let start_time = std::time::Instant::now();
        
        // Pre-execution validation
        // Create a simple flash loan strategy for validation
        let strategy = compound_arb::CompoundArbitrage::new(
            rust_decimal::Decimal::from(10),
            15,
            self.config.aave_pool_address,
        );
        let mut validator = self.execution_validator.write().await;
        let validation = validator
            .validate_opportunity(&strategy_result.opportunity, &strategy).await?;
        
        if !validation.is_valid {
            return Ok(ExecutionResult {
                success: false,
                transaction_hash: None,
                profit_usd: 0.0,
                gas_cost_usd: 0.0,
                execution_time_ms: start_time.elapsed().as_millis() as u64,
                error: Some(format!("Validation failed: {:?}", validation.failure_reasons)),
            });
        }
        
        // Execute based on mode
        let result = match execution_mode {
            ExecutionMode::FlashLoan => {
                self.execute_flash_loan_strategy(strategy_result).await?
            }
            ExecutionMode::Capital => {
                self.execute_capital_strategy(strategy_result).await?
            }
        };
        
        Ok(ExecutionResult {
            success: result.success,
            transaction_hash: result.transaction_hash,
            profit_usd: result.profit_usd,
            gas_cost_usd: result.gas_cost_usd,
            execution_time_ms: start_time.elapsed().as_millis() as u64,
            error: result.error,
        })
    }
    
    async fn execute_flash_loan_strategy(&self, strategy_result: &StrategyResult) -> Result<ExecutionResult> {
        info!("Executing flash loan arbitrage strategy for {} tokens", strategy_result.token_path.len());
        
        // CRITICAL: Validate path with REAL DEX quotes before execution
        let path_valid = self.validate_with_real_dex_quotes(strategy_result).await?;
        if !path_valid {
            warn!("Path validation failed with real DEX quotes - aborting to prevent losses");
            return Ok(ExecutionResult {
                success: false,
                transaction_hash: None,
                profit_usd: 0.0,
                gas_cost_usd: 0.0,
                execution_time_ms: 0,
                error: Some("Real DEX quote validation failed".to_string()),
            });
        }
        
        // Build flash loan request
        let flash_loan_request = self.build_flash_loan_request(strategy_result).await?;
        
        // Validate execution before proceeding
        let mut validator = self.execution_validator.write().await;
        let validation = validator
            .validate_execution(strategy_result)
            .await?;
        
        if !validation.is_valid {
            warn!("Flash loan execution validation failed: {}", validation.error_reason.unwrap_or_default());
            return Ok(ExecutionResult {
                success: false,
                transaction_hash: None,
                profit_usd: 0.0,
                gas_cost_usd: 0.0,
                execution_time_ms: 0,
                error: Some("Execution validation failed".to_string()),
            });
        }
        
        // Execute flash loan via Aave V3
        match self.aave_client.execute_flash_loan(&flash_loan_request).await {
            Ok(tx_hash) => {
                info!("Flash loan executed successfully: {:?}", tx_hash);
                
                // Calculate actual costs and profits
                let gas_cost_usd = self.calculate_gas_cost(validation.gas_estimate).await?;
                let net_profit = strategy_result.expected_profit_usd - gas_cost_usd;
                
                Ok(ExecutionResult {
                    success: true,
                    transaction_hash: Some(tx_hash),
                    profit_usd: net_profit,
                    gas_cost_usd,
                    execution_time_ms: 0, // Set by caller
                    error: None,
                })
            }
            Err(e) => {
                error!("Flash loan execution failed: {}", e);
                Ok(ExecutionResult {
                    success: false,
                    transaction_hash: None,
                    profit_usd: 0.0,
                    gas_cost_usd: 0.0,
                    execution_time_ms: 0,
                    error: Some(format!("Flash loan failed: {}", e)),
                })
            }
        }
    }
    
    async fn execute_capital_strategy(&self, strategy_result: &StrategyResult) -> Result<ExecutionResult> {
        info!("Executing capital-based arbitrage strategy");
        
        // TODO: Implement capital-based execution using existing wallet balance
        // This would build direct swap transactions and execute via Flashbots
        
        Ok(ExecutionResult {
            success: false,
            transaction_hash: None,
            profit_usd: 0.0,
            gas_cost_usd: 0.0,
            execution_time_ms: 0,
            error: Some("Capital execution not yet implemented".to_string()),
        })
    }
    
    /// Build flash loan request from strategy result
    async fn build_flash_loan_request(&self, strategy_result: &StrategyResult) -> Result<FlashLoanRequest> {
        // Get the input token (first in path) and amount
        let input_token = strategy_result.token_path.first()
            .ok_or_else(|| anyhow::anyhow!("Token path cannot be empty"))?;
        
        // Convert amount to U256 (assuming 18 decimals for simplicity)
        let amount_wei = U256::from_dec_str(&format!("{:.0}", strategy_result.amount_in * 1e18))
            .map_err(|e| anyhow::anyhow!("Failed to convert amount: {}", e))?;
        
        // Build arbitrage execution parameters
        let params = self.build_arbitrage_params(strategy_result).await?;
        
        // The receiver should be our arbitrage contract (for now, use wallet address)
        let receiver_address = self.get_arbitrage_contract_address().await?;
        
        Ok(FlashLoanRequest {
            asset: *input_token,
            amount: amount_wei,
            strategy: strategy_result.strategy_type.to_string(),
            params,
            receiver_address,
        })
    }
    
    /// Build arbitrage execution parameters (ABI-encoded calldata)
    async fn build_arbitrage_params(&self, strategy_result: &StrategyResult) -> Result<Bytes> {
        // For now, encode basic parameters as bytes
        // In production, this would be proper ABI encoding for the arbitrage contract
        let params_string = format!(
            "arbitrage:{}:{}:{}",
            strategy_result.strategy_type,
            strategy_result.token_path.len(),
            strategy_result.expected_profit_usd
        );
        
        Ok(Bytes::from(params_string.into_bytes()))
    }
    
    /// Get arbitrage contract address (placeholder)
    async fn get_arbitrage_contract_address(&self) -> Result<Address> {
        // TODO: Deploy and return actual arbitrage contract address
        // For now, return the wallet address as a placeholder
        let private_key = self.config.private_key.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Private key required"))?;
        let wallet: LocalWallet = private_key.parse()?;
        Ok(wallet.address())
    }
    
    /// Validate arbitrage path with REAL DEX quotes - NO MOCKS
    async fn validate_with_real_dex_quotes(&self, strategy_result: &StrategyResult) -> Result<bool> {
        info!("Validating arbitrage path with REAL DEX quotes");
        
        if strategy_result.token_path.len() < 2 {
            error!("Invalid token path length: {}", strategy_result.token_path.len());
            return Ok(false);
        }
        
        // Get starting amount (convert to Wei)
        let amount_in = U256::from_dec_str(&format!("{:.0}", 1e18))
            .unwrap_or(U256::from(1_000_000_000_000_000_000u128)); // 1 token default
        
        let mut dex_integration = self.dex_integration.write().await;
        
        // Validate the entire path with real liquidity
        let path_valid = dex_integration.validate_arbitrage_path(
            strategy_result.token_path.clone(),
            amount_in
        ).await?;
        
        if !path_valid {
            warn!("Arbitrage path validation failed - no profit or insufficient liquidity");
            return Ok(false);
        }
        
        // Double-check each hop for safety
        let mut current_amount = amount_in;
        for i in 0..strategy_result.token_path.len() - 1 {
            let token_in = strategy_result.token_path[i];
            let token_out = strategy_result.token_path[i + 1];
            
            match dex_integration.find_best_quote(token_in, token_out, current_amount).await {
                Ok(quote) => {
                    debug!("Hop {}: {} -> {} (impact: {:.2}%)", 
                           i, current_amount, quote.amount_out, quote.price_impact);
                    
                    // Reject if price impact too high
                    if quote.price_impact > 5.0 {
                        warn!("Price impact too high at hop {}: {:.2}%", i, quote.price_impact);
                        return Ok(false);
                    }
                    
                    current_amount = quote.amount_out;
                }
                Err(e) => {
                    error!("Failed to get real quote for hop {}: {}", i, e);
                    return Ok(false);
                }
            }
        }
        
        // Verify profitability
        let profit_ratio = if amount_in > U256::zero() {
            current_amount.as_u128() as f64 / amount_in.as_u128() as f64
        } else {
            0.0
        };
        
        // Require at least 0.5% profit after fees
        if profit_ratio <= 1.005 {
            warn!("Insufficient profit with real quotes: ratio = {:.4}", profit_ratio);
            return Ok(false);
        }
        
        info!("✅ Path validated with REAL DEX quotes: profit ratio = {:.4}", profit_ratio);
        Ok(true)
    }
    
    /// Calculate gas cost in USD
    async fn calculate_gas_cost(&self, gas_estimate: u64) -> Result<f64> {
        // Get current gas price
        let gas_price = self.provider.get_gas_price().await?;
        let gas_price_gwei = gas_price.as_u64() as f64 / 1e9;
        
        // Calculate cost in native token
        let gas_cost_native = gas_price_gwei * gas_estimate as f64 * 1e-9;
        
        // CRITICAL FIX: Use live price oracle instead of hardcoded $0.80
        // Get live MATIC price from price oracle
        let provider = Provider::<Http>::try_from(&self.config.rpc_url)?;
        let provider = Arc::new(provider);
        
        // Create secure registry for price oracle
        let secure_registry = Arc::new(
            SecureRegistryManager::new(self.config.chain_id, self.config.rpc_url.clone()).await?
        );
        let mut price_oracle = crate::price_oracle::LivePriceOracle::new(provider, secure_registry);
        
        let native_price_usd = match price_oracle.get_live_matic_price().await {
            Ok(price) => {
                debug!("Using live MATIC price: ${:.4}", price);
                price
            }
            Err(e) => {
                warn!("Failed to get live MATIC price, using conservative fallback: {}", e);
                1.0 // Conservative fallback, NOT the dangerous $0.80
            }
        };
        
        let gas_cost_usd = gas_cost_native * native_price_usd;
        
        debug!("Gas cost calculation: {} gas @ {:.2} gwei = {:.6} MATIC = ${:.4}", 
               gas_estimate, gas_price_gwei, gas_cost_native, gas_cost_usd);
        
        Ok(gas_cost_usd)
    }
}