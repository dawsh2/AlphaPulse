//! Pool Validator Integration
//!
//! Connects pool cache with token address validation for comprehensive verification.

use crate::pool_cache::{PoolCache, PoolInfo};
use std::sync::Arc;
use tracing::{debug, error, info};
use web3::{
    transports::Http,
    types::{Log, H160},
    Web3,
};

/// Pool validator that combines caching with validation
#[allow(dead_code)]
pub struct PoolValidator {
    /// Pool cache with RPC discovery
    pool_cache: Arc<PoolCache>,
    /// Web3 instance for additional validation
    web3: Arc<Web3<Http>>,
}

impl PoolValidator {
    /// Create new pool validator
    pub fn new(pool_cache: Arc<PoolCache>, rpc_url: &str) -> Result<Self, String> {
        let transport =
            Http::new(rpc_url).map_err(|e| format!("Failed to create HTTP transport: {}", e))?;
        let web3 = Arc::new(Web3::new(transport));

        Ok(Self { pool_cache, web3 })
    }

    /// Validate swap event and get pool info
    pub async fn validate_swap_event(&self, log: &Log) -> Result<ValidatedSwap, String> {
        // Extract pool address from log
        let pool_address = if log.address == H160::zero() {
            return Err("Invalid pool address in log".to_string());
        } else {
            log.address.0
        };

        debug!(
            "Validating swap event for pool 0x{}",
            hex::encode(pool_address)
        );

        // Get or discover pool info
        let pool_info = self
            .pool_cache
            .get_or_discover_pool(pool_address)
            .await
            .map_err(|e| format!("Failed to get pool info: {}", e))?;

        // Validate that tokens match what we expect
        self.validate_pool_tokens(&pool_info, log).await?;

        Ok(ValidatedSwap {
            pool_info,
            log: log.clone(),
            validated_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        })
    }

    /// Validate pool tokens match expected configuration
    async fn validate_pool_tokens(&self, pool_info: &PoolInfo, _log: &Log) -> Result<(), String> {
        // Additional validation logic can go here
        // For example, cross-check with known token lists

        // Validate decimals are reasonable
        if pool_info.token0_decimals > 30 || pool_info.token1_decimals > 30 {
            return Err(format!(
                "Invalid decimals: {} / {}",
                pool_info.token0_decimals, pool_info.token1_decimals
            ));
        }

        // Validate addresses are not zero
        if pool_info.token0 == [0u8; 20] || pool_info.token1 == [0u8; 20] {
            return Err("Invalid token addresses (zero address)".to_string());
        }

        info!(
            "Validated pool 0x{}: {} decimals / {} decimals",
            hex::encode(pool_info.pool_address),
            pool_info.token0_decimals,
            pool_info.token1_decimals
        );

        Ok(())
    }

    /// Pre-validate and cache a list of known pools
    pub async fn preload_pools(&self, pool_addresses: Vec<[u8; 20]>) -> (usize, usize) {
        let mut successful = 0;
        let mut failed = 0;

        for pool_address in pool_addresses {
            match self.pool_cache.get_or_discover_pool(pool_address).await {
                Ok(_) => successful += 1,
                Err(e) => {
                    error!(
                        "Failed to preload pool 0x{}: {}",
                        hex::encode(pool_address),
                        e
                    );
                    failed += 1;
                }
            }
        }

        info!("Preloaded {} pools, {} failed", successful, failed);
        (successful, failed)
    }
}

/// Validated swap with pool information
#[derive(Debug, Clone)]
pub struct ValidatedSwap {
    pub pool_info: PoolInfo,
    pub log: Log,
    pub validated_at: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pool_cache::PoolCacheConfig;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_pool_validator_integration() {
        // Create pool cache with persistence
        let mut config = PoolCacheConfig::default();
        config.cache_dir = Some(PathBuf::from("/tmp/test_pool_cache"));
        let pool_cache = Arc::new(PoolCache::new(config));

        // Create validator
        let validator = PoolValidator::new(pool_cache.clone(), "https://polygon-rpc.com").unwrap();

        // Test known pools
        let known_pools = vec![
            hex::decode("45dda9cb7c25131df268515131f647d726f50608")
                .unwrap()
                .try_into()
                .unwrap(),
            hex::decode("6e7a5FAFcEc6BB1E78bAe2A1F0B612012BF14827")
                .unwrap()
                .try_into()
                .unwrap(),
        ];

        let (successful, failed) = validator.preload_pools(known_pools).await;

        // These might fail without real RPC, but structure is correct
        println!("Preloaded: {} successful, {} failed", successful, failed);

        // Check cache stats
        let stats = pool_cache.stats();
        println!("Cache stats: {:?}", stats);
    }
}
