use anyhow::{Context, Result};
use tracing::{debug, info, error};

#[derive(Debug, Clone)]
pub struct TokenInfo {
    pub symbol: String,
    pub address: String,
    pub decimals: u8,
}

#[derive(Debug, Clone)]
pub struct SwapEvent {
    pub pool_address: String,
    pub token0: TokenInfo,
    pub token1: TokenInfo,
    pub amount0_in_raw: u128,
    pub amount1_in_raw: u128,
    pub amount0_out_raw: u128,
    pub amount1_out_raw: u128,
    pub tx_hash: String,
    pub block_number: u64,
}

#[derive(Debug)]
pub struct PriceCalculation {
    pub raw_price: f64,
    pub inverted_price: f64,
    pub final_price: f64,
    pub base_token: String,
    pub quote_token: String,
    pub calculation_steps: Vec<String>,
}

pub struct POLPriceCalculator {
    debug_mode: bool,
}

impl POLPriceCalculator {
    pub fn new(debug_mode: bool) -> Self {
        Self { debug_mode }
    }

    /// Calculate price from swap event with comprehensive debugging
    pub fn calculate_price(&self, swap: &SwapEvent) -> Result<PriceCalculation> {
        let mut steps = Vec::new();
        
        info!("🔍 CALCULATING PRICE FOR SWAP:");
        info!("  Pool: {}", swap.pool_address);
        info!("  Token0: {} ({} decimals)", swap.token0.symbol, swap.token0.decimals);
        info!("  Token1: {} ({} decimals)", swap.token1.symbol, swap.token1.decimals);
        info!("  TX: {}", swap.tx_hash);

        // Step 1: Convert raw amounts to decimal-adjusted amounts
        let amount0_in = self.convert_raw_amount(swap.amount0_in_raw, swap.token0.decimals);
        let amount1_in = self.convert_raw_amount(swap.amount1_in_raw, swap.token1.decimals);
        let amount0_out = self.convert_raw_amount(swap.amount0_out_raw, swap.token0.decimals);
        let amount1_out = self.convert_raw_amount(swap.amount1_out_raw, swap.token1.decimals);

        steps.push(format!("Raw amounts: 0_in={}, 1_in={}, 0_out={}, 1_out={}", 
            swap.amount0_in_raw, swap.amount1_in_raw, swap.amount0_out_raw, swap.amount1_out_raw));
        steps.push(format!("Decimal-adjusted: 0_in={:.6}, 1_in={:.6}, 0_out={:.6}, 1_out={:.6}", 
            amount0_in, amount1_in, amount0_out, amount1_out));

        info!("  📊 AMOUNTS:");
        info!("    Raw: amount0_in={}, amount1_in={}, amount0_out={}, amount1_out={}", 
            swap.amount0_in_raw, swap.amount1_in_raw, swap.amount0_out_raw, swap.amount1_out_raw);
        info!("    Adjusted: amount0_in={:.6}, amount1_in={:.6}, amount0_out={:.6}, amount1_out={:.6}", 
            amount0_in, amount1_in, amount0_out, amount1_out);

        // Step 2: Determine swap direction and calculate raw price
        let (raw_price, swap_direction) = if amount0_in > 0.0 && amount1_out > 0.0 {
            // Selling token0 for token1: price = token1_out / token0_in
            let price = amount1_out / amount0_in;
            steps.push(format!("Swap direction: Selling {} for {} (price = {}/{} = {:.6})", 
                swap.token0.symbol, swap.token1.symbol, amount1_out, amount0_in, price));
            (price, format!("{}->{}", swap.token0.symbol, swap.token1.symbol))
        } else if amount1_in > 0.0 && amount0_out > 0.0 {
            // Selling token1 for token0: price = token1_in / token0_out (inverted)
            let price = amount1_in / amount0_out;
            steps.push(format!("Swap direction: Selling {} for {} (price = {}/{} = {:.6})", 
                swap.token1.symbol, swap.token0.symbol, amount1_in, amount0_out, price));
            (price, format!("{}->{}", swap.token1.symbol, swap.token0.symbol))
        } else {
            return Err(anyhow::anyhow!("Invalid swap: no clear direction"));
        };

        info!("  💱 SWAP ANALYSIS:");
        info!("    Direction: {}", swap_direction);
        info!("    Raw price: {:.6}", raw_price);

        // Step 3: Determine quote currency and price orientation
        let (base_token, quote_token, should_invert) = self.determine_price_orientation(
            &swap.token0.symbol, &swap.token1.symbol, &swap_direction)?;

        steps.push(format!("Price orientation: base={}, quote={}, should_invert={}", 
            base_token, quote_token, should_invert));

        info!("  🎯 PRICE ORIENTATION:");
        info!("    Base token: {}", base_token);
        info!("    Quote token: {}", quote_token);
        info!("    Should invert: {}", should_invert);

        // Step 4: Apply inversion if needed
        let final_price = if should_invert && raw_price > 0.0 {
            let inverted = 1.0 / raw_price;
            steps.push(format!("Applied inversion: {:.6} -> {:.6}", raw_price, inverted));
            info!("    Inverted price: {:.6} -> {:.6}", raw_price, inverted);
            inverted
        } else {
            steps.push(format!("No inversion needed: {:.6}", raw_price));
            raw_price
        };

        info!("  ✅ FINAL RESULT:");
        info!("    Price: {:.6} {} per {}", final_price, quote_token, base_token);

        // Step 5: Sanity check for POL
        if base_token == "POL" || quote_token == "POL" {
            self.validate_pol_price(final_price, &base_token, &quote_token, &mut steps)?;
        }

        Ok(PriceCalculation {
            raw_price,
            inverted_price: if should_invert { 1.0 / raw_price } else { raw_price },
            final_price,
            base_token,
            quote_token,
            calculation_steps: steps,
        })
    }

