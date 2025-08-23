use crate::{PoolInfo, ArbitrageOpportunity};
use anyhow::Result;
use dashmap::DashMap;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use parking_lot::RwLock;
use tracing::{debug, warn};

/// Pool state manager with optimized indices for fast arbitrage detection
pub struct PoolState {
    // Primary pool storage - keyed by pool address
    pools: DashMap<String, PoolInfo>,
    
    // Token pair indices for fast arbitrage detection
    // Key: "token0:token1" (sorted), Value: Vec<pool_addresses>
    token_pair_index: DashMap<String, Vec<String>>,
    
    // Cross-token opportunities index  
    // Key: token_address, Value: Vec<pool_addresses> containing that token
    token_index: DashMap<String, Vec<String>>,
    
    // Exchange-specific indices
    // Key: exchange_name, Value: Vec<pool_addresses>
    exchange_index: DashMap<String, Vec<String>>,
    
    // Performance metrics
    total_updates: AtomicU64,
    last_cleanup: AtomicU64,
    
    // Known token metadata for fast lookups
    token_metadata: RwLock<HashMap<String, TokenMetadata>>,
    
    // USDC variants for cross-token arbitrage
    usdc_variants: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TokenMetadata {
    pub symbol: String,
    pub decimals: u8,
    pub is_stablecoin: bool,
    pub is_wrapped_native: bool,
}

impl PoolState {
    pub fn new() -> Self {
        Self {
            pools: DashMap::new(),
            token_pair_index: DashMap::new(),
            token_index: DashMap::new(),
            exchange_index: DashMap::new(),
            total_updates: AtomicU64::new(0),
            last_cleanup: AtomicU64::new(current_time_ns()),
            token_metadata: RwLock::new(Self::init_token_metadata()),
            usdc_variants: vec![
                "0x2791bca1f2de4661ed88a30c99a7a9449aa84174".to_string(), // USDC.e
                "0x3c499c542cef5e3811e1192ce70d8cc03d5c3359".to_string(), // USDC
            ],
        }
    }
    
    /// Initialize known token metadata for Polygon
    fn init_token_metadata() -> HashMap<String, TokenMetadata> {
        let mut metadata = HashMap::new();
        
        // Major stablecoins
        metadata.insert("0x2791bca1f2de4661ed88a30c99a7a9449aa84174".to_string(), TokenMetadata {
            symbol: "USDC.e".to_string(),
            decimals: 6,
            is_stablecoin: true,
            is_wrapped_native: false,
        });
        metadata.insert("0x3c499c542cef5e3811e1192ce70d8cc03d5c3359".to_string(), TokenMetadata {
            symbol: "USDC".to_string(),
            decimals: 6,
            is_stablecoin: true,
            is_wrapped_native: false,
        });
        metadata.insert("0xc2132d05d31c914a87c6611c10748aeb04b58e8f".to_string(), TokenMetadata {
            symbol: "USDT".to_string(),
            decimals: 6,
            is_stablecoin: true,
            is_wrapped_native: false,
        });
        metadata.insert("0x8f3cf7ad23cd3cadbd9735aff958023239c6a063".to_string(), TokenMetadata {
            symbol: "DAI".to_string(),
            decimals: 18,
            is_stablecoin: true,
            is_wrapped_native: false,
        });
        
        // Native and wrapped tokens
        metadata.insert("0x0d500b1d8e8ef31e21c99d1db9a6444d3adf1270".to_string(), TokenMetadata {
            symbol: "WPOL".to_string(),
            decimals: 18,
            is_stablecoin: false,
            is_wrapped_native: true,
        });
        metadata.insert("0x7ceb23fd6bc0add59e62ac25578270cff1b9f619".to_string(), TokenMetadata {
            symbol: "WETH".to_string(),
            decimals: 18,
            is_stablecoin: false,
            is_wrapped_native: false,
        });
        metadata.insert("0x1bfd67037b42cf73acf2047067bd4f2c47d9bfd6".to_string(), TokenMetadata {
            symbol: "WBTC".to_string(),
            decimals: 8,
            is_stablecoin: false,
            is_wrapped_native: false,
        });
        
        metadata
    }
    
    /// Update pool state with automatic index maintenance
    pub fn update_pool(&self, pool: PoolInfo) {
        let address = pool.address.clone();
        let token0 = pool.token0.clone();
        let token1 = pool.token1.clone();
        let exchange = pool.exchange.clone();
        
        // Update main pool storage
        self.pools.insert(address.clone(), pool);
        
        // Update indices for fast arbitrage detection
        self.update_indices(&address, &token0, &token1, &exchange);
        
        // Track metrics
        self.total_updates.fetch_add(1, Ordering::Relaxed);
    }
    
    /// Update search indices for fast lookups
    fn update_indices(&self, pool_address: &str, token0: &str, token1: &str, exchange: &str) {
        // Create sorted token pair key for consistent lookups
        let pair_key = if token0 < token1 {
            format!("{}:{}", token0, token1)
        } else {
            format!("{}:{}", token1, token0)
        };
        
        // Update token pair index
        self.token_pair_index.entry(pair_key)
            .and_modify(|pools| {
                if !pools.contains(&pool_address.to_string()) {
                    pools.push(pool_address.to_string());
                }
            })
            .or_insert_with(|| vec![pool_address.to_string()]);
        
        // Update individual token indices
        for token in [token0, token1] {
            self.token_index.entry(token.to_string())
                .and_modify(|pools| {
                    if !pools.contains(&pool_address.to_string()) {
                        pools.push(pool_address.to_string());
                    }
                })
                .or_insert_with(|| vec![pool_address.to_string()]);
        }
        
        // Update exchange index
        self.exchange_index.entry(exchange.to_string())
            .and_modify(|pools| {
                if !pools.contains(&pool_address.to_string()) {
                    pools.push(pool_address.to_string());
                }
            })
            .or_insert_with(|| vec![pool_address.to_string()]);
    }
    
