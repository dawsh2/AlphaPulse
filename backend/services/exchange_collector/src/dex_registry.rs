use anyhow::Result;
use std::collections::HashMap;
use parking_lot::RwLock;
use std::sync::Arc;
use crate::graph_client::GraphClient;

/// Registry for mapping pools to their DEX
/// This data is immutable once set - pools never change DEX
pub struct DexRegistry {
    // Pool address -> DEX name
    pool_to_dex: Arc<RwLock<HashMap<String, String>>>,
    
    // The Graph client for definitive DEX identification
    graph_client: GraphClient,
    
    // Known factory addresses on Polygon (fallback)
    quickswap_factory: String,
    sushiswap_factory: String,
    uniswap_v3_factory: String,
}

impl DexRegistry {
    pub fn new() -> Self {
        let registry = Self {
            pool_to_dex: Arc::new(RwLock::new(HashMap::new())),
            graph_client: GraphClient::new(),
            quickswap_factory: "0x5757371414417b8C6CAad45bAeF941aBc7d3Ab32".to_lowercase(),
            sushiswap_factory: "0xc35DADB65012eC5796536bD9864eD8773aBc7d3Ab32".to_lowercase(),
            uniswap_v3_factory: "0x1F98431c8aD98523631AE4a59f267346ea31F984".to_lowercase(),
        };
        
        // Preload known pools (these are verified)
        registry.preload_known_pools();
        registry
    }
    
    /// Get DEX name for a pool, with caching
    pub async fn get_dex_for_pool(&self, pool_address: &str) -> String {
        let addr = pool_address.to_lowercase();
        
        // Check cache first
        {
            let cache = self.pool_to_dex.read();
            if let Some(dex) = cache.get(&addr) {
                return dex.clone();
            }
        }
        
        // Not in cache, identify it
        let dex = self.identify_dex(&addr).await;
        
        // Cache it forever (pools never change DEX)
        {
            let mut cache = self.pool_to_dex.write();
            cache.insert(addr.clone(), dex.clone());
        }
        
        dex
    }
    
    /// Identify DEX from pool by checking factory addresses via RPC
    async fn identify_dex(&self, pool_address: &str) -> String {
        // Primary: Always attempt RPC call to get real factory address
        match self.check_factory_via_rpc(pool_address).await {
            Ok(dex) => {
                tracing::info!("✅ Pool {} identified as {} via factory check", pool_address, dex);
                return dex;
            }
            Err(e) => {
                tracing::warn!("⚠️ Factory check failed for {}: {}", pool_address, e);
            }
        }
        
        // Fallback: Mark as Unknown DEX - do NOT fabricate data
        // Better to have accurate partial data than complete fake data
        tracing::warn!("🔍 Pool {} marked as Unknown DEX - requires investigation", pool_address);
        "unknown".to_string()
    }
    
    /// Check which factory created a pool by querying the pool's factory() method
    async fn check_factory_via_rpc(&self, pool_address: &str) -> Result<String> {
        // nosec: Reading environment variable for RPC configuration
        let rpc_url = if let Ok(alchemy_key) = std::env::var("ALCHEMY_API_KEY") {
            if alchemy_key != "demo" && alchemy_key.len() > 10 {
                format!("https://polygon-mainnet.g.alchemy.com/v2/{}", alchemy_key)
            } else {
                "https://polygon-rpc.com".to_string()
            }
        } else {
            "https://polygon-rpc.com".to_string()
        };

        // Implement retry logic with exponential backoff
        let mut attempt = 0;
        let max_attempts = 3;
        
        while attempt < max_attempts {
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()?;
                
            // Call factory() method on the pool contract
            let factory_call = serde_json::json!({
                "jsonrpc": "2.0",
                "method": "eth_call",
                "params": [{
                    "to": pool_address,
                    "data": "0xc45a0155" // factory() method selector
                }, "latest"],
                "id": 1
            });
            
            match client
                .post(&rpc_url)
                .json(&factory_call)
                .send()
                .await
            {
                Ok(response) => {
                    if response.status().is_success() {
                        match response.json::<serde_json::Value>().await {
                            Ok(result) => {
                                return self.parse_factory_response(result, pool_address);
                            }
                            Err(e) => {
                                tracing::warn!("Failed to parse RPC response for {} (attempt {}): {}", pool_address, attempt + 1, e);
                            }
                        }
                    } else {
                        tracing::warn!("RPC request failed for {} (attempt {}) with status: {}", pool_address, attempt + 1, response.status());
                    }
                }
                Err(e) => {
                    tracing::warn!("RPC request error for {} (attempt {}): {}", pool_address, attempt + 1, e);
                }
            }
            
            attempt += 1;
            if attempt < max_attempts {
                let delay = std::time::Duration::from_millis(100 * (1 << attempt)); // Exponential backoff
                tokio::time::sleep(delay).await;
            }
        }
        
        Err(anyhow::anyhow!("All RPC attempts failed for {}", pool_address))
    }
    