    fn convert_raw_amount(&self, raw_amount: u128, decimals: u8) -> f64 {
        (raw_amount as f64) / (10_f64.powi(decimals as i32))
    }

    fn determine_price_orientation(&self, token0: &str, token1: &str, swap_direction: &str) 
        -> Result<(String, String, bool)> {
        
        let is_quote_currency = |token: &str| -> bool {
            matches!(token, "USDC" | "USDT" | "DAI" | "USD")
        };

        // For POL pairs, we want POL as base and stablecoin as quote
        // So POL/USDC means: price in USDC per POL (like $0.23 per POL)
        
        if token0 == "POL" && is_quote_currency(token1) {
            // POL is token0, stablecoin is token1 - perfect
            // If swap was POL->USDC, price is already correct (USDC per POL)
            // If swap was USDC->POL, price needs inversion
            let should_invert = swap_direction.starts_with(token1);
            Ok((token0.to_string(), token1.to_string(), should_invert))
        } else if token1 == "POL" && is_quote_currency(token0) {
            // POL is token1, stablecoin is token0 - need to swap roles
            // We want POL as base, so invert if swap was stablecoin->POL
            let should_invert = !swap_direction.starts_with(token0);
            Ok((token1.to_string(), token0.to_string(), should_invert))
        } else if is_quote_currency(token1) {
            // token1 is quote, token0 is base
            let should_invert = swap_direction.starts_with(token1);
            Ok((token0.to_string(), token1.to_string(), should_invert))
        } else if is_quote_currency(token0) {
            // token0 is quote, token1 is base
            let should_invert = swap_direction.starts_with(token0);
            Ok((token1.to_string(), token0.to_string(), should_invert))
        } else {
            // Neither is quote - default to token0 as base, token1 as quote
            Ok((token0.to_string(), token1.to_string(), false))
        }
    }

    fn validate_pol_price(&self, price: f64, base_token: &str, quote_token: &str, steps: &mut Vec<String>) 
        -> Result<()> {
        
        if base_token == "POL" && quote_token == "USDC" {
            info!("  🧪 POL VALIDATION:");
            info!("    Current price: ${:.6} USDC per POL", price);
            
            if price < 0.05 {
                error!("    ❌ POL price too low! Expected ~$0.23, got ${:.6}", price);
                let expected_correction = 0.23 / price;
                error!("    💡 Correction factor needed: {:.1}x", expected_correction);
                steps.push(format!("POL VALIDATION FAILED: ${:.6} is too low (need {:.1}x correction)", 
                    price, expected_correction));
            } else if price > 2.0 {
                error!("    ❌ POL price too high! Expected ~$0.23, got ${:.6}", price);
                steps.push(format!("POL VALIDATION FAILED: ${:.6} is too high", price));
            } else {
                info!("    ✅ POL price looks reasonable: ${:.6}", price);
                steps.push(format!("POL VALIDATION PASSED: ${:.6} is in reasonable range", price));
            }
        }

        Ok(())
    }

    /// Test with a known swap transaction
    pub fn test_known_transaction(&self) -> Result<()> {
        info!("🧪 TESTING WITH KNOWN POL/USDC SWAP");
        
        // This is a synthetic test transaction based on typical QuickSwap POL/USDC swap
        let test_swap = SwapEvent {
            pool_address: "0x5b0d2536f0c970b8d9cbf3959460fb97ce808ade".to_string(),
            token0: TokenInfo {
                symbol: "POL".to_string(),
                address: "0x455e53908ebca69b99aa59e89e77b6d1e4b9bc61".to_string(),
                decimals: 18,
            },
            token1: TokenInfo {
                symbol: "USDC".to_string(),
                address: "0x2791bca1f2de4661ed88a30c99a7a9449aa84174".to_string(),
                decimals: 6,
            },
            // Example: Selling 1000 POL for 230 USDC (price = $0.23/POL)
            amount0_in_raw: 1000000000000000000000,  // 1000 POL (18 decimals)
            amount1_in_raw: 0,
            amount0_out_raw: 0,
            amount1_out_raw: 230000000, // 230 USDC (6 decimals)
            tx_hash: "0x1234567890abcdef".to_string(),
            block_number: 12345,
        };

        let result = self.calculate_price(&test_swap)?;
        
        info!("🎯 TEST RESULT:");
        info!("  Calculated price: ${:.6} {} per {}", 
            result.final_price, result.quote_token, result.base_token);
        info!("  Expected: ~$0.23 USDC per POL");
        
        if (result.final_price - 0.23).abs() < 0.01 {
            info!("  ✅ TEST PASSED!");
        } else {
            error!("  ❌ TEST FAILED! Expected ~$0.23, got ${:.6}", result.final_price);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pol_usdc_swap() {
        let calculator = POLPriceCalculator::new(true);
        calculator.test_known_transaction().unwrap();
    }
}