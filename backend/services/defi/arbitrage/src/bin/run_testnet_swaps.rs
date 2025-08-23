// Testnet Swap Execution Tool
// Executes real swaps on testnets to validate the arbitrage system

use anyhow::{Result, Context};
use clap::{Parser, Subcommand};
use std::env;
use tracing::{info, warn, error};
use tracing_subscriber::FmtSubscriber;

use arbitrage::testing::testnet_swap_executor::{TestnetSwapExecutor, TestnetSwapConfig};

#[derive(Parser)]
#[command(name = "testnet-swaps")]
#[command(about = "Execute real swaps on Polygon testnets for validation")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    
    /// Log level (debug, info, warn, error)
    #[arg(long, default_value = "info")]
    log_level: String,
    
    /// Private key for testnet wallet
    #[arg(long, env = "TESTNET_PRIVATE_KEY")]
    private_key: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Check wallet balances
    Balance {
        /// Network to use (mumbai or amoy)
        #[arg(long, default_value = "mumbai")]
        network: String,
    },
    
    /// Execute a single test swap
    Swap {
        /// Network to use (mumbai or amoy)
        #[arg(long, default_value = "mumbai")]
        network: String,
        
        /// Token to swap from
        #[arg(long)]
        token_in: String,
        
        /// Token to swap to
        #[arg(long)]
        token_out: String,
        
        /// Amount to swap
        #[arg(long)]
        amount: f64,
    },
    
    /// Run comprehensive test suite
    Suite {
        /// Network to use (mumbai or amoy)
        #[arg(long, default_value = "mumbai")]
        network: String,
    },
    
    /// Run arbitrage simulation
    Arbitrage {
        /// Network to use (mumbai or amoy)
        #[arg(long, default_value = "mumbai")]
        network: String,
        
        /// Amount for arbitrage test
        #[arg(long, default_value = "0.1")]
        amount: f64,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Setup logging
    let log_level = cli.log_level.parse().unwrap_or(tracing::Level::INFO);
    let subscriber = FmtSubscriber::builder()
        .with_max_level(log_level)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;
    
    if cli.private_key.is_empty() {
        error!("❌ Private key required. Set TESTNET_PRIVATE_KEY environment variable.");
        std::process::exit(1);
    }
    
    match cli.command {
        Commands::Balance { network } => {
            check_balances(network, &cli.private_key).await?;
        }
        Commands::Swap { network, token_in, token_out, amount } => {
            execute_single_swap(network, &cli.private_key, &token_in, &token_out, amount).await?;
        }
        Commands::Suite { network } => {
            run_test_suite(network, &cli.private_key).await?;
        }
        Commands::Arbitrage { network, amount } => {
            run_arbitrage_simulation(network, &cli.private_key, amount).await?;
        }
    }
    
    Ok(())
}

/// Check wallet balances
async fn check_balances(network: String, private_key: &str) -> Result<()> {
    info!("💰 Checking balances on {}", network);
    
    let config = get_network_config(&network)?;
    let executor = TestnetSwapExecutor::new(config, private_key).await?;
    
    executor.check_balances().await?;
    
    info!("✅ Balance check complete");
    Ok(())
}

/// Execute a single test swap
async fn execute_single_swap(
    network: String,
    private_key: &str,
    token_in: &str,
    token_out: &str,
    amount: f64,
) -> Result<()> {
    info!("🔄 Executing swap: {} {} -> {} on {}", amount, token_in, token_out, network);
    
    let config = get_network_config(&network)?;
    let mut executor = TestnetSwapExecutor::new(config, private_key).await?;
    
    // Check balances first
    executor.check_balances().await?;
    
    // Execute the swap
    let result = executor.execute_test_swap(token_in, token_out, amount).await?;
    
    if result.success {
        info!("✅ Swap completed successfully!");
        info!("   TX Hash: {:?}", result.tx_hash);
        info!("   Gas Used: {:,}", result.gas_used);
        info!("   Amount Out: {}", result.amount_out);
        info!("   Slippage: {:.2}%", result.actual_slippage);
        info!("   Execution Time: {}ms", result.execution_time_ms);
    } else {
        error!("❌ Swap failed: {}", result.error_message.unwrap_or_default());
    }
    
    Ok(())
}

/// Run comprehensive test suite
async fn run_test_suite(network: String, private_key: &str) -> Result<()> {
    info!("🧪 Running comprehensive test suite on {}", network);
    
    let config = get_network_config(&network)?;
    let mut executor = TestnetSwapExecutor::new(config, private_key).await?;
    
    // Check balances first
    executor.check_balances().await?;
    
    // Run the test suite
    let results = executor.run_test_suite().await?;
    
    // Print summary
    if results.success_rate > 80.0 {
        info!("🎉 Test suite PASSED with {:.1}% success rate", results.success_rate);
    } else {
        warn!("⚠️ Test suite had issues with {:.1}% success rate", results.success_rate);
    }
    
    Ok(())
}

