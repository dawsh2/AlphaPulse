use super::{DexPool, PoolType, Price, 
            UNISWAP_V3_MINT_SIGNATURE, UNISWAP_V3_BURN_SIGNATURE, UNISWAP_V3_COLLECT_SIGNATURE};
use alphapulse_protocol::{SwapEvent, SwapEventCore, UniswapV3SwapEvent,
                         PoolEvent, PoolEventCore, UniswapV3PoolEvent, PoolUpdateType};
use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::OnceCell;
use tracing::{debug, warn};

/// UniswapV3 pool implementation with tick-based mathematics
/// V3 pools use concentrated liquidity and different swap event structure
pub struct UniswapV3Pool {
    address: String,
    dex_name: String,
    rpc_url: String,
    client: Arc<Client>, // Changed to Arc to share connection pool
    // Cache token addresses and fee tier - only fetch once per pool
    token_cache: OnceCell<(String, String, u32)>, // (token0, token1, fee)
}

impl UniswapV3Pool {
    pub fn new(address: String, dex_name: String, rpc_url: String) -> Self {
        // Create a properly configured HTTP client with connection pooling
        let client = Arc::new(Client::builder()
            .pool_max_idle_per_host(2)    // Limit idle connections per host
            .pool_idle_timeout(std::time::Duration::from_secs(10)) // Cleanup idle connections faster
            .timeout(std::time::Duration::from_secs(1))            // Very short timeout for pool queries
            .build()
            .expect("Failed to create HTTP client for V3 pool"));
            
        Self {
            address,
            dex_name,
            rpc_url,
            client,
            token_cache: OnceCell::new(),
        }
    }
    
    /// Create V3 pool with shared HTTP client to prevent connection leaks
    pub fn new_with_client(address: String, dex_name: String, rpc_url: String, shared_client: Arc<reqwest::Client>) -> Self {
        debug!("🔗 V3 Pool {} using TRULY shared HTTP client", address);
        Self {
            address,
            dex_name,
            rpc_url,
            client: shared_client, // Use the Arc directly, DON'T clone the client!
            token_cache: OnceCell::new(),
        }
    }
    
    /// Query token0, token1, and fee tier from the V3 pool
    async fn query_pool_info(&self) -> Result<(String, String, u32)> {
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
        
        // fee() selector: 0xddca3f43
        let fee_call = json!({
            "jsonrpc": "2.0",
            "method": "eth_call",
            "params": [{
                "to": pool_address,
                "data": "0xddca3f43"
            }, "latest"],
            "id": 3
        });
        
        // Make all calls in parallel
        let (t0_resp, t1_resp, fee_resp) = tokio::join!(
            self.client.post(&self.rpc_url).json(&token0_call).send(),
            self.client.post(&self.rpc_url).json(&token1_call).send(),
            self.client.post(&self.rpc_url).json(&fee_call).send()
        );
        
        let t0_json: serde_json::Value = t0_resp?.json().await?;
        let t1_json: serde_json::Value = t1_resp?.json().await?;
        let fee_json: serde_json::Value = fee_resp?.json().await?;
        
        if let (Some(token0_hex), Some(token1_hex), Some(fee_hex)) = (
            t0_json["result"].as_str(),
            t1_json["result"].as_str(),
            fee_json["result"].as_str()
        ) {
            // Extract address from return data (last 20 bytes = 40 hex chars)
            let token0_addr = format!("0x{}", &token0_hex[26..66]);
            let token1_addr = format!("0x{}", &token1_hex[26..66]);
            
            // Parse fee as u32
            let fee = u32::from_str_radix(&fee_hex[2..], 16)?;
            
            debug!("V3 Pool {} info: token0={}, token1={}, fee={}", 
                self.address, token0_addr, token1_addr, fee);
            
            Ok((token0_addr, token1_addr, fee))
        } else {
            Err(anyhow::anyhow!("Failed to query V3 pool info for {}: token0={:?}, token1={:?}, fee={:?}", 
                self.address, t0_json["result"], t1_json["result"], fee_json["result"]))
        }
    }
    