    /// Parse the factory response and map to DEX name
    fn parse_factory_response(&self, result: serde_json::Value, pool_address: &str) -> Result<String> {
        // Check for RPC errors first
        if let Some(error) = result.get("error") {
            return Err(anyhow::anyhow!("RPC error: {:?}", error));
        }
        
        if let Some(factory_hex) = result.get("result").and_then(|r| r.as_str()) {
            // Convert hex to address (remove 0x and pad/truncate to 40 chars)
            let factory_addr = if factory_hex.len() >= 42 {
                format!("0x{}", &factory_hex[factory_hex.len()-40..]).to_lowercase()
            } else {
                return Err(anyhow::anyhow!("Invalid factory address format: {}", factory_hex));
            };
            
            // Match against known factory addresses
            match factory_addr.as_str() {
                "0x5757371414417b8c6caad45baef941abc7d3ab32" => {
                    tracing::info!("🎯 Factory match: {} -> QuickSwap", pool_address);
                    Ok("quickswap".to_string())
                },
                "0xc35dadb65012ec5796536bd9864ed8773abc74c4" => {
                    tracing::info!("🎯 Factory match: {} -> SushiSwap", pool_address);
                    Ok("sushiswap".to_string())
                },
                "0x1f98431c8ad98523631ae4a59f267346ea31f984" => {
                    tracing::info!("🎯 Factory match: {} -> Uniswap V3", pool_address);
                    Ok("uniswapv3".to_string())
                },
                _ => {
                    tracing::warn!("❓ Unknown factory {} for pool {}", factory_addr, pool_address);
                    Err(anyhow::anyhow!("Unknown factory: {}", factory_addr))
                },
            }
        } else {
            Err(anyhow::anyhow!("No factory address returned from RPC"))
        }
    }
    
