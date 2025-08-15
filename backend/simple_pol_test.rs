fn main() {
    println!("🧪 POL Price Calculator - Simple Test");
    println!("=====================================\n");

    // Test 1: Perfect case - selling 1000 POL for 230 USDC
    println!("📋 TEST 1: Selling 1000 POL for 230 USDC (should give $0.23/POL)");
    test_pol_swap(
        1_000_000_000_000_000_000_000,  // 1000 POL (18 decimals)
        0,
        0,
        230_000_000, // 230 USDC (6 decimals)
        "POL->USDC"
    );

    println!("\n{}", "=".repeat(50));

    // Test 2: Reverse direction - buying 1000 POL with 230 USDC  
    println!("📋 TEST 2: Buying 1000 POL with 230 USDC (should give $0.23/POL)");
    test_pol_swap(
        0,
        230_000_000, // 230 USDC (6 decimals)
        1_000_000_000_000_000_000_000,  // 1000 POL (18 decimals)
        0,
        "USDC->POL"
    );

    println!("\n{}", "=".repeat(50));

    // Test 3: What would give us the wrong $0.0125 price?
    println!("📋 TEST 3: What amounts would give us $0.0125? (current wrong price)");
    test_pol_swap(
        1_000_000_000_000_000_000_000,  // 1000 POL
        0,
        0,
        12_500_000, // 12.5 USDC (18.4x less than correct 230 USDC)
        "POL->USDC (wrong amounts)"
    );

    println!("\n{}", "=".repeat(50));

    // Test 4: What if we have wrong decimal handling?
    println!("📋 TEST 4: What if USDC had wrong decimals? (18 instead of 6)");
    test_wrong_decimals();
}

fn test_pol_swap(amount0_in_raw: u128, amount1_in_raw: u128, amount0_out_raw: u128, amount1_out_raw: u128, description: &str) {
    // POL has 18 decimals, USDC has 6 decimals
    let pol_decimals = 18;
    let usdc_decimals = 6;
    
    println!("  🔍 {} Swap Analysis:", description);
    
    // Convert raw amounts (assuming token0=POL, token1=USDC)
    let amount0_in = (amount0_in_raw as f64) / (10_f64.powi(pol_decimals));
    let amount1_in = (amount1_in_raw as f64) / (10_f64.powi(usdc_decimals));
    let amount0_out = (amount0_out_raw as f64) / (10_f64.powi(pol_decimals));
    let amount1_out = (amount1_out_raw as f64) / (10_f64.powi(usdc_decimals));
    
    println!("    Raw amounts: POL_in={}, USDC_in={}, POL_out={}, USDC_out={}", 
        amount0_in_raw, amount1_in_raw, amount0_out_raw, amount1_out_raw);
    println!("    Adjusted: POL_in={:.2}, USDC_in={:.2}, POL_out={:.2}, USDC_out={:.2}", 
        amount0_in, amount1_in, amount0_out, amount1_out);

    // Calculate price based on swap direction
    let price = if amount0_in > 0.0 && amount1_out > 0.0 {
        // Selling POL for USDC: price = USDC_out / POL_in
        let p = amount1_out / amount0_in;
        println!("    Direction: Selling POL for USDC");
        println!("    Calculation: {} USDC / {} POL = ${:.6} per POL", amount1_out, amount0_in, p);
        p
    } else if amount1_in > 0.0 && amount0_out > 0.0 {
        // Buying POL with USDC: price = USDC_in / POL_out  
        let p = amount1_in / amount0_out;
        println!("    Direction: Buying POL with USDC");
        println!("    Calculation: {} USDC / {} POL = ${:.6} per POL", amount1_in, amount0_out, p);
        p
    } else {
        println!("    ❌ Invalid swap direction");
        return;
    };
    
    println!("    💰 RESULT: ${:.6} USDC per POL", price);
    
    // Validate against expected POL price
    let expected = 0.23;
    if (price - expected).abs() < 0.01 {
        println!("    ✅ CORRECT! Price matches expected ~$0.23");
    } else if (price - 0.0125).abs() < 0.001 {
        println!("    🔍 This matches the WRONG price we're seeing ($0.0125)");
        let factor = expected / price;
        println!("    💡 Correction factor needed: {:.1}x", factor);
    } else {
        println!("    ❓ Unexpected price");
    }
}

fn test_wrong_decimals() {
    println!("  🔍 Testing if USDC decimals were wrong (18 instead of 6):");
    
    // Same raw amounts as Test 1, but treat USDC as having 18 decimals
    let amount0_in_raw = 1_000_000_000_000_000_000_000u128;  // 1000 POL
    let amount1_out_raw = 230_000_000u128;  // What we think is 230 USDC (6 decimals)
    
    // Convert with wrong USDC decimals (18 instead of 6)
    let pol_in = (amount0_in_raw as f64) / (10_f64.powi(18));
    let usdc_out_wrong = (amount1_out_raw as f64) / (10_f64.powi(18));  // Wrong!
    let usdc_out_correct = (amount1_out_raw as f64) / (10_f64.powi(6));  // Correct
    
    let price_wrong = usdc_out_wrong / pol_in;
    let price_correct = usdc_out_correct / pol_in;
    
    println!("    With 6 decimals (correct): ${:.6} USDC per POL", price_correct);
    println!("    With 18 decimals (wrong):  ${:.6} USDC per POL", price_wrong);
    
    let factor = price_correct / price_wrong;
    println!("    Ratio: {:.0}x", factor);
    
    if factor > 1_000_000.0 {
        println!("    🔍 This could explain the issue! Wrong decimal assumption gives massive error");
    }
}