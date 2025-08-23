use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::env;
use std::path::Path;
use ethers::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbitrageConfig {
    // Execution settings
    pub execution_mode: String,  // "flash_loan" or "capital"
    pub simulation_required: bool,
    pub flash_loans_enabled: bool,
    
    // Profitability thresholds
    pub min_profit_usd: f64,
    pub min_profit_percentage: f64,
    pub min_flash_loan_profit_usd: f64,
    pub min_compound_profit_usd: f64,
    pub max_gas_cost_usd: f64,
    pub min_confidence_score: f64,
    
    // Risk management
    pub max_opportunity_age_ms: u64,
    pub max_slippage_percentage: f64,
    pub max_capital_percentage: f64,
    pub position_size_usd: f64,
    
    // Network settings
    pub rpc_url: String,
    pub private_key: Option<String>,
    pub gas_price_gwei: f64,
    // REMOVED: matic_price_usd field entirely - CRITICAL SAFETY FIX
    // This hardcoded value was causing production losses - now uses LivePriceOracle
    
    // Flash loan settings
    pub aave_pool_address: Address,
    pub flash_loan_fee_percentage: f64,
    
    // Compound arbitrage settings
    pub compound_enabled: bool,
    pub max_token_path_length: usize,
    pub compound_confidence_threshold: f64,
    
    // DEX settings
    pub quickswap_router: Address,
    pub sushiswap_router: Address,
    pub uniswap_v3_router: Address,
    
    // Token addresses
    pub wmatic_address: Address,
    pub usdc_address: Address,
    pub usdt_address: Address,
    pub weth_address: Address,
    
    // MEV protection settings
    pub mev_protection_enabled: bool,
    pub flashbots_relay_url: String,
    pub flashbots_url: Option<String>,
    pub flashbots_signing_key: Option<String>,
    pub private_mempool_threshold_usd: f64,
    pub chain_id: u64,
    
    // Strategy-specific settings
    pub simple_strategy: SimpleStrategyConfig,
    pub triangular_strategy: TriangularStrategyConfig,
    pub compound_strategy: CompoundStrategyConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleStrategyConfig {
    pub enabled: bool,
    pub min_profit_usd: f64,
    pub max_slippage: f64,
    pub min_liquidity_usd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriangularStrategyConfig {
    pub enabled: bool,
    pub min_profit_usd: f64,
    pub max_slippage: f64,
    pub max_hops: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompoundStrategyConfig {
    pub enabled: bool,
    pub min_profit_usd: f64,
    pub max_slippage: f64,
    pub max_path_length: usize,
    pub min_confidence: f64,
    pub path_discovery_depth: usize,
}

impl Default for ArbitrageConfig {
    fn default() -> Self {
        Self {
            // Execution settings
            execution_mode: "flash_loan".to_string(),
            simulation_required: true,
            flash_loans_enabled: true,
            
            // Profitability thresholds
            min_profit_usd: 10.0,
            min_profit_percentage: 0.005, // 0.5%
            min_flash_loan_profit_usd: 20.0,
            min_compound_profit_usd: 50.0,
            max_gas_cost_usd: 5.0,
            min_confidence_score: 0.8,
            
            // Risk management
            max_opportunity_age_ms: 5000, // 5 seconds
            max_slippage_percentage: 0.01, // 1%
            max_capital_percentage: 0.5, // 50% of balance
            position_size_usd: 1000.0,
            
            // Network settings (Polygon mainnet)
            rpc_url: "https://polygon-rpc.com".to_string(),
            private_key: None,
            gas_price_gwei: 30.0,
            // REMOVED: matic_price_usd hardcoded value - now uses live price oracle
            // This was CRITICAL SAFETY FIX - hardcoded $0.80 was causing massive losses
            
            // Flash loan settings (Aave V3 on Polygon)
            aave_pool_address: "0x794a61358D6845594F94dc1DB02A252b5b4814aD".parse().unwrap(),
            flash_loan_fee_percentage: 0.0009, // 0.09%
            
            // Compound arbitrage settings
            compound_enabled: true,
            max_token_path_length: 15,
            compound_confidence_threshold: 0.85,
            
            // DEX addresses (Polygon)
            quickswap_router: "0xa5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff".parse().unwrap(),
            sushiswap_router: "0x1b02dA8Cb0d097eB8D57A175b88c7D8b47997506".parse().unwrap(),
            uniswap_v3_router: "0xE592427A0AEce92De3Edee1F18E0157C05861564".parse().unwrap(),
            
            // Token addresses (Polygon)
            wmatic_address: "0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270".parse().unwrap(),
            usdc_address: "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174".parse().unwrap(),
            usdt_address: "0xc2132D05D31c914a87C6611C10748AEb04B58e8F".parse().unwrap(),
            weth_address: "0x7ceB23fD6bC0adD59E62ac25578270cFf1b9f619".parse().unwrap(),
            
            // MEV protection settings
            mev_protection_enabled: true,
            flashbots_relay_url: "https://relay.flashbots.net".to_string(),
            flashbots_url: Some("https://relay.flashbots.net".to_string()),
            flashbots_signing_key: None,
            private_mempool_threshold_usd: 100.0, // Use private mempool for trades >$100
            chain_id: 137, // Polygon mainnet
            
            // Strategy-specific settings
            simple_strategy: SimpleStrategyConfig {
                enabled: true,
                min_profit_usd: 5.0,
                max_slippage: 0.005, // 0.5%
                min_liquidity_usd: 1000.0,
            },
            
            triangular_strategy: TriangularStrategyConfig {
                enabled: true,
                min_profit_usd: 15.0,
                max_slippage: 0.01, // 1%
                max_hops: 3,
            },
            
            compound_strategy: CompoundStrategyConfig {
                enabled: true,
                min_profit_usd: 50.0,
                max_slippage: 0.02, // 2%
                max_path_length: 15,
                min_confidence: 0.8,
                path_discovery_depth: 3,
            },
        }
    }
}

impl ArbitrageConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self> {
        let mut config = Self::default();
        
        // Execution settings
        if let Ok(mode) = env::var("EXECUTION_MODE") {
            config.execution_mode = mode;
        }
        
        if let Ok(sim) = env::var("SIMULATION_REQUIRED") {
            config.simulation_required = sim.parse().unwrap_or(true);
        }
        
        if let Ok(flash) = env::var("FLASH_LOANS_ENABLED") {
            config.flash_loans_enabled = flash.parse().unwrap_or(true);
        }
        
        // Profitability thresholds
        if let Ok(profit) = env::var("MIN_PROFIT_USD") {
            config.min_profit_usd = profit.parse().unwrap_or(config.min_profit_usd);
        }
        
        if let Ok(profit_pct) = env::var("MIN_PROFIT_PERCENTAGE") {
            config.min_profit_percentage = profit_pct.parse().unwrap_or(config.min_profit_percentage);
        }
        
        if let Ok(flash_profit) = env::var("MIN_FLASH_LOAN_PROFIT_USD") {
            config.min_flash_loan_profit_usd = flash_profit.parse().unwrap_or(config.min_flash_loan_profit_usd);
        }
        
        if let Ok(compound_profit) = env::var("MIN_COMPOUND_PROFIT_USD") {
            config.min_compound_profit_usd = compound_profit.parse().unwrap_or(config.min_compound_profit_usd);
        }
        
        if let Ok(gas_cost) = env::var("MAX_GAS_COST_USD") {
            config.max_gas_cost_usd = gas_cost.parse().unwrap_or(config.max_gas_cost_usd);
        }
        
        // Risk management
        if let Ok(age) = env::var("MAX_OPPORTUNITY_AGE_MS") {
            config.max_opportunity_age_ms = age.parse().unwrap_or(config.max_opportunity_age_ms);
        }
        
        if let Ok(slippage) = env::var("MAX_SLIPPAGE_PERCENTAGE") {
            config.max_slippage_percentage = slippage.parse().unwrap_or(config.max_slippage_percentage);
        }
        
        if let Ok(capital_pct) = env::var("MAX_CAPITAL_PERCENTAGE") {
            config.max_capital_percentage = capital_pct.parse().unwrap_or(config.max_capital_percentage);
        }
        
        // Network settings
        if let Ok(rpc) = env::var("RPC_URL") {
            config.rpc_url = rpc;
        }
        
        if let Ok(key) = env::var("PRIVATE_KEY") {
            config.private_key = Some(key);
        }
        
        if let Ok(gas_price) = env::var("GAS_PRICE_GWEI") {
            config.gas_price_gwei = gas_price.parse().unwrap_or(config.gas_price_gwei);
        }
        
        // REMOVED: MATIC_PRICE_USD environment variable - CRITICAL SAFETY FIX
        // This hardcoded price mechanism was causing production losses
        // Now using LivePriceOracle for real-time pricing
        
        // Compound arbitrage settings
        if let Ok(compound) = env::var("COMPOUND_ENABLED") {
            config.compound_enabled = compound.parse().unwrap_or(config.compound_enabled);
        }
        
        // MEV protection settings
        if let Ok(mev) = env::var("MEV_PROTECTION_ENABLED") {
            config.mev_protection_enabled = mev.parse().unwrap_or(config.mev_protection_enabled);
        }
        
        if let Ok(relay) = env::var("FLASHBOTS_RELAY_URL") {
            config.flashbots_relay_url = relay;
        }
        
        if let Ok(key) = env::var("FLASHBOTS_SIGNING_KEY") {
            config.flashbots_signing_key = Some(key);
        }
        
        if let Ok(threshold) = env::var("PRIVATE_MEMPOOL_THRESHOLD_USD") {
            config.private_mempool_threshold_usd = threshold.parse().unwrap_or(config.private_mempool_threshold_usd);
        }
        
        if let Ok(path_len) = env::var("MAX_TOKEN_PATH_LENGTH") {
            config.max_token_path_length = path_len.parse().unwrap_or(config.max_token_path_length);
        }
        
        // Contract addresses (allow override)
        if let Ok(aave) = env::var("AAVE_POOL_ADDRESS") {
            config.aave_pool_address = aave.parse()?;
        }
        
        // Strategy-specific overrides
        if let Ok(simple_profit) = env::var("SIMPLE_MIN_PROFIT_USD") {
            config.simple_strategy.min_profit_usd = simple_profit.parse().unwrap_or(config.simple_strategy.min_profit_usd);
        }
        
        if let Ok(triangular_profit) = env::var("TRIANGULAR_MIN_PROFIT_USD") {
            config.triangular_strategy.min_profit_usd = triangular_profit.parse().unwrap_or(config.triangular_strategy.min_profit_usd);
        }
        
        if let Ok(compound_profit) = env::var("COMPOUND_MIN_PROFIT_USD") {
            config.compound_strategy.min_profit_usd = compound_profit.parse().unwrap_or(config.compound_strategy.min_profit_usd);
        }
        
        Ok(config)
    }
    
    /// Load configuration from file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }
    
    /// Save configuration to file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
    
    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        // Validate execution mode
        if !matches!(self.execution_mode.as_str(), "flash_loan" | "capital") {
            return Err(anyhow::anyhow!("Invalid execution mode: {}", self.execution_mode));
        }
        
        // Validate profit thresholds
        if self.min_profit_usd <= 0.0 {
            return Err(anyhow::anyhow!("min_profit_usd must be positive"));
        }
        
        if self.min_profit_percentage <= 0.0 || self.min_profit_percentage >= 1.0 {
            return Err(anyhow::anyhow!("min_profit_percentage must be between 0 and 1"));
        }
        
        // Validate slippage
        if self.max_slippage_percentage <= 0.0 || self.max_slippage_percentage >= 1.0 {
            return Err(anyhow::anyhow!("max_slippage_percentage must be between 0 and 1"));
        }
        
        // Validate capital percentage
        if self.max_capital_percentage <= 0.0 || self.max_capital_percentage > 1.0 {
            return Err(anyhow::anyhow!("max_capital_percentage must be between 0 and 1"));
        }
        
        // Validate private key if in capital mode
        if self.execution_mode == "capital" && self.private_key.is_none() {
            return Err(anyhow::anyhow!("Private key required for capital execution mode"));
        }
        
        // Validate compound settings
        if self.compound_enabled && self.max_token_path_length < 3 {
            return Err(anyhow::anyhow!("max_token_path_length must be at least 3 for compound arbitrage"));
        }
        
        Ok(())
    }
    
    /// Get execution mode enum
    pub fn get_execution_mode(&self) -> execution::ExecutionMode {
        match self.execution_mode.as_str() {
            "flash_loan" => execution::ExecutionMode::FlashLoan,
            "capital" => execution::ExecutionMode::Capital,
            _ => execution::ExecutionMode::FlashLoan, // Default
        }
    }
}

// Import execution module for the enum
use crate::execution;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_config() {
        let config = ArbitrageConfig::default();
        assert!(config.validate().is_ok());
    }
    
    #[test]
    fn test_config_validation() {
        let mut config = ArbitrageConfig::default();
        
        // Test invalid execution mode
        config.execution_mode = "invalid".to_string();
        assert!(config.validate().is_err());
        
        // Test invalid profit threshold
        config = ArbitrageConfig::default();
        config.min_profit_usd = -1.0;
        assert!(config.validate().is_err());
        
        // Test capital mode without private key
        config = ArbitrageConfig::default();
        config.execution_mode = "capital".to_string();
        config.private_key = None;
        assert!(config.validate().is_err());
    }
}