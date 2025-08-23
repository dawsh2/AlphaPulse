// DEX Price Oracle
// Fetches prices from DEX liquidity pools as backup to Chainlink

use anyhow::{Result, Context};
use ethers::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, warn, error, debug};

/// DEX-based price oracle using liquidity pool quotes
pub struct DexPriceOracle {
    provider: Arc<Provider<Http>>,
    routers: Vec<RouterInfo>,
    stable_coins: Vec<Address>,
}

#[derive(Debug, Clone)]
struct RouterInfo {
    name: String,
    address: Address,
    factory: Address,
}

impl DexPriceOracle {
    pub async fn new(provider: Arc<Provider<Http>>) -> Result<Self> {
        let routers = vec![
            RouterInfo {
                name: "QuickSwap".to_string(),
                address: "0xa5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff".parse()?,
                factory: "0x5757371414417b8C6CAad45bAeF941aBc7d3Ab32".parse()?,
            },
            RouterInfo {
                name: "SushiSwap".to_string(),
                address: "0x1b02dA8Cb0d097eB8D57A175b88c7D8b47997506".parse()?,
                factory: "0xc35DADB65012eC5796536bD9864eD8773aBc74C4".parse()?,
            },
        ];
        
        let stable_coins = vec![
            "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174".parse()?, // USDC
            "0xc2132D05D31c914a87C6611C10748AEb04B58e8F".parse()?, // USDT
            "0x8f3Cf7ad23Cd3CaDbD9735AFf958023239c6A063".parse()?, // DAI
        ];
        
        info!("🔄 DEX price oracle initialized with {} routers and {} stablecoins", 
              routers.len(), stable_coins.len());
        
        Ok(Self {
            provider,
            routers,
            stable_coins,
        })
    }
    
    /// Get token price by finding best stablecoin pair
    pub async fn get_price(&self, token: Address) -> Result<f64> {
        // Skip if token is already a stablecoin
        if self.stable_coins.contains(&token) {
            return Ok(1.0);
        }
        
        let mut best_price = None;
        let mut best_liquidity = 0.0;
        
        // Try each router and stablecoin combination
        for router in &self.routers {
            for &stable in &self.stable_coins {
                if let Ok((price, liquidity)) = self.get_price_from_router(router, token, stable).await {
                    debug!("💱 {} price via {}: ${:.6} (liquidity: ${:.0})", 
                           router.name, self.stable_symbol(stable), price, liquidity);
                    
                    // Prefer pairs with higher liquidity
                    if liquidity > best_liquidity {
                        best_price = Some(price);
                        best_liquidity = liquidity;
                    }
                }
            }
        }
        
        match best_price {
            Some(price) => {
                info!("💰 DEX price for {:?}: ${:.6} (best liquidity: ${:.0})", 
                      token, price, best_liquidity);
                Ok(price)
            }
            None => Err(anyhow::anyhow!("No liquidity found for token {:?}", token)),
        }
    }
    
    /// Get price from specific router and stablecoin pair
    async fn get_price_from_router(
        &self,
        router: &RouterInfo,
        token: Address,
        stable: Address,
    ) -> Result<(f64, f64)> {
        // Get quote for 1 token
        let amount_in = U256::exp10(18); // 1 token (assuming 18 decimals)
        let path = vec![token, stable];
        
        // Router ABI
        let abi = ethers::abi::parse_abi(&[
            "function getAmountsOut(uint amountIn, address[] calldata path) external view returns (uint[] memory amounts)"
        ])?;
        
        let router_contract = Contract::new(router.address, abi, self.provider.clone());
        
        let amounts: Vec<U256> = router_contract
            .method::<_, Vec<U256>>("getAmountsOut", (amount_in, path))?
            .call()
            .await
            .context("Failed to get amounts out")?;
        
        let amount_out = amounts.get(1)
            .ok_or_else(|| anyhow::anyhow!("No output amount"))?;
        
        // Convert to USD (stablecoins have 6 decimals typically)
        let price = amount_out.as_u128() as f64 / 1e6;
        
        // Get liquidity estimate
        let liquidity = self.estimate_pool_liquidity(router, token, stable).await.unwrap_or(0.0);
        
        Ok((price, liquidity))
    }
    
