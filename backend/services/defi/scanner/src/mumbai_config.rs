use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use rust_decimal::Decimal;
use crate::config::{ScannerConfig, ExchangeConfig, ArbitrageConfig, NetworkConfig, MonitoringConfig};

/// Mumbai testnet configuration for DeFi arbitrage scanner
pub struct MumbaiConfig;

impl MumbaiConfig {
    /// Create scanner configuration for Mumbai testnet
    pub fn create_mumbai_config() -> Result<ScannerConfig> {
        dotenv::dotenv().ok();

        // Mumbai testnet exchanges with lower liquidity requirements
        let exchanges = vec![
            ExchangeConfig {
                name: "quickswap".to_string(),
                enabled: true,
                rpc_url: "https://polygon-mumbai.g.alchemy.com/v2/demo".to_string(),
                factory_address: "0x5757371414417b8C6CAad45bAeF941aBc7d3Ab32".to_string(), // QuickSwap V2 Factory
                router_address: "0xa5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff".to_string(), // QuickSwap Router
                fee_percentage: Decimal::new(3, 3), // 0.3%
                min_liquidity_usd: Decimal::new(1000, 0), // Lower for testnet
                max_pools: 500,
            },
            ExchangeConfig {
                name: "sushiswap".to_string(),
                enabled: true,
                rpc_url: "https://polygon-mumbai.g.alchemy.com/v2/demo".to_string(),
                factory_address: "0xc35DADB65012eC5796536bD9864eD8773aBc74C4".to_string(), // SushiSwap Factory
                router_address: "0x1b02dA8Cb0d097eB8D57A175b88c7D8b47997506".to_string(), // SushiSwap Router
                fee_percentage: Decimal::new(3, 3), // 0.3%
                min_liquidity_usd: Decimal::new(500, 0), // Even lower for testnet
                max_pools: 300,
            },
            ExchangeConfig {
                name: "uniswap_v3".to_string(),
                enabled: true,
                rpc_url: "https://polygon-mumbai.g.alchemy.com/v2/demo".to_string(),
                factory_address: "0x1F98431c8aD98523631AE4a59f267346ea31F984".to_string(), // Uniswap V3 Factory
                router_address: "0xE592427A0AEce92De3Edee1F18E0157C05861564".to_string(), // Uniswap V3 Router
                fee_percentage: Decimal::new(3, 3), // Variable fees
                min_liquidity_usd: Decimal::new(100, 0), // Very low for testnet
                max_pools: 200,
            },
        ];

        // Mumbai arbitrage configuration with lower thresholds for testing
        let arbitrage = ArbitrageConfig {
            min_profit_usd: Decimal::new(1, 0), // $1 minimum for testnet testing
            min_profit_percentage: Decimal::new(5, 3), // 0.5% minimum
            max_gas_cost_usd: Decimal::new(10, 0), // $10 max for testnet
            max_slippage_percentage: Decimal::new(1, 1), // 1% max slippage
            confidence_threshold: 0.7, // Lower threshold for testing
            opportunity_timeout_ms: 10000, // 10 seconds for testnet
        };

        // Mumbai network configuration
        let network = NetworkConfig {
            chain_id: 80001, // Mumbai testnet
            rpc_url: "https://polygon-mumbai.g.alchemy.com/v2/demo".to_string(),
            gas_price_gwei: Some(Decimal::new(1, 0)), // 1 gwei for testnet
            max_gas_limit: 500_000,
            block_confirmation_count: 1,
        };

        let monitoring = MonitoringConfig {
            metrics_port: 9091, // Different port for testnet
            log_level: "debug".to_string(), // More verbose for testing
            relay_socket_path: "/tmp/alphapulse/mumbai_relay.sock".to_string(),
        };

        Ok(ScannerConfig {
            exchanges,
            arbitrage,
            network,
            monitoring,
        })
    }
    
    /// Mumbai token addresses for testing
    pub fn get_mumbai_tokens() -> HashMap<&'static str, &'static str> {
        let mut tokens = HashMap::new();
        
        // Major test tokens on Mumbai
        tokens.insert("USDC", "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174"); // USDC.e
        tokens.insert("USDC_NATIVE", "0x3c499c542cef5e3811e1192ce70d8cc03d5c3359"); // Native USDC
        tokens.insert("USDT", "0xc2132D05D31c914a87C6611C10748AeB04B58e8F"); // USDT
        tokens.insert("WMATIC", "0x9c3C9283D3e44854697Cd22D3Faa240Cfb032889"); // Wrapped MATIC
        tokens.insert("WETH", "0xA6FA4fB5f76172d178d61B04b0ecd319C5d1C0aa"); // Wrapped ETH
        tokens.insert("DAI", "0x001B3B4d0F3714Ca98ba10F6042DaEbF0B1B7b6F"); // DAI
        tokens.insert("LINK", "0x326C977E6efc84E512bB9C30f76E30c160eD06FB"); // Chainlink
        
        tokens
    }
    
