use super::{DexPool, PoolType, Price, SwapEvent, calculate_standard_price_and_volume};
use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;
use tracing::{debug, warn};

/// SushiSwap-specific pool implementation
pub struct SushiSwapPool {
    address: String,
    rpc_url: String,
    client: Client,
}

impl SushiSwapPool {
    pub fn new(address: String, rpc_url: String) -> Self {
        Self {
            address,
            rpc_url,
            client: Client::new(),
        }
    }
    
    /// Query token0 and token1 addresses from the pool
    async fn query_tokens(&self) -> Result<(String, String)> {
        // token0() selector: 0x0dfe1681
        let token0_call = json!({
            "jsonrpc": "2.0",
            "method": "eth_call",
            "params": [{
                "to": self.address,
                "data": "0x0dfe1681"
            }, "latest"],
            "id": 1
        });
        
        // token1() selector: 0xd21220a7
        let token1_call = json!({
            "jsonrpc": "2.0",
            "method": "eth_call",
            "params": [{
                "to": self.address,
                "data": "0xd21220a7"
            }, "latest"],
            "id": 2
        });
        
        // Make both calls in parallel
        let (t0_resp, t1_resp) = tokio::join!(
            self.client.post(&self.rpc_url).json(&token0_call).send(),
            self.client.post(&self.rpc_url).json(&token1_call).send()
        );
        
        let t0_json: serde_json::Value = t0_resp?.json().await?;
        let t1_json: serde_json::Value = t1_resp?.json().await?;
        
        if let (Some(token0_hex), Some(token1_hex)) = (
            t0_json["result"].as_str(),
            t1_json["result"].as_str()
        ) {
            // Extract address from return data (last 20 bytes = 40 hex chars)
            let token0_addr = format!("0x{}", &token0_hex[26..66]);
            let token1_addr = format!("0x{}", &token1_hex[26..66]);
            
            debug!("SushiSwap pool {} tokens: token0={}, token1={}", 
                self.address, token0_addr, token1_addr);
            
            Ok((token0_addr, token1_addr))
        } else {
            Err(anyhow::anyhow!("Failed to query SushiSwap pool tokens"))
        }
    }
}

#[async_trait]
impl DexPool for SushiSwapPool {
    fn dex_name(&self) -> &str {
        "sushiswap"
    }
    
    fn address(&self) -> &str {
        &self.address
    }
    
    async fn get_tokens(&self) -> Result<(String, String)> {
        self.query_tokens().await
    }
    
    fn parse_swap_event(&self, data: &str) -> Result<SwapEvent> {
        // SushiSwap uses UniswapV2 swap event format
        let hex_data = data.strip_prefix("0x").unwrap_or(data);
        
        if hex_data.len() < 256 {
            return Err(anyhow::anyhow!("Invalid SushiSwap swap event data length"));
        }
        
        // Parse amounts (each is 32 bytes = 64 hex chars)
        let amount0_in_raw = u128::from_str_radix(&hex_data[0..64], 16)? as f64;
        let amount1_in_raw = u128::from_str_radix(&hex_data[64..128], 16)? as f64;
        let amount0_out_raw = u128::from_str_radix(&hex_data[128..192], 16)? as f64;
        let amount1_out_raw = u128::from_str_radix(&hex_data[192..256], 16)? as f64;
        
        Ok(SwapEvent {
            pool_address: self.address.clone(),
            tx_hash: String::new(), // Will be filled by caller
            block_number: 0,        // Will be filled by caller
            amount0_in: amount0_in_raw,
            amount1_in: amount1_in_raw,
            amount0_out: amount0_out_raw,
            amount1_out: amount1_out_raw,
            to_address: String::new(),
            from_address: String::new(),
        })
    }
    
    fn calculate_price(&self, swap: &SwapEvent) -> Price {
        // Use standard calculation - same across all UniswapV2-style DEXes
        let (price, volume) = calculate_standard_price_and_volume(swap);
        
        if price == 0.0 {
            warn!("SushiSwap: Complex swap with multiple ins/outs, using simple ratio");
        }
        
        Price {
            token0_symbol: String::new(), // Will be filled by caller
            token1_symbol: String::new(), // Will be filled by caller
            price,
            volume,
            timestamp: 0, // Will be filled by caller
        }
    }
    
    fn pool_type(&self) -> PoolType {
        PoolType::UniswapV2
    }
    
    fn format_symbol(&self, token0: &str, token1: &str) -> String {
        // SushiSwap-specific symbol formatting
        format!("SUSHI:{}/{}", token0, token1)
    }
}