    /// Safely parse hex amount as signed i256, handling two's complement representation
    fn safe_parse_signed_amount(hex_str: &str) -> Result<f64> {
        // Ethereum uses 256-bit two's complement for signed integers
        // Values starting with 8-F in the first hex digit are negative
        
        if hex_str.is_empty() {
            return Ok(0.0);
        }
        
        // Pad to 64 characters if needed
        let padded = if hex_str.len() < 64 {
            format!("{:0>64}", hex_str)
        } else {
            hex_str.to_string()
        };
        
        // Check if this is a negative number by looking at the first hex digit
        let first_char = padded.chars().next().unwrap_or('0');
        let is_negative = matches!(first_char, '8' | '9' | 'a' | 'b' | 'c' | 'd' | 'e' | 'f' | 'A' | 'B' | 'C' | 'D' | 'E' | 'F');
        
        if is_negative {
            // This is a negative number in two's complement
            // For V3, negative amounts indicate "tokens sent"
            
            // Check if this is a "small" negative number (close to -1)
            // This happens when all bits are set (0xffffff...ffffff)
            let is_small_negative = padded.chars().all(|c| matches!(c, 'f' | 'F'));
            
            if is_small_negative {
                // All F's means -1
                return Ok(-1.0);
            }
            
            // For other negative numbers, we need to properly handle two's complement
            // The issue is that we're dealing with 256-bit signed integers
            // To get the actual negative value:
            // 1. Take the full 256-bit value
            // 2. Compute: -(2^256 - value)
            // Since we can't handle 256-bit math directly, we'll use a workaround:
            // For swap amounts, the actual values are usually small (< 1M tokens)
            // So we can use the fact that negative numbers close to 0 have many F's
            
            // Count leading F's to estimate magnitude
            let leading_fs = padded.chars().take_while(|&c| c == 'f' || c == 'F').count();
            
            if leading_fs >= 48 {
                // This is a small negative number (lots of F's)
                // Parse the complement from the end
                let complement_hex = &padded[48..64];
                match u64::from_str_radix(complement_hex, 16) {
                    Ok(complement) => {
                        // For small negatives, the actual value is approximately:
                        // -(2^64 - complement) when there are many leading F's
                        let actual_value = if complement > u64::MAX / 2 {
                            // Very small negative (close to 0)
                            -((u64::MAX - complement + 1) as f64)
                        } else {
                            // Moderately small negative
                            -(complement as f64)
                        };
                        debug!("Parsed negative V3 amount: {} -> {} (leading F's: {})", hex_str, actual_value, leading_fs);
                        Ok(actual_value)
                    },
                    Err(_) => {
                        warn!("Failed to parse negative amount {}, using -1", hex_str);
                        Ok(-1.0)
                    }
                }
            } else {
                // This is a very large negative number (probably an error)
                warn!("V3 amount appears to be extremely large negative: {}, using -1", hex_str);
                Ok(-1.0)
            }
        } else {
            // Positive number - parse what we can
            // Use the last 64 bits to avoid overflow
            let lower_bits = if padded.len() >= 16 {
                &padded[padded.len() - 16..]
            } else {
                &padded
            };
            
            match u64::from_str_radix(lower_bits, 16) {
                Ok(value) => {
                    let result = value as f64;
                    debug!("Parsed positive V3 amount: {} -> {}", hex_str, result);
                    Ok(result)
                },
                Err(_) => {
                    warn!("Failed to parse positive amount {}, using 0", hex_str);
                    Ok(0.0)
                }
            }
        }
    }
    
    /// Safely parse hex amount as unsigned u128, handling overflow gracefully
    fn safe_parse_unsigned_amount(hex_str: &str) -> Result<u128> {
        // Try parsing as u128 first
        match u128::from_str_radix(hex_str, 16) {
            Ok(value) => Ok(value),
            Err(_) => {
                // If overflow, return max u128 to cap the value
                warn!("Amount overflow in hex {}, capping to u128::MAX", hex_str);
                Ok(u128::MAX)
            }
        }
    }
    
    /// Convert sqrt price X96 to human-readable price
    /// V3 stores price as sqrtPriceX96 = sqrt(price) * 2^96
    fn sqrt_price_x96_to_price(&self, sqrt_price_x96: u128) -> f64 {
        let sqrt_price_x96_f = sqrt_price_x96 as f64;
        let q96 = 2_f64.powi(96);
        let sqrt_price = sqrt_price_x96_f / q96;
        sqrt_price * sqrt_price // price = (sqrt_price)^2
    }
    