    /// Contract addresses for Mumbai deployment
    pub fn get_contract_addresses() -> MumbaiContracts {
        MumbaiContracts {
            aave_pool: "0x9198F13B08E299d85E096929fA9781A1E3d5d827".to_string(),
            // These will be filled after deployment
            huff_extreme: None,
            huff_mev: None,
            huff_ultra: None,
            // Placeholder for deployment
            flash_arbitrage_solidity: None,
        }
    }
    
    /// Create environment variables for Mumbai testing
    pub fn setup_mumbai_env() -> Vec<(&'static str, &'static str)> {
        vec![
            ("CHAIN_ID", "80001"),
            ("RPC_URL", "https://rpc-mumbai.maticvigil.com"),
            ("MIN_PROFIT_USD", "1"),
            ("GAS_PRICE_GWEI", "1"),
            ("RUST_LOG", "debug"),
            ("SCANNER_MODE", "mumbai_testnet"),
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MumbaiContracts {
    pub aave_pool: String,
    pub huff_extreme: Option<String>,
    pub huff_mev: Option<String>,
    pub huff_ultra: Option<String>,
    pub flash_arbitrage_solidity: Option<String>,
}

impl MumbaiContracts {
    /// Update contract addresses after deployment
    pub fn update_deployed_contracts(
        &mut self,
        huff_extreme: String,
        huff_mev: String,
        huff_ultra: String,
        solidity_baseline: String,
    ) {
        self.huff_extreme = Some(huff_extreme);
        self.huff_mev = Some(huff_mev);
        self.huff_ultra = Some(huff_ultra);
        self.flash_arbitrage_solidity = Some(solidity_baseline);
    }
    
    /// Check if all contracts are deployed
    pub fn all_deployed(&self) -> bool {
        self.huff_extreme.is_some() 
            && self.huff_mev.is_some() 
            && self.huff_ultra.is_some() 
            && self.flash_arbitrage_solidity.is_some()
    }
}

/// Mumbai-specific scanner optimizations
pub struct MumbaiOptimizations;

impl MumbaiOptimizations {
    /// Adjust gas calculations for Mumbai testnet
    pub fn adjust_for_mumbai_gas() -> Decimal {
        // Mumbai typically has very low gas prices
        Decimal::new(1, 0) // 1 gwei baseline
    }
    
    /// Calculate minimum viable arbitrage for Mumbai
    pub fn min_viable_arbitrage_mumbai() -> Decimal {
        // With Huff contracts, even tiny arbitrages become viable on testnet
        // Gas cost ~3,800 gas * 1 gwei = 0.0000038 MATIC = ~$0.000003
        Decimal::new(1, 2) // $0.01 minimum
    }
    
    /// Testnet-specific monitoring intervals
    pub fn get_scan_intervals() -> (u64, u64, u64) {
        (
            50,   // 50ms for opportunity detection (faster for testing)
            1000, // 1s for pool updates
            5000, // 5s for gas price updates
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_mumbai_config_creation() {
        let config = MumbaiConfig::create_mumbai_config().unwrap();
        
        assert_eq!(config.network.chain_id, 80001);
        assert!(config.exchanges.len() >= 2);
        assert!(config.arbitrage.min_profit_usd < Decimal::new(10, 0));
    }
    
    #[test]
    fn test_mumbai_tokens() {
        let tokens = MumbaiConfig::get_mumbai_tokens();
        
        assert!(tokens.contains_key("USDC"));
        assert!(tokens.contains_key("WMATIC"));
        assert!(tokens.contains_key("WETH"));
    }
    
    #[test]
    fn test_mumbai_optimizations() {
        let min_arb = MumbaiOptimizations::min_viable_arbitrage_mumbai();
        let gas_price = MumbaiOptimizations::adjust_for_mumbai_gas();
        
        assert!(min_arb < Decimal::new(1, 0)); // Less than $1
        assert!(gas_price < Decimal::new(10, 0)); // Less than 10 gwei
    }
}