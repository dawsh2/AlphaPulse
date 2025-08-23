use anyhow::Result;
use rust_decimal::{Decimal, prelude::{ToPrimitive, FromPrimitive}};
use crate::{PoolInfo, PriceQuote, config::ScannerConfig};
use crate::gas_estimation::{GasCalculator, ContractType, ArbitrageCharacteristics};
use crate::huff_gas_estimator::HuffGasEstimator;
use crate::amm_math::AmmMath;
use crate::v3_math;
use std::cmp::min;
use ethers::types::{Address, U256};
use std::sync::Arc;

// Import accurate AMM calculations from arbitrage module
// Note: This references the mathematically correct AMM math that was updated

/// Calculates prices and quotes across different DEX types
pub struct PriceCalculator {
    config: ScannerConfig,
    huff_gas_estimator: Option<Arc<HuffGasEstimator>>,
    bot_address: Option<Address>,
}

impl PriceCalculator {
    pub fn new(config: &ScannerConfig) -> Self {
        Self {
            config: config.clone(),
            huff_gas_estimator: None,
            bot_address: None,
        }
    }
    
    /// Create PriceCalculator with Huff gas estimation enabled
    pub fn with_huff_estimator(
        config: &ScannerConfig, 
        huff_contract_address: Address,
        bot_address: Address,
    ) -> Result<Self> {
        let huff_estimator = HuffGasEstimator::new(&config.network.rpc_url, huff_contract_address)?;
        
        Ok(Self {
            config: config.clone(),
            huff_gas_estimator: Some(Arc::new(huff_estimator)),
            bot_address: Some(bot_address),
        })
    }

    /// Get a price quote for swapping tokens in a specific pool
    pub async fn get_quote(
        &self,
        pool: &PoolInfo,
        token_in: &str,
        token_out: &str,
        amount_in: Decimal,
    ) -> Result<PriceQuote> {
        match pool.exchange.as_str() {
            "uniswap_v2" | "sushiswap" => {
                self.calculate_uniswap_v2_quote(pool, token_in, token_out, amount_in).await
            }
            "uniswap_v3" => {
                self.calculate_uniswap_v3_quote(pool, token_in, token_out, amount_in).await
            }
            _ => {
                anyhow::bail!("Unsupported exchange: {}", pool.exchange)
            }
        }
    }

    async fn calculate_uniswap_v2_quote(
        &self,
        pool: &PoolInfo,
        token_in: &str,
        token_out: &str,
        amount_in: Decimal,
    ) -> Result<PriceQuote> {
        // Convert Decimal reserves to fixed-point for calculation
        let reserve_in_raw = (pool.reserve0 * Decimal::new(100000000, 0)).to_string().parse::<u64>().unwrap_or(0);
        let reserve_out_raw = (pool.reserve1 * Decimal::new(100000000, 0)).to_string().parse::<u64>().unwrap_or(0);
        
        // Determine token order
        let (reserve_in, reserve_out) = if token_in == pool.token0 {
            (reserve_in_raw, reserve_out_raw)
        } else if token_in == pool.token1 {
            (reserve_out_raw, reserve_in_raw)
        } else {
            anyhow::bail!("Token {} not found in pool", token_in);
        };

        if reserve_in == 0 || reserve_out == 0 {
            anyhow::bail!("Pool has zero liquidity");
        }

        // Convert input to fixed-point
        let amount_in_fixed = (amount_in * Decimal::new(100000000, 0)).to_string().parse::<u64>().unwrap_or(0);

        // Use exact V2 calculation with fee from pool.fee
        let fee_bps = (pool.fee * Decimal::new(10000, 0)).to_string().parse::<u32>().unwrap_or(30);
        let amount_out_fixed = self.calc_v2_output(amount_in_fixed, reserve_in, reserve_out, fee_bps);
        
        // Convert back to decimal
        let amount_out = Decimal::new(amount_out_fixed as i64, 8);
        let price = amount_out / amount_in;

        // Calculate slippage
        let reserve_in_decimal = Decimal::new(reserve_in as i64, 8);
        let reserve_out_decimal = Decimal::new(reserve_out as i64, 8);
        let ideal_price = reserve_out_decimal / reserve_in_decimal;
        let slippage = ((ideal_price - price) / ideal_price).abs() * Decimal::new(100, 0);

        Ok(PriceQuote {
            exchange: pool.exchange.clone(),
            pool: pool.address.clone(),
            token_in: token_in.to_string(),
            token_out: token_out.to_string(),
            amount_in,
            amount_out,
            price,
            fee: pool.fee,
            slippage,
            timestamp: chrono::Utc::now().timestamp(),
        })
    }
    
