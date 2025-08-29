//! Non-blocking pool discovery with chains.json optimization
//!
//! This module handles asynchronous pool discovery with:
//! - Zero RPC calls for known tokens (from chains.json)
//! - Non-blocking concurrent RPC discovery for unknown tokens
//! - Automatic queueing and deduplication of discovery requests

use super::{PoolCache, PoolInfo, PoolCacheError};
use crate::config::{ChainConfig, get_token_by_address};
use torq_types::tlv::DEXProtocol;
use torq_types::VenueId;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};
use tokio::time::{timeout, Duration};
use tracing::{debug, info, warn, error};
use web3::{
    transports::Http,
    types::{CallRequest, H160},
    Web3,
};

/// Pool discovery request
#[derive(Debug)]
pub struct DiscoveryRequest {
    pub pool_address: [u8; 20],
    pub response_tx: oneshot::Sender<Result<PoolInfo, PoolCacheError>>,
}

/// Non-blocking pool discovery service
pub struct PoolDiscoveryService {
    /// Chain configuration with known tokens
    chain_config: Arc<ChainConfig>,
    /// Web3 instance for RPC calls
    web3: Arc<Web3<Http>>,
    /// Request queue for discovery
    request_rx: mpsc::Receiver<DiscoveryRequest>,
    /// Maximum concurrent discoveries
    max_concurrent: usize,
    /// RPC timeout
    rpc_timeout: Duration,
}

impl PoolDiscoveryService {
    /// Create new discovery service
    pub fn new(
        chain_config: Arc<ChainConfig>,
        web3: Arc<Web3<Http>>,
        request_rx: mpsc::Receiver<DiscoveryRequest>,
        max_concurrent: usize,
        rpc_timeout_ms: u64,
    ) -> Self {
        Self {
            chain_config,
            web3,
            request_rx,
            max_concurrent,
            rpc_timeout: Duration::from_millis(rpc_timeout_ms),
        }
    }

    /// Run the discovery service (spawned as background task)
    pub async fn run(mut self) {
        info!("Pool discovery service started");
        
        // Use a semaphore to limit concurrent discoveries
        let semaphore = Arc::new(tokio::sync::Semaphore::new(self.max_concurrent));
        
        while let Some(request) = self.request_rx.recv().await {
            let semaphore = semaphore.clone();
            let chain_config = self.chain_config.clone();
            let web3 = self.web3.clone();
            let timeout_duration = self.rpc_timeout;
            
            // Spawn non-blocking discovery task
            tokio::spawn(async move {
                // Acquire permit for rate limiting
                let _permit = semaphore.acquire().await.unwrap();
                
                let pool_addr = H160::from_slice(&request.pool_address);
                debug!("Starting discovery for pool 0x{}", hex::encode(request.pool_address));
                
                // Perform discovery with timeout
                let result = timeout(
                    timeout_duration,
                    discover_pool_with_config(pool_addr, chain_config, web3)
                ).await;
                
                // Send response back
                let response = match result {
                    Ok(Ok(pool_info)) => {
                        info!("Successfully discovered pool 0x{}", hex::encode(request.pool_address));
                        Ok(pool_info)
                    },
                    Ok(Err(e)) => {
                        warn!("Pool discovery failed: {}", e);
                        Err(PoolCacheError::RpcDiscoveryFailed(e.to_string()))
                    },
                    Err(_) => {
                        warn!("Pool discovery timed out for 0x{}", hex::encode(request.pool_address));
                        Err(PoolCacheError::DiscoveryTimeout(request.pool_address))
                    }
                };
                
                // Send might fail if receiver dropped, that's OK
                let _ = request.response_tx.send(response);
            });
        }
        
        info!("Pool discovery service shutting down");
    }
}

/// Discover pool with chains.json optimization
async fn discover_pool_with_config(
    pool_addr: H160,
    chain_config: Arc<ChainConfig>,
    web3: Arc<Web3<Http>>,
) -> Result<PoolInfo> {
    // Phase 1: Get token addresses from pool
    let (token0_addr, token1_addr) = get_pool_tokens(&web3, pool_addr).await?;
    
    // Phase 2: Get decimals (check chains.json first!)
    let token0_decimals = get_token_decimals_optimized(&chain_config, &web3, token0_addr).await?;
    let token1_decimals = get_token_decimals_optimized(&chain_config, &web3, token1_addr).await?;
    
    // Phase 3: Detect pool type and fee
    let (pool_type, fee_tier) = detect_pool_type(&web3, pool_addr).await?;
    
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    Ok(PoolInfo {
        pool_address: pool_addr.0,
        token0: token0_addr.0,
        token1: token1_addr.0,
        token0_decimals,
        token1_decimals,
        pool_type,
        fee_tier,
        venue: VenueId::Polygon,
        discovered_at: now,
        last_seen: now,
    })
}

