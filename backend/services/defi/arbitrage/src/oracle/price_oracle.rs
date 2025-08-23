// Unified Price Oracle Interface
// Provides live price data from multiple sources with fallback mechanisms

use anyhow::{Result, Context};
use ethers::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::{info, warn, error, debug};

use crate::oracle::{ChainlinkOracle, DexPriceOracle};

/// Price data with metadata
#[derive(Debug, Clone)]
pub struct PriceData {
    pub price_usd: f64,
    pub timestamp: u64,
    pub source: PriceSource,
    pub confidence: f64,  // 0.0 to 1.0
    pub staleness_seconds: u64,
}

/// Price source identifier
#[derive(Debug, Clone, PartialEq)]
pub enum PriceSource {
    Chainlink,
    DexQuote,
    CachedValue,
    Fallback,
}

/// Main price oracle that aggregates multiple sources
pub struct PriceOracle {
    chainlink: Arc<ChainlinkOracle>,
    dex_oracle: Arc<DexPriceOracle>,
    price_cache: Arc<RwLock<HashMap<Address, PriceData>>>,
    config: OracleConfig,
}

#[derive(Debug, Clone)]
pub struct OracleConfig {
    pub max_staleness_seconds: u64,
    pub min_confidence: f64,
    pub use_chainlink: bool,
    pub use_dex_quotes: bool,
    pub cache_ttl_seconds: u64,
    pub fallback_prices: HashMap<Address, f64>,
}

impl Default for OracleConfig {
    fn default() -> Self {
        let mut fallback_prices = HashMap::new();
        
        // Add some conservative fallback prices for emergency use
        fallback_prices.insert(
            "0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270".parse().unwrap(), // WMATIC
            0.70 // Conservative MATIC price
        );
        fallback_prices.insert(
            "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174".parse().unwrap(), // USDC
            1.00 // USDC should be $1
        );
        fallback_prices.insert(
            "0xc2132D05D31c914a87C6611C10748AEb04B58e8F".parse().unwrap(), // USDT
            1.00 // USDT should be $1
        );
        
        Self {
            max_staleness_seconds: 300, // 5 minutes
            min_confidence: 0.7,
            use_chainlink: true,
            use_dex_quotes: true,
            cache_ttl_seconds: 60, // 1 minute cache
            fallback_prices,
        }
    }
}

impl PriceOracle {
    pub async fn new(provider: Arc<Provider<Http>>, config: OracleConfig) -> Result<Self> {
        let chainlink = Arc::new(ChainlinkOracle::new(provider.clone()).await?);
        let dex_oracle = Arc::new(DexPriceOracle::new(provider).await?);
        
        info!("🔮 Price oracle initialized with multiple sources");
        info!("  Chainlink: {}", config.use_chainlink);
        info!("  DEX quotes: {}", config.use_dex_quotes);
        info!("  Cache TTL: {}s", config.cache_ttl_seconds);
        info!("  Fallback prices: {}", config.fallback_prices.len());
        
        Ok(Self {
            chainlink,
            dex_oracle,
            price_cache: Arc::new(RwLock::new(HashMap::new())),
            config,
        })
    }
    
