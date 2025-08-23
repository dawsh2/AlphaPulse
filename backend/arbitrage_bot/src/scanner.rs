use anyhow::Result;
use ethers::prelude::*;
use std::sync::Arc;
use std::collections::HashMap;
use crate::{ArbitrageOpportunity, Config};
use chrono::Utc;

// DEX Router addresses on Polygon
const QUICKSWAP_ROUTER: &str = "0xa5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff";
const SUSHISWAP_ROUTER: &str = "0x1b02dA8Cb0d097eB8D57A175b88c7D8b47997506";
const UNISWAP_V3_ROUTER: &str = "0xE592427A0AEce92De3Edee1F18E0157C05861564";

// Important token addresses
const USDC: &str = "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174";
const WMATIC: &str = "0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270";
const WETH: &str = "0x7ceB23fD6bC0adD59E62ac25578270cFf1b9f619";

pub struct ArbitrageScanner {
    provider: Arc<Provider<Ws>>,
    pub config: Config,
    pool_cache: HashMap<Address, PoolInfo>,
}

#[derive(Clone, Debug)]
struct PoolInfo {
    address: Address,
    token0: Address,
    token1: Address,
    reserves0: U256,
    reserves1: U256,
    fee: u32,
    dex_type: DexType,
    router: Address,
}

#[derive(Clone, Debug, PartialEq)]
enum DexType {
    UniswapV2,
    UniswapV3,
    Stable,
}

impl ArbitrageScanner {
    pub fn new(provider: Arc<Provider<Ws>>, config: Config) -> Self {
        Self {
            provider,
            config,
            pool_cache: HashMap::new(),
        }
    }
    
    pub async fn scan_for_opportunities(&self) -> Result<Vec<ArbitrageOpportunity>> {
        let mut opportunities = Vec::new();
        
        // Get latest block for fresh data
        let block = self.provider.get_block_number().await?;
        
        // Scan major token pairs
        let pairs = vec![
            (USDC, WMATIC),
            (USDC, WETH),
            (WMATIC, WETH),
        ];
        
        for (token0, token1) in pairs {
            // Find all pools for this pair
            let pools = self.find_pools_for_pair(token0, token1).await?;
            
            if pools.len() < 2 {
                continue; // Need at least 2 pools for arbitrage
            }
            
            // Check all pool combinations
            for i in 0..pools.len() {
                for j in i+1..pools.len() {
                    if let Some(opp) = self.check_arbitrage_opportunity(
                        &pools[i],
                        &pools[j],
                        block
                    ).await? {
                        opportunities.push(opp);
                    }
                }
            }
        }
        
        Ok(opportunities)
    }
    
    async fn find_pools_for_pair(&self, token0: &str, token1: &str) -> Result<Vec<PoolInfo>> {
        let mut pools = Vec::new();
        
        // QuickSwap V2 pool
        if let Ok(pool) = self.get_v2_pool(token0, token1, QUICKSWAP_ROUTER).await {
            pools.push(pool);
        }
        
        // SushiSwap V2 pool
        if let Ok(pool) = self.get_v2_pool(token0, token1, SUSHISWAP_ROUTER).await {
            pools.push(pool);
        }
        
        // Uniswap V3 pools (multiple fee tiers)
        for fee in [500u32, 3000, 10000] {
            if let Ok(pool) = self.get_v3_pool(token0, token1, fee).await {
                pools.push(pool);
            }
        }
        
        Ok(pools)
    }
    
