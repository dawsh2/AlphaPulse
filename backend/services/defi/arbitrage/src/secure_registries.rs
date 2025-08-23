// SECURE Token Registry - ADDRESS-ONLY, NO SYMBOL DEPENDENCIES
// ELIMINATES: Honeypots, fake tokens, symbol manipulation attacks

use anyhow::{Result, anyhow};
use ethers::prelude::*;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use serde::{Serialize, Deserialize};

/// SECURE token information - ADDRESS IS THE ONLY SOURCE OF TRUTH
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecureTokenInfo {
    pub address: Address,
    pub decimals: u8,
    pub is_verified: bool,           // Only true for manually verified addresses
    pub is_wrapped_native: bool,     // Only true if address == chain's wrapped native
    pub is_known_stable: bool,       // Only true if address in verified stable list
    pub last_validated: u64,
    // NOTE: NO SYMBOL FIELD - symbols are unreliable and exploitable
}

/// SECURE chain configuration with ONLY verified addresses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecureChainConfig {
    pub chain_id: u64,
    pub name: String,
    pub rpc_urls: Vec<String>,
    pub wrapped_native: Address,
    
    // VERIFIED stablecoins - ONLY OFFICIAL CONTRACT ADDRESSES
    pub verified_stables: HashSet<Address>,
    
    // VERIFIED major tokens - manually curated, not symbol-based
    pub verified_tokens: HashSet<Address>,
    
    // DEX configurations
    pub dexs: HashMap<String, DexConfig>,
    pub chainlink_feeds: HashMap<String, Address>,
    
    // Security settings
    pub allow_unknown_tokens: bool,  // false for production
    pub max_token_cache_age: u64,    // seconds
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DexConfig {
    pub name: String,
    pub router_address: Address,
    pub factory_address: Address,
    pub quoter_address: Option<Address>,
    pub fee_tiers: Vec<u32>,
    pub swap_gas_estimate: u64,
    pub is_verified: bool,  // Only verified DEXs allowed
}

/// PRODUCTION-HARDENED token registry
pub struct SecureRegistryManager {
    chain_config: SecureChainConfig,
    token_cache: Arc<RwLock<HashMap<Address, SecureTokenInfo>>>,
    http_client: reqwest::Client,
    provider: Arc<Provider<Http>>,
}

impl SecureRegistryManager {
    pub async fn new(chain_id: u64, rpc_url: String) -> Result<Self> {
        let provider = Provider::<Http>::try_from(&rpc_url)?;
        let provider = Arc::new(provider);

        let chain_config = Self::load_secure_chain_config(chain_id)?;
        
        let http_client = reqwest::Client::builder()
            .pool_max_idle_per_host(4)
            .timeout(std::time::Duration::from_secs(5))
            .build()?;

        let manager = Self {
            chain_config,
            token_cache: Arc::new(RwLock::new(HashMap::new())),
            http_client,
            provider,
        };

        // Preload ONLY verified tokens
        manager.preload_verified_tokens().await?;
        
        info!("🔒 SecureRegistryManager initialized for {} (chain {}) with {} verified tokens", 
              manager.chain_config.name, manager.chain_config.chain_id,
              manager.chain_config.verified_tokens.len() + manager.chain_config.verified_stables.len());

        Ok(manager)
    }