    /// Get active liquidity from the pool contract
    pub async fn get_active_liquidity(&self) -> Result<(u128, u128, i32)> {
        // Query slot0 for current state
        let slot0_call = json!({
            "jsonrpc": "2.0",
            "method": "eth_call",
            "params": [{
                "to": self.address,
                "data": "0x3850c7bd"  // slot0() selector
            }, "latest"],
            "id": 1
        });
        
        // Query liquidity() for active liquidity
        let liquidity_call = json!({
            "jsonrpc": "2.0",
            "method": "eth_call",
            "params": [{
                "to": self.address,
                "data": "0x1a686502"  // liquidity() selector
            }, "latest"],
            "id": 2
        });
        
        let (slot0_resp, liq_resp) = tokio::join!(
            self.client.post(&self.rpc_url).json(&slot0_call).send(),
            self.client.post(&self.rpc_url).json(&liquidity_call).send()
        );
        
        let slot0_json: serde_json::Value = slot0_resp?.json().await?;
        let liq_json: serde_json::Value = liq_resp?.json().await?;
        
        if let (Some(slot0_hex), Some(liq_hex)) = (
            slot0_json["result"].as_str(),
            liq_json["result"].as_str()
        ) {
            // Parse slot0 data
            let slot0_data = slot0_hex.strip_prefix("0x").unwrap_or(slot0_hex);
            // sqrtPriceX96 is first 160 bits (40 hex chars)
            let sqrt_price_x96 = u128::from_str_radix(&slot0_data[0..40.min(slot0_data.len())], 16)?;
            // tick is next 24 bits (6 hex chars) - need to handle as signed
            let tick_hex = &slot0_data[40..46.min(slot0_data.len())];
            let tick = i32::from_str_radix(tick_hex, 16).unwrap_or(0);
            
            // Parse liquidity
            let liq_data = liq_hex.strip_prefix("0x").unwrap_or(liq_hex);
            let liquidity = u128::from_str_radix(liq_data, 16)?;
            
            Ok((liquidity, sqrt_price_x96, tick))
        } else {
            Ok((0, 0, 0))
        }
    }
    
    /// Calculate exact output for a V3 swap (simplified - single tick)
    pub fn calculate_v3_output(&self, amount_in: u128, liquidity: u128, sqrt_price_x96: u128, fee_pips: u32, zero_for_one: bool) -> (u128, f64) {
        if liquidity == 0 {
            return (0, 1.0);
        }
        
        // Apply fee (fee_pips = fee * 100, e.g., 3000 = 0.3%)
        let amount_in_after_fee = amount_in * (1_000_000 - fee_pips as u128) / 1_000_000;
        
        let (amount_out, sqrt_price_x96_new) = if zero_for_one {
            // Swapping token0 for token1
            let delta_sqrt = (amount_in_after_fee * (2u128.pow(96))) / liquidity;
            let sqrt_price_x96_new = sqrt_price_x96.saturating_sub(delta_sqrt);
            let amount_out = liquidity * (sqrt_price_x96 - sqrt_price_x96_new) / (2u128.pow(96));
            (amount_out, sqrt_price_x96_new)
        } else {
            // Swapping token1 for token0 
            let numerator = liquidity * (2u128.pow(96)) * amount_in_after_fee;
            let denominator = liquidity * (2u128.pow(96)) + amount_in_after_fee * sqrt_price_x96;
            let delta_sqrt = numerator / denominator;
            let sqrt_price_x96_new = sqrt_price_x96 + delta_sqrt;
            let amount_out = liquidity * delta_sqrt / sqrt_price_x96_new;
            (amount_out, sqrt_price_x96_new)
        };
        
        // Calculate price impact
        let price_impact = ((sqrt_price_x96_new as f64 - sqrt_price_x96 as f64).abs() / sqrt_price_x96 as f64).min(1.0);
        
        (amount_out, price_impact)
    }
}

#[async_trait]
impl DexPool for UniswapV3Pool {
    fn dex_name(&self) -> &str {
        &self.dex_name
    }
    
    fn address(&self) -> &str {
        &self.address
    }
    
