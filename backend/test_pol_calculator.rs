use anyhow::Result;
use tracing_subscriber;

mod pol_price_calculator;
use pol_price_calculator::{POLPriceCalculator, SwapEvent, TokenInfo};

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    println!("🧪 POL Price Calculator - Standalone Test");
    println!("========================================\n");

    let calculator = POLPriceCalculator::new(true);

    // Test 1: Known good transaction (synthetic)
    println!("📋 TEST 1: Synthetic POL->USDC swap (should give $0.23/POL)");
    test_synthetic_swap(&calculator)?;

    println!("\n" + "=".repeat(50));

    // Test 2: Reverse swap direction
    println!("📋 TEST 2: Synthetic USDC->POL swap (should give $0.23/POL)");
    test_reverse_swap(&calculator)?;

    println!("\n" + "=".repeat(50));

    // Test 3: Edge case - very small amounts
    println!("📋 TEST 3: Small amount swap");
    test_small_amounts(&calculator)?;

    println!("\n" + "=".repeat(50));

    // Test 4: Current live data simulation
    println!("📋 TEST 4: Simulate current wrong calculation");
    test_wrong_calculation(&calculator)?;

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

fn test_small_amounts(calculator: &POLPriceCalculator) -> Result<()> {
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
        // Selling 1 POL for 0.23 USDC
        amount0_in_raw: 1_000_000_000_000_000_000,  // 1 POL (18 decimals)
        amount1_in_raw: 0,
        amount0_out_raw: 0,
        amount1_out_raw: 230_000, // 0.23 USDC (6 decimals)
        tx_hash: "0xtest3".to_string(),
        block_number: 12347,
    };

    let result = calculator.calculate_price(&swap)?;
    
    println!("Expected: $0.23 USDC per POL");
    println!("Actual:   ${:.6} {} per {}", result.final_price, result.quote_token, result.base_token);
    
    if (result.final_price - 0.23).abs() < 0.01 {
        println!("✅ PASS");
    } else {
        println!("❌ FAIL - Price calculation incorrect");
    }

    Ok(())
}

fn test_wrong_calculation(calculator: &POLPriceCalculator) -> Result<()> {
    println!("Simulating the current system's wrong calculation...");
    
    // Let's see what happens if we use wrong decimal handling
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
        // What if the current system is somehow getting these raw values?
        // Let's reverse-engineer: if final price is $0.0125 instead of $0.23,
        // then maybe the raw amounts are interpreted differently
        amount0_in_raw: 1_000_000_000_000_000_000_000,  // 1000 POL 
        amount1_in_raw: 0,
        amount0_out_raw: 0,
        amount1_out_raw: 12_500_000, // ~12.5 USDC instead of 230 USDC
        tx_hash: "0xtest4".to_string(),
        block_number: 12348,
    };

    let result = calculator.calculate_price(&swap)?;
    
    println!("This would give: ${:.6} {} per {}", result.final_price, result.quote_token, result.base_token);
    
    if (result.final_price - 0.0125).abs() < 0.001 {
        println!("🔍 This matches the wrong price we're seeing!");
        println!("💡 The issue might be in how we parse the raw amounts from blockchain");
    }

    Ok(())
}