    /// Load SECURE chain configuration with ONLY verified addresses
    fn load_secure_chain_config(chain_id: u64) -> Result<SecureChainConfig> {
        match chain_id {
            137 => {
                // Polygon Mainnet - VERIFIED ADDRESSES ONLY
                let mut verified_stables = HashSet::new();
                
                // OFFICIAL stablecoin contracts (manually verified)
                verified_stables.insert("0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174".parse()?); // USDC (bridged)
                verified_stables.insert("0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359".parse()?); // USDC.e (native)
                verified_stables.insert("0xc2132D05D31c914a87C6611C10748AEb04B58e8F".parse()?); // USDT
                verified_stables.insert("0x8f3Cf7ad23Cd3CaDbD9735AFf958023239c6A063".parse()?); // DAI
                
                let mut verified_tokens = HashSet::new();
                
                // OFFICIAL major token contracts (manually verified)
                verified_tokens.insert("0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270".parse()?); // WMATIC
                verified_tokens.insert("0x7ceB23fD6bC0adD59E62ac25578270cFf1b9f619".parse()?); // WETH
                verified_tokens.insert("0x1bfd67037b42cf73acf2047067bd4f2c47d9bfd6".parse()?); // WBTC
                verified_tokens.insert("0x53e0bca35ec356bd5dddfebbd1fc0fd03fabad39".parse()?); // LINK
                verified_tokens.insert("0xd6df932a45c0f255f85145f286ea0b292b21c90b".parse()?); // AAVE

                let mut dexs = HashMap::new();
                
                // VERIFIED DEX configurations
                dexs.insert("quickswap".to_string(), DexConfig {
                    name: "QuickSwap".to_string(),
                    router_address: "0xa5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff".parse()?,
                    factory_address: "0x5757371414417b8C6CAad45bAeF941aBc7d3Ab32".parse()?,
                    quoter_address: None,
                    fee_tiers: vec![30],
                    swap_gas_estimate: 150_000,
                    is_verified: true,
                });

                dexs.insert("sushiswap".to_string(), DexConfig {
                    name: "SushiSwap".to_string(),
                    router_address: "0x1b02dA8Cb0d097eB8D57A175b88c7D8b47997506".parse()?,
                    factory_address: "0xc35DADB65012eC5796536bD9864eD8773aBc74C4".parse()?,
                    quoter_address: None,
                    fee_tiers: vec![30],
                    swap_gas_estimate: 160_000,
                    is_verified: true,
                });

                let mut chainlink_feeds = HashMap::new();
                chainlink_feeds.insert("MATIC/USD".to_string(), 
                    "0xAB594600376Ec9fD91F8e885dADF0CE036862dE0".parse()?);
                chainlink_feeds.insert("ETH/USD".to_string(),
                    "0xF9680D99D6C9589e2a93a78A04A279e509205945".parse()?);

                Ok(SecureChainConfig {
                    chain_id: 137,
                    name: "Polygon Mainnet".to_string(),
                    rpc_urls: vec!["https://polygon-rpc.com".to_string()],
                    wrapped_native: "0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270".parse()?,
                    verified_stables,
                    verified_tokens,
                    dexs,
                    chainlink_feeds,
                    allow_unknown_tokens: false, // 🔒 PRODUCTION: DENY unknown tokens
                    max_token_cache_age: 3600,   // 1 hour max cache
                })
            }
            80001 => {
                // Mumbai testnet - minimal verified set
                let verified_stables = HashSet::new(); // No verified stables on testnet
                let mut verified_tokens = HashSet::new();
                verified_tokens.insert("0x9c3C9283D3e44854697Cd22D3Faa240Cfb032889".parse()?); // WMATIC (Mumbai)

                Ok(SecureChainConfig {
                    chain_id: 80001,
                    name: "Polygon Mumbai".to_string(),
                    rpc_urls: vec!["https://rpc-mumbai.maticvigil.com".to_string()],
                    wrapped_native: "0x9c3C9283D3e44854697Cd22D3Faa240Cfb032889".parse()?,
                    verified_stables,
                    verified_tokens,
                    dexs: HashMap::new(),
                    chainlink_feeds: HashMap::new(),
                    allow_unknown_tokens: true, // Testnet allows discovery
                    max_token_cache_age: 300,   // 5 minute cache for testnet
                })
            }
            _ => Err(anyhow!("Unsupported chain ID: {}", chain_id))
        }
    }