    async fn get_v2_pool(&self, token0: &str, token1: &str, router: &str) -> Result<PoolInfo> {
        // V2 Factory address derivation
        let factory_abi = r#"[{"inputs":[{"internalType":"address","name":"","type":"address"},{"internalType":"address","name":"","type":"address"}],"name":"getPair","outputs":[{"internalType":"address","name":"","type":"address"}],"stateMutability":"view","type":"function"}]"#;
        
        // Get factory from router (simplified - in production would query router)
        let factory_address = match router {
            QUICKSWAP_ROUTER => "0x5757371414417b8C6CAad45bAeF941aBc7d3Ab32",
            SUSHISWAP_ROUTER => "0xc35DADB65012eC5796536bD9864eD8773aBc74C4",
            _ => return Err(anyhow::anyhow!("Unknown router")),
        };
        
        let factory: Address = factory_address.parse()?;
        let factory_contract = Contract::new(
            factory,
            serde_json::from_str::<Abi>(factory_abi)?,
            self.provider.clone()
        );
        
        // Get pair address
        let token0_addr: Address = token0.parse()?;
        let token1_addr: Address = token1.parse()?;
        
        let pair_address: Address = factory_contract
            .method::<_, Address>("getPair", (token0_addr, token1_addr))?
            .call()
            .await?;
        
        if pair_address == Address::zero() {
            return Err(anyhow::anyhow!("No pair found"));
        }
        
        // Get reserves
        let pair_abi = r#"[{"constant":true,"inputs":[],"name":"getReserves","outputs":[{"internalType":"uint112","name":"_reserve0","type":"uint112"},{"internalType":"uint112","name":"_reserve1","type":"uint112"},{"internalType":"uint32","name":"_blockTimestampLast","type":"uint32"}],"payable":false,"stateMutability":"view","type":"function"}]"#;
        
        let pair_contract = Contract::new(
            pair_address,
            serde_json::from_str::<Abi>(pair_abi)?,
            self.provider.clone()
        );
        
        let (reserve0, reserve1, _): (U256, U256, u32) = pair_contract
            .method::<_, (U256, U256, u32)>("getReserves", ())?
            .call()
            .await?;
        
        Ok(PoolInfo {
            address: pair_address,
            token0: token0_addr,
            token1: token1_addr,
            reserves0: reserve0,
            reserves1: reserve1,
            fee: 300, // 0.3% for V2
            dex_type: DexType::UniswapV2,
            router: router.parse()?,
        })
    }
    
    async fn get_v3_pool(&self, token0: &str, token1: &str, fee: u32) -> Result<PoolInfo> {
        // V3 pool address computation
        let factory = "0x1F98431c8aD98523631AE4a59f267346ea31F984"; // Uniswap V3 factory
        
        // For V3, we'd compute pool address deterministically
        // Simplified version - in production would use CREATE2 address computation
        
        // Get pool state
        let pool_abi = r#"[{"inputs":[],"name":"slot0","outputs":[{"internalType":"uint160","name":"sqrtPriceX96","type":"uint160"},{"internalType":"int24","name":"tick","type":"int24"},{"internalType":"uint16","name":"observationIndex","type":"uint16"},{"internalType":"uint16","name":"observationCardinality","type":"uint16"},{"internalType":"uint16","name":"observationCardinalityNext","type":"uint16"},{"internalType":"uint8","name":"feeProtocol","type":"uint8"},{"internalType":"bool","name":"unlocked","type":"bool"}],"stateMutability":"view","type":"function"}]"#;
        
        // This is simplified - would need actual pool address computation
        let pool_address = Address::zero(); // Placeholder
        
        Ok(PoolInfo {
            address: pool_address,
            token0: token0.parse()?,
            token1: token1.parse()?,
            reserves0: U256::from(1000000), // Would query liquidity
            reserves1: U256::from(1000000),
            fee,
            dex_type: DexType::UniswapV3,
            router: UNISWAP_V3_ROUTER.parse()?,
        })
    }
    