    async fn get_tokens(&self) -> Result<(String, String)> {
        // Use cached pool info if available, otherwise fetch once
        // Note: Some pools (like Algebra/QuickSwap V3) emit V3 events but have different interfaces
        let result = self.token_cache.get_or_try_init(|| async {
            self.query_pool_info().await
        }).await;
        
        match result {
            Ok((token0, token1, _fee)) => Ok((token0.clone(), token1.clone())),
            Err(e) => {
                // If pool info query fails, this might be an Algebra pool or other V3-like pool
                // Return placeholder tokens to allow event processing to continue
                warn!("Failed to query V3 pool info for {}: {}. This might be an Algebra pool.", self.address, e);
                warn!("Using placeholder tokens UNKNOWN0/UNKNOWN1 to maintain deep equality");
                
                // Return placeholder tokens so the swap can still be processed
                // This maintains deep equality (input has output) even if token info is unavailable
                Ok(("UNKNOWN0".to_string(), "UNKNOWN1".to_string()))
            }
        }
    }
    
    fn parse_swap_event(&self, data: &str) -> Result<SwapEvent> {
        // UniswapV3 Swap event data layout:
        // sender and recipient are INDEXED (not in data!)
        // Data contains only:
        // bytes 0-32: amount0 (int256)
        // bytes 32-64: amount1 (int256)
        // bytes 64-96: sqrtPriceX96 (uint160)
        // bytes 96-128: liquidity (uint128)
        // bytes 128-160: tick (int24, padded to 32 bytes)
        
        let hex_data = data.strip_prefix("0x").unwrap_or(data);
        
        if hex_data.len() < 320 { // 5 * 64 hex chars = 320
            return Err(anyhow::anyhow!("Invalid V3 swap event data length: {} (expected >= 320)", hex_data.len()));
        }
        
        // Parse amounts (signed integers in V3) - handle overflow gracefully
        let amount0_raw = Self::safe_parse_signed_amount(&hex_data[0..64])?;
        let amount1_raw = Self::safe_parse_signed_amount(&hex_data[64..128])?;
        
        // Parse sqrtPriceX96, liquidity, and tick (adjusted offsets)
        let sqrt_price_x96 = Self::safe_parse_unsigned_amount(&hex_data[128..192])? as u128;
        let _liquidity = Self::safe_parse_unsigned_amount(&hex_data[192..256])?;
        let tick = Self::safe_parse_signed_amount(&hex_data[256..320])? as i32;
        
        // V3 uses signed amounts - positive means received, negative means sent
        let (amount0_in, amount0_out) = if amount0_raw < 0.0 {
            (-amount0_raw, 0.0) // Sent token0 (amount0 is negative)
        } else {
            (0.0, amount0_raw) // Received token0 (amount0 is positive)
        };
        
        let (amount1_in, amount1_out) = if amount1_raw < 0.0 {
            (-amount1_raw, 0.0) // Sent token1 (amount1 is negative)
        } else {
            (0.0, amount1_raw) // Received token1 (amount1 is positive)
        };
        
        debug!("V3 swap parsed: amount0_raw={}, amount1_raw={} -> in0={}, out0={}, in1={}, out1={}", 
               amount0_raw, amount1_raw, amount0_in, amount0_out, amount1_in, amount1_out);
        
        // Use zero IDs as placeholders - caller will populate with actual InstrumentIds
        let zero_id = alphapulse_protocol::message_protocol::InstrumentId::from_u64(0);
        
        Ok(SwapEvent::UniswapV3(UniswapV3SwapEvent {
            core: SwapEventCore {
                timestamp_ns: 0,    // Will be filled by caller
                pool_id: zero_id,   // Will be filled by caller
                token0_id: zero_id, // Will be filled by caller
                token1_id: zero_id, // Will be filled by caller
                tx_hash: String::new(),
                block_number: 0,
                amount0_in: amount0_in as u128,
                amount1_in: amount1_in as u128,
                amount0_out: amount0_out as u128,
                amount1_out: amount1_out as u128,
                sender: String::new(),
                recipient: String::new(),
            },
            sqrt_price_x96,
            tick,
            liquidity: 0,  // Will be filled by caller with actual liquidity
            fee_tier: self.fee() as u32,
        }))
    }
    
    fn calculate_price(&self, swap: &SwapEvent, token0_decimals: u8, token1_decimals: u8) -> Price {
        // Use V3-specific price calculation that handles sqrtPriceX96
        let (price, volume) = super::calculate_v3_price_and_volume(swap, token0_decimals, token1_decimals);
        
        if price == 0.0 {
            warn!("V3 complex swap with multiple ins/outs, using simple ratio");
        }
        
        Price {
            token0_symbol: String::new(), // Will be filled by caller
            token1_symbol: String::new(), // Will be filled by caller
            price,
            volume,
            timestamp: 0, // Will be filled by caller
        }
    }
    