    /// Get all pools for a token pair (for direct arbitrage)
    pub fn get_pools_for_pair(&self, token0: &str, token1: &str) -> Vec<PoolInfo> {
        let pair_key = if token0 < token1 {
            format!("{}:{}", token0, token1)
        } else {
            format!("{}:{}", token1, token0)
        };
        
        self.token_pair_index.get(&pair_key)
            .map(|pool_addresses| {
                pool_addresses
                    .iter()
                    .filter_map(|addr| self.pools.get(addr).map(|entry| entry.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }
    
    /// Get all pools containing a specific token (for cross-token arbitrage)
    pub fn get_pools_for_token(&self, token: &str) -> Vec<PoolInfo> {
        self.token_index.get(token)
            .map(|pool_addresses| {
                pool_addresses
                    .iter()
                    .filter_map(|addr| self.pools.get(addr).map(|entry| entry.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }
    
    /// Get pools by exchange
    pub fn get_pools_by_exchange(&self, exchange: &str) -> Vec<PoolInfo> {
        self.exchange_index.get(exchange)
            .map(|pool_addresses| {
                pool_addresses
                    .iter()
                    .filter_map(|addr| self.pools.get(addr).map(|entry| entry.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }
    
    /// Get pool by address
    pub fn get_pool(&self, address: &str) -> Option<PoolInfo> {
        self.pools.get(address).map(|entry| entry.clone())
    }
    
    /// Get all pools
    pub fn get_all_pools(&self) -> Vec<PoolInfo> {
        self.pools.iter().map(|entry| entry.clone()).collect()
    }
    
    /// Find cross-token arbitrage opportunities (USDC variants)
    pub fn find_cross_token_opportunities(&self, pool: &PoolInfo) -> Vec<PoolInfo> {
        let mut opportunities = Vec::new();
        
        // Check if pool contains USDC variants
        let has_usdc_variant = self.usdc_variants.contains(&pool.token0) || 
                             self.usdc_variants.contains(&pool.token1);
        
        if !has_usdc_variant {
            return opportunities;
        }
        
        // Find pools with other USDC variants
        for variant in &self.usdc_variants {
            if variant == &pool.token0 || variant == &pool.token1 {
                continue; // Skip same variant
            }
            
            let variant_pools = self.get_pools_for_token(variant);
            opportunities.extend(variant_pools);
        }
        
        opportunities
    }
    
    /// Get token metadata
    pub fn get_token_metadata(&self, address: &str) -> Option<TokenMetadata> {
        self.token_metadata.read().get(address).cloned()
    }
    
    /// Clean up stale pools
    pub fn cleanup_stale_pools(&self, max_age_seconds: u64) -> usize {
        let now = current_time_ns();
        let cutoff_ns = now - (max_age_seconds * 1_000_000_000);
        
        let mut removed_count = 0;
        let mut stale_pools = Vec::new();
        
        // Find stale pools
        for entry in self.pools.iter() {
            let pool_age_ns = entry.last_updated as u64 * 1_000_000_000;
            if pool_age_ns < cutoff_ns {
                stale_pools.push(entry.address.clone());
            }
        }
        
        // Remove from main storage and indices
        for address in stale_pools {
            if let Some((_, _)) = self.pools.remove(&address) {
                removed_count += 1;
                // Note: For performance, we don't clean indices immediately
                // They will be cleaned during the next rebuild cycle
            }
        }
        
        self.last_cleanup.store(now, Ordering::Relaxed);
        removed_count
    }
    
    /// Rebuild indices for consistency (run periodically)
    pub fn rebuild_indices(&self) {
        debug!("Rebuilding pool indices for consistency");
        
        // Clear all indices
        self.token_pair_index.clear();
        self.token_index.clear();
        self.exchange_index.clear();
        
        // Rebuild from current pools
        for entry in self.pools.iter() {
            let pool = entry.value();
            self.update_indices(&pool.address, &pool.token0, &pool.token1, &pool.exchange);
        }
    }
    
    /// Get performance statistics
    pub fn get_stats(&self) -> PoolStateStats {
        PoolStateStats {
            total_pools: self.pools.len(),
            total_updates: self.total_updates.load(Ordering::Relaxed),
            last_cleanup_ns: self.last_cleanup.load(Ordering::Relaxed),
            token_pairs: self.token_pair_index.len(),
            indexed_tokens: self.token_index.len(),
            indexed_exchanges: self.exchange_index.len(),
        }
    }
}

#[derive(Debug)]
pub struct PoolStateStats {
    pub total_pools: usize,
    pub total_updates: u64,
    pub last_cleanup_ns: u64,
    pub token_pairs: usize,
    pub indexed_tokens: usize,
    pub indexed_exchanges: usize,
}

/// Get current time in nanoseconds
pub fn current_time_ns() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64
}