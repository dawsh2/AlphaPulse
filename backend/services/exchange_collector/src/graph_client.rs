use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use parking_lot::RwLock;
use std::sync::Arc;
use tracing::{debug, info, warn};

#[derive(Debug, Serialize)]
struct GraphQuery {
    query: String,
}

#[derive(Debug, Deserialize)]
struct GraphResponse<T> {
    data: Option<T>,
    errors: Option<Vec<GraphError>>,
}

#[derive(Debug, Deserialize)]
struct GraphError {
    message: String,
}

#[derive(Debug, Deserialize)]
struct PoolQueryData {
    pools: Vec<PoolInfo>,
}

#[derive(Debug, Deserialize)]
struct PoolInfo {
    id: String,
    token0: TokenInfo,
    token1: TokenInfo,
}

#[derive(Debug, Deserialize)]
struct TokenInfo {
    id: String,
    symbol: String,
    decimals: String,
}

#[derive(Debug, Deserialize)]
struct FactoryQueryData {
    factory: Option<FactoryInfo>,
}

#[derive(Debug, Deserialize)]
struct FactoryInfo {
    id: String,
}

pub struct GraphClient {
    cache: Arc<RwLock<HashMap<String, String>>>,
    quickswap_endpoint: String,
    sushiswap_endpoint: String,
    uniswapv3_endpoint: String,
    client: reqwest::Client,
}