    fn parse_pool_event(&self, event_signature: &str, data: &str, topics: &[String]) -> Result<PoolEvent> {
        let hex_data = data.strip_prefix("0x").unwrap_or(data);
        
        match event_signature {
            UNISWAP_V3_MINT_SIGNATURE => {
                // Mint(address sender, address indexed owner, int24 indexed tickLower, int24 indexed tickUpper, uint128 amount, uint256 amount0, uint256 amount1)
                // Topics: [signature, owner, tickLower, tickUpper]
                // Data: sender, amount, amount0, amount1
                if hex_data.len() < 256 {
                    return Err(anyhow::anyhow!("Invalid V3 mint event data length"));
                }
                
                // Parse hex data
                let sender = format!("0x{}", &hex_data[24..64]); // sender (address, first 20 bytes)
                let liquidity = u128::from_str_radix(&hex_data[64..128], 16).unwrap_or(0);
                let amount0 = u128::from_str_radix(&hex_data[128..192], 16).unwrap_or(0);
                let amount1 = u128::from_str_radix(&hex_data[192..256], 16).unwrap_or(0);
                
                // Parse indexed topics
                let owner = if topics.len() > 1 { 
                    format!("0x{}", &topics[1][26..66]) 
                } else { 
                    String::new() 
                };
                let tick_lower = if topics.len() > 2 {
                    i32::from_str_radix(topics[2].strip_prefix("0x").unwrap_or(&topics[2]), 16)
                        .unwrap_or(0)
                } else { 0 };
                let tick_upper = if topics.len() > 3 {
                    i32::from_str_radix(topics[3].strip_prefix("0x").unwrap_or(&topics[3]), 16)
                        .unwrap_or(0)
                } else { 0 };
                
                Ok(PoolEvent::UniswapV3Mint(UniswapV3PoolEvent {
                    core: PoolEventCore {
                        timestamp_ns: 0, // Will be filled by caller
                                                tx_hash: String::new(),
                        block_number: 0,
                        log_index: 0,
                        token0_address: String::new(), // Will be filled by caller
                        token1_address: String::new(),
                        token0_symbol: String::new(),
                        token1_symbol: String::new(),
                        event_type: PoolUpdateType::Mint,
                        sender,
                    },
                    owner,
                    tick_lower,
                    tick_upper,
                    liquidity,
                    amount0,
                    amount1,
                    amount0_collected: 0,
                    amount1_collected: 0,
                    sqrt_price_x96_after: 0, // Would need pool state query to fill
                    tick_after: 0,
                    liquidity_after: 0,
                    token0_decimals: 18, // Will be filled by caller with actual decimals
                    token1_decimals: 18, // Will be filled by caller with actual decimals
                }))
            },
            UNISWAP_V3_BURN_SIGNATURE => {
                // Burn(address indexed owner, int24 indexed tickLower, int24 indexed tickUpper, uint128 amount, uint256 amount0, uint256 amount1)
                // Topics: [signature, owner, tickLower, tickUpper]
                // Data: amount, amount0, amount1
                if hex_data.len() < 192 {
                    return Err(anyhow::anyhow!("Invalid V3 burn event data length"));
                }
                
                let liquidity = u128::from_str_radix(&hex_data[0..64], 16).unwrap_or(0);
                let amount0 = u128::from_str_radix(&hex_data[64..128], 16).unwrap_or(0);
                let amount1 = u128::from_str_radix(&hex_data[128..192], 16).unwrap_or(0);
                
                let owner = if topics.len() > 1 { 
                    format!("0x{}", &topics[1][26..66]) 
                } else { 
                    String::new() 
                };
                let tick_lower = if topics.len() > 2 {
                    i32::from_str_radix(topics[2].strip_prefix("0x").unwrap_or(&topics[2]), 16)
                        .unwrap_or(0)
                } else { 0 };
                let tick_upper = if topics.len() > 3 {
                    i32::from_str_radix(topics[3].strip_prefix("0x").unwrap_or(&topics[3]), 16)
                        .unwrap_or(0)
                } else { 0 };
                
                Ok(PoolEvent::UniswapV3Burn(UniswapV3PoolEvent {
                    core: PoolEventCore {
                        timestamp_ns: 0,
                                                tx_hash: String::new(),
                        block_number: 0,
                        log_index: 0,
                        pool_id: alphapulse_protocol::message_protocol::InstrumentId::from_u64(0),
                        token0_id: alphapulse_protocol::message_protocol::InstrumentId::from_u64(0),
                        token1_id: alphapulse_protocol::message_protocol::InstrumentId::from_u64(0),
                        event_type: PoolUpdateType::Burn,
                        sender: String::new(),
                    },
                    owner,
                    tick_lower,
                    tick_upper,
                    liquidity,
                    amount0,
                    amount1,
                    amount0_collected: 0,
                    amount1_collected: 0,
                    sqrt_price_x96_after: 0,
                    tick_after: 0,
                    liquidity_after: 0,
                    token0_decimals: 18, // Will be filled by caller with actual decimals
                    token1_decimals: 18, // Will be filled by caller with actual decimals
                }))
            },
            UNISWAP_V3_COLLECT_SIGNATURE => {
                // Collect(address indexed owner, address recipient, int24 indexed tickLower, int24 indexed tickUpper, uint128 amount0, uint128 amount1)
                // Topics: [signature, owner, tickLower, tickUpper]
                // Data: recipient, amount0, amount1
                if hex_data.len() < 192 {
                    return Err(anyhow::anyhow!("Invalid V3 collect event data length"));
                }
                
                let recipient = format!("0x{}", &hex_data[24..64]);
                let amount0_collected = u128::from_str_radix(&hex_data[64..128], 16).unwrap_or(0);
                let amount1_collected = u128::from_str_radix(&hex_data[128..192], 16).unwrap_or(0);
                
                let owner = if topics.len() > 1 { 
                    format!("0x{}", &topics[1][26..66]) 
                } else { 
                    String::new() 
                };
                let tick_lower = if topics.len() > 2 {
                    i32::from_str_radix(topics[2].strip_prefix("0x").unwrap_or(&topics[2]), 16)
                        .unwrap_or(0)
                } else { 0 };
                let tick_upper = if topics.len() > 3 {
                    i32::from_str_radix(topics[3].strip_prefix("0x").unwrap_or(&topics[3]), 16)
                        .unwrap_or(0)
                } else { 0 };
                
                Ok(PoolEvent::UniswapV3Collect(UniswapV3PoolEvent {
                    core: PoolEventCore {
                        timestamp_ns: 0,
                                                tx_hash: String::new(),
                        block_number: 0,
                        log_index: 0,
                        pool_id: alphapulse_protocol::message_protocol::InstrumentId::from_u64(0),
                        token0_id: alphapulse_protocol::message_protocol::InstrumentId::from_u64(0),
                        token1_id: alphapulse_protocol::message_protocol::InstrumentId::from_u64(0),
                        event_type: PoolUpdateType::Collect,
                        sender: recipient,
                    },
                    owner,
                    tick_lower,
                    tick_upper,
                    liquidity: 0, // Collect doesn't change liquidity
                    amount0: 0,
                    amount1: 0,
                    amount0_collected,
                    amount1_collected,
                    sqrt_price_x96_after: 0,
                    tick_after: 0,
                    liquidity_after: 0,
                    token0_decimals: 18, // Will be filled by caller with actual decimals
                    token1_decimals: 18, // Will be filled by caller with actual decimals
                }))
            },
            _ => Err(anyhow::anyhow!("Unknown V3 pool event signature: {}", event_signature))
        }
    }

    fn pool_type(&self) -> PoolType {
        PoolType::UniswapV3
    }
    
    fn format_symbol(&self, token0: &str, token1: &str) -> String {
        // Include fee tier in V3 symbol for disambiguation
        let fee = self.token_cache.get().map(|(_,_,f)| *f).unwrap_or(3000); // Default to 0.3%
        format!("{}:v3-{}:{}/{}", self.dex_name, fee, token0, token1)
    }
}

/// V3-specific utilities
impl UniswapV3Pool {
    /// Get the fee tier for this pool (in basis points)
    /// Common tiers: 500 (0.05%), 3000 (0.3%), 10000 (1%)
    pub async fn get_fee_tier(&self) -> Result<u32> {
        let (_token0, _token1, fee) = self.token_cache.get_or_try_init(|| async {
            self.query_pool_info().await
        }).await?;
        
        Ok(*fee)
    }
    
    /// Estimate gas cost for V3 swap (higher than V2 due to complexity)
    pub fn estimate_gas_cost(&self) -> u64 {
        // V3 swaps are more expensive than V2
        150_000 // Approximate gas for V3 swap
    }
}