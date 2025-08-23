pub mod uniswap_v2;
pub mod uniswap_v3;
pub mod sushiswap;

pub use uniswap_v2::UniswapV2;
pub use uniswap_v3::UniswapV3;
pub use sushiswap::Sushiswap;

use async_trait::async_trait;
use anyhow::Result;
use rust_decimal::Decimal;
use crate::{PoolInfo, PriceQuote};

/// Trait for exchange-specific implementations
#[async_trait]
pub trait DexProtocol: Send + Sync {
    /// Get the name of this DEX protocol
    fn name(&self) -> &str;

    /// Discover pools for this exchange
    async fn discover_pools(&self, max_pools: usize) -> Result<Vec<PoolInfo>>;

    /// Update reserves for a specific pool
    async fn update_pool(&self, pool_address: &str) -> Result<PoolInfo>;

    /// Calculate a price quote for a trade
    async fn get_quote(
        &self,
        pool: &PoolInfo,
        token_in: &str,
        token_out: &str,
        amount_in: Decimal,
    ) -> Result<PriceQuote>;

    /// Get current gas price estimate for this exchange
    async fn get_gas_price(&self) -> Result<Decimal>;

    /// Validate that a pool is suitable for arbitrage
    fn validate_pool(&self, pool: &PoolInfo) -> bool {
        // Default validation
        pool.reserve0 > Decimal::ZERO && 
        pool.reserve1 > Decimal::ZERO &&
        !pool.token0.is_empty() &&
        !pool.token1.is_empty()
    }
}