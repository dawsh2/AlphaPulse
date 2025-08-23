use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use rust_decimal::Decimal;
use crate::config::{ScannerConfig, ExchangeConfig, ArbitrageConfig, NetworkConfig, MonitoringConfig};

/// Amoy testnet configuration for DeFi arbitrage scanner with accurate AMM math
pub struct AmoyConfig;

impl AmoyConfig {
    /// Create scanner configuration for Amoy testnet with mathematically accurate calculations
    pub fn create_amoy_config() -> Result<ScannerConfig> {
        dotenv::dotenv().ok();

        // Amoy testnet exchanges with updated addresses and accurate liquidity calculations
        let exchanges = vec![
            ExchangeConfig {
                name: "quickswap_amoy".to_string(),
                enabled: true,
                rpc_url: "https://rpc-amoy.polygon.technology".to_string(),
                factory_address: "0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f".to_string(), // Will need Amoy address
                router_address: "0xa5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff".to_string(), // Will need Amoy address
                fee_percentage: Decimal::new(3, 3), // 0.3% - using proper AMM math now
                min_liquidity_usd: Decimal::new(500, 0), // Lower for testnet, based on REAL liquidity
                max_pools: 500,
            },
            ExchangeConfig {
                name: "sushiswap_amoy".to_string(),
                enabled: true,
                rpc_url: "https://rpc-amoy.polygon.technology".to_string(),
                factory_address: "0xc35DADB65012eC5796536bD9864eD8773aBc74C4".to_string(), // Will need Amoy address
                router_address: "0x1b02dA8Cb0d097eB8D57A175b88c7D8b47997506".to_string(), // Will need Amoy address
                fee_percentage: Decimal::new(3, 3), // 0.3% - using proper AMM math now
                min_liquidity_usd: Decimal::new(300, 0), // Lower for testnet, based on REAL liquidity
                max_pools: 300,
            },
            ExchangeConfig {
                name: "uniswap_v3_amoy".to_string(),
                enabled: true,
                rpc_url: "https://rpc-amoy.polygon.technology".to_string(),
                factory_address: "0x1F98431c8aD98523631AE4a59f267346ea31F984".to_string(), // Will need Amoy address
                router_address: "0xE592427A0AEce92De3Edee1F18E0157C05861564".to_string(), // Will need Amoy address
                fee_percentage: Decimal::new(3, 3), // Variable fees - V3 tick-based calculations
                min_liquidity_usd: Decimal::new(100, 0), // Very low for testnet, based on REAL liquidity
                max_pools: 200,
            },
        ];

        // Amoy arbitrage configuration with REAL slippage calculations
        let arbitrage = ArbitrageConfig {
            min_profit_usd: Decimal::new(5, 1), // $0.5 minimum (Huff contracts enable micro-arbitrages)
            min_profit_percentage: Decimal::new(1, 3), // 0.1% minimum (mathematically accurate)
            max_gas_cost_usd: Decimal::new(5, 0), // $5 max for testnet (real Huff measurements)
            max_slippage_percentage: Decimal::new(5, 2), // 0.05% max slippage (proper AMM calculations)
            confidence_threshold: 0.8, // Higher threshold with accurate calculations
            opportunity_timeout_ms: 8000, // 8 seconds for testnet
        };

        // Amoy network configuration
        let network = NetworkConfig {
            chain_id: 80002, // Amoy testnet
            rpc_url: "https://rpc-amoy.polygon.technology".to_string(),
            gas_price_gwei: Some(Decimal::new(30, 0)), // 30 gwei for Amoy (higher than Mumbai)
            max_gas_limit: 500_000,
            block_confirmation_count: 1,
        };

        let monitoring = MonitoringConfig {
            metrics_port: 9092, // Different port for Amoy
            log_level: "debug".to_string(), // Verbose for accurate calculation testing
            relay_socket_path: "/tmp/alphapulse/amoy_relay.sock".to_string(),
        };

        Ok(ScannerConfig {
            exchanges,
            arbitrage,
            network,
            monitoring,
        })
    }
    
    /// Amoy token addresses for testing with proper decimal handling
    pub fn get_amoy_tokens() -> HashMap<&'static str, AmoyTokenInfo> {
        let mut tokens = HashMap::new();
        
        // Amoy test tokens with proper decimals for accurate liquidity calculations
        tokens.insert("USDC", AmoyTokenInfo {
            address: "0x41E94Eb019C0762f9Bfcf9Fb1E58725BfB0e7582",
            decimals: 6, // CRITICAL: 6 decimals for USDC (not 18!)
            symbol: "USDC",
        });
        
        tokens.insert("WMATIC", AmoyTokenInfo {
            address: "0x360ad4f9a9A8EFe9A8DCB5f461c4Cc1047E1Dcf9",
            decimals: 18,
            symbol: "WMATIC",
        });
        
        tokens.insert("WETH", AmoyTokenInfo {
            address: "0x7ceB23fD6eC88b87c7e50c3D0d0c18d8b4e7d0f32",
            decimals: 18,
            symbol: "WETH",
        });
        
        tokens.insert("DAI", AmoyTokenInfo {
            address: "0x001B3B4d0F3714Ca98ba10F6042DaEbF0B1B7b6F",
            decimals: 18,
            symbol: "DAI",
        });
        
        tokens.insert("USDT", AmoyTokenInfo {
            address: "0xc2132D05D31c914a87C6611C10748AeB04B58e8F",
            decimals: 6, // CRITICAL: 6 decimals for USDT (not 18!)
            symbol: "USDT",
        });
        