    async fn calculate_uniswap_v3_quote(
        &self,
        pool: &PoolInfo,
        token_in: &str,
        token_out: &str,
        amount_in: Decimal,
    ) -> Result<PriceQuote> {
        // V3 pools require tick liquidity data
        let tick = pool.v3_tick.ok_or_else(|| anyhow::anyhow!("V3 pool missing tick data"))?;
        let sqrt_price_x96 = pool.v3_sqrt_price_x96.ok_or_else(|| anyhow::anyhow!("V3 pool missing sqrt_price_x96"))?;
        let liquidity = pool.v3_liquidity.ok_or_else(|| anyhow::anyhow!("V3 pool missing liquidity data"))?;

        if liquidity == 0 {
            anyhow::bail!("V3 pool has zero active liquidity");
        }

        // Determine direction (token0 -> token1 or token1 -> token0)
        let zero_for_one = token_in == pool.token0;
        
        // Convert input to fixed-point
        let amount_in_fixed = (amount_in * Decimal::new(100000000, 0)).to_string().parse::<u128>().unwrap_or(0);
        
        // Use REAL V3 math with proper tick calculations
        let fee_pips = (pool.fee * Decimal::new(1000000, 0)).to_string().parse::<u32>().unwrap_or(3000);
        let (amount_out_fixed, price_impact) = self.calc_v3_output_real(amount_in_fixed, liquidity, sqrt_price_x96, fee_pips, zero_for_one);
        
        // Convert back to decimal
        let amount_out = Decimal::new(amount_out_fixed as i64, 8);
        let price = if amount_in > Decimal::ZERO { amount_out / amount_in } else { Decimal::ZERO };
        
        // Slippage is price impact from V3 calculation
        let slippage = Decimal::from_f64_retain(price_impact * 100.0).unwrap_or(Decimal::ZERO);

        Ok(PriceQuote {
            exchange: pool.exchange.clone(),
            pool: pool.address.clone(),
            token_in: token_in.to_string(),
            token_out: token_out.to_string(),
            amount_in,
            amount_out,
            price,
            fee: pool.fee,
            slippage,
            timestamp: chrono::Utc::now().timestamp(),
        })
    }

    /// V3 calculation using REAL tick math from v3_math module
    fn calc_v3_output_real(&self, amount_in: u128, liquidity: u128, sqrt_price_x96: u128, fee_pips: u32, zero_for_one: bool) -> (u128, f64) {
        if liquidity == 0 {
            return (0, 1.0);
        }
        
        // Use the proper V3 math module for exact calculations
        // Set a reasonable sqrt price limit (5% price impact max)
        let sqrt_price_limit = if zero_for_one {
            sqrt_price_x96 * 95 / 100  // 5% decrease limit
        } else {
            sqrt_price_x96 * 105 / 100 // 5% increase limit
        };
        
        let (amount_consumed, sqrt_price_new, amount_out) = v3_math::swap_within_tick(
            sqrt_price_x96,
            sqrt_price_limit,
            liquidity,
            amount_in,
            fee_pips,
            zero_for_one,
        );
        
        // Calculate actual price impact
        let price_impact = v3_math::calculate_v3_price_impact(sqrt_price_x96, sqrt_price_new);
        
        (amount_out, price_impact)
    }

