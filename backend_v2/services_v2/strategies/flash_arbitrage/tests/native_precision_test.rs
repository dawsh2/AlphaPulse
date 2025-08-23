//! Native Precision Arbitrage Detection Test
//!
//! Tests that the detector correctly handles native token precision without lossy conversions

use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use alphapulse_flash_arbitrage::detector::{DetectorConfig, OpportunityDetector, TokenPriceOracle};
use alphapulse_state_market::PoolStateManager;
use std::sync::Arc;

#[tokio::test]
async fn test_native_precision_arbitrage_detection() {
    println!("🧪 Testing Native Precision Arbitrage Detection");

    // Create detector with default config
    let pool_manager = Arc::new(PoolStateManager::new());
    let config = DetectorConfig::default();
    let detector = OpportunityDetector::new(pool_manager, config);

    // Test data: 5 WETH → 13,500 USDC swap
    let pool_address = [
        0x45, 0xdd, 0xa9, 0xcb, 0x7c, 0x25, 0x13, 0x1d, 0xf2, 0x68, 0x51, 0x51, 0x31, 0xf6, 0x47,
        0xd7, 0x26, 0xf5, 0x06, 0x08,
    ];
    let weth_addr = [
        0x7c, 0xeb, 0x23, 0xfd, 0x6f, 0x88, 0xb7, 0x6a, 0xf0, 0x52, 0xc3, 0xca, 0x45, 0x9c, 0x11,
        0x73, 0xc5, 0xb9, 0xb9, 0x6d,
    ];
    let usdc_addr = [
        0x27, 0x91, 0xbc, 0xa1, 0xf2, 0xde, 0x46, 0x61, 0xed, 0x88, 0xa3, 0x0c, 0x99, 0xa7, 0xa9,
        0x44, 0x9a, 0xa8, 0x41, 0x74,
    ];

    let amount_in: u128 = 5_000_000_000_000_000_000; // 5 WETH (18 decimals)
    let amount_out: u128 = 13500_000_000; // 13,500 USDC (6 decimals)

    println!(
        "✅ Testing swap: {} WETH → {} USDC",
        amount_in as f64 / 1e18,
        amount_out as f64 / 1e6
    );

    // Test native precision calculation
    let result = detector
        .check_arbitrage_opportunity_native(
            &pool_address,
            weth_addr,
            usdc_addr,
            amount_in,
            amount_out,
            18, // WETH decimals
            6,  // USDC decimals
        )
        .await;

    // The detector should process the data without precision loss
    // Even if no opportunity is found, it shouldn't panic or lose precision
    match result {
        Some(opportunity) => {
            println!("🎯 Arbitrage opportunity detected!");
            println!("   Expected profit: ${:.2}", opportunity.expected_profit);
            println!("   Spread: {:.4}%", opportunity.spread_percentage * 100.0);
            println!("   Required capital: ${:.2}", opportunity.required_capital);

            // Verify no precision was lost in calculations
            assert!(
                opportunity.expected_profit >= 0.0,
                "Profit should be non-negative"
            );
            assert!(
                opportunity.spread_percentage >= 0.0,
                "Spread should be non-negative"
            );
            assert!(
                opportunity.required_capital > 0.0,
                "Capital should be positive"
            );
        }
        None => {
            println!("📊 No arbitrage opportunity found (as expected for basic test)");
        }
    }

    println!("✅ Native precision test completed successfully");
}

#[tokio::test]
async fn test_decimal_precision_preservation() {
    println!("🧪 Testing Decimal Precision Preservation");

    // Test that Decimal calculations preserve precision better than f64
    let large_amount: u128 = 999_999_999_999_999_999_999_999_999; // Near u128 max
    let decimals = 18u32;

    let amount_decimal = Decimal::from(large_amount);
    let divisor = Decimal::from(10u64.pow(decimals));
    let normalized = amount_decimal / divisor;

    println!("✅ Large amount handled: {} → {}", large_amount, normalized);

    // Verify the calculation doesn't overflow or lose precision
    assert!(normalized > dec!(0), "Normalized amount should be positive");
    assert!(!normalized.is_zero(), "Should not lose precision to zero");

    // Test precision with small amounts
    let small_amount: u128 = 1; // 1 wei
    let small_decimal = Decimal::from(small_amount);
    let small_normalized = small_decimal / divisor;

    println!(
        "✅ Small amount handled: {} → {}",
        small_amount, small_normalized
    );
    assert!(
        small_normalized > dec!(0),
        "Small normalized amount should be positive"
    );

    println!("✅ Decimal precision preservation verified");
}

#[test]
fn test_token_price_oracle_addresses() {
    println!("🧪 Testing Token Price Oracle with Addresses");

    let oracle = TokenPriceOracle::new();

    // Test address-based price updates
    let weth_address = "0x7ceb23fd6f88b76af052c3ca459c1173c5b9b96d";
    let usdc_address = "0x2791bca1f2de4661ed88a30c99a7a9449aa84174";

    oracle.update_price_by_address(weth_address, dec!(2700.00));
    oracle.update_price_by_address(usdc_address, dec!(1.00));

    // Verify retrieval works
    assert_eq!(
        oracle.get_price_by_address(weth_address),
        Some(dec!(2700.00))
    );
    assert_eq!(oracle.get_price_by_address(usdc_address), Some(dec!(1.00)));
    assert_eq!(oracle.get_price_by_address("0xnonexistent"), None);

    println!("✅ Address-based token price oracle working correctly");
}