    /// Estimate pool liquidity
    async fn estimate_pool_liquidity(
        &self,
        router: &RouterInfo,
        token_a: Address,
        token_b: Address,
    ) -> Result<f64> {
        // Get pair address
        let pair_address = self.get_pair_address(router.factory, token_a, token_b).await?;
        
        // Get reserves
        let reserves = self.get_pool_reserves(pair_address).await?;
        
        // Estimate USD liquidity (simplified)
        let token_a_liquidity = reserves.0.as_u128() as f64 / 1e18;
        let token_b_liquidity = reserves.1.as_u128() as f64 / 1e6; // Assuming stablecoin
        
        // Return the stablecoin side as liquidity estimate
        Ok(token_b_liquidity * 2.0) // Total pool liquidity
    }
    
    /// Get pair address from factory
    async fn get_pair_address(
        &self,
        factory: Address,
        token_a: Address,
        token_b: Address,
    ) -> Result<Address> {
        let abi = ethers::abi::parse_abi(&[
            "function getPair(address tokenA, address tokenB) external view returns (address pair)"
        ])?;
        
        let factory_contract = Contract::new(factory, abi, self.provider.clone());
        
        let pair: Address = factory_contract
            .method::<_, Address>("getPair", (token_a, token_b))?
            .call()
            .await?;
        
        if pair == Address::zero() {
            return Err(anyhow::anyhow!("No pair exists"));
        }
        
        Ok(pair)
    }
    
    /// Get pool reserves
    async fn get_pool_reserves(&self, pair: Address) -> Result<(U256, U256)> {
        let abi = ethers::abi::parse_abi(&[
            "function getReserves() external view returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast)"
        ])?;
        
        let pair_contract = Contract::new(pair, abi, self.provider.clone());
        
        let (reserve0, reserve1, _): (U256, U256, u32) = pair_contract
            .method::<_, (U256, U256, u32)>("getReserves", ())?
            .call()
            .await?;
        
        Ok((reserve0, reserve1))
    }
    
    /// Get price with liquidity weighting
    pub async fn get_weighted_price(&self, token: Address) -> Result<f64> {
        let mut total_price_weight = 0.0;
        let mut total_weight = 0.0;
        
        // Collect prices from all available pairs
        for router in &self.routers {
            for &stable in &self.stable_coins {
                if let Ok((price, liquidity)) = self.get_price_from_router(router, token, stable).await {
                    let weight = liquidity.sqrt(); // Use sqrt of liquidity as weight
                    total_price_weight += price * weight;
                    total_weight += weight;
                }
            }
        }
        
        if total_weight > 0.0 {
            let weighted_price = total_price_weight / total_weight;
            info!("📊 Liquidity-weighted DEX price for {:?}: ${:.6}", token, weighted_price);
            Ok(weighted_price)
        } else {
            Err(anyhow::anyhow!("No liquidity available for weighted price"))
        }
    }
    
    /// Get all supported tokens (tokens with sufficient liquidity)
    pub async fn get_supported_tokens(&self) -> Vec<Address> {
        // This would scan for tokens with sufficient liquidity
        // For now, return empty - would be implemented based on needs
        Vec::new()
    }
    
    /// Check if sufficient liquidity exists for reliable pricing
    pub async fn has_sufficient_liquidity(&self, token: Address, min_liquidity_usd: f64) -> bool {
        if let Ok(price) = self.get_price(token).await {
            // Check if any pair has sufficient liquidity
            for router in &self.routers {
                for &stable in &self.stable_coins {
                    if let Ok((_, liquidity)) = self.get_price_from_router(router, token, stable).await {
                        if liquidity >= min_liquidity_usd {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }
    
    /// Helper: Get stablecoin symbol for logging
    fn stable_symbol(&self, stable: Address) -> &str {
        match stable {
            _ if stable == "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174".parse().unwrap() => "USDC",
            _ if stable == "0xc2132D05D31c914a87C6611C10748AEb04B58e8F".parse().unwrap() => "USDT",
            _ if stable == "0x8f3Cf7ad23Cd3CaDbD9735AFf958023239c6A063".parse().unwrap() => "DAI",
            _ => "STABLE",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_dex_oracle_initialization() {
        let provider = Arc::new(Provider::<Http>::try_from("https://polygon-rpc.com").unwrap());
        let oracle = DexPriceOracle::new(provider).await.unwrap();
        
        assert_eq!(oracle.routers.len(), 2);
        assert_eq!(oracle.stable_coins.len(), 3);
    }
    
    #[tokio::test]
    async fn test_stablecoin_price() {
        let provider = Arc::new(Provider::<Http>::try_from("https://polygon-rpc.com").unwrap());
        let oracle = DexPriceOracle::new(provider).await.unwrap();
        
        let usdc: Address = "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174".parse().unwrap();
        let price = oracle.get_price(usdc).await.unwrap();
        
        assert_eq!(price, 1.0);
    }
}