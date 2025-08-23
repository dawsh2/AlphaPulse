use super::{DexPool, PoolType, Price, calculate_standard_price_and_volume, 
            UNISWAP_V2_MINT_SIGNATURE, UNISWAP_V2_BURN_SIGNATURE, UNISWAP_V2_SYNC_SIGNATURE};
use alphapulse_protocol::{SwapEvent, SwapEventCore, UniswapV2SwapEvent,
                         PoolEvent, PoolEventCore, UniswapV2PoolEvent, PoolUpdateType};
use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::OnceCell;
use tracing::{debug, warn};

/// UniswapV2-style pool (QuickSwap, SushiSwap, etc.)
pub struct UniswapV2Pool {
    address: String,
    dex_name: String,
    rpc_url: String,
    client: Arc<Client>, // Changed to Arc to share the connection pool
    // Cache token addresses - only fetch once per pool
    token_cache: OnceCell<(String, String)>,
}

impl UniswapV2Pool {
    pub fn new(address: String, dex_name: String, rpc_url: String) -> Self {
        // Create a properly configured HTTP client with connection pooling
        let client = Arc::new(Client::builder()
            .pool_max_idle_per_host(2)    // Limit idle connections per host
            .pool_idle_timeout(std::time::Duration::from_secs(10)) // Cleanup idle connections faster
            .timeout(std::time::Duration::from_secs(1))            // Very short timeout for pool queries
            .build()
            .expect("Failed to create HTTP client for pool"));
            
        Self {
            address,
            dex_name,
            rpc_url,
            client,
            token_cache: OnceCell::new(),
        }
    }
    
    /// Create V2 pool with shared HTTP client to prevent connection leaks
    pub fn new_with_client(address: String, dex_name: String, rpc_url: String, shared_client: Arc<reqwest::Client>) -> Self {
        debug!("🔗 V2 Pool {} using TRULY shared HTTP client", address);
        Self {
            address,
            dex_name,
            rpc_url,
            client: shared_client, // Use the Arc directly, DON'T clone the client!
            token_cache: OnceCell::new(),
        }
    }
    
    /// Safely parse hex amount, handling overflow gracefully
    fn safe_parse_amount(hex_str: &str) -> Result<f64> {
        // Try parsing as u128 first
        match u128::from_str_radix(hex_str, 16) {
            Ok(value) => Ok(value as f64),
            Err(_) => {
                // If overflow, parse manually by taking a subset that fits
                // For amounts too large for u128, we'll use the first 30 hex chars
                // This preserves the most significant digits
                let truncated = if hex_str.len() > 30 { &hex_str[0..30] } else { hex_str };
                match u128::from_str_radix(truncated, 16) {
                    Ok(value) => {
                        // Scale up by the number of digits we truncated
                        let scale_factor = 16_f64.powi((hex_str.len() - truncated.len()) as i32);
                        Ok((value as f64) * scale_factor)
                    },
                    Err(e) => Err(anyhow::anyhow!("Failed to parse amount {}: {}", hex_str, e))
                }
            }
        }
    }
    