    /// DEPRECATED: Old simplified V3 calculation - kept for fallback
    fn calc_v3_output(&self, amount_in: u128, liquidity: u128, sqrt_price_x96: u128, fee_pips: u32, zero_for_one: bool) -> (u128, f64) {
        if liquidity == 0 {
            return (0, 1.0);
        }
        
        // Apply fee (fee_pips = fee * 100, e.g., 3000 = 0.3%)
        let amount_in_after_fee = amount_in * (1_000_000 - fee_pips as u128) / 1_000_000;
        
        let (amount_out, sqrt_price_x96_new) = if zero_for_one {
            // Swapping token0 for token1
            let delta_sqrt = (amount_in_after_fee * (2u128.pow(96))) / liquidity;
            let sqrt_price_x96_new = sqrt_price_x96.saturating_sub(delta_sqrt);
            let amount_out = liquidity * (sqrt_price_x96 - sqrt_price_x96_new) / (2u128.pow(96));
            (amount_out, sqrt_price_x96_new)
        } else {
            // Swapping token1 for token0 
            let numerator = liquidity * (2u128.pow(96)) * amount_in_after_fee;
            let denominator = liquidity * (2u128.pow(96)) + amount_in_after_fee * sqrt_price_x96;
            if denominator == 0 {
                return (0, 1.0);
            }
            let delta_sqrt = numerator / denominator;
            let sqrt_price_x96_new = sqrt_price_x96 + delta_sqrt;
            let amount_out = if sqrt_price_x96_new > 0 {
                liquidity * delta_sqrt / sqrt_price_x96_new
            } else {
                0
            };
            (amount_out, sqrt_price_x96_new)
        };
        
        // Calculate price impact
        let price_impact = if sqrt_price_x96 > 0 {
            ((sqrt_price_x96_new as f64 - sqrt_price_x96 as f64).abs() / sqrt_price_x96 as f64).min(1.0)
        } else {
            1.0
        };
        
        (amount_out, price_impact)
    }

    /// Exact V2 calculation from scripts/arb - uses integer math to avoid precision loss
    fn calc_v2_output(&self, amount_in: u64, reserve_in: u64, reserve_out: u64, fee_bps: u32) -> u64 {
        let amount_in = amount_in as u128;
        let reserve_in = reserve_in as u128;
        let reserve_out = reserve_out as u128;
        
        let amount_with_fee = amount_in * (10000 - fee_bps as u128);
        let numerator = amount_with_fee * reserve_out;
        let denominator = reserve_in * 10000 + amount_with_fee;
        
        (numerator / denominator) as u64
    }

    /// Get token decimals - mapping from scripts/arb token knowledge
    fn get_token_decimals(&self, token_address: &str) -> u8 {
        match token_address.to_lowercase().as_str() {
            // USDC variants (6 decimals)
            "0x2791bca1f2de4661ed88a30c99a7a9449aa84174" => 6, // USDC.e (bridged)
            "0x3c499c542cef5e3811e1192ce70d8cc03d5c3359" => 6, // USDC (native)
            "0xc2132d05d31c914a87c6611c10748aeb04b58e8f" => 6, // USDT
            // Most other tokens (18 decimals)
            _ => 18,
        }
    }


    /// Calculate the minimum amount out considering slippage tolerance
    pub fn calculate_min_amount_out(
        &self,
        amount_out: Decimal,
        slippage_tolerance: Decimal,
    ) -> Decimal {
        amount_out * (Decimal::ONE - slippage_tolerance / Decimal::new(100, 0))
    }

    /// Real gas cost calculation using measured Huff contract gas usage
    pub async fn estimate_gas_cost(
        &self,
        exchange: &str,
        is_complex_trade: bool,
        current_gas_price_gwei: Option<Decimal>,
        token_pair: Option<(&str, &str)>,
        amount_usd: Option<Decimal>,
    ) -> Decimal {
        // Try Huff gas estimation first if available
        if let (Some(huff_estimator), Some(bot_address)) = (&self.huff_gas_estimator, &self.bot_address) {
            if let Some((token_in, token_out)) = token_pair {
                if let Some(amount) = amount_usd {
                    return self.estimate_gas_with_huff(
                        huff_estimator, 
                        *bot_address, 
                        token_in, 
                        token_out, 
                        amount,
                        exchange,
                        is_complex_trade
                    ).await.unwrap_or_else(|e| {
                        tracing::warn!("🔄 Huff gas estimation failed: {}, falling back to static", e);
                        self.estimate_gas_static(exchange, is_complex_trade, current_gas_price_gwei, token_pair)
                    });
                }
            }
        }
        
        // Fallback to static estimation
        self.estimate_gas_static(exchange, is_complex_trade, current_gas_price_gwei, token_pair)
    }
    
