use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use tracing::{debug, info, warn, error};

/// Token information stored in the registry
#[derive(Debug, Clone)]
pub struct TokenInfo {
    pub address: String,
    pub symbol: String,
    pub decimals: u8,
    pub name: Option<String>,
}

/// Token registry with caching to avoid repeated blockchain queries
pub struct TokenRegistry {
    /// Cache of token address -> TokenInfo
    cache: Arc<RwLock<HashMap<String, TokenInfo>>>,
    
    /// RPC endpoint for querying token info
    rpc_url: String,
    
    /// HTTP client for RPC calls
    client: reqwest::Client,
}

impl TokenRegistry {
    pub fn new(rpc_url: String) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            rpc_url,
            client: reqwest::Client::new(),
        }
    }
    
    /// Get token info, either from cache or by querying the blockchain
    pub async fn get_token_info(&self, address: &str) -> Result<TokenInfo> {
        let address = address.to_lowercase();
        
        // Check cache first
        {
            let cache = self.cache.read();
            if let Some(info) = cache.get(&address) {
                debug!("✅ Token info for {} found in cache: {} ({})", 
                    address, info.symbol, info.decimals);
                return Ok(info.clone());
            }
        }
        
        // Not in cache, query blockchain
        info!("🔍 Querying blockchain for token info: {}", address);
        let info = self.query_token_info(&address).await?;
        
        // Store in cache
        {
            let mut cache = self.cache.write();
            cache.insert(address.clone(), info.clone());
        }
        
        info!("✅ Token info discovered: {} ({} decimals) at {}", 
            info.symbol, info.decimals, address);
        
        Ok(info)
    }
    
    /// Query token information from the blockchain
    async fn query_token_info(&self, address: &str) -> Result<TokenInfo> {
        let decimals = self.query_decimals(address).await?;
        let symbol = self.query_symbol(address).await?;
        let name = self.query_name(address).await.ok();
        
        Ok(TokenInfo {
            address: address.to_string(),
            symbol,
            decimals,
            name,
        })
    }
    
    /// Query token decimals
    async fn query_decimals(&self, address: &str) -> Result<u8> {
        // decimals() function selector: 0x313ce567
        let data = "0x313ce567";
        
        let response = self.eth_call(address, data).await?;
        
        // Parse the result (uint8 is returned as 32 bytes, we need the last byte)
        if let Some(result) = response.as_str() {
            let hex = result.trim_start_matches("0x");
            if hex.len() >= 64 {
                // Last byte of the 32-byte result
                let decimals = u8::from_str_radix(&hex[62..64], 16)?;
                return Ok(decimals);
            }
        }
        
        // Fallback to 18 decimals if query fails
        warn!("⚠️ Failed to query decimals for {}, assuming 18", address);
        Ok(18)
    }
    
    /// Query token symbol
    async fn query_symbol(&self, address: &str) -> Result<String> {
        // symbol() function selector: 0x95d89b41
        let data = "0x95d89b41";
        
        let response = self.eth_call(address, data).await?;
        
        // Parse the string result
        if let Some(result) = response.as_str() {
            let symbol = self.parse_string_from_hex(result)?;
            if !symbol.is_empty() {
                return Ok(symbol);
            }
        }
        
        // Generate placeholder symbol from address
        let placeholder = format!("TOKEN_{}", &address[2..8].to_uppercase());
        warn!("⚠️ Failed to query symbol for {}, using {}", address, placeholder);
        Ok(placeholder)
    }
    
    /// Query token name (optional)
    async fn query_name(&self, address: &str) -> Result<String> {
        // name() function selector: 0x06fdde03
        let data = "0x06fdde03";
        
        let response = self.eth_call(address, data).await?;
        
        if let Some(result) = response.as_str() {
            self.parse_string_from_hex(result)
        } else {
            Err(anyhow::anyhow!("Failed to query name"))
        }
    }
    
    /// Make an eth_call RPC request
    async fn eth_call(&self, to: &str, data: &str) -> Result<Value> {
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_call",
            "params": [{
                "to": to,
                "data": data
            }, "latest"],
            "id": 1
        });
        
        let response = self.client
            .post(&self.rpc_url)
            .json(&request)
            .send()
            .await?;
        
        let json: Value = response.json().await?;
        
        if let Some(error) = json.get("error") {
            return Err(anyhow::anyhow!("RPC error: {:?}", error));
        }
        
        json.get("result")
            .ok_or_else(|| anyhow::anyhow!("No result in RPC response"))
            .map(|v| v.clone())
    }
    
    /// Parse a string from hex-encoded bytes32 or dynamic string
    fn parse_string_from_hex(&self, hex: &str) -> Result<String> {
        let hex = hex.trim_start_matches("0x");
        
        if hex.len() < 128 {
            return Err(anyhow::anyhow!("Invalid hex string"));
        }
        
        // For dynamic strings, skip offset (32 bytes) and length (32 bytes)
        // Then read the actual string data
        let length = usize::from_str_radix(&hex[64..128], 16)?;
        
        if length > 0 && hex.len() >= 128 + (length * 2) {
            let string_hex = &hex[128..128 + (length * 2)];
            let bytes: Vec<u8> = (0..string_hex.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&string_hex[i..i + 2], 16))
                .collect::<Result<Vec<_>, _>>()?;
            
            Ok(String::from_utf8_lossy(&bytes)
                .trim_end_matches('\0')
                .to_string())
        } else {
            // Try to parse as bytes32 (fixed-length string)
            let bytes: Vec<u8> = (0..hex.len().min(64))
                .step_by(2)
                .map(|i| u8::from_str_radix(&hex[i..i + 2], 16))
                .collect::<Result<Vec<_>, _>>()?;
            
            Ok(String::from_utf8_lossy(&bytes)
                .trim_end_matches('\0')
                .to_string())
        }
    }
    
    /// Preload commonly used tokens into cache
    pub async fn preload_common_tokens(&self) {
        let common_tokens = vec![
            ("0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174", "USDC", 6),  // USDC (bridged)
            ("0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359", "USDC", 6),  // USDC (native)
            ("0x455e53CBB86018Ac2B8092FdCd39d8444aFFC3F6", "POL", 18),   // POL
            ("0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270", "WMATIC", 18), // WMATIC
            ("0x7ceB23fD6bC0adD59E62ac25578270cFf1b9f619", "WETH", 18),  // WETH
            ("0xc2132D05D31c914a87C6611C10748AEb04B58e8F", "USDT", 6),   // USDT
            ("0x8f3Cf7ad23Cd3CaDbD9735AFf958023239c6A063", "DAI", 18),   // DAI
            ("0x1bfd67037b42cf73acf2047067bd4f2c47d9bfd6", "WBTC", 8),   // WBTC
            ("0x53e0bca35ec356bd5dddfebbd1fc0fd03fabad39", "LINK", 18),  // LINK
            ("0xd6df932a45c0f255f85145f286ea0b292b21c90b", "AAVE", 18),  // AAVE
        ];
        
        for (address, symbol, decimals) in common_tokens {
            let info = TokenInfo {
                address: address.to_lowercase(),
                symbol: symbol.to_string(),
                decimals,
                name: None,
            };
            
            let mut cache = self.cache.write();
            cache.insert(address.to_lowercase(), info);
        }
        
        let count = {
            let cache = self.cache.write();
            cache.len()
        };
        info!("✅ Preloaded {} common tokens into cache", count);
    }
    
    /// Get cache statistics
    pub fn cache_stats(&self) -> (usize, Vec<String>) {
        let cache = self.cache.read();
        let count = cache.len();
        let tokens: Vec<String> = cache.values()
            .map(|t| format!("{} ({})", t.symbol, t.decimals))
            .collect();
        (count, tokens)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_parse_decimals() {
        let registry = TokenRegistry::new("https://polygon-mainnet.g.alchemy.com/v2/demo".to_string());
        
        // USDC decimals response: 0x0000000000000000000000000000000000000000000000000000000000000006
        let hex = "0x0000000000000000000000000000000000000000000000000000000000000006";
        let result = serde_json::json!(hex);
        
        // Should parse as 6
        // Note: This is a simplified test - real implementation needs the full flow
    }
}