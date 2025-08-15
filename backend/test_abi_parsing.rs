fn main() {
    println!("🔍 Testing ABI Parsing Issues");
    println!("==============================\n");

    // Test what happens with realistic POL/USDC swap amounts
    test_realistic_amounts();
    
    println!("\n{}", "=".repeat(50));
    
    // Test u128 vs u256 parsing
    test_u128_vs_u256();
    
    println!("\n{}", "=".repeat(50));
    
    // Test f64 precision loss
    test_f64_precision();
}

fn test_realistic_amounts() {
    println!("📋 TEST: Realistic POL/USDC swap amounts");
    
    // Simulate a swap: 1000 POL (18 decimals) for 230 USDC (6 decimals)
    // Raw amounts on blockchain:
    let pol_amount_raw = 1_000_000_000_000_000_000_000u128;  // 1000 POL with 18 zeros
    let usdc_amount_raw = 230_000_000u128;  // 230 USDC with 6 zeros
    
    println!("  Expected amounts:");
    println!("    1000 POL raw:  {}", pol_amount_raw);
    println!("    230 USDC raw:  {}", usdc_amount_raw);
    
    // Convert to hex (as blockchain would provide)
    let pol_hex = format!("{:064x}", pol_amount_raw);
    let usdc_hex = format!("{:064x}", usdc_amount_raw);
    
    println!("  As hex (64 chars each):");
    println!("    POL:  {}", pol_hex);
    println!("    USDC: {}", usdc_hex);
    
    // Parse back (simulating our current method)
    let parsed_pol = u128::from_str_radix(&pol_hex, 16).unwrap();
    let parsed_usdc = u128::from_str_radix(&usdc_hex, 16).unwrap();
    
    println!("  Parsed back:");
    println!("    POL:  {} (matches: {})", parsed_pol, parsed_pol == pol_amount_raw);
    println!("    USDC: {} (matches: {})", parsed_usdc, parsed_usdc == usdc_amount_raw);
    
    // Apply decimal conversion
    let pol_adjusted = (parsed_pol as f64) / (10_f64.powi(18));
    let usdc_adjusted = (parsed_usdc as f64) / (10_f64.powi(6));
    
    println!("  Decimal adjusted:");
    println!("    POL:  {:.2}", pol_adjusted);
    println!("    USDC: {:.2}", usdc_adjusted);
    
    // Calculate price
    let price = usdc_adjusted / pol_adjusted;
    println!("  Price: ${:.6} per POL", price);
    
    if (price - 0.23).abs() < 0.01 {
        println!("  ✅ CORRECT!");
    } else {
        println!("  ❌ WRONG!");
    }
}

fn test_u128_vs_u256() {
    println!("📋 TEST: u128 vs u256 limits");
    
    let u128_max = u128::MAX;
    println!("  u128 max: {}", u128_max);
    println!("  u128 max hex: {:032x}", u128_max);
    
    // Test a large uint256 value that exceeds u128
    let large_hex = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"; // Max uint256
    
    println!("  Max uint256 hex: {}", large_hex);
    
    // Try to parse first 32 hex chars (16 bytes = u128 max size)
    let truncated_hex = &large_hex[32..64]; // Take last 32 chars (16 bytes)
    match u128::from_str_radix(truncated_hex, 16) {
        Ok(val) => println!("  Parsed (truncated): {}", val),
        Err(e) => println!("  Parse error: {}", e),
    }
    
    // Show what happens if we get a large amount
    let very_large_amount = "0000000000000000000000000000000000000000000000056BC75E2D630EB5E0"; // Large POL amount
    match u128::from_str_radix(very_large_amount, 16) {
        Ok(val) => {
            println!("  Large amount parsed: {}", val);
            let adjusted = (val as f64) / (10_f64.powi(18));
            println!("  Adjusted: {:.2} POL", adjusted);
        },
        Err(e) => println!("  Large amount parse error: {}", e),
    }
}

fn test_f64_precision() {
    println!("📋 TEST: f64 precision with large numbers");
    
    let large_u128 = 1_000_000_000_000_000_000_000u128; // 1000 POL raw
    let as_f64 = large_u128 as f64;
    let back_to_u128 = as_f64 as u128;
    
    println!("  Original u128:  {}", large_u128);
    println!("  As f64:         {:.0}", as_f64);
    println!("  Back to u128:   {}", back_to_u128);
    println!("  Precision lost: {}", large_u128 != back_to_u128);
    
    if large_u128 != back_to_u128 {
        let loss = large_u128 - back_to_u128;
        println!("  Amount lost:    {}", loss);
        let percentage = (loss as f64) / (large_u128 as f64) * 100.0;
        println!("  Percentage:     {:.6}%", percentage);
    }
    
    // Test with an even larger number
    let very_large = u128::MAX / 2;
    let as_f64_large = very_large as f64;
    let back_large = as_f64_large as u128;
    
    println!("\n  Very large test:");
    println!("  Original:  {}", very_large);
    println!("  As f64:    {:.0}", as_f64_large);
    println!("  Back:      {}", back_large);
    println!("  Equal:     {}", very_large == back_large);
}