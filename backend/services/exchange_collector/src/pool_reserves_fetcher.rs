use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Pool reserve fetcher using HTTP RPC calls
pub struct PoolReservesFetcher {
    rpc_url: String,
    client: reqwest::Client,
}

#[derive(Debug, Clone)]
pub struct PoolReserves {
    pub pool_address: String,
    pub token0: String,
    pub token1: String,
    pub reserve0: f64,
    pub reserve1: f64,
    pub liquidity_usd: f64,
}

impl PoolReservesFetcher {
    pub fn new(rpc_url: String) -> Self {
        Self {
            rpc_url,
            client: reqwest::Client::new(),
        }
    }

    /// Fetch reserves for a V2 pool using eth_call to getReserves()
    pub async fn fetch_v2_reserves(&self, pool_address: &str) -> Result<(u128, u128)> {
        // getReserves() function selector
        let data = "0x0902f1ac";
        
        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_call",
            "params": [
                {
                    "to": pool_address,
                    "data": data
                },
                "latest"
            ],
            "id": 1
        });

        let response = self.client
            .post(&self.rpc_url)
            .json(&payload)
            .send()
            .await?;

        let result: serde_json::Value = response.json().await?;
        
        if let Some(hex_result) = result["result"].as_str() {
            if hex_result.len() > 130 {  // Must have at least reserve0 and reserve1
                let hex_data = &hex_result[2..]; // Remove 0x prefix
                
                // Parse reserve0 (first 32 bytes / 64 hex chars)
                let reserve0 = u128::from_str_radix(&hex_data[0..64], 16)?;
                
                // Parse reserve1 (second 32 bytes / 64 hex chars)
                let reserve1 = u128::from_str_radix(&hex_data[64..128], 16)?;
                
                return Ok((reserve0, reserve1));
            }
        }
        
        anyhow::bail!("Failed to fetch reserves for {}", pool_address)
    }

    /// Fetch reserves for multiple pools
    pub async fn fetch_multiple_pools(&self, pools: Vec<(&str, &str, &str, u8, u8)>) -> Vec<PoolReserves> {
        let mut results = Vec::new();
        
        for (address, token0, token1, decimals0, decimals1) in pools {
            match self.fetch_v2_reserves(address).await {
                Ok((r0, r1)) => {
                    let reserve0 = r0 as f64 / 10_f64.powi(decimals0 as i32);
                    let reserve1 = r1 as f64 / 10_f64.powi(decimals1 as i32);
                    
                    // Simple USD estimation (assumes USDC/USDT = $1)
                    let liquidity_usd = if token1 == "USDC" || token1 == "USDT" {
                        reserve1 * 2.0  // Double the stablecoin reserves
                    } else if token0 == "USDC" || token0 == "USDT" {
                        reserve0 * 2.0
                    } else {
                        // For non-stable pairs, use rough estimate
                        (reserve0 + reserve1) * 100.0
                    };
                    
                    results.push(PoolReserves {
                        pool_address: address.to_string(),
                        token0: token0.to_string(),
                        token1: token1.to_string(),
                        reserve0,
                        reserve1,
                        liquidity_usd,
                    });
                }
                Err(e) => {
                    tracing::warn!("Failed to fetch reserves for {}: {}", address, e);
                }
            }
        }
        
        results
    }
}

// Known pool addresses with their token pairs and decimals
pub fn get_major_pools() -> Vec<(&'static str, &'static str, &'static str, u8, u8)> {
    vec![
        // QuickSwap pools (address, token0, token1, decimals0, decimals1)
        ("0x6e7a5FAFcec6BB1e78bAE2A1F0B612012BF14827", "WMATIC", "USDC", 18, 6),
        ("0xadbF1854e5883eB8aa7BAf50705338739e558E5b", "WETH", "USDC", 18, 6),
        ("0xF6422B997c7F54D1c6a6e103bcb1499EeA0a7046", "WBTC", "USDC", 8, 6),
        ("0xf04adBF75cDFc5eD26eeA4bbbb991DB002036Bdd", "USDC", "DAI", 6, 18),
        ("0x2cF7252e74036d1Da831d11089D326296e64a728", "USDC", "USDT", 6, 6),
        
        // SushiSwap pools
        ("0xcd353F79d9FADe311fC3119B841e1f456b54e858", "WMATIC", "USDC", 18, 6),
        ("0x34965ba0ac2451A34a0471F04CCa3F990b8dea27", "WETH", "USDC", 18, 6),
    ]
}