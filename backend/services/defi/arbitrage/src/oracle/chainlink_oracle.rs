// Chainlink Oracle Integration
// Fetches prices from Chainlink price feeds

use anyhow::{Result, Context};
use ethers::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, warn, error, debug};

/// Chainlink oracle for fetching reliable price data
pub struct ChainlinkOracle {
    provider: Arc<Provider<Http>>,
    price_feeds: HashMap<Address, Address>, // token -> price feed address
}

impl ChainlinkOracle {
    pub async fn new(provider: Arc<Provider<Http>>) -> Result<Self> {
        let mut price_feeds = HashMap::new();
        
        // Polygon mainnet Chainlink price feeds
        price_feeds.insert(
            "0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270".parse()?, // WMATIC
            "0xAB594600376Ec9fD91F8e885dADF0CE036862dE0".parse()?, // MATIC/USD
        );
        price_feeds.insert(
            "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174".parse()?, // USDC
            "0xfE4A8cc5b5B2366C1B58Bea3858e81843581b2F7".parse()?, // USDC/USD
        );
        price_feeds.insert(
            "0xc2132D05D31c914a87C6611C10748AEb04B58e8F".parse()?, // USDT
            "0x0A6513e40db6EB1b165753AD52E80663aeA50545".parse()?, // USDT/USD
        );
        price_feeds.insert(
            "0x1BFD67037B42Cf73acF2047067bd4F2C47D9BfD6".parse()?, // WBTC
            "0xDE31F8bFBD8c84b5360CFacCa3539B938dd78ae6".parse()?, // BTC/USD
        );
        price_feeds.insert(
            "0x7ceB23fD6bC0adD59E62ac25578270cFf1b9f619".parse()?, // WETH
            "0xF9680D99D6C9589e2a93a78A04A279e509205945".parse()?, // ETH/USD
        );
        
        info!("📡 Chainlink oracle initialized with {} price feeds", price_feeds.len());
        
        Ok(Self {
            provider,
            price_feeds,
        })
    }
    
    /// Get price from Chainlink price feed
    pub async fn get_price(&self, token: Address) -> Result<f64> {
        let feed_address = self.price_feeds.get(&token)
            .ok_or_else(|| anyhow::anyhow!("No Chainlink feed for token {:?}", token))?;
        
        debug!("📡 Fetching Chainlink price for {:?} from feed {:?}", token, feed_address);
        
        // Chainlink aggregator ABI
        let abi = ethers::abi::parse_abi(&[
            "function latestRoundData() external view returns (uint80 roundId, int256 answer, uint256 startedAt, uint256 updatedAt, uint80 answeredInRound)",
            "function decimals() external view returns (uint8)"
        ])?;
        
        let feed = Contract::new(*feed_address, abi, self.provider.clone());
        
        // Get latest price data
        let (round_id, answer, _started_at, updated_at, _answered_in_round): (u64, I256, U256, U256, u64) = feed
            .method::<_, (u64, I256, U256, U256, u64)>("latestRoundData", ())?
            .call()
            .await
            .context("Failed to fetch latest round data")?;
        
        // Get decimals
        let decimals: u8 = feed
            .method::<_, u8>("decimals", ())?
            .call()
            .await
            .context("Failed to fetch decimals")?;
        
        // Validate the data
        if answer.is_negative() {
            return Err(anyhow::anyhow!("Negative price from Chainlink feed"));
        }
        
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let price_age = current_time - updated_at.as_u64();
        if price_age > 3600 { // 1 hour
            warn!("⚠️ Chainlink price is {} seconds old for {:?}", price_age, token);
        }
        
        // Convert to USD with proper decimals
        let price_raw = answer.as_u128() as f64;
        let price_usd = price_raw / 10_f64.powi(decimals as i32);
        
        debug!("📡 Chainlink price for {:?}: ${:.6} (round: {}, age: {}s)", 
               token, price_usd, round_id, price_age);
        
        // Basic sanity check
        if price_usd <= 0.0 || price_usd > 1_000_000.0 {
            return Err(anyhow::anyhow!("Unreasonable price from Chainlink: ${:.6}", price_usd));
        }
        
        Ok(price_usd)
    }
    
    /// Get multiple prices efficiently
    pub async fn get_prices(&self, tokens: &[Address]) -> Result<HashMap<Address, f64>> {
        let mut prices = HashMap::new();
        
        for &token in tokens {
            if let Ok(price) = self.get_price(token).await {
                prices.insert(token, price);
            }
        }
        
        Ok(prices)
    }
    
    /// Check if feed is available for token
    pub fn has_feed(&self, token: Address) -> bool {
        self.price_feeds.contains_key(&token)
    }
    
    /// Get all supported tokens
    pub fn supported_tokens(&self) -> Vec<Address> {
        self.price_feeds.keys().copied().collect()
    }
    
    /// Add a new price feed
    pub fn add_feed(&mut self, token: Address, feed: Address) {
        self.price_feeds.insert(token, feed);
        info!("📡 Added Chainlink feed for {:?}: {:?}", token, feed);
    }
    
    /// Get feed health status
    pub async fn get_feed_health(&self, token: Address) -> Result<FeedHealthStatus> {
        let feed_address = self.price_feeds.get(&token)
            .ok_or_else(|| anyhow::anyhow!("No feed for token"))?;
        
        let abi = ethers::abi::parse_abi(&[
            "function latestRoundData() external view returns (uint80 roundId, int256 answer, uint256 startedAt, uint256 updatedAt, uint80 answeredInRound)"
        ])?;
        
        let feed = Contract::new(*feed_address, abi, self.provider.clone());
        
        let (_round_id, answer, _started_at, updated_at, _answered_in_round): (u64, I256, U256, U256, u64) = feed
            .method::<_, (u64, I256, U256, U256, u64)>("latestRoundData", ())?
            .call()
            .await?;
        
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let age_seconds = current_time - updated_at.as_u64();
        let is_positive = !answer.is_negative();
        
        let status = if !is_positive {
            FeedHealthStatus::Unhealthy("Negative price".to_string())
        } else if age_seconds > 7200 { // 2 hours
            FeedHealthStatus::Stale(age_seconds)
        } else if age_seconds > 3600 { // 1 hour
            FeedHealthStatus::Warning(age_seconds)
        } else {
            FeedHealthStatus::Healthy
        };
        
        Ok(status)
    }
}

#[derive(Debug, Clone)]
pub enum FeedHealthStatus {
    Healthy,
    Warning(u64), // age in seconds
    Stale(u64),   // age in seconds
    Unhealthy(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_chainlink_initialization() {
        let provider = Arc::new(Provider::<Http>::try_from("https://polygon-rpc.com").unwrap());
        let oracle = ChainlinkOracle::new(provider).await.unwrap();
        
        assert!(oracle.supported_tokens().len() > 0);
        
        let wmatic: Address = "0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270".parse().unwrap();
        assert!(oracle.has_feed(wmatic));
    }
}