    /// Get current price with fallback logic
    pub async fn get_price(&self, token: Address) -> Result<PriceData> {
        // Check cache first
        if let Some(cached) = self.get_cached_price(token).await {
            if !self.is_stale(&cached) {
                debug!("📋 Using cached price for {:?}: ${:.4}", token, cached.price_usd);
                return Ok(cached);
            }
        }
        
        // Try primary sources
        let mut prices = Vec::new();
        
        // Try Chainlink first (most reliable)
        if self.config.use_chainlink {
            if let Ok(chainlink_price) = self.chainlink.get_price(token).await {
                prices.push(PriceData {
                    price_usd: chainlink_price,
                    timestamp: self.current_timestamp(),
                    source: PriceSource::Chainlink,
                    confidence: 0.95,
                    staleness_seconds: 0,
                });
                debug!("📡 Chainlink price for {:?}: ${:.4}", token, chainlink_price);
            }
        }
        
        // Try DEX quotes as backup
        if self.config.use_dex_quotes {
            if let Ok(dex_price) = self.dex_oracle.get_price(token).await {
                prices.push(PriceData {
                    price_usd: dex_price,
                    timestamp: self.current_timestamp(),
                    source: PriceSource::DexQuote,
                    confidence: 0.8,
                    staleness_seconds: 0,
                });
                debug!("🔄 DEX quote for {:?}: ${:.4}", token, dex_price);
            }
        }
        
        // Select best price
        let best_price = if let Some(price) = self.select_best_price(prices) {
            price
        } else {
            // Fall back to cached value if available
            if let Some(cached) = self.get_cached_price(token).await {
                warn!("⚠️ Using stale cached price for {:?}", token);
                PriceData {
                    price_usd: cached.price_usd,
                    timestamp: cached.timestamp,
                    source: PriceSource::CachedValue,
                    confidence: cached.confidence * 0.5, // Reduce confidence for stale data
                    staleness_seconds: self.current_timestamp() - cached.timestamp,
                }
            } else {
                // Last resort: fallback price
                let fallback_price = self.config.fallback_prices.get(&token)
                    .copied()
                    .unwrap_or(0.0);
                
                if fallback_price > 0.0 {
                    warn!("🆘 Using fallback price for {:?}: ${:.4}", token, fallback_price);
                    PriceData {
                        price_usd: fallback_price,
                        timestamp: self.current_timestamp(),
                        source: PriceSource::Fallback,
                        confidence: 0.3,
                        staleness_seconds: u64::MAX,
                    }
                } else {
                    return Err(anyhow::anyhow!("No price available for token {:?}", token));
                }
            }
        };
        
        // Cache the result
        self.cache_price(token, best_price.clone()).await;
        
        info!("💰 Price for {:?}: ${:.4} (source: {:?}, confidence: {:.1}%)", 
              token, best_price.price_usd, best_price.source, best_price.confidence * 100.0);
        
        Ok(best_price)
    }
    
    /// Get MATIC price specifically (commonly needed)
    pub async fn get_matic_price(&self) -> Result<f64> {
        let wmatic_address: Address = "0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270".parse()?;
        let price_data = self.get_price(wmatic_address).await?;
        Ok(price_data.price_usd)
    }
    
    /// Get USDC price (should be close to $1.00)
    pub async fn get_usdc_price(&self) -> Result<f64> {
        let usdc_address: Address = "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174".parse()?;
        let price_data = self.get_price(usdc_address).await?;
        Ok(price_data.price_usd)
    }
    
    /// Calculate gas cost in USD
    pub async fn calculate_gas_cost_usd(&self, gas_units: u64, gas_price: U256) -> Result<f64> {
        let matic_price = self.get_matic_price().await?;
        let gas_cost_matic = (gas_units as f64 * gas_price.as_u128() as f64) / 1e18;
        let gas_cost_usd = gas_cost_matic * matic_price;
        
        debug!("⛽ Gas cost: {} units @ {} Gwei = {:.6} MATIC (${:.4})", 
               gas_units, gas_price.as_u128() as f64 / 1e9, gas_cost_matic, gas_cost_usd);
        
        Ok(gas_cost_usd)
    }
    
    /// Calculate token value in USD
    pub async fn calculate_token_value_usd(&self, token: Address, amount: U256) -> Result<f64> {
        let price_data = self.get_price(token).await?;
        let token_amount = amount.as_u128() as f64 / 1e18; // Assuming 18 decimals
        let value_usd = token_amount * price_data.price_usd;
        
        debug!("💎 Token value: {} tokens @ ${:.4} = ${:.4}", 
               token_amount, price_data.price_usd, value_usd);
        
        Ok(value_usd)
    }
    
