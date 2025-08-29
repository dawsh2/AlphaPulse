//! Chain configuration loader for adapter services
//!
//! Provides static chain data (tokens, routers, event signatures) loaded from
//! JSON configuration files. This data is used as an optimization to avoid
//! unnecessary RPC calls for well-known tokens and contracts.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use anyhow::Result;

/// Complete chain configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainConfig {
    /// Numeric chain ID (e.g., 137 for Polygon)
    pub chain_id: u64,
    /// RPC endpoint configuration
    pub rpc_endpoints: RpcEndpoints,
    /// Known token configurations
    pub tokens: HashMap<String, TokenInfo>,
    /// DEX router addresses
    pub dex_routers: HashMap<String, String>,
    /// Event signatures for filtering logs
    pub event_signatures: HashMap<String, String>,
}

/// RPC endpoint configuration with fallbacks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcEndpoints {
    /// Primary RPC endpoint
    pub primary: String,
    /// Fallback endpoints for resilience
    pub fallback: Vec<String>,
}

/// Token information for fast lookups
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    /// Token contract address
    pub address: String,
    /// Token decimals
    pub decimals: u8,
    /// Token symbol
    pub symbol: String,
}

/// Load chain configuration from JSON file
pub fn load_chain_config(chain_name: &str) -> Result<ChainConfig> {
    let config_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("config")
        .join("chains.json");
    
    let config_str = std::fs::read_to_string(&config_path)?;
    let mut chains: HashMap<String, ChainConfig> = serde_json::from_str(&config_str)?;
    
    chains.remove(chain_name)
        .ok_or_else(|| anyhow::anyhow!("Chain {} not found in configuration", chain_name))
}

/// Get token info by address (case-insensitive)
pub fn get_token_by_address(config: &ChainConfig, address: &str) -> Option<&TokenInfo> {
    let normalized = address.to_lowercase();
    config.tokens.values().find(|token| {
        token.address.to_lowercase() == normalized
    })
}

/// Get event signature by name
pub fn get_event_signature(config: &ChainConfig, event_name: &str) -> Option<&str> {
    config.event_signatures.get(event_name).map(|s| s.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_polygon_config() {
        let config = load_chain_config("polygon").unwrap();
        assert_eq!(config.chain_id, 137);
        
        // Check USDC token
        let usdc = config.tokens.get("USDC").unwrap();
        assert_eq!(usdc.decimals, 6);
        assert_eq!(usdc.symbol, "USDC");
    }
    
    #[test]
    fn test_get_token_by_address() {
        let config = load_chain_config("polygon").unwrap();
        
        // Test case-insensitive lookup
        let token = get_token_by_address(&config, "0x2791bca1f2de4661ed88a30c99a7a9449aa84174");
        assert!(token.is_some());
        assert_eq!(token.unwrap().symbol, "USDC");
    }
}