impl GraphClient {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            // Using working Graph endpoints for Polygon
            quickswap_endpoint: "https://api.thegraph.com/subgraphs/name/sameepsi/quickswap-v3".to_string(),
            sushiswap_endpoint: "https://api.thegraph.com/subgraphs/name/sushiswap/exchange-polygon".to_string(),
            uniswapv3_endpoint: "https://api.thegraph.com/subgraphs/name/ianlapham/uniswap-v3-polygon".to_string(),
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap(),
        }
    }
    
    /// Get DEX name for a pool address using The Graph
    pub async fn get_dex_for_pool(&self, pool_address: &str) -> Result<String> {
        let addr = pool_address.to_lowercase();
        
        // Check cache first
        {
            let cache = self.cache.read();
            if let Some(dex) = cache.get(&addr) {
                return Ok(dex.clone());
            }
        }
        
        // Query each DEX subgraph to find the pool
        let dex = self.identify_dex_via_graph(&addr).await?;
        
        // Cache it forever (pools never change DEX)
        {
            let mut cache = self.cache.write();
            cache.insert(addr.clone(), dex.clone());
        }
        
        Ok(dex)
    }
    
    /// Query The Graph to identify which DEX owns this pool
    async fn identify_dex_via_graph(&self, pool_address: &str) -> Result<String> {
        // Try QuickSwap V3 first (most active on Polygon)
        if let Ok(found) = self.check_quickswap_pool(pool_address).await {
            if found {
                debug!("Pool {} identified as QuickSwap via The Graph", pool_address);
                return Ok("quickswap".to_string());
            }
        }
        
        // Try SushiSwap
        if let Ok(found) = self.check_sushiswap_pool(pool_address).await {
            if found {
                debug!("Pool {} identified as SushiSwap via The Graph", pool_address);
                return Ok("sushiswap".to_string());
            }
        }
        
        // Try Uniswap V3
        if let Ok(found) = self.check_uniswapv3_pool(pool_address).await {
            if found {
                debug!("Pool {} identified as Uniswap V3 via The Graph", pool_address);
                return Ok("uniswapv3".to_string());
            }
        }
        
        // Default to QuickSwap if not found (most common on Polygon)
        warn!("Pool {} not found in any subgraph, defaulting to QuickSwap", pool_address);
        Ok("quickswap".to_string())
    }
    
    async fn check_quickswap_pool(&self, pool_address: &str) -> Result<bool> {
        let query = format!(
            r#"{{
                pools(where: {{ id: "{}" }}) {{
                    id
                    token0 {{
                        id
                        symbol
                        decimals
                    }}
                    token1 {{
                        id
                        symbol
                        decimals
                    }}
                }}
            }}"#,
            pool_address
        );
        
        match self.client
            .post(&self.quickswap_endpoint)
            .json(&GraphQuery { query })
            .send()
            .await
        {
            Ok(response) => {
                if response.status().is_success() {
                    match response.json::<GraphResponse<PoolQueryData>>().await {
                        Ok(graph_response) => {
                            if let Some(data) = graph_response.data {
                                return Ok(!data.pools.is_empty());
                            }
                        }
                        Err(e) => {
                            debug!("Failed to parse QuickSwap graph response: {}", e);
                        }
                    }
                } else {
                    debug!("QuickSwap graph API returned status: {}", response.status());
                }
            }
            Err(e) => {
                debug!("QuickSwap graph query failed: {}", e);
            }
        }
        
        Ok(false)
    }
    
    async fn check_sushiswap_pool(&self, pool_address: &str) -> Result<bool> {
        // SushiSwap uses 'pairs' instead of 'pools' in their subgraph
        let query = format!(
            r#"{{
                pairs(where: {{ id: "{}" }}) {{
                    id
                    token0 {{
                        id
                        symbol
                        decimals
                    }}
                    token1 {{
                        id
                        symbol
                        decimals
                    }}
                }}
            }}"#,
            pool_address
        );
        
        let response = self.client
            .post(&self.sushiswap_endpoint)
            .json(&GraphQuery { query })
            .send()
            .await;
        
        match response {
            Ok(resp) => {
                if let Ok(json) = resp.json::<serde_json::Value>().await {
                    if let Some(data) = json.get("data") {
                        if let Some(pairs) = data.get("pairs") {
                            if let Some(arr) = pairs.as_array() {
                                return Ok(!arr.is_empty());
                            }
                        }
                    }
                }
            }
            Err(e) => {
                debug!("SushiSwap graph query failed: {}", e);
            }
        }
        
        Ok(false)
    }
    
    async fn check_uniswapv3_pool(&self, pool_address: &str) -> Result<bool> {
        let query = format!(
            r#"{{
                pools(where: {{ id: "{}" }}) {{
                    id
                    token0 {{
                        id
                        symbol
                        decimals
                    }}
                    token1 {{
                        id
                        symbol
                        decimals
                    }}
                }}
            }}"#,
            pool_address
        );
        
        let response = self.client
            .post(&self.uniswapv3_endpoint)
            .json(&GraphQuery { query })
            .send()
            .await;
        
        match response {
            Ok(resp) => {
                if let Ok(json) = resp.json::<GraphResponse<PoolQueryData>>().await {
                    if let Some(data) = json.data {
                        return Ok(!data.pools.is_empty());
                    }
                }
            }
            Err(e) => {
                debug!("Uniswap V3 graph query failed: {}", e);
            }
        }
        
        Ok(false)
    }
    
    /// Get pool token information from The Graph
    pub async fn get_pool_info(&self, pool_address: &str, dex: &str) -> Result<(String, String, u8, u8)> {
        let query = format!(
            r#"{{
                pools(where: {{ id: "{}" }}) {{
                    token0 {{
                        symbol
                        decimals
                    }}
                    token1 {{
                        symbol
                        decimals
                    }}
                }}
            }}"#,
            pool_address.to_lowercase()
        );
        
        let endpoint = match dex {
            "quickswap" => &self.quickswap_endpoint,
            "sushiswap" => &self.sushiswap_endpoint,
            "uniswapv3" => &self.uniswapv3_endpoint,
            _ => &self.quickswap_endpoint,
        };
        
        let response: GraphResponse<PoolQueryData> = self.client
            .post(endpoint)
            .json(&GraphQuery { query })
            .send()
            .await?
            .json()
            .await?;
        
        if let Some(data) = response.data {
            if let Some(pool) = data.pools.first() {
                let token0_decimals = pool.token0.decimals.parse::<u8>().unwrap_or(18);
                let token1_decimals = pool.token1.decimals.parse::<u8>().unwrap_or(18);
                
                return Ok((
                    pool.token0.symbol.clone(),
                    pool.token1.symbol.clone(),
                    token0_decimals,
                    token1_decimals,
                ));
            }
        }
        
        Err(anyhow::anyhow!("Pool not found in Graph"))
    }
}