//! Polygon Arbitrage Validation Test
//!
//! Specific validation tests for Polygon arbitrage detection and V3 math

use torq_e2e_tests::{
    framework::{TestConfig, TestFramework, ValidationLevel},
    scenarios::PolygonArbitrageTest,
};
use tokio_test;

#[tokio::test]
async fn test_polygon_arbitrage_detection() {
    // Initialize tracing for test debugging
    let _ = tracing_test::TracingTest::new();

    let config = TestConfig {
        timeout_secs: 300,
        cleanup: true,
        verbose: true,
        validation_level: ValidationLevel::Comprehensive,
        data_dir: std::path::PathBuf::from("/tmp/polygon_arbitrage_test"),
    };

    let framework = TestFramework::new(config).expect("Failed to create test framework");

    let test = PolygonArbitrageTest {
        use_live_data: true, // Use real Polygon data
        target_pairs: vec!["WETH/USDC".to_string()],
        min_arbitrage_opportunities: 1, // Lower threshold for test
        max_detection_latency_ms: 100,
        min_profit_threshold_usd: 5.0, // Lower threshold for test
    };

    let result = framework
        .run_scenario(test)
        .await
        .expect("Failed to run Polygon arbitrage test");

    // Validate test results
    assert!(result.success, "Polygon arbitrage test should pass");
    assert!(
        result.metrics.messages_processed > 0,
        "Should process some messages"
    );

    // Check for arbitrage-specific validations
    let arbitrage_validations: Vec<_> = result
        .validation_results
        .iter()
        .filter(|v| v.validator.contains("arbitrage") || v.validator.contains("profit"))
        .collect();

    assert!(
        !arbitrage_validations.is_empty(),
        "Should have arbitrage-specific validations"
    );

    println!("Test completed successfully:");
    println!(
        "  Messages processed: {}",
        result.metrics.messages_processed
    );
    println!("  Signals generated: {}", result.metrics.signals_generated);
    println!(
        "  Average latency: {} ms",
        result.metrics.avg_latency_ns / 1_000_000
    );
    println!("  Duration: {:?}", result.duration);
}

#[tokio::test]
async fn test_v3_math_precision() {
    // Test that V3 math calculations maintain precision
    // This test focuses on the mathematical correctness

    use torq_flash_arbitrage::math::v3_math::{
        calculate_optimal_swap_amount, calculate_v3_output_amount,
    };
    use rust_decimal::Decimal;

    // Test case: WETH/USDC pair with realistic liquidity
    let token0_reserve = Decimal::from_str("1000.0").unwrap(); // 1000 WETH
    let token1_reserve = Decimal::from_str("3500000.0").unwrap(); // 3.5M USDC
    let fee_rate = Decimal::from_str("0.003").unwrap(); // 0.3% fee

    // Calculate optimal swap for a price discrepancy
    let price_difference = Decimal::from_str("0.01").unwrap(); // 1% price difference

    let optimal_amount =
        calculate_optimal_swap_amount(token0_reserve, token1_reserve, fee_rate, price_difference)
            .expect("Should calculate optimal amount");

    assert!(
        optimal_amount > Decimal::ZERO,
        "Optimal amount should be positive"
    );
    assert!(
        optimal_amount < token0_reserve,
        "Should not exceed available liquidity"
    );

    // Test output calculation
    let output_amount =
        calculate_v3_output_amount(optimal_amount, token0_reserve, token1_reserve, fee_rate)
            .expect("Should calculate output amount");

    assert!(output_amount > Decimal::ZERO, "Output should be positive");

    // Verify no precision loss in calculations
    let precision_check = optimal_amount.scale();
    assert!(
        precision_check >= 8,
        "Should maintain at least 8 decimal places"
    );

    println!("V3 Math precision test passed:");
    println!("  Optimal swap amount: {}", optimal_amount);
    println!("  Expected output: {}", output_amount);
    println!("  Decimal precision: {}", precision_check);
}

#[tokio::test]
#[ignore] // Ignore by default as it requires live network access
async fn test_live_polygon_arbitrage() {
    // This test runs against live Polygon data and should only be run manually
    // when you want to validate real arbitrage detection

    let config = TestConfig {
        timeout_secs: 600, // 10 minutes for live data
        cleanup: true,
        verbose: true,
        validation_level: ValidationLevel::Comprehensive,
        data_dir: std::path::PathBuf::from("/tmp/live_polygon_test"),
    };

    let framework = TestFramework::new(config).expect("Failed to create test framework");

    let test = PolygonArbitrageTest {
        use_live_data: true,
        target_pairs: vec![
            "WETH/USDC".to_string(),
            "WMATIC/USDC".to_string(),
            "WBTC/USDC".to_string(),
        ],
        min_arbitrage_opportunities: 5, // Look for 5 real opportunities
        max_detection_latency_ms: 50,
        min_profit_threshold_usd: 15.0, // Realistic profit threshold
    };

    let result = framework
        .run_scenario(test)
        .await
        .expect("Failed to run live Polygon arbitrage test");

    // Detailed analysis of live results
    println!("\n🎯 LIVE POLYGON ARBITRAGE TEST RESULTS:");
    println!("  Success: {}", result.success);
    println!("  Duration: {:?}", result.duration);
    println!(
        "  Messages processed: {}",
        result.metrics.messages_processed
    );
    println!("  Signals generated: {}", result.metrics.signals_generated);
    println!(
        "  Throughput: {:.1} msg/s",
        result.metrics.throughput_msg_per_sec
    );
    println!(
        "  Max latency: {} ms",
        result.metrics.max_latency_ns / 1_000_000
    );

    // Print validation results
    println!("\n📊 VALIDATION RESULTS:");
    for validation in &result.validation_results {
        let status = if validation.passed { "✅" } else { "❌" };
        println!(
            "  {} {}: {}",
            status, validation.validator, validation.message
        );

        if let Some(ref details) = validation.details {
            if validation.validator == "profit_estimation" {
                println!(
                    "    Details: {}",
                    serde_json::to_string_pretty(details).unwrap_or_default()
                );
            }
        }
    }

    // The test should find real arbitrage opportunities if the market is active
    if result.success {
        println!("\n🚀 Successfully detected arbitrage opportunities on live Polygon data!");
    } else {
        println!(
            "\n⚠️  No arbitrage opportunities found (this may be normal during low volatility)"
        );
    }
}

// Helper function to run quick validation
pub async fn validate_arbitrage_system() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Running quick Polygon arbitrage validation...");

    let config = TestConfig {
        timeout_secs: 120,
        cleanup: true,
        verbose: false,
        validation_level: ValidationLevel::DataIntegrity,
        data_dir: std::path::PathBuf::from("/tmp/quick_arbitrage_validation"),
    };

    let framework = TestFramework::new(config)?;

    let test = PolygonArbitrageTest {
        use_live_data: true,
        target_pairs: vec!["WETH/USDC".to_string()],
        min_arbitrage_opportunities: 1,
        max_detection_latency_ms: 200,
        min_profit_threshold_usd: 1.0,
    };

    let result = framework.run_scenario(test).await?;

    if result.success {
        println!("✅ Arbitrage system validation passed!");
    } else {
        println!("❌ Arbitrage system validation failed!");
        if let Some(ref error) = result.error_message {
            println!("   Error: {}", error);
        }
    }

    Ok(())
}
