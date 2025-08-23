use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use rust_decimal::Decimal;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannerConfig {
    pub exchanges: Vec<ExchangeConfig>,
    pub arbitrage: ArbitrageConfig,
    pub network: NetworkConfig,
    pub monitoring: MonitoringConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeConfig {
    pub name: String,
    pub enabled: bool,
    pub rpc_url: String,
    pub factory_address: String,
    pub router_address: String,
    pub fee_percentage: Decimal,
    pub min_liquidity_usd: Decimal,
    pub max_pools: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbitrageConfig {
    pub min_profit_usd: Decimal,
    pub min_profit_percentage: Decimal,
    pub max_gas_cost_usd: Decimal,
    pub max_slippage_percentage: Decimal,
    pub confidence_threshold: f64,
    pub opportunity_timeout_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub chain_id: u64,
    pub rpc_url: String,
    pub gas_price_gwei: Option<Decimal>,
    pub max_gas_limit: u64,
    pub block_confirmation_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    pub metrics_port: u16,
    pub log_level: String,
    pub relay_socket_path: String,
}

impl ScannerConfig {
    pub fn from_env() -> Result<Self> {
        dotenv::dotenv().ok();

        let exchanges = vec![
            ExchangeConfig {
                name: "uniswap_v2".to_string(),
                enabled: true,
                rpc_url: std::env::var("ALCHEMY_RPC_URL")
                    .context("ALCHEMY_RPC_URL required")?,
                factory_address: "0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f".to_string(),
                router_address: "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D".to_string(),
                fee_percentage: Decimal::new(3, 3), // 0.3%
                min_liquidity_usd: Decimal::new(0, 0), // No minimum
                max_pools: 1000,
            },
            ExchangeConfig {
                name: "uniswap_v3".to_string(),
                enabled: true,
                rpc_url: std::env::var("ALCHEMY_RPC_URL")
                    .context("ALCHEMY_RPC_URL required")?,
                factory_address: "0x1F98431c8aD98523631AE4a59f267346ea31F984".to_string(),
                router_address: "0xE592427A0AEce92De3Edee1F18E0157C05861564".to_string(),
                fee_percentage: Decimal::new(3, 3), // Variable fees
                min_liquidity_usd: Decimal::new(0, 0), // No minimum
                max_pools: 1000,
            },
            ExchangeConfig {
                name: "sushiswap".to_string(),
                enabled: true,
                rpc_url: std::env::var("ALCHEMY_RPC_URL")
                    .context("ALCHEMY_RPC_URL required")?,
                factory_address: "0xC0AEe478e3658e2610c5F7A4A2E1777cE9e4f2Ac".to_string(),
                router_address: "0xd9e1cE17f2641f24aE83637ab66a2cca9C378B9F".to_string(),
                fee_percentage: Decimal::new(3, 3), // 0.3%
                min_liquidity_usd: Decimal::new(0, 0), // No minimum
                max_pools: 500,
            },
        ];

        let arbitrage = ArbitrageConfig {
            min_profit_usd: std::env::var("MIN_PROFIT_USD")
                .unwrap_or("0".to_string())
                .parse::<Decimal>()
                .context("Invalid MIN_PROFIT_USD")?,
            min_profit_percentage: Decimal::new(0, 0), // 0% - no minimum
            max_gas_cost_usd: Decimal::new(1000, 0), // High limit - let profitability decide
            max_slippage_percentage: Decimal::new(100, 0), // 100% - let profitability decide
            confidence_threshold: 0.0, // Take any opportunity
            opportunity_timeout_ms: 5000,
        };

        let network = NetworkConfig {
            chain_id: 137, // Polygon
            rpc_url: std::env::var("ALCHEMY_RPC_URL")
                .context("ALCHEMY_RPC_URL required")?,
            gas_price_gwei: std::env::var("GAS_PRICE_GWEI")
                .ok()
                .and_then(|s| s.parse().ok()),
            max_gas_limit: 500_000,
            block_confirmation_count: 1,
        };

        let monitoring = MonitoringConfig {
            metrics_port: 9090,
            log_level: std::env::var("RUST_LOG")
                .unwrap_or("info".to_string()),
            relay_socket_path: "/tmp/alphapulse/relay.sock".to_string(),
        };

        Ok(ScannerConfig {
            exchanges,
            arbitrage,
            network,
            monitoring,
        })
    }

    pub fn enabled_exchanges(&self) -> Vec<&ExchangeConfig> {
        self.exchanges.iter().filter(|e| e.enabled).collect()
    }
}

impl Default for ScannerConfig {
    fn default() -> Self {
        ScannerConfig {
            exchanges: vec![
                ExchangeConfig {
                    name: "uniswap_v2".to_string(),
                    enabled: true,
                    rpc_url: "https://polygon-rpc.com".to_string(),
                    factory_address: "0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f".to_string(),
                    router_address: "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D".to_string(),
                    fee_percentage: Decimal::new(3, 3), // 0.3%
                    min_liquidity_usd: Decimal::new(0, 0), // $0 - no minimum liquidity required
                    max_pools: 100,
                },
            ],
            arbitrage: ArbitrageConfig {
                min_profit_usd: Decimal::new(0, 0),     // $0 - take any profit
                min_profit_percentage: Decimal::new(0, 0), // 0%
                max_gas_cost_usd: Decimal::new(1000, 0),   // $1000 max gas (high for any profitable trade)
                max_slippage_percentage: Decimal::new(100, 0), // 100% - any slippage if profitable
                confidence_threshold: 0.0,              // 0 threshold - take any opportunity
                opportunity_timeout_ms: 30000,          // 30s timeout
            },
            network: NetworkConfig {
                chain_id: 137,
                rpc_url: "https://polygon-rpc.com".to_string(),
                gas_price_gwei: Some(Decimal::new(25, 0)), // 25 gwei
                max_gas_limit: 500_000,
                block_confirmation_count: 1,
            },
            monitoring: MonitoringConfig {
                metrics_port: 9090,
                log_level: "info".to_string(),
                relay_socket_path: "/tmp/alphapulse/relay.sock".to_string(),
            },
        }
    }
}