    /// SECURE token lookup - ADDRESS-BASED ONLY
    pub async fn get_secure_token_info(&self, address: Address) -> Result<SecureTokenInfo> {
        // Check cache first
        {
            let cache = self.token_cache.read().await;
            if let Some(info) = cache.get(&address) {
                let age = current_timestamp() - info.last_validated;
                if age < self.chain_config.max_token_cache_age {
                    debug!("Secure token info for {:?} found in cache", address);
                    return Ok(info.clone());
                }
            }
        }

        // SECURITY CHECK: Is this a verified token?
        let is_verified = self.is_verified_token(address);
        let is_wrapped_native = address == self.chain_config.wrapped_native;
        let is_known_stable = self.chain_config.verified_stables.contains(&address);

        // PRODUCTION SECURITY: Block unknown tokens if configured
        if !is_verified && !self.chain_config.allow_unknown_tokens {
            return Err(anyhow!(
                "🚨 SECURITY: Unknown token {:?} blocked. Only verified tokens allowed in production.", 
                address
            ));
        }

        // Discover token info (ONLY decimals, no symbols)
        let info = self.discover_secure_token_info(address, is_verified, is_wrapped_native, is_known_stable).await?;

        // Cache the result
        {
            let mut cache = self.token_cache.write().await;
            cache.insert(address, info.clone());
        }

        if is_verified {
            info!("✅ Verified token accessed: {:?} ({} decimals)", address, info.decimals);
        } else {
            warn!("⚠️ Unknown token discovered: {:?} ({} decimals) - USE WITH CAUTION", address, info.decimals);
        }

        Ok(info)
    }

    /// Discover SECURE token info - NO SYMBOL DEPENDENCIES
    async fn discover_secure_token_info(
        &self, 
        address: Address, 
        is_verified: bool,
        is_wrapped_native: bool,
        is_known_stable: bool
    ) -> Result<SecureTokenInfo> {
        // Query ONLY decimals (the only reliable ERC20 field)
        let decimals = self.query_token_decimals_secure(address).await?;

        Ok(SecureTokenInfo {
            address,
            decimals,
            is_verified,
            is_wrapped_native,
            is_known_stable,
            last_validated: current_timestamp(),
        })
    }

    /// SECURE decimals query with validation
    async fn query_token_decimals_secure(&self, address: Address) -> Result<u8> {
        let response = self.make_eth_call(address, "0x313ce567").await?; // decimals()
        
        if let Some(result) = response.as_str() {
            let hex = result.trim_start_matches("0x");
            if hex.len() >= 64 {
                let decimals = u8::from_str_radix(&hex[62..64], 16)?;
                
                // SECURITY: Validate reasonable decimals range
                if decimals > 50 {
                    return Err(anyhow!("🚨 SECURITY: Suspicious decimals {} for token {:?}", decimals, address));
                }
                
                return Ok(decimals);
            }
        }
        
        Err(anyhow!("Failed to query decimals for token {:?}", address))
    }

    /// Check if token is in verified list (ADDRESS-BASED ONLY)
    fn is_verified_token(&self, address: Address) -> bool {
        self.chain_config.verified_tokens.contains(&address) ||
        self.chain_config.verified_stables.contains(&address) ||
        address == self.chain_config.wrapped_native
    }

    /// SECURE stablecoin check - ADDRESS-BASED ONLY
    pub fn is_verified_stablecoin(&self, address: Address) -> bool {
        self.chain_config.verified_stables.contains(&address)
    }

    /// Get wrapped native token address
    pub fn get_wrapped_native(&self) -> Address {
        self.chain_config.wrapped_native
    }

    /// Get verified stable token addresses
    pub fn get_verified_stables(&self) -> Vec<Address> {
        self.chain_config.verified_stables.iter().copied().collect()
    }

    /// Preload ONLY verified tokens
    async fn preload_verified_tokens(&self) -> Result<()> {
        let mut cache = self.token_cache.write().await;
        
        // Preload wrapped native
        let wrapped_native_info = SecureTokenInfo {
            address: self.chain_config.wrapped_native,
            decimals: 18, // Standard for wrapped native tokens
            is_verified: true,
            is_wrapped_native: true,
            is_known_stable: false,
            last_validated: current_timestamp(),
        };
        cache.insert(self.chain_config.wrapped_native, wrapped_native_info);

        // Preload verified stables
        for &stable_addr in &self.chain_config.verified_stables {
            let stable_info = SecureTokenInfo {
                address: stable_addr,
                decimals: if stable_addr.to_string().to_lowercase().contains("2791bca1f2de4661ed88a30c99a7a9449aa84174") 
                         || stable_addr.to_string().to_lowercase().contains("c2132d05d31c914a87c6611c10748aeb04b58e8f") {
                    6 // USDC/USDT on Polygon
                } else {
                    18 // Default for other stables
                },
                is_verified: true,
                is_wrapped_native: false,
                is_known_stable: true,
                last_validated: current_timestamp(),
            };
            cache.insert(stable_addr, stable_info);
        }

        let count = cache.len();
        info!("🔒 Preloaded {} verified tokens", count);
        Ok(())
    }

