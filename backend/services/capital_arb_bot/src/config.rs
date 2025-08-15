use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub rpc_url: String,
    pub private_key: String,
    pub chain_id: u64,
    pub min_profit_usd: f64,
    pub max_gas_price_gwei: f64,
    pub max_opportunity_age_ms: u64,
    pub simulation_mode: bool,
    pub max_trade_percentage: f64,  // Max % of wallet balance to use per trade
    pub slippage_tolerance: f64,    // Max slippage tolerance (e.g., 0.005 = 0.5%)
}

impl Config {
    pub fn from_env() -> Result<Self> {
        // Load .env file
        dotenv::dotenv().ok();
        Ok(Self {
            rpc_url: env::var("POLYGON_RPC_URL")
                .unwrap_or_else(|_| "https://polygon-mainnet.public.blastapi.io".to_string()),
            
            private_key: env::var("PRIVATE_KEY")
                .context("PRIVATE_KEY environment variable not set")?,
            
            chain_id: env::var("CHAIN_ID")
                .unwrap_or_else(|_| "137".to_string())
                .parse()
                .context("Invalid CHAIN_ID")?,
            
            min_profit_usd: env::var("MIN_PROFIT_USD")
                .unwrap_or_else(|_| "5.0".to_string())
                .parse()
                .context("Invalid MIN_PROFIT_USD")?,
            
            max_gas_price_gwei: env::var("MAX_GAS_PRICE_GWEI")
                .unwrap_or_else(|_| "100.0".to_string())
                .parse()
                .context("Invalid MAX_GAS_PRICE_GWEI")?,
            
            max_opportunity_age_ms: env::var("MAX_OPPORTUNITY_AGE_MS")
                .unwrap_or_else(|_| "5000".to_string())
                .parse()
                .context("Invalid MAX_OPPORTUNITY_AGE_MS")?,
            
            simulation_mode: env::var("SIMULATION_MODE")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .context("Invalid SIMULATION_MODE")?,
            
            max_trade_percentage: env::var("MAX_TRADE_PERCENTAGE")
                .unwrap_or_else(|_| "0.5".to_string())
                .parse()
                .context("Invalid MAX_TRADE_PERCENTAGE")?,
            
            slippage_tolerance: env::var("SLIPPAGE_TOLERANCE")
                .unwrap_or_else(|_| "0.005".to_string())
                .parse()
                .context("Invalid SLIPPAGE_TOLERANCE")?,
        })
    }

    pub fn from_file(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .context("Failed to read config file")?;
        serde_json::from_str(&content)
            .context("Failed to parse config file")
    }
}