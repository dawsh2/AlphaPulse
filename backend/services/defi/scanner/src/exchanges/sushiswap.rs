// SIMPLIFIED: SushiSwap is just a Uniswap V2 fork with identical AMM math
// Instead of duplicating logic, we use the UniswapV2 implementation
// with SushiSwap-specific addresses

use async_trait::async_trait;
use anyhow::Result;
use rust_decimal::Decimal;
use crate::{PoolInfo, PriceQuote};
use super::{DexProtocol, uniswap_v2::UniswapV2};

/// SushiSwap protocol - reuses UniswapV2 implementation since it's a fork
pub struct Sushiswap {
    inner: UniswapV2,
}

impl Sushiswap {
    pub fn new(
        name: String,
        rpc_url: String,
        factory_address: String,
        router_address: String,
    ) -> Self {
        Self {
            // Create UniswapV2 instance with SushiSwap addresses
            inner: UniswapV2::new(name, rpc_url, factory_address, router_address),
        }
    }
}

#[async_trait]
impl DexProtocol for Sushiswap {
    fn name(&self) -> &str {
        self.inner.name()
    }

    async fn discover_pools(&self, max_pools: usize) -> Result<Vec<PoolInfo>> {
        // Delegate to UniswapV2 implementation
        self.inner.discover_pools(max_pools).await
    }

    async fn update_pool(&self, pool_address: &str) -> Result<PoolInfo> {
        // Delegate to UniswapV2 implementation
        self.inner.update_pool(pool_address).await
    }

    async fn get_quote(
        &self,
        pool: &PoolInfo,
        token_in: &str,
        token_out: &str,
        amount_in: Decimal,
    ) -> Result<PriceQuote> {
        // Delegate to UniswapV2 implementation  
        self.inner.get_quote(pool, token_in, token_out, amount_in).await
    }

    async fn get_gas_price(&self) -> Result<Decimal> {
        // Delegate to UniswapV2 implementation
        self.inner.get_gas_price().await
    }
}