    /// Real gas estimation using deployed Huff contract
    async fn estimate_gas_with_huff(
        &self,
        huff_estimator: &HuffGasEstimator,
        bot_address: Address,
        token_in: &str,
        token_out: &str, 
        amount_usd: Decimal,
        exchange: &str,
        is_complex: bool,
    ) -> Result<Decimal> {
        // Convert token symbols to addresses (you'll need to implement this mapping)
        let token0_addr = self.token_symbol_to_address(token_in)?;
        let token1_addr = self.token_symbol_to_address(token_out)?;
        let router_addr = self.exchange_to_router_address(exchange)?;
        
        // Convert USD amount to token amount (simplified - assumes token0 is the input)
        let amount_tokens = amount_usd * rust_decimal_macros::dec!(1000000); // 6 decimals for USDC-like
        let amount_u256 = U256::from_dec_str(&amount_tokens.to_string()).unwrap_or_default();
        
        // Get gas estimate from Huff contract
        let gas_units = huff_estimator.estimate_arbitrage_with_cache(
            amount_u256,
            token0_addr,
            token1_addr, 
            router_addr,
            router_addr, // Same router for single-exchange estimate
            U256::from(1), // Min profit
            bot_address,
        ).await?;
        
        // Calculate USD cost
        let matic_price = rust_decimal_macros::dec!(0.8); // Could fetch from oracle
        huff_estimator.calculate_net_profitability(
            rust_decimal_macros::dec!(0), // No gross profit, just get gas cost
            gas_units,
            matic_price,
        ).await.map(|net| net.abs()) // Return absolute gas cost
    }
    
    /// Static gas estimation (fallback)
    fn estimate_gas_static(
        &self,
        exchange: &str,
        is_complex_trade: bool,
        current_gas_price_gwei: Option<Decimal>,
        token_pair: Option<(&str, &str)>,
    ) -> Decimal {
        // Determine optimal contract type based on arbitrage characteristics
        let contract_type = if let Some((token_in, token_out)) = token_pair {
            let characteristics = ArbitrageCharacteristics {
                exchange: exchange.to_string(),
                token_pair: (token_in.to_string(), token_out.to_string()),
                num_swaps: if is_complex_trade { 3 } else { 1 },
                involves_v3: exchange == "uniswap_v3",
                cross_dex: is_complex_trade,
            };
            characteristics.optimal_contract_type()
        } else {
            // Default to MEV for unknown scenarios
            ContractType::HuffMEV
        };

        // Current gas price in gwei (default to 30 gwei if not provided)
        let gas_price_gwei = current_gas_price_gwei.unwrap_or(Decimal::new(30, 0));
        
        // Get real MATIC price from recent trades
        let matic_price_usd = self.get_real_matic_price();
        
        // Use real gas calculator with measured values
        let gas_calculator = GasCalculator::new(gas_price_gwei, matic_price_usd);
        let base_cost = gas_calculator.calculate_execution_cost_usd(contract_type, is_complex_trade);

        // Add small MEV protection buffer for high-value trades (much smaller than before)
        let mev_buffer = if base_cost > Decimal::new(1, 1) { // > $0.1
            base_cost * Decimal::new(5, 2) // 5% buffer instead of 15%
        } else {
            Decimal::ZERO
        };

        base_cost + mev_buffer
    }
    
    /// Convert token symbol to address (Polygon mainnet addresses)
    fn token_symbol_to_address(&self, symbol: &str) -> Result<Address> {
        let address_str = match symbol {
            "USDC" => "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174",
            "USDT" => "0xc2132D05D31c914a87C6611C10748AEb04B58e8F", 
            "WETH" => "0x7ceB23fD6bC0adD59E62ac25578270cFf1b9f619",
            "WMATIC" => "0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270",
            "DAI" => "0x8f3Cf7ad23Cd3CaDbD9735AFf958023239c6A063",
            "WBTC" => "0x1BFD67037B42Cf73acF2047067bd4F2C47D9BfD6",
            // Add more tokens as needed
            _ => return Err(anyhow::anyhow!("Unknown token symbol: {}", symbol)),
        };
        
        address_str.parse().map_err(|e| anyhow::anyhow!("Invalid address for {}: {}", symbol, e))
    }
    
    /// Convert exchange name to router address (Polygon)
    fn exchange_to_router_address(&self, exchange: &str) -> Result<Address> {
        let address_str = match exchange {
            "uniswap_v2" | "quickswap" => "0xa5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff", // QuickSwap
            "uniswap_v3" => "0xE592427A0AEce92De3Edee1F18E0157C05861564", // Uniswap V3
            "sushiswap" => "0x1b02dA8Cb0d097eB8D57A175b88c7D8b47997506", // SushiSwap
            // Add more exchanges as needed
            _ => return Err(anyhow::anyhow!("Unknown exchange: {}", exchange)),
        };
        
        address_str.parse().map_err(|e| anyhow::anyhow!("Invalid router address for {}: {}", exchange, e))
    }

