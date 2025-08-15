use anyhow::Result;
use tracing::{info, error};

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

    pub fn calculate_price(&self, swap: &SwapEvent) -> Result<PriceCalculation> {
        let mut steps = Vec::new();
        
        println!("🔍 CALCULATING PRICE FOR SWAP:");
        println!("  Pool: {}", swap.pool_address);
        println!("  Token0: {} ({} decimals)", swap.token0.symbol, swap.token0.decimals);
        println!("  Token1: {} ({} decimals)", swap.token1.symbol, swap.token1.decimals);
        println!("  TX: {}", swap.tx_hash);

        // Step 1: Convert raw amounts to decimal-adjusted amounts
        let amount0_in = self.convert_raw_amount(swap.amount0_in_raw, swap.token0.decimals);
        let amount1_in = self.convert_raw_amount(swap.amount1_in_raw, swap.token1.decimals);
        let amount0_out = self.convert_raw_amount(swap.amount0_out_raw, swap.token0.decimals);
        let amount1_out = self.convert_raw_amount(swap.amount1_out_raw, swap.token1.decimals);

        steps.push(format!("Raw amounts: 0_in={}, 1_in={}, 0_out={}, 1_out={}", 
            swap.amount0_in_raw, swap.amount1_in_raw, swap.amount0_out_raw, swap.amount1_out_raw));
        steps.push(format!("Decimal-adjusted: 0_in={:.6}, 1_in={:.6}, 0_out={:.6}, 1_out={:.6}", 
            amount0_in, amount1_in, amount0_out, amount1_out));

        println!("  📊 AMOUNTS:");
        println!("    Raw: amount0_in={}, amount1_in={}, amount0_out={}, amount1_out={}", 
            swap.amount0_in_raw, swap.amount1_in_raw, swap.amount0_out_raw, swap.amount1_out_raw);
        println!("    Adjusted: amount0_in={:.6}, amount1_in={:.6}, amount0_out={:.6}, amount1_out={:.6}", 
            amount0_in, amount1_in, amount0_out, amount1_out);

        // Step 2: Determine swap direction and calculate raw price
        let (raw_price, swap_direction) = if amount0_in > 0.0 && amount1_out > 0.0 {
            // Selling token0 for token1: price = token1_out / token0_in
            let price = amount1_out / amount0_in;
            steps.push(format!("Swap direction: Selling {} for {} (price = {}/{} = {:.6})", 
                swap.token0.symbol, swap.token1.symbol, amount1_out, amount0_in, price));
            (price, format!("{}->{}", swap.token0.symbol, swap.token1.symbol))
        } else if amount1_in > 0.0 && amount0_out > 0.0 {
            // Selling token1 for token0: price = token1_in / token0_out
            let price = amount1_in / amount0_out;
            steps.push(format!("Swap direction: Selling {} for {} (price = {}/{} = {:.6})", 
                swap.token1.symbol, swap.token0.symbol, amount1_in, amount0_out, price));
            (price, format!("{}->{}", swap.token1.symbol, swap.token0.symbol))
        } else {
            return Err(anyhow::anyhow!("Invalid swap: no clear direction"));
        };

        println!("  💱 SWAP ANALYSIS:");
        println!("    Direction: {}", swap_direction);
        println!("    Raw price: {:.6}", raw_price);

        // Step 3: Determine quote currency and price orientation
        let (base_token, quote_token, should_invert) = self.determine_price_orientation(
            &swap.token0.symbol, &swap.token1.symbol, &swap_direction)?;

        steps.push(format!("Price orientation: base={}, quote={}, should_invert={}", 
            base_token, quote_token, should_invert));

        println!("  🎯 PRICE ORIENTATION:");
        println!("    Base token: {}", base_token);
        println!("    Quote token: {}", quote_token);
        println!("    Should invert: {}", should_invert);

        // Step 4: Apply inversion if needed
        let final_price = if should_invert && raw_price > 0.0 {
            let inverted = 1.0 / raw_price;
            steps.push(format!("Applied inversion: {:.6} -> {:.6}", raw_price, inverted));
            println!("    Inverted price: {:.6} -> {:.6}", raw_price, inverted);
            inverted
        } else {
            steps.push(format!("No inversion needed: {:.6}", raw_price));
            raw_price
        };

        println!("  ✅ FINAL RESULT:");
        println!("    Price: {:.6} {} per {}", final_price, quote_token, base_token);

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
        if token0 == "POL" && is_quote_currency(token1) {
            let should_invert = swap_direction.starts_with(token1);
            Ok((token0.to_string(), token1.to_string(), should_invert))
        } else if token1 == "POL" && is_quote_currency(token0) {
            let should_invert = !swap_direction.starts_with(token0);
            Ok((token1.to_string(), token0.to_string(), should_invert))
        } else if is_quote_currency(token1) {
            let should_invert = swap_direction.starts_with(token1);
            Ok((token0.to_string(), token1.to_string(), should_invert))
        } else if is_quote_currency(token0) {
            let should_invert = swap_direction.starts_with(token0);
            Ok((token1.to_string(), token0.to_string(), should_invert))
        } else {
            Ok((token0.to_string(), token1.to_string(), false))
        }
    }

    fn validate_pol_price(&self, price: f64, base_token: &str, quote_token: &str, steps: &mut Vec<String>) 
        -> Result<()> {
        
        if base_token == "POL" && quote_token == "USDC" {
            println!("  🧪 POL VALIDATION:");
            println!("    Current price: ${:.6} USDC per POL", price);
            
            if price < 0.05 {
                println!("    ❌ POL price too low! Expected ~$0.23, got ${:.6}", price);
                let expected_correction = 0.23 / price;
                println!("    💡 Correction factor needed: {:.1}x", expected_correction);
                steps.push(format!("POL VALIDATION FAILED: ${:.6} is too low (need {:.1}x correction)", 
                    price, expected_correction));
            } else if price > 2.0 {
                println!("    ❌ POL price too high! Expected ~$0.23, got ${:.6}", price);
                steps.push(format!("POL VALIDATION FAILED: ${:.6} is too high", price));
            } else {
                println!("    ✅ POL price looks reasonable: ${:.6}", price);
                steps.push(format!("POL VALIDATION PASSED: ${:.6} is in reasonable range", price));
            }
        }

        Ok(())
    }
}