    /// Get multiple prices efficiently
    pub async fn get_prices(&self, tokens: &[Address]) -> Result<HashMap<Address, PriceData>> {
        let mut prices = HashMap::new();
        
        // Get prices in parallel
        let futures = tokens.iter().map(|&token| async move {
            match self.get_price(token).await {
                Ok(price) => Some((token, price)),
                Err(e) => {
                    warn!("Failed to get price for {:?}: {}", token, e);
                    None
                }
            }
        });
        
        let results = futures::future::join_all(futures).await;
        
        for result in results {
            if let Some((token, price)) = result {
                prices.insert(token, price);
            }
        }
        
        Ok(prices)
    }
    
    /// Validate price sanity (detect obvious errors)
    pub fn validate_price(&self, token: Address, price: f64) -> bool {
        // Basic sanity checks
        if price <= 0.0 || price > 1_000_000.0 {
            warn!("🚨 Suspicious price for {:?}: ${:.4}", token, price);
            return false;
        }
        
        // Check against fallback prices for reasonableness
        if let Some(&fallback) = self.config.fallback_prices.get(&token) {
            let ratio = price / fallback;
            if ratio < 0.1 || ratio > 10.0 {
                warn!("🚨 Price seems unreasonable for {:?}: ${:.4} (fallback: ${:.4})", 
                      token, price, fallback);
                return false;
            }
        }
        
        true
    }
    
    /// Update configuration
    pub async fn update_config(&mut self, config: OracleConfig) {
        self.config = config;
        info!("⚙️ Oracle configuration updated");
    }
    
    /// Clear price cache
    pub async fn clear_cache(&self) {
        let mut cache = self.price_cache.write().await;
        cache.clear();
        info!("🗑️ Price cache cleared");
    }
    
    /// Get cache statistics
    pub async fn get_cache_stats(&self) -> (usize, usize) {
        let cache = self.price_cache.read().await;
        let total_entries = cache.len();
        let fresh_entries = cache.values()
            .filter(|price| !self.is_stale(price))
            .count();
        
        (fresh_entries, total_entries)
    }
    
    // Helper methods
    
    async fn get_cached_price(&self, token: Address) -> Option<PriceData> {
        let cache = self.price_cache.read().await;
        cache.get(&token).cloned()
    }
    
    async fn cache_price(&self, token: Address, price: PriceData) {
        let mut cache = self.price_cache.write().await;
        cache.insert(token, price);
    }
    
    fn is_stale(&self, price: &PriceData) -> bool {
        let age = self.current_timestamp() - price.timestamp;
        age > self.config.cache_ttl_seconds
    }
    
    fn current_timestamp(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
    
    fn select_best_price(&self, mut prices: Vec<PriceData>) -> Option<PriceData> {
        if prices.is_empty() {
            return None;
        }
        
        // Filter out prices with low confidence
        prices.retain(|p| p.confidence >= self.config.min_confidence);
        
        if prices.is_empty() {
            return None;
        }
        
        // Sort by confidence (descending), then by freshness
        prices.sort_by(|a, b| {
            b.confidence.partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.staleness_seconds.cmp(&b.staleness_seconds))
        });
        
        prices.into_iter().next()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_price_validation() {
        let config = OracleConfig::default();
        let provider = Arc::new(Provider::<Http>::try_from("http://localhost:8545").unwrap());
        let oracle = PriceOracle::new(provider, config);
        
        // Would test price validation logic
        // assert!(oracle.validate_price(address, 1.0));
    }
    
    #[test]
    fn test_staleness_check() {
        let price = PriceData {
            price_usd: 1.0,
            timestamp: 1000,
            source: PriceSource::Chainlink,
            confidence: 0.95,
            staleness_seconds: 0,
        };
        
        // Would test staleness logic
        // assert!(!oracle.is_stale(&price));
    }
}