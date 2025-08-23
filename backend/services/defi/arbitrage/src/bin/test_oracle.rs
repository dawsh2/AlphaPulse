// Oracle Integration Test Tool
// Demonstrates live price fetching from multiple sources

use anyhow::{Result, Context};
use clap::{Parser, Subcommand};
use ethers::prelude::*;
use std::sync::Arc;
use tracing::{info, warn, error};
use tracing_subscriber::FmtSubscriber;

use arbitrage::oracle::{PriceOracle, ChainlinkOracle, DexPriceOracle, OracleConfig};

#[derive(Parser)]
#[command(name = "test-oracle")]
#[command(about = "Test price oracle integration with live data")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    
    /// Log level (debug, info, warn, error)
    #[arg(long, default_value = "info")]
    log_level: String,
    
    /// RPC URL
    #[arg(long, default_value = "https://polygon-rpc.com")]
    rpc_url: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Test Chainlink price feeds
    Chainlink {
        /// Token address to check
        #[arg(long)]
        token: Option<String>,
    },
    
    /// Test DEX price quotes
    Dex {
        /// Token address to check
        #[arg(long)]
        token: Option<String>,
    },
    
    /// Test unified price oracle
    Oracle {
        /// Token address to check
        #[arg(long)]
        token: Option<String>,
    },
    
    /// Compare all price sources
    Compare {
        /// Tokens to compare (comma-separated addresses)
        #[arg(long)]
        tokens: Option<String>,
    },
    
    /// Run gas cost estimation test
    Gas {
        /// Gas units to test
        #[arg(long, default_value = "200000")]
        gas_units: u64,
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
    
    let provider = Arc::new(Provider::<Http>::try_from(&cli.rpc_url)?);
    
    match cli.command {
        Commands::Chainlink { token } => {
            test_chainlink_oracle(provider, token).await?;
        }
        Commands::Dex { token } => {
            test_dex_oracle(provider, token).await?;
        }
        Commands::Oracle { token } => {
            test_unified_oracle(provider, token).await?;
        }
        Commands::Compare { tokens } => {
            compare_price_sources(provider, tokens).await?;
        }
        Commands::Gas { gas_units } => {
            test_gas_cost_estimation(provider, gas_units).await?;
        }
    }
    
    Ok(())
}

/// Test Chainlink oracle
async fn test_chainlink_oracle(provider: Arc<Provider<Http>>, token: Option<String>) -> Result<()> {
    info!("📡 Testing Chainlink Oracle");
    info!("=" .repeat(40));
    
    let oracle = ChainlinkOracle::new(provider).await?;
    
    let test_tokens = if let Some(token_str) = token {
        vec![token_str.parse()?]
    } else {
        oracle.supported_tokens()
    };
    
    info!("🔍 Testing {} tokens", test_tokens.len());
    
    for token_addr in test_tokens {
        info!("\n📍 Token: {:?}", token_addr);
        
        match oracle.get_price(token_addr).await {
            Ok(price) => {
                info!("✅ Chainlink price: ${:.6}", price);
                
                // Check feed health
                match oracle.get_feed_health(token_addr).await {
                    Ok(health) => {
                        info!("🏥 Feed health: {:?}", health);
                    }
                    Err(e) => {
                        warn!("⚠️ Could not check feed health: {}", e);
                    }
                }
            }
            Err(e) => {
                error!("❌ Failed to get Chainlink price: {}", e);
            }
        }
    }
    
    Ok(())
}

/// Test DEX oracle
async fn test_dex_oracle(provider: Arc<Provider<Http>>, token: Option<String>) -> Result<()> {
    info!("🔄 Testing DEX Oracle");
    info!("=" .repeat(40));
    
    let oracle = DexPriceOracle::new(provider).await?;
    
    let test_tokens = if let Some(token_str) = token {
        vec![token_str.parse()?]
    } else {
        // Default test tokens
        vec![
            "0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270".parse()?, // WMATIC
            "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174".parse()?, // USDC
        ]
    };
    
    for token_addr in test_tokens {
        info!("\n📍 Token: {:?}", token_addr);
        
        match oracle.get_price(token_addr).await {
            Ok(price) => {
                info!("✅ DEX price: ${:.6}", price);
                
                // Test liquidity-weighted price
                match oracle.get_weighted_price(token_addr).await {
                    Ok(weighted_price) => {
                        info!("📊 Liquidity-weighted price: ${:.6}", weighted_price);
                    }
                    Err(e) => {
                        warn!("⚠️ Could not get weighted price: {}", e);
                    }
                }
                
                // Check liquidity
                let min_liquidity = 10000.0; // $10k
                let has_liquidity = oracle.has_sufficient_liquidity(token_addr, min_liquidity).await;
                info!("💧 Sufficient liquidity (>${:.0}): {}", min_liquidity, has_liquidity);
            }
            Err(e) => {
                error!("❌ Failed to get DEX price: {}", e);
            }
        }
    }
    
    Ok(())
}