/// Get token decimals with chains.json optimization
async fn get_token_decimals_optimized(
    chain_config: &ChainConfig,
    web3: &Web3<Http>,
    token_addr: H160,
) -> Result<u8> {
    let addr_str = format!("0x{}", hex::encode(token_addr));
    
    // Check chains.json first - NO RPC NEEDED!
    if let Some(token_info) = get_token_by_address(chain_config, &addr_str) {
        debug!("Found {} in chains.json, decimals: {}", token_info.symbol, token_info.decimals);
        return Ok(token_info.decimals);
    }
    
    // Not in chains.json, need RPC call
    debug!("Token {} not in chains.json, fetching via RPC", addr_str);
    get_token_decimals_via_rpc(web3, token_addr).await
}

/// Get token addresses from pool contract
async fn get_pool_tokens(web3: &Web3<Http>, pool_addr: H160) -> Result<(H160, H160)> {
    // token0() selector: 0x0dfe1681
    let token0_call = CallRequest {
        to: Some(pool_addr),
        data: Some(hex::decode("0dfe1681").unwrap().into()),
        ..Default::default()
    };
    
    // token1() selector: 0xd21220a7
    let token1_call = CallRequest {
        to: Some(pool_addr),
        data: Some(hex::decode("d21220a7").unwrap().into()),
        ..Default::default()
    };
    
    // Execute both calls concurrently
    let (token0_result, token1_result) = tokio::try_join!(
        web3.eth().call(token0_call, None),
        web3.eth().call(token1_call, None)
    )?;
    
    // Parse addresses from results
    if token0_result.0.len() >= 32 && token1_result.0.len() >= 32 {
        let mut token0_bytes = [0u8; 20];
        let mut token1_bytes = [0u8; 20];
        token0_bytes.copy_from_slice(&token0_result.0[12..32]);
        token1_bytes.copy_from_slice(&token1_result.0[12..32]);
        
        Ok((H160::from(token0_bytes), H160::from(token1_bytes)))
    } else {
        Err(anyhow::anyhow!("Invalid token address response from pool"))
    }
}

/// Get token decimals via RPC
async fn get_token_decimals_via_rpc(web3: &Web3<Http>, token_addr: H160) -> Result<u8> {
    // decimals() selector: 0x313ce567
    let call = CallRequest {
        to: Some(token_addr),
        data: Some(hex::decode("313ce567").unwrap().into()),
        ..Default::default()
    };
    
    let result = web3.eth().call(call, None).await?;
    
    if result.0.len() >= 32 {
        Ok(result.0[31])
    } else {
        Err(anyhow::anyhow!("Invalid decimals response"))
    }
}

/// Detect pool type (V2 vs V3) and fee tier
async fn detect_pool_type(web3: &Web3<Http>, pool_addr: H160) -> Result<(DEXProtocol, Option<u32>)> {
    // Try to call fee() - V3 pools have this, V2 pools don't
    // fee() selector: 0xddca3f43
    let fee_call = CallRequest {
        to: Some(pool_addr),
        data: Some(hex::decode("ddca3f43").unwrap().into()),
        ..Default::default()
    };
    
    match web3.eth().call(fee_call, None).await {
        Ok(result) if result.0.len() >= 32 => {
            // V3 pool - extract fee from last 4 bytes
            let mut fee_bytes = [0u8; 4];
            fee_bytes.copy_from_slice(&result.0[28..32]);
            let fee = u32::from_be_bytes(fee_bytes);
            
            // Determine specific V3 variant based on fee tiers
            let pool_type = if fee == 100 || fee == 500 || fee == 3000 || fee == 10000 {
                DEXProtocol::UniswapV3
            } else {
                DEXProtocol::QuickswapV3
            };
            
            Ok((pool_type, Some(fee)))
        }
        _ => {
            // V2 pool (no fee() function)
            Ok((DEXProtocol::UniswapV2, Some(30))) // V2 has 0.3% fee
        }
    }
}