    /// Preload known pools to avoid queries
    fn preload_known_pools(&self) {
        let mut cache = self.pool_to_dex.write();
        
        // Major QuickSwap pools (verified) - focusing on high-volume pairs
        cache.insert("0x6e7a5fafcec6bb1e78bae2a1f0b612012bf14827".to_string(), "quickswap".to_string()); // WMATIC/USDC.e
        cache.insert("0x853ee4b2063c514516871ab40063b45fb6daa497".to_string(), "quickswap".to_string()); // USDC/WETH
        cache.insert("0x5ca6ca6c3709e1e6cfe74a50cf6b2b6ba2dadd67".to_string(), "quickswap".to_string()); // WMATIC/WETH
        cache.insert("0xf6422aa4dd2dd85f0833e12137bb9c2b2aaae55b".to_string(), "quickswap".to_string()); // USDC/USDT
        cache.insert("0x45a01e4e04f14f7a4a6702c74187c5f6222033cd".to_string(), "quickswap".to_string()); // DAI/USDC
        cache.insert("0x191c10aa4af7c30e871e70c95db0e4eb77237530".to_string(), "quickswap".to_string()); // More QS pools
        cache.insert("0x4a35582a710e1f4b2030a3f826da20bfb6703c09".to_string(), "quickswap".to_string());
        cache.insert("0xadbf1854e5883eb8aa7baf50705338739e558e5b".to_string(), "quickswap".to_string()); 
        cache.insert("0x65d43b64e3b31965cd5ea367d4c2b94c03084797".to_string(), "quickswap".to_string()); // Active pool from logs
        
        // SushiSwap pools (major pairs)
        cache.insert("0x882df4b0fb50a229c3b4124eb18c759911485bfb".to_string(), "sushiswap".to_string()); // DAI/LGNS
        cache.insert("0x116ff0d1caa91a6b94276b3471f33dbeb52073e7".to_string(), "sushiswap".to_string()); 
        cache.insert("0x4b1f1e2435a9c96f7330faea190ef6a7c8d70001".to_string(), "sushiswap".to_string()); // WETH/USDC
        cache.insert("0x396e655c309676caf0acf4607a868e0cded876db".to_string(), "sushiswap".to_string()); // WMATIC/USDC
        
        // UniswapV3 pools (major pairs)
        cache.insert("0x1e67124681b402064cd0abe8ed1b5c79d2e02f64".to_string(), "uniswapv3".to_string());
        cache.insert("0x45dda9cb7c25131df268515131f647d726f50608".to_string(), "uniswapv3".to_string()); // WETH/USDC
        cache.insert("0xa374094527e1673a86de625aa59517c5de346d32".to_string(), "uniswapv3".to_string()); // WMATIC/USDC
        
        // Mark frequently seen unknown pools to avoid repeated RPC calls
        let unknown_pools = vec![
            "0x8312a29a91d9fac706f4d2adeb1fa4540fad1673",
            "0x34965ba0ac2451a34a0471f04cca3f990b8dea27", 
            "0x853ee4b2a13f8a742d64c8f088be7ba2131f670d",
            "0xcf2abff7b321ccaaaf4faca391aa4ffc87efec13",
            "0x1a9221261dc445d773e66075b9e9e52f40e15ab1", // Causing massive rate limits
            "0x84964d9f9480a1db644c2b2d1022765179a40f68", // Another unknown factory
            "0x88fe363c2c011b9dc3e8aceeec79e1a752e66a92", // Recent unknown pool
            "0x65d43b64e3b31965cd5ea367d4c2b94c03084797"  // Recent unknown pool
        ];
        
        for pool in unknown_pools {
            cache.insert(pool.to_string(), "unknown".to_string());
        }
        
        tracing::info!("✅ Preloaded {} known pools into DEX registry cache", cache.len());
    }
    
    /// Get enhanced pool information from The Graph
    pub async fn get_pool_info(&self, pool_address: &str, dex_name: &str) -> Result<(String, String, u8, u8)> {
        match self.graph_client.get_pool_info(pool_address, dex_name).await {
            Ok(info) => Ok(info),
            Err(e) => {
                tracing::warn!("Failed to get pool info from The Graph for {}: {}", pool_address, e);
                Err(e)
            }
        }
    }
    
    /// Get cache statistics with DEX distribution and Unknown percentage
    pub fn get_stats(&self) -> (usize, Vec<(String, usize)>, f64) {
        let cache = self.pool_to_dex.read();
        let total = cache.len();
        
        let mut dex_counts: HashMap<String, usize> = HashMap::new();
        for dex in cache.values() {
            *dex_counts.entry(dex.clone()).or_insert(0) += 1;
        }
        
        let unknown_count = dex_counts.get("unknown").unwrap_or(&0);
        let unknown_percentage = if total > 0 {
            (*unknown_count as f64 / total as f64) * 100.0
        } else {
            0.0
        };
        
        let mut counts: Vec<_> = dex_counts.into_iter().collect();
        counts.sort_by(|a, b| b.1.cmp(&a.1));
        
        (total, counts, unknown_percentage)
    }
}