/// Test unified oracle
async fn test_unified_oracle(provider: Arc<Provider<Http>>, token: Option<String>) -> Result<()> {
    info!("🔮 Testing Unified Price Oracle");
    info!("=" .repeat(50));
    
    let config = OracleConfig::default();
    let oracle = PriceOracle::new(provider, config).await?;
    
    let test_tokens = if let Some(token_str) = token {
        vec![token_str.parse()?]
    } else {
        vec![
            "0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270".parse()?, // WMATIC
            "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174".parse()?, // USDC
            "0xc2132D05D31c914a87C6611C10748AEb04B58e8F".parse()?, // USDT
        ]
    };
    
    for token_addr in test_tokens {
        info!("\n📍 Token: {:?}", token_addr);
        
        match oracle.get_price(token_addr).await {
            Ok(price_data) => {
                info!("✅ Oracle price: ${:.6}", price_data.price_usd);
                info!("📊 Source: {:?}", price_data.source);
                info!("🎯 Confidence: {:.1}%", price_data.confidence * 100.0);
                info!("⏰ Age: {}s", price_data.staleness_seconds);
            }
            Err(e) => {
                error!("❌ Failed to get oracle price: {}", e);
            }
        }
    }
    
    // Test cache statistics
    let (fresh, total) = oracle.get_cache_stats().await;
    info!("\n📋 Cache stats: {}/{} fresh entries", fresh, total);
    
    Ok(())
}

/// Compare all price sources
async fn compare_price_sources(provider: Arc<Provider<Http>>, tokens: Option<String>) -> Result<()> {
    info!("🔍 Comparing All Price Sources");
    info!("=" .repeat(60));
    
    let test_tokens: Vec<Address> = if let Some(token_str) = tokens {
        token_str.split(',')
            .map(|s| s.trim().parse())
            .collect::<Result<Vec<_>, _>>()?
    } else {
        vec![
            "0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270".parse()?, // WMATIC
        ]
    };
    
    // Initialize all oracles
    let chainlink = ChainlinkOracle::new(provider.clone()).await?;
    let dex_oracle = DexPriceOracle::new(provider.clone()).await?;
    let unified = PriceOracle::new(provider, OracleConfig::default()).await?;
    
    for token_addr in test_tokens {
        info!("\n📍 Comparing prices for: {:?}", token_addr);
        info!("-" .repeat(50));
        
        // Chainlink price
        match chainlink.get_price(token_addr).await {
            Ok(price) => info!("📡 Chainlink:    ${:.6}", price),
            Err(_) => info!("📡 Chainlink:    Not available"),
        }
        
        // DEX price
        match dex_oracle.get_price(token_addr).await {
            Ok(price) => info!("🔄 DEX Quote:    ${:.6}", price),
            Err(_) => info!("🔄 DEX Quote:    Not available"),
        }
        
        // Unified oracle price
        match unified.get_price(token_addr).await {
            Ok(price_data) => {
                info!("🔮 Unified:      ${:.6} ({:?})", price_data.price_usd, price_data.source);
            }
            Err(_) => info!("🔮 Unified:      Not available"),
        }
    }
    
    Ok(())
}

/// Test gas cost estimation
async fn test_gas_cost_estimation(provider: Arc<Provider<Http>>, gas_units: u64) -> Result<()> {
    info!("⛽ Testing Gas Cost Estimation");
    info!("=" .repeat(40));
    
    let config = OracleConfig::default();
    let oracle = PriceOracle::new(provider.clone(), config).await?;
    
    // Get current gas price
    let gas_price = provider.get_gas_price().await?;
    
    info!("📊 Gas parameters:");
    info!("  Gas units: {:,}", gas_units);
    info!("  Gas price: {} Gwei", gas_price.as_u128() as f64 / 1e9);
    
    // Calculate gas cost using oracle
    match oracle.calculate_gas_cost_usd(gas_units, gas_price).await {
        Ok(cost_usd) => {
            info!("💰 Gas cost: ${:.6}", cost_usd);
            
            // Show breakdown
            let cost_matic = (gas_units as f64 * gas_price.as_u128() as f64) / 1e18;
            let matic_price = oracle.get_matic_price().await?;
            
            info!("📋 Breakdown:");
            info!("  Cost in MATIC: {:.8}", cost_matic);
            info!("  MATIC price: ${:.4}", matic_price);
            info!("  Total USD: ${:.6}", cost_usd);
            
            // Show percentage of different trade sizes
            let trade_sizes = [10.0, 100.0, 1000.0];
            info!("\n📊 Gas cost as % of trade:");
            for trade_size in trade_sizes {
                let percentage = (cost_usd / trade_size) * 100.0;
                info!("  ${:.0} trade: {:.3}%", trade_size, percentage);
            }
        }
        Err(e) => {
            error!("❌ Failed to calculate gas cost: {}", e);
        }
    }
    
    Ok(())
}

/// Print help information
fn print_usage() {
    println!("\n📚 Oracle Test Usage Examples:");
    println!("===============================");
    println!();
    println!("1. Test Chainlink feeds:");
    println!("   cargo run --bin test-oracle -- chainlink");
    println!();
    println!("2. Test DEX quotes:");
    println!("   cargo run --bin test-oracle -- dex");
    println!();
    println!("3. Test unified oracle:");
    println!("   cargo run --bin test-oracle -- oracle");
    println!();
    println!("4. Compare all sources:");
    println!("   cargo run --bin test-oracle -- compare");
    println!();
    println!("5. Test specific token:");
    println!("   cargo run --bin test-oracle -- oracle --token 0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270");
    println!();
    println!("6. Test gas estimation:");
    println!("   cargo run --bin test-oracle -- gas --gas-units 350000");
}