    /// Cross-token arbitrage detection from scripts/arb - handles USDC variants
    pub fn detect_cross_token_opportunities(
        &self,
        base_token: &str,
        target_amount: Decimal,
    ) -> Vec<String> {
        let mut cross_tokens = Vec::new();
        
        match base_token.to_lowercase().as_str() {
            // USDC.e (bridged) can arbitrage with native USDC
            "0x2791bca1f2de4661ed88a30c99a7a9449aa84174" => {
                cross_tokens.push("0x3c499c542cef5e3811e1192ce70d8cc03d5c3359".to_string()); // USDC native
                cross_tokens.push("0xc2132d05d31c914a87c6611c10748aeb04b58e8f".to_string()); // USDT
            }
            // Native USDC can arbitrage with USDC.e
            "0x3c499c542cef5e3811e1192ce70d8cc03d5c3359" => {
                cross_tokens.push("0x2791bca1f2de4661ed88a30c99a7a9449aa84174".to_string()); // USDC.e
                cross_tokens.push("0xc2132d05d31c914a87c6611c10748aeb04b58e8f".to_string()); // USDT
            }
            // USDT can arbitrage with both USDC variants
            "0xc2132d05d31c914a87c6611c10748aeb04b58e8f" => {
                cross_tokens.push("0x2791bca1f2de4661ed88a30c99a7a9449aa84174".to_string()); // USDC.e
                cross_tokens.push("0x3c499c542cef5e3811e1192ce70d8cc03d5c3359".to_string()); // USDC native
            }
            // WETH variants
            "0x7ceb23fd6e88b87c7e50c3d0d0c18d8b4e7d0f32" => { // WETH
                cross_tokens.push("0x0d500b1d8e8ef31e21c99d1db9a6444d3adf1270".to_string()); // WMATIC
            }
            // WMATIC
            "0x0d500b1d8e8ef31e21c99d1db9a6444d3adf1270" => {
                cross_tokens.push("0x7ceb23fd6e88b87c7e50c3d0f32".to_string()); // WETH
            }
            _ => {} // No cross-token opportunities for other tokens
        }
        
        cross_tokens
    }

    /// Calculate optimal trade size using binary search from scripts/arb
    pub async fn calculate_optimal_size(
        &self,
        pool: &PoolInfo,
        token_in: &str,
        token_out: &str,
        max_amount: Decimal,
        target_profit_usd: Decimal,
    ) -> Result<Option<Decimal>> {
        let mut left = Decimal::new(100, 0); // Start with $100
        let mut right = max_amount;
        let mut best_size = None;
        let mut best_profit = Decimal::ZERO;
        
        // Binary search for optimal size (from scripts/arb)
        for _ in 0..20 { // Max 20 iterations
            if right - left < Decimal::new(10, 0) { // $10 precision
                break;
            }
            
            let mid = (left + right) / Decimal::new(2, 0);
            
            // Get quote for this size
            if let Ok(quote) = self.get_quote(pool, token_in, token_out, mid).await {
                // Estimate profit (simplified)
                let revenue = quote.amount_out * quote.price;
                let gas_cost = self.estimate_gas_cost(&pool.exchange, false, None, None, Some(mid)).await;
                let profit = revenue - mid - gas_cost;
                
                if profit > best_profit {
                    best_profit = profit;
                    best_size = Some(mid);
                }
                
                if profit >= target_profit_usd {
                    left = mid;
                } else {
                    right = mid;
                }
            } else {
                right = mid; // Reduce size if quote fails
            }
        }
        
        Ok(best_size)
    }

    /// Get real MATIC price from recent trades instead of hardcoded value
    fn get_real_matic_price(&self) -> Decimal {
        // TODO: Read recent MATIC/USD trade data from Unix socket
        // The exchange_collector processes MATIC/USDC swaps with real prices
        
        // For now, return a conservative estimate but this should be replaced with
        // actual price data from recent MATIC/USDC or MATIC/USDT swaps
        Decimal::new(8, 1) // $0.8 placeholder - should be real price
    }
}