/// Run arbitrage simulation
async fn run_arbitrage_simulation(network: String, private_key: &str, amount: f64) -> Result<()> {
    info!("🎯 Running arbitrage simulation on {}", network);
    
    let config = get_network_config(&network)?;
    let mut executor = TestnetSwapExecutor::new(config, private_key).await?;
    
    // Check balances first
    executor.check_balances().await?;
    
    info!("📊 Simulating arbitrage cycle: WMATIC -> USDC -> WMATIC");
    
    // Step 1: WMATIC -> USDC
    info!("Step 1: {} WMATIC -> USDC", amount);
    let result1 = executor.execute_test_swap("WMATIC", "USDC", amount).await?;
    
    if !result1.success {
        error!("❌ First swap failed: {}", result1.error_message.unwrap_or_default());
        return Ok(());
    }
    
    // Calculate amount for second swap (90% of received amount for safety)
    let usdc_received = result1.amount_out.as_u128() as f64 / 1e18 * 0.9;
    
    // Wait a bit between swaps
    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
    
    // Step 2: USDC -> WMATIC
    info!("Step 2: {:.6} USDC -> WMATIC", usdc_received);
    let result2 = executor.execute_test_swap("USDC", "WMATIC", usdc_received).await?;
    
    if !result2.success {
        error!("❌ Second swap failed: {}", result2.error_message.unwrap_or_default());
        return Ok(());
    }
    
    // Calculate arbitrage result
    let initial_amount = amount;
    let final_amount = result2.amount_out.as_u128() as f64 / 1e18;
    let total_gas_cost = (result1.gas_used + result2.gas_used) as f64 * 
                        result1.gas_price.as_u128() as f64 / 1e18;
    
    let gross_profit = final_amount - initial_amount;
    let net_profit = gross_profit - total_gas_cost;
    let profit_percentage = (net_profit / initial_amount) * 100.0;
    
    info!("\n🎯 ARBITRAGE SIMULATION RESULTS:");
    info!("================================");
    info!("Initial amount: {:.6} WMATIC", initial_amount);
    info!("Final amount: {:.6} WMATIC", final_amount);
    info!("Gross profit: {:.6} WMATIC", gross_profit);
    info!("Gas cost: {:.8} WMATIC", total_gas_cost);
    info!("Net profit: {:.6} WMATIC", net_profit);
    info!("Profit percentage: {:.3}%", profit_percentage);
    info!("Total slippage: {:.2}%", result1.actual_slippage + result2.actual_slippage);
    info!("Total execution time: {}ms", result1.execution_time_ms + result2.execution_time_ms);
    
    if net_profit > 0.0 {
        info!("✅ Arbitrage simulation PROFITABLE!");
    } else if net_profit > -0.001 {
        warn!("📊 Arbitrage simulation break-even");
    } else {
        warn!("❌ Arbitrage simulation shows loss");
    }
    
    Ok(())
}

/// Get network configuration
fn get_network_config(network: &str) -> Result<TestnetSwapConfig> {
    match network.to_lowercase().as_str() {
        "mumbai" => Ok(TestnetSwapConfig::mumbai()),
        "amoy" => Ok(TestnetSwapConfig::amoy()),
        _ => Err(anyhow::anyhow!("Unsupported network: {}. Use 'mumbai' or 'amoy'", network)),
    }
}

/// Print help information
fn print_usage_examples() {
    println!("\n📚 Usage Examples:");
    println!("=================");
    println!();
    println!("1. Check balances:");
    println!("   cargo run --bin run_testnet_swaps -- balance --network mumbai");
    println!();
    println!("2. Execute a single swap:");
    println!("   cargo run --bin run_testnet_swaps -- swap --network mumbai --token-in WMATIC --token-out USDC --amount 0.1");
    println!();
    println!("3. Run test suite:");
    println!("   cargo run --bin run_testnet_swaps -- suite --network mumbai");
    println!();
    println!("4. Run arbitrage simulation:");
    println!("   cargo run --bin run_testnet_swaps -- arbitrage --network mumbai --amount 0.1");
    println!();
    println!("📝 Setup:");
    println!("  export TESTNET_PRIVATE_KEY=\"your_private_key_here\"");
    println!("  Get test tokens from: https://faucet.polygon.technology/");
}