    /// Make eth_call RPC request
    async fn make_eth_call(&self, to: Address, data: &str) -> Result<serde_json::Value> {
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_call",
            "params": [{
                "to": format!("{:?}", to),
                "data": data
            }, "latest"],
            "id": 1
        });
        
        let response = self.http_client
            .post(&self.chain_config.rpc_urls[0])
            .json(&request)
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;
        
        response.get("result")
            .ok_or_else(|| anyhow!("No result in RPC response"))
            .map(|v| v.clone())
    }

    /// Get DEX configuration (verified only)
    pub fn get_dex_config(&self, dex_name: &str) -> Result<&DexConfig> {
        let config = self.chain_config.dexs.get(dex_name)
            .ok_or_else(|| anyhow!("DEX '{}' not configured", dex_name))?;
        
        if !config.is_verified {
            return Err(anyhow!("🚨 SECURITY: DEX '{}' not verified", dex_name));
        }
        
        Ok(config)
    }

    /// Get chain ID
    pub fn get_chain_id(&self) -> u64 {
        self.chain_config.chain_id
    }

    /// Get Chainlink price feed address
    pub fn get_chainlink_feed(&self, pair: &str) -> Result<Address> {
        self.chain_config.chainlink_feeds.get(pair)
            .copied()
            .ok_or_else(|| anyhow!("Chainlink feed not configured for pair: {}", pair))
    }

    /// Get security stats
    pub async fn get_security_stats(&self) -> SecurityStats {
        let cache = self.token_cache.read().await;
        let total_cached = cache.len();
        let verified_count = cache.values().filter(|t| t.is_verified).count();
        let unknown_count = total_cached - verified_count;
        
        SecurityStats {
            total_tokens_cached: total_cached,
            verified_tokens: verified_count,
            unknown_tokens: unknown_count,
            verified_stables_count: self.chain_config.verified_stables.len(),
            allow_unknown_tokens: self.chain_config.allow_unknown_tokens,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SecurityStats {
    pub total_tokens_cached: usize,
    pub verified_tokens: usize,
    pub unknown_tokens: usize,
    pub verified_stables_count: usize,
    pub allow_unknown_tokens: bool,
}

fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_secure_registry_rejects_unknown_tokens() {
        let registry = SecureRegistryManager::new(137, "https://polygon-rpc.com".to_string()).await.unwrap();
        
        // Should accept verified USDC
        let usdc: Address = "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174".parse().unwrap();
        assert!(registry.get_secure_token_info(usdc).await.is_ok());
        
        // Should reject unknown token if allow_unknown_tokens = false
        let fake_token: Address = "0x1234567890123456789012345678901234567890".parse().unwrap();
        assert!(registry.get_secure_token_info(fake_token).await.is_err());
    }

    #[test]
    fn test_verified_stablecoin_detection() {
        let config = SecureRegistryManager::load_secure_chain_config(137).unwrap();
        let registry = SecureRegistryManager {
            chain_config: config,
            token_cache: Arc::new(RwLock::new(HashMap::new())),
            http_client: reqwest::Client::new(),
            provider: Arc::new(Provider::try_from("http://localhost:8545").unwrap()),
        };

        // Verified USDC should be detected as stablecoin
        let usdc: Address = "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174".parse().unwrap();
        assert!(registry.is_verified_stablecoin(usdc));
        
        // Random address should NOT be detected as stablecoin
        let random: Address = "0x1234567890123456789012345678901234567890".parse().unwrap();
        assert!(!registry.is_verified_stablecoin(random));
    }
}