    /// Query token0 and token1 addresses from the pool
    async fn query_tokens(&self) -> Result<(String, String)> {
        // Ensure address has 0x prefix for RPC calls
        let pool_address = if self.address.starts_with("0x") { 
            self.address.clone() 
        } else { 
            format!("0x{}", self.address) 
        };
        
        // token0() selector: 0x0dfe1681
        let token0_call = json!({
            "jsonrpc": "2.0",
            "method": "eth_call",
            "params": [{
                "to": pool_address,
                "data": "0x0dfe1681"
            }, "latest"],
            "id": 1
        });
        
        // token1() selector: 0xd21220a7
        let token1_call = json!({
            "jsonrpc": "2.0",
            "method": "eth_call",
            "params": [{
                "to": pool_address,
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
            
            debug!("Pool {} tokens: token0={}, token1={}", 
                self.address, token0_addr, token1_addr);
            
            Ok((token0_addr, token1_addr))
        } else {
            Err(anyhow::anyhow!("Failed to query pool tokens for {}: token0_result={:?}, token1_result={:?}", 
                self.address, t0_json["result"], t1_json["result"]))
        }
    }
}

#[async_trait]
impl DexPool for UniswapV2Pool {
    fn dex_name(&self) -> &str {
        &self.dex_name
    }
    
    fn address(&self) -> &str {
        &self.address
    }
    
    async fn get_tokens(&self) -> Result<(String, String)> {
        // Use cached tokens if available, otherwise fetch once
        let tokens = self.token_cache.get_or_try_init(|| async {
            self.query_tokens().await
        }).await?;
        
        Ok(tokens.clone())
    }
    
    fn parse_pool_event(&self, event_signature: &str, data: &str, topics: &[String]) -> Result<PoolEvent> {
        let hex_data = data.strip_prefix("0x").unwrap_or(data);
        
        match event_signature {
            UNISWAP_V2_MINT_SIGNATURE => {
                // Mint(address indexed sender, uint amount0, uint amount1)
                // Data: amount0, amount1
                // Topics: [signature, sender]
                if hex_data.len() < 128 {
                    return Err(anyhow::anyhow!("Invalid mint event data length"));
                }
                
                let amount0 = Self::safe_parse_amount(&hex_data[0..64])? as u128;
                let amount1 = Self::safe_parse_amount(&hex_data[64..128])? as u128;
                let sender = if topics.len() > 1 { 
                    format!("0x{}", &topics[1][26..66]) 
                } else { 
                    String::new() 
                };
                
                Ok(PoolEvent::UniswapV2Mint(UniswapV2PoolEvent {
                    core: PoolEventCore {
                        timestamp_ns: 0, // Will be filled by caller
                        pool_id: alphapulse_protocol::message_protocol::InstrumentId::from_u64(0),
                        token0_id: alphapulse_protocol::message_protocol::InstrumentId::from_u64(0),
                        token1_id: alphapulse_protocol::message_protocol::InstrumentId::from_u64(0),
                        tx_hash: String::new(),
                        block_number: 0,
                        log_index: 0,
                        event_type: PoolUpdateType::Mint,
                        sender,
                    },
                    liquidity: 0, // V2 Mint doesn't include liquidity in event data
                    amount0,
                    amount1,
                    to: String::new(), // Not in event data, would need to parse from transaction
                    reserves0_after: 0, // Will be updated from pool state
                    reserves1_after: 0,
                    token0_decimals: 18, // Will be filled by caller with actual decimals
                    token1_decimals: 18, // Will be filled by caller with actual decimals
                }))
            },
            UNISWAP_V2_BURN_SIGNATURE => {
                // Burn(address indexed sender, uint amount0, uint amount1, address indexed to)
                // Data: amount0, amount1
                // Topics: [signature, sender, to]
                if hex_data.len() < 128 {
                    return Err(anyhow::anyhow!("Invalid burn event data length"));
                }
                
                let amount0 = Self::safe_parse_amount(&hex_data[0..64])? as u128;
                let amount1 = Self::safe_parse_amount(&hex_data[64..128])? as u128;
                let sender = if topics.len() > 1 { 
                    format!("0x{}", &topics[1][26..66]) 
                } else { 
                    String::new() 
                };
                let to = if topics.len() > 2 { 
                    format!("0x{}", &topics[2][26..66]) 
                } else { 
                    String::new() 
                };
                
                Ok(PoolEvent::UniswapV2Burn(UniswapV2PoolEvent {
                    core: PoolEventCore {
                        timestamp_ns: 0,
                        pool_id: alphapulse_protocol::message_protocol::InstrumentId::from_u64(0),
                        token0_id: alphapulse_protocol::message_protocol::InstrumentId::from_u64(0),
                        token1_id: alphapulse_protocol::message_protocol::InstrumentId::from_u64(0),
                        tx_hash: String::new(),
                        block_number: 0,
                        log_index: 0,
                        event_type: PoolUpdateType::Burn,
                        sender,
                    },
                    liquidity: 0, // V2 Burn doesn't include liquidity in event data
                    amount0,
                    amount1,
                    to,
                    reserves0_after: 0,
                    reserves1_after: 0,
                    token0_decimals: 18, // Will be filled by caller with actual decimals
                    token1_decimals: 18, // Will be filled by caller with actual decimals
                }))
            },
            UNISWAP_V2_SYNC_SIGNATURE => {
                // Sync(uint112 reserve0, uint112 reserve1)
                // Data: reserve0, reserve1
                if hex_data.len() < 128 {
                    return Err(anyhow::anyhow!("Invalid sync event data length"));
                }
                
                let reserve0 = Self::safe_parse_amount(&hex_data[0..64])? as u128;
                let reserve1 = Self::safe_parse_amount(&hex_data[64..128])? as u128;
                
                Ok(PoolEvent::UniswapV2Sync(UniswapV2PoolEvent {
                    core: PoolEventCore {
                        timestamp_ns: 0,
                        pool_id: alphapulse_protocol::message_protocol::InstrumentId::from_u64(0),
                        token0_id: alphapulse_protocol::message_protocol::InstrumentId::from_u64(0),
                        token1_id: alphapulse_protocol::message_protocol::InstrumentId::from_u64(0),
                        tx_hash: String::new(),
                        block_number: 0,
                        log_index: 0,
                        event_type: PoolUpdateType::Sync,
                        sender: String::new(),
                    },
                    liquidity: 0,
                    amount0: 0,
                    amount1: 0,
                    to: String::new(),
                    reserves0_after: reserve0,
                    reserves1_after: reserve1,
                    token0_decimals: 18, // Will be filled by caller with actual decimals
                    token1_decimals: 18, // Will be filled by caller with actual decimals
                }))
            },
            _ => Err(anyhow::anyhow!("Unknown V2 pool event signature: {}", event_signature))
        }
    }
    
    fn parse_swap_event(&self, data: &str) -> Result<SwapEvent> {
        // UniswapV2 Swap event data layout:
        // bytes 0-32: amount0In
        // bytes 32-64: amount1In
        // bytes 64-96: amount0Out
        // bytes 96-128: amount1Out
        
        let hex_data = data.strip_prefix("0x").unwrap_or(data);
        
        if hex_data.len() < 256 {
            return Err(anyhow::anyhow!("Invalid swap event data length"));
        }
        
        // Parse amounts (each is 32 bytes = 64 hex chars)
        // Handle overflow gracefully - if the number is too large for u128, cap it
        let amount0_in_raw = Self::safe_parse_amount(&hex_data[0..64])?;
        let amount1_in_raw = Self::safe_parse_amount(&hex_data[64..128])?;
        let amount0_out_raw = Self::safe_parse_amount(&hex_data[128..192])?;
        let amount1_out_raw = Self::safe_parse_amount(&hex_data[192..256])?;
        
        // Note: Decimals should be applied by the caller who has token info
        // This just parses raw values
        
        // Return raw event - caller will fill in token info and convert amounts
        // Use zero IDs as placeholders - caller will populate with actual InstrumentIds
        let zero_id = alphapulse_protocol::message_protocol::InstrumentId::from_u64(0);
        
        Ok(SwapEvent::UniswapV2(UniswapV2SwapEvent {
            core: SwapEventCore {
                timestamp_ns: 0,    // Will be filled by caller
                pool_id: zero_id,   // Will be filled by caller
                token0_id: zero_id, // Will be filled by caller
                token1_id: zero_id, // Will be filled by caller
                tx_hash: String::new(),
                block_number: 0,
                amount0_in: amount0_in_raw as u128,
                amount1_in: amount1_in_raw as u128,
                amount0_out: amount0_out_raw as u128,
                amount1_out: amount1_out_raw as u128,
                sender: String::new(),
                recipient: String::new(),
            },
            reserves_after: (0, 0), // Will be filled by caller with actual reserves
            fee_bps: 30,            // V2 standard 0.3% = 30 basis points
        }))
    }
    
    fn calculate_price(&self, swap: &SwapEvent, token0_decimals: u8, token1_decimals: u8) -> Price {
        // Extract core data from the swap event
        let core = swap.core();
        
        // Apply decimal conversion to raw amounts
        let decimals0_factor = 10_f64.powi(token0_decimals as i32);
        let decimals1_factor = 10_f64.powi(token1_decimals as i32);
        
        let amount0_in = (core.amount0_in as f64) / decimals0_factor;
        let amount1_in = (core.amount1_in as f64) / decimals1_factor;
        let amount0_out = (core.amount0_out as f64) / decimals0_factor;
        let amount1_out = (core.amount1_out as f64) / decimals1_factor;
        
        // Calculate price with decimal-adjusted amounts
        const MIN_AMOUNT: f64 = 0.000001; // Minimum amount to consider valid
        
        let (price, volume) = if amount0_in > MIN_AMOUNT && amount1_out > MIN_AMOUNT {
            // Swapping token0 -> token1
            let price = amount1_out / amount0_in;
            let volume = amount0_in * price;
            (price, volume)
        } else if amount1_in > MIN_AMOUNT && amount0_out > MIN_AMOUNT {
            // Swapping token1 -> token0
            let price = amount1_in / amount0_out;
            let volume = amount0_out * price;
            (price, volume)
        } else {
            if core.amount0_in > 0 || core.amount1_in > 0 || core.amount0_out > 0 || core.amount1_out > 0 {
                warn!("Complex swap or dust trade: raw amounts in0={}, in1={}, out0={}, out1={}", 
                      core.amount0_in, core.amount1_in, core.amount0_out, core.amount1_out);
            }
            (0.0, 0.0)
        };
        
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
        // Generic UniswapV2-style formatting
        format!("{}:{}/{}", self.dex_name, token0, token1)
    }
}