    async fn check_arbitrage_opportunity(
        &self,
        pool_a: &PoolInfo,
        pool_b: &PoolInfo,
        _block: U64
    ) -> Result<Option<ArbitrageOpportunity>> {
        // Calculate prices in both pools
        let price_a = self.calculate_price(pool_a);
        let price_b = self.calculate_price(pool_b);
        
        // Check for profitable spread
        let spread = ((price_a - price_b).abs() / price_a.min(price_b)) * 100.0;
        
        if spread < 0.1 {
            return Ok(None); // Too small
        }
        
        // Determine buy and sell pools
        let (buy_pool, sell_pool) = if price_a < price_b {
            (pool_a, pool_b)
        } else {
            (pool_b, pool_a)
        };
        
        // Calculate optimal trade size
        let trade_size = self.calculate_optimal_size(buy_pool, sell_pool, spread);
        
        // Estimate profit
        let gross_profit = trade_size * (spread / 100.0);
        let total_fee = trade_size * ((buy_pool.fee + sell_pool.fee) as f64 / 1_000_000.0);
        let gas_cost = self.estimate_gas_cost().await?;
        let net_profit = gross_profit - total_fee - gas_cost;
        
        if net_profit < self.config.min_profit_usd {
            return Ok(None);
        }
        
        // Calculate confidence score
        let confidence = self.calculate_confidence(spread, trade_size, buy_pool, sell_pool);
        
        let opp = ArbitrageOpportunity {
            id: format!("{:?}-{:?}-{}", buy_pool.address, sell_pool.address, Utc::now().timestamp()),
            timestamp: Utc::now(),
            buy_pool: buy_pool.address,
            sell_pool: sell_pool.address,
            buy_router: buy_pool.router,
            sell_router: sell_pool.router,
            token0: pool_a.token0,
            token1: pool_a.token1,
            profit_usd: net_profit,
            spread_pct: spread,
            size_usd: trade_size,
            confidence,
            gas_estimate: U256::from(300000), // Estimated gas for arbitrage
        };
        
        Ok(Some(opp))
    }
    
    fn calculate_price(&self, pool: &PoolInfo) -> f64 {
        // Simple price calculation
        // In production, would handle decimals properly
        let price = pool.reserves1.as_u128() as f64 / pool.reserves0.as_u128() as f64;
        
        // Adjust for pool type
        match pool.dex_type {
            DexType::Stable => {
                // Stable pools use different pricing curve
                // This is simplified
                price * 0.999
            },
            _ => price
        }
    }
    
    fn calculate_optimal_size(&self, buy_pool: &PoolInfo, sell_pool: &PoolInfo, spread: f64) -> f64 {
        // Calculate optimal arbitrage size
        // Simplified version - in production would use proper formulas
        
        let max_size = self.config.max_position_size_usd;
        
        // Limit by liquidity (don't use more than 10% of pool)
        let buy_liquidity = buy_pool.reserves0.as_u128() as f64 / 1e6; // Assuming USDC decimals
        let sell_liquidity = sell_pool.reserves0.as_u128() as f64 / 1e6;
        
        let liquidity_limit = buy_liquidity.min(sell_liquidity) * 0.1;
        
        // Size that maximizes profit
        let optimal = (spread * 1000.0).min(max_size).min(liquidity_limit);
        
        optimal
    }
    
    fn calculate_confidence(&self, spread: f64, size: f64, _buy_pool: &PoolInfo, _sell_pool: &PoolInfo) -> f64 {
        let mut confidence = 1.0;
        
        // Reduce confidence for very small spreads
        if spread < 0.2 {
            confidence *= 0.7;
        }
        
        // Reduce confidence for very large trades
        if size > 5000.0 {
            confidence *= 0.8;
        }
        
        // Boost confidence for medium spreads
        if spread > 0.5 && spread < 2.0 {
            confidence *= 1.1;
        }
        
        confidence.min(1.0).max(0.0)
    }
    
    async fn estimate_gas_cost(&self) -> Result<f64> {
        let gas_price = self.provider.get_gas_price().await?;
        let gas_estimate = U256::from(300000); // Typical arbitrage gas
        
        let gas_cost_wei = gas_price * gas_estimate;
        let gas_cost_matic = gas_cost_wei.as_u128() as f64 / 1e18;
        
        // Convert MATIC to USD (simplified - would query price oracle)
        let matic_price = 0.8; // Approximate MATIC price
        Ok(gas_cost_matic * matic_price)
    }
}