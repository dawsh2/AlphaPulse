//! End-to-End Test Runner for AlphaPulse

use alphapulse_e2e_tests::{
    framework::{TestConfig, TestFramework, ValidationLevel},
    scenarios::{KrakenToDashboardTest, PolygonArbitrageTest},
    TestResult,
};
use anyhow::Result;
use clap::Parser;
use serde_json;
use std::path::PathBuf;
use tracing::{error, info, Level};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Test scenario to run
    #[arg(short, long, default_value = "all")]
    scenario: String,

    /// Validation level
    #[arg(short, long, default_value = "comprehensive")]
    validation: String,

    /// Test timeout in seconds
    #[arg(short, long, default_value_t = 300)]
    timeout: u64,

    /// Output results to file
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Skip cleanup after test
    #[arg(long)]
    no_cleanup: bool,

    /// Use live Kraken data (instead of mock)
    #[arg(long)]
    live_data: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize tracing
    let log_level = if args.verbose {
        Level::DEBUG
    } else {
        Level::INFO
    };

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(format!("alphapulse_e2e_tests={}", log_level).parse()?)
                .add_directive(format!("alphapulse_adapters={}", log_level).parse()?)
                .add_directive(format!("alphapulse_protocol={}", log_level).parse()?),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting AlphaPulse E2E Test Suite");
    info!("Version: {}", env!("CARGO_PKG_VERSION"));

    // Parse validation level
    let validation_level = match args.validation.as_str() {
        "basic" => ValidationLevel::Basic,
        "data" => ValidationLevel::DataIntegrity,
        "comprehensive" => ValidationLevel::Comprehensive,
        _ => {
            error!("Invalid validation level: {}", args.validation);
            return Ok(());
        }
    };

    // Create test configuration
    let config = TestConfig {
        timeout_secs: args.timeout,
        cleanup: !args.no_cleanup,
        verbose: args.verbose,
        validation_level,
        data_dir: PathBuf::from("/tmp/alphapulse_e2e_tests"),
    };

    // Create test framework
    let framework = TestFramework::new(config)?;

    // Run tests based on scenario
    let results = match args.scenario.as_str() {
        "kraken" | "kraken_to_dashboard" => {
            let test = KrakenToDashboardTest {
                use_live_data: args.live_data,
                expected_messages: 50,
                max_latency_ms: 100,
            };
            vec![framework.run_scenario(test).await?]
        }

        "polygon" | "polygon_arbitrage" => {
            let test = PolygonArbitrageTest {
                use_live_data: args.live_data,
                target_pairs: vec![
                    "WETH/USDC".to_string(),
                    "WMATIC/USDC".to_string(),
                    "WBTC/USDC".to_string(),
                ],
                min_arbitrage_opportunities: 3,
                max_detection_latency_ms: 50,
                min_profit_threshold_usd: 10.0,
            };
            vec![framework.run_scenario(test).await?]
        }

        "all" => {
            info!("Running all test scenarios");
            let mut results = Vec::new();

            // Kraken to Dashboard test
            let kraken_test = KrakenToDashboardTest {
                use_live_data: args.live_data,
                expected_messages: 30,
                max_latency_ms: 100,
            };
            results.push(framework.run_scenario(kraken_test).await?);

            // Polygon Arbitrage test
            let polygon_test = PolygonArbitrageTest {
                use_live_data: args.live_data,
                target_pairs: vec!["WETH/USDC".to_string(), "WMATIC/USDC".to_string()],
                min_arbitrage_opportunities: 2,
                max_detection_latency_ms: 100,
                min_profit_threshold_usd: 5.0,
            };
            results.push(framework.run_scenario(polygon_test).await?);

            results
        }

        _ => {
            error!("Unknown test scenario: {}", args.scenario);
            return Ok(());
        }
    };

    // Print results summary
    print_results_summary(&results);

    // Save results to file if requested
    if let Some(output_path) = args.output {
        save_results_to_file(&results, &output_path).await?;
        info!("Results saved to: {}", output_path.display());
    }

    // Exit with error code if any tests failed
    let all_passed = results.iter().all(|r| r.success);
    if !all_passed {
        error!("Some tests failed");
        std::process::exit(1);
    }

    info!("All tests passed successfully!");
    Ok(())
}

fn print_results_summary(results: &[TestResult]) {
    println!("\n═══════════════════════════════════════");
    println!("        TEST RESULTS SUMMARY");
    println!("═══════════════════════════════════════");

    let total_tests = results.len();
    let passed_tests = results.iter().filter(|r| r.success).count();
    let failed_tests = total_tests - passed_tests;

    println!("Total Tests: {}", total_tests);
    println!("Passed:      {} ✓", passed_tests);
    println!("Failed:      {} ✗", failed_tests);
    println!();

    for result in results {
        let status = if result.success {
            "✓ PASS"
        } else {
            "✗ FAIL"
        };
        let duration_ms = result.duration.as_millis();

        println!("{} {} ({} ms)", status, result.scenario_name, duration_ms);

        if !result.success {
            if let Some(ref error) = result.error_message {
                println!("      Error: {}", error);
            }
        }

        // Print key metrics
        println!(
            "      Messages: {} | Throughput: {:.1} msg/s | Max Latency: {} ms",
            result.metrics.messages_processed,
            result.metrics.throughput_msg_per_sec,
            result.metrics.max_latency_ns / 1_000_000
        );

        if result.metrics.signals_generated > 0 {
            println!(
                "      Signals Generated: {}",
                result.metrics.signals_generated
            );
        }

        // Print validation failures
        let failed_validations: Vec<_> = result
            .validation_results
            .iter()
            .filter(|v| !v.passed)
            .collect();

        if !failed_validations.is_empty() {
            println!("      Validation Failures:");
            for validation in failed_validations {
                println!("        - {}: {}", validation.validator, validation.message);
            }
        }

        println!();
    }

    println!("═══════════════════════════════════════");
}

async fn save_results_to_file(results: &[TestResult], path: &PathBuf) -> Result<()> {
    let json_output = serde_json::json!({
        "test_run": {
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "version": env!("CARGO_PKG_VERSION"),
            "total_tests": results.len(),
            "passed_tests": results.iter().filter(|r| r.success).count(),
            "failed_tests": results.iter().filter(|r| !r.success).count(),
        },
        "results": results
    });

    tokio::fs::write(path, serde_json::to_string_pretty(&json_output)?).await?;
    Ok(())
}
