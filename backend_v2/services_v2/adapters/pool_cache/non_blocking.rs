//! Non-blocking pool cache interface
//!
//! Provides async methods that never block on RPC calls.
//! Discovery requests are queued and processed in the background.

use super::{PoolCache, PoolInfo, PoolCacheError};
use super::discovery::{DiscoveryRequest, PoolDiscoveryService};
use crate::config::ChainConfig;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, info, warn};

/// Non-blocking pool cache handle
#[derive(Clone)]
pub struct NonBlockingPoolCache {
    /// Inner pool cache
    inner: Arc<PoolCache>,
    /// Channel to send discovery requests
    discovery_tx: mpsc::Sender<DiscoveryRequest>,
}

impl NonBlockingPoolCache {
    /// Create a new non-blocking pool cache with discovery service
    pub async fn new(
        chain_config: Arc<ChainConfig>,
        cache_dir: Option<std::path::PathBuf>,
    ) -> Result<Self, anyhow::Error> {
        // Create the inner pool cache
        let inner = Arc::new(PoolCache::new_with_config(
            super::PoolCacheConfig {
                primary_rpc: chain_config.rpc_endpoints.primary.clone(),
                backup_rpcs: chain_config.rpc_endpoints.fallback.clone(),
                max_concurrent_discoveries: 10,
                rpc_timeout_ms: 5000,
                max_retries: 3,
                rate_limit_per_sec: 100,
                cache_dir,
                chain_id: chain_config.chain_id,
            }
        ).await?);
        
        // Create discovery channel
        let (discovery_tx, discovery_rx) = mpsc::channel(100);
        
        // Create Web3 instance for discovery service
        let web3 = Arc::new(
            web3::Web3::new(
                web3::transports::Http::new(&chain_config.rpc_endpoints.primary)?
            )
        );
        
        // Create and spawn discovery service
        let discovery_service = PoolDiscoveryService::new(
            chain_config,
            web3,
            discovery_rx,
            10, // max concurrent discoveries
            5000, // RPC timeout ms
        );
        
        tokio::spawn(async move {
            discovery_service.run().await;
        });
        
        Ok(Self {
            inner,
            discovery_tx,
        })
    }
    
    /// Get pool info - returns immediately with cached data or None
    /// 
    /// This method NEVER blocks on RPC calls. If the pool is not cached,
    /// it returns None and triggers background discovery.
    pub async fn get_pool_non_blocking(&self, pool_address: [u8; 20]) -> Option<PoolInfo> {
        // Check cache first
        if let Some(pool_info) = self.inner.get_pool(pool_address) {
            debug!("Pool cache hit for 0x{}", hex::encode(pool_address));
            return Some(pool_info);
        }
        
        // Not in cache - trigger background discovery
        debug!("Pool cache miss for 0x{}, triggering background discovery", hex::encode(pool_address));
        self.trigger_discovery(pool_address);
        
        None
    }
    
    /// Get pool info with async wait for discovery
    /// 
    /// This method will wait for discovery to complete if the pool is not cached.
    /// Use this when you need the pool info and can afford to wait.
    pub async fn get_pool_async(&self, pool_address: [u8; 20]) -> Result<PoolInfo, PoolCacheError> {
        // Check cache first
        if let Some(pool_info) = self.inner.get_pool(pool_address) {
            return Ok(pool_info);
        }
        
        // Not in cache - request discovery and wait
        let (response_tx, response_rx) = oneshot::channel();
        
        let request = DiscoveryRequest {
            pool_address,
            response_tx,
        };
        
        // Send discovery request
        if self.discovery_tx.send(request).await.is_err() {
            return Err(PoolCacheError::Other(
                anyhow::anyhow!("Discovery service is not running")
            ));
        }
        
        // Wait for response
        match response_rx.await {
            Ok(Ok(pool_info)) => {
                // Cache the discovered pool
                self.inner.insert_pool(pool_info.clone());
                Ok(pool_info)
            },
            Ok(Err(e)) => Err(e),
            Err(_) => Err(PoolCacheError::Other(
                anyhow::anyhow!("Discovery service dropped the request")
            )),
        }
    }
    
    /// Trigger background discovery for a pool
    /// 
    /// This method returns immediately and discovery happens in the background.
    /// The discovered pool will be available in cache for future requests.
    pub fn trigger_discovery(&self, pool_address: [u8; 20]) {
        // Check if already discovering
        if self.inner.is_discovery_in_progress(pool_address) {
            debug!("Discovery already in progress for 0x{}", hex::encode(pool_address));
            return;
        }
        
        // Mark as discovering
        self.inner.mark_discovery_in_progress(pool_address);
        
        // Clone for async block
        let discovery_tx = self.discovery_tx.clone();
        let inner = self.inner.clone();
        
        // Spawn task to send discovery request
        tokio::spawn(async move {
            let (response_tx, response_rx) = oneshot::channel();
            
            let request = DiscoveryRequest {
                pool_address,
                response_tx,
            };
            
            // Send discovery request (ignore if channel closed)
            if discovery_tx.send(request).await.is_ok() {
                // Wait for response and cache it
                if let Ok(Ok(pool_info)) = response_rx.await {
                    info!("Background discovery completed for 0x{}", hex::encode(pool_address));
                    inner.insert_pool(pool_info);
                }
            }
            
            // Clear discovery flag
            inner.clear_discovery_in_progress(pool_address);
        });
    }
    
    /// Pre-warm cache with a list of pool addresses
    /// 
    /// This triggers background discovery for multiple pools.
    /// Useful at startup to pre-populate the cache.
    pub fn prewarm_pools(&self, pool_addresses: Vec<[u8; 20]>) {
        info!("Pre-warming cache with {} pools", pool_addresses.len());
        
        for pool_address in pool_addresses {
            self.trigger_discovery(pool_address);
        }
    }
    
    /// Get cache statistics
    pub fn get_stats(&self) -> super::PoolCacheStats {
        self.inner.get_stats()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_non_blocking_cache() {
        // This would need proper test setup with mock RPC
        // For now, just ensure it compiles
    }
}