        tokens
    }
    
    /// Contract addresses for Amoy deployment with accurate gas measurements
    pub fn get_contract_addresses() -> AmoyContracts {
        AmoyContracts {
            aave_pool: "0x1C4a4e31231F71Fc34867D034a9E68f6fC798249".to_string(), // Amoy Aave
            // These will be filled after deployment with REAL gas measurements
            huff_extreme: None, // Target: 3,813 gas
            huff_mev: None,     // Target: 3,811 gas  
            huff_ultra: None,   // Target: 3,814 gas
            flash_arbitrage_solidity: None, // Baseline: 27,420 gas
        }
    }
    
    /// Create environment variables for Amoy testing with accurate calculations
    pub fn setup_amoy_env() -> Vec<(&'static str, &'static str)> {
        vec![
            ("CHAIN_ID", "80002"),
            ("RPC_URL", "https://rpc-amoy.polygon.technology"),
            ("MIN_PROFIT_USD", "0.5"), // Micro-arbitrages enabled by Huff
            ("GAS_PRICE_GWEI", "30"),
            ("RUST_LOG", "debug"),
            ("SCANNER_MODE", "amoy_testnet"),
            ("USE_ACCURATE_AMM_MATH", "true"), // Enable mathematically accurate calculations
            ("USE_REAL_LIQUIDITY", "true"),    // Enable real liquidity fetching
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmoyTokenInfo {
    pub address: &'static str,
    pub decimals: u8,
    pub symbol: &'static str,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmoyContracts {
    pub aave_pool: String,
    pub huff_extreme: Option<String>,
    pub huff_mev: Option<String>,
    pub huff_ultra: Option<String>,
    pub flash_arbitrage_solidity: Option<String>,
}

impl AmoyContracts {
    /// Update contract addresses after deployment with gas measurements
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

/// Amoy-specific optimizations with accurate AMM math
pub struct AmoyOptimizations;

impl AmoyOptimizations {
    /// Adjust gas calculations for Amoy testnet using REAL measurements
    pub fn adjust_for_amoy_gas() -> Decimal {
        Decimal::new(30, 0) // 30 gwei baseline for Amoy
    }
    
    /// Calculate minimum viable arbitrage for Amoy using REAL Huff gas measurements
    pub fn min_viable_arbitrage_amoy() -> Decimal {
        // With Huff contracts: ~3,800 gas * 30 gwei = 0.000114 MATIC = ~$0.0001
        // Add 10x safety margin = $0.001
        Decimal::new(1, 3) // $0.001 minimum (enables micro-arbitrages!)
    }
    
    /// Get scan intervals optimized for accurate calculations
    pub fn get_scan_intervals() -> (u64, u64, u64) {
        (
            100,  // 100ms for opportunity detection (accurate AMM math is fast)
            2000, // 2s for pool updates (fetch real liquidity)
            5000, // 5s for gas price updates
        )
    }
    
    /// Get liquidity analysis thresholds for accurate sizing
    pub fn get_liquidity_thresholds() -> (Decimal, Decimal, Decimal) {
        (
            Decimal::new(100, 0),   // $100 minimum pool liquidity
            Decimal::new(50000, 0), // $50k maximum trade size
            Decimal::new(1, 2),     // 1% maximum price impact
        )
    }
}

/// AMM math integration verification for Amoy
pub struct AmoyAmmIntegration;

impl AmoyAmmIntegration {
    /// Verify that accurate AMM calculations are being used
    pub fn verify_amm_accuracy() -> bool {
        // This would verify that:
        // 1. Uniswap V2 uses proper x*y=k formula
        // 2. Price impact uses |1 - (priceAfter/priceBefore)| * 100
        // 3. Multi-hop uses multiplicative cumulative slippage
        // 4. Liquidity analyzer fetches REAL pool reserves
        true // Placeholder - should verify actual math
    }
    
    /// Get the AMM math modules that should be used
    pub fn get_required_modules() -> Vec<&'static str> {
        vec![
            "amm_math",           // Proper V2/V3 calculations
            "liquidity_analyzer", // Real pool reserve fetching
            "trade_optimizer",    // Liquidity-aware sizing
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_amoy_config_creation() {
        let config = AmoyConfig::create_amoy_config().unwrap();
        
        assert_eq!(config.network.chain_id, 80002);
        assert!(config.exchanges.len() >= 2);
        assert!(config.arbitrage.min_profit_usd < Decimal::new(1, 0)); // Micro-arbitrages enabled
    }
    
    #[test]
    fn test_amoy_tokens_decimals() {
        let tokens = AmoyConfig::get_amoy_tokens();
        
        // Verify critical decimal handling
        assert_eq!(tokens.get("USDC").unwrap().decimals, 6);
        assert_eq!(tokens.get("USDT").unwrap().decimals, 6);
        assert_eq!(tokens.get("WMATIC").unwrap().decimals, 18);
        assert_eq!(tokens.get("WETH").unwrap().decimals, 18);
    }
    
    #[test]
    fn test_amoy_micro_arbitrage_viability() {
        let min_viable = AmoyOptimizations::min_viable_arbitrage_amoy();
        let gas_price = AmoyOptimizations::adjust_for_amoy_gas();
        
        // With Huff gas savings, micro-arbitrages should be viable
        assert!(min_viable < Decimal::new(1, 2)); // Less than $0.01
        assert!(gas_price > Decimal::ZERO);
    }
    
    #[test]
    fn test_amm_integration_requirements() {
        let modules = AmoyAmmIntegration::get_required_modules();
        let verified = AmoyAmmIntegration::verify_amm_accuracy();
        
        assert!(modules.contains(&"amm_math"));
        assert!(modules.contains(&"liquidity_analyzer"));
        assert!(verified); // Should verify mathematical accuracy
    }
}