fn main() -> Result<()> {
    println!("🧪 POL Price Calculator - Standalone Test");
    println!("========================================\n");

    let calculator = POLPriceCalculator::new(true);

    // Test 1: Known good transaction (synthetic)
    println!("📋 TEST 1: Synthetic POL->USDC swap (should give $0.23/POL)");
    test_synthetic_swap(&calculator)?;

    println!("\n" + &"=".repeat(50));

    // Test 2: Reverse swap direction
    println!("📋 TEST 2: Synthetic USDC->POL swap (should give $0.23/POL)");
    test_reverse_swap(&calculator)?;

    println!("\n" + &"=".repeat(50));

    // Test 3: What happens with raw amounts that would give us the wrong answer
    println!("📋 TEST 3: Simulate wrong raw amounts (to match current system)");
    test_wrong_amounts(&calculator)?;

    Ok(())
}

fn test_synthetic_swap(calculator: &POLPriceCalculator) -> Result<()> {
    let swap = SwapEvent {
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
        // Selling 1000 POL for 230 USDC (price = $0.23/POL)
        amount0_in_raw: 1_000_000_000_000_000_000_000,  // 1000 POL (18 decimals)
        amount1_in_raw: 0,
        amount0_out_raw: 0,
        amount1_out_raw: 230_000_000, // 230 USDC (6 decimals)
        tx_hash: "0xtest1".to_string(),
        block_number: 12345,
    };

    let result = calculator.calculate_price(&swap)?;
    
    println!("Expected: $0.23 USDC per POL");
    println!("Actual:   ${:.6} {} per {}", result.final_price, result.quote_token, result.base_token);
    
    if (result.final_price - 0.23).abs() < 0.01 {
        println!("✅ PASS");
    } else {
        println!("❌ FAIL - Price calculation incorrect");
        for step in &result.calculation_steps {
            println!("  📝 {}", step);
        }
    }

    Ok(())
}

fn test_reverse_swap(calculator: &POLPriceCalculator) -> Result<()> {
    let swap = SwapEvent {
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
        // Buying 1000 POL with 230 USDC (price = $0.23/POL)
        amount0_in_raw: 0,
        amount1_in_raw: 230_000_000, // 230 USDC (6 decimals)
        amount0_out_raw: 1_000_000_000_000_000_000_000,  // 1000 POL (18 decimals)
        amount1_out_raw: 0,
        tx_hash: "0xtest2".to_string(),
        block_number: 12346,
    };

    let result = calculator.calculate_price(&swap)?;
    
    println!("Expected: $0.23 USDC per POL");
    println!("Actual:   ${:.6} {} per {}", result.final_price, result.quote_token, result.base_token);
    
    if (result.final_price - 0.23).abs() < 0.01 {
        println!("✅ PASS");
    } else {
        println!("❌ FAIL - Price calculation incorrect");
        for step in &result.calculation_steps {
            println!("  📝 {}", step);
        }
    }

    Ok(())
}

fn test_wrong_amounts(calculator: &POLPriceCalculator) -> Result<()> {
    // Let's see what happens if we simulate the exact amounts that would give us $0.0125
    // If true price is $0.23 and we get $0.0125, then:
    // 0.0125 = amount1_out / amount0_in
    // If amount0_in = 1000 POL, then amount1_out = 12.5 USDC (not 230 USDC)
    
    let swap = SwapEvent {
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
        // What if we're getting wrong amounts from the blockchain?
        amount0_in_raw: 1_000_000_000_000_000_000_000,  // 1000 POL
        amount1_in_raw: 0,
        amount0_out_raw: 0,
        amount1_out_raw: 12_500_000, // 12.5 USDC instead of 230 USDC
        tx_hash: "0xtest3".to_string(),
        block_number: 12347,
    };

    let result = calculator.calculate_price(&swap)?;
    
    println!("This simulation gives: ${:.6} {} per {}", result.final_price, result.quote_token, result.base_token);
    
    if (result.final_price - 0.0125).abs() < 0.001 {
        println!("🔍 This EXACTLY matches the wrong price we're seeing!");
        println!("💡 The issue is in the raw amounts we're parsing from blockchain events");
        println!("💡 We're somehow getting ~18.4x less USDC than we should be");
    } else {
        println!("🤔 This doesn't match the wrong price - issue is elsewhere");
    }

    Ok(())
}