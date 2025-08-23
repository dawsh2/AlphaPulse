use anyhow::Result;
use ethers::{
    prelude::*,
    providers::{Provider, Ws},
    types::{Address, U256},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use futures::StreamExt;
use std::collections::HashMap;
use chrono::{DateTime, Utc};

mod scanner;
mod executor;
mod flash_loan;
mod mev_protection;

use scanner::ArbitrageScanner;
use executor::ArbitrageExecutor;
use flash_loan::FlashLoanExecutor;
use mev_protection::MevProtector;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    // Execution parameters
    pub min_profit_usd: f64,
    pub max_gas_price_gwei: u64,
    pub min_confidence_score: f64,
    pub use_flash_loans: bool,
    pub execute_trades: bool,
    
    // Network
    pub rpc_url: String,
    pub chain_id: u64,
    
    // MEV Protection
    pub use_flashbots: bool,
    pub flashbots_relay_url: Option<String>,
    pub max_priority_fee_gwei: u64,
    
    // Capital management
    pub max_position_size_usd: f64,
    pub reserve_balance_matic: f64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            min_profit_usd: 1.0,
            max_gas_price_gwei: 100,
            min_confidence_score: 0.8,
            use_flash_loans: true,
            execute_trades: false, // Start in simulation
            
            rpc_url: "wss://polygon-bor.publicnode.com".to_string(),
            chain_id: 137, // Polygon
            
            use_flashbots: true,
            flashbots_relay_url: Some("https://polygon-relay.marlin.org".to_string()),
            max_priority_fee_gwei: 50,
            
            max_position_size_usd: 10000.0,
            reserve_balance_matic: 10.0,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ArbitrageOpportunity {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub buy_pool: Address,
    pub sell_pool: Address,
    pub buy_router: Address,
    pub sell_router: Address,
    pub token0: Address,
    pub token1: Address,
    pub profit_usd: f64,
    pub spread_pct: f64,
    pub size_usd: f64,
    pub confidence: f64,
    pub gas_estimate: U256,
}

pub struct ArbitrageBot {
    config: Arc<Config>,
    provider: Arc<Provider<Ws>>,
    scanner: Arc<ArbitrageScanner>,
    executor: Arc<ArbitrageExecutor>,
    flash_loan: Arc<FlashLoanExecutor>,
    mev_protector: Arc<MevProtector>,
    opportunities: Arc<RwLock<HashMap<String, ArbitrageOpportunity>>>,
    stats: Arc<RwLock<BotStats>>,
}

#[derive(Debug, Default, Serialize)]
struct BotStats {
    pub opportunities_found: u64,
    pub trades_executed: u64,
    pub trades_successful: u64,
    pub total_profit_usd: f64,
    pub total_gas_spent: U256,
    pub start_time: Option<DateTime<Utc>>,
}

impl ArbitrageBot {
    pub async fn new(config: Config) -> Result<Self> {
        info!("🚀 Initializing ArbitrageBot");
        
        // Connect to Polygon via WebSocket for lowest latency
        let provider = Provider::<Ws>::connect(&config.rpc_url).await?;
        let provider = Arc::new(provider);
        
        // Initialize components
        let scanner = Arc::new(ArbitrageScanner::new(provider.clone(), config.clone()));
        let executor = Arc::new(ArbitrageExecutor::new(provider.clone(), config.clone()));
        let flash_loan = Arc::new(FlashLoanExecutor::new(provider.clone(), config.clone()));
        let mev_protector = Arc::new(MevProtector::new(provider.clone(), config.clone()));
        
        Ok(Self {
            config: Arc::new(config),
            provider,
            scanner,
            executor,
            flash_loan,
            mev_protector,
            opportunities: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(BotStats::default())),
        })
    }
    
    pub async fn run(&self) -> Result<()> {
        info!("🤖 Starting Arbitrage Bot");
        info!("⚙️  Configuration:");
        info!("  Min Profit: ${}", self.config.min_profit_usd);
        info!("  Max Gas: {} gwei", self.config.max_gas_price_gwei);
        info!("  Flash Loans: {}", self.config.use_flash_loans);
        info!("  MEV Protection: {}", self.config.use_flashbots);
        info!("  Mode: {}", if self.config.execute_trades { "LIVE" } else { "SIMULATION" });
        
        // Update start time
        {
            let mut stats = self.stats.write().await;
            stats.start_time = Some(Utc::now());
        }
        
        // Start components
        let mut handles = vec![];
        
        // Scanner task
        let scanner_handle = self.start_scanner();
        handles.push(scanner_handle);
        
        // Executor task
        let executor_handle = self.start_executor();
        handles.push(executor_handle);
        
        // Stats reporter task
        let stats_handle = self.start_stats_reporter();
        handles.push(stats_handle);
        
        // Metrics server
        let metrics_handle = self.start_metrics_server();
        handles.push(metrics_handle);
        
        // Wait for all tasks
        futures::future::join_all(handles).await;
        
        Ok(())
    }
    
    fn start_scanner(&self) -> tokio::task::JoinHandle<()> {
        let scanner = self.scanner.clone();
        let opportunities = self.opportunities.clone();
        let stats = self.stats.clone();
        
        tokio::spawn(async move {
            info!("🔍 Scanner started");
            
            loop {
                match scanner.scan_for_opportunities().await {
                    Ok(new_opps) => {
                        let mut opps = opportunities.write().await;
                        let mut stats = stats.write().await;
                        
                        for opp in new_opps {
                            if opp.confidence >= scanner.config.min_confidence_score {
                                info!(
                                    "🎯 Opportunity found: ${:.2} profit, {:.3}% spread",
                                    opp.profit_usd, opp.spread_pct
                                );
                                
                                opps.insert(opp.id.clone(), opp);
                                stats.opportunities_found += 1;
                            }
                        }
                    }
                    Err(e) => {
                        error!("Scanner error: {}", e);
                    }
                }
                
                // Brief pause before next scan
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        })
    }
    
    fn start_executor(&self) -> tokio::task::JoinHandle<()> {
        let executor = self.executor.clone();
        let flash_loan = self.flash_loan.clone();
        let mev_protector = self.mev_protector.clone();
        let opportunities = self.opportunities.clone();
        let stats = self.stats.clone();
        let config = self.config.clone();
        
        tokio::spawn(async move {
            info!("⚡ Executor started");
            
            loop {
                // Get executable opportunities
                let executable = {
                    let opps = opportunities.read().await;
                    opps.values()
                        .filter(|o| o.profit_usd >= config.min_profit_usd)
                        .cloned()
                        .collect::<Vec<_>>()
                };
                
                for opp in executable {
                    info!("🎬 Executing opportunity: {}", opp.id);
                    
                    // Check gas price
                    if let Ok(gas_price) = executor.provider.get_gas_price().await {
                        let gas_gwei = gas_price.as_u64() / 1_000_000_000;
                        
                        if gas_gwei > config.max_gas_price_gwei {
                            warn!("⛽ Gas too high: {} gwei", gas_gwei);
                            continue;
                        }
                    }
                    
                    let result = if config.execute_trades {
                        // Real execution
                        if config.use_flash_loans {
                            flash_loan.execute_with_flash_loan(&opp).await
                        } else {
                            executor.execute_with_capital(&opp).await
                        }
                    } else {
                        // Simulation
                        info!("🔬 SIMULATION: Would execute trade");
                        Ok(H256::zero())
                    };
                    
                    // Update stats
                    let mut stats = stats.write().await;
                    stats.trades_executed += 1;
                    
                    match result {
                        Ok(tx_hash) => {
                            info!("✅ Trade executed: {:?}", tx_hash);
                            stats.trades_successful += 1;
                            stats.total_profit_usd += opp.profit_usd;
                            
                            // Remove executed opportunity
                            opportunities.write().await.remove(&opp.id);
                        }
                        Err(e) => {
                            error!("❌ Execution failed: {}", e);
                        }
                    }
                }
                
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        })
    }
    
    fn start_stats_reporter(&self) -> tokio::task::JoinHandle<()> {
        let stats = self.stats.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
            
            loop {
                interval.tick().await;
                
                let stats = stats.read().await;
                let runtime = stats.start_time
                    .map(|start| (Utc::now() - start).num_seconds())
                    .unwrap_or(0);
                
                info!("📊 Statistics Report");
                info!("  Runtime: {}s", runtime);
                info!("  Opportunities Found: {}", stats.opportunities_found);
                info!("  Trades Executed: {}", stats.trades_executed);
                info!("  Success Rate: {:.1}%", 
                    if stats.trades_executed > 0 {
                        (stats.trades_successful as f64 / stats.trades_executed as f64) * 100.0
                    } else {
                        0.0
                    }
                );
                info!("  Total Profit: ${:.2}", stats.total_profit_usd);
                info!("  Profit/Hour: ${:.2}", 
                    if runtime > 0 {
                        stats.total_profit_usd / (runtime as f64 / 3600.0)
                    } else {
                        0.0
                    }
                );
            }
        })
    }
    
    fn start_metrics_server(&self) -> tokio::task::JoinHandle<()> {
        let stats = self.stats.clone();
        
        tokio::spawn(async move {
            // Prometheus metrics endpoint
            let app = axum::Router::new()
                .route("/metrics", axum::routing::get(move || {
                    let stats = stats.clone();
                    async move {
                        let stats = stats.read().await;
                        format!(
                            "# HELP arbitrage_opportunities_total Total opportunities found\n\
                            # TYPE arbitrage_opportunities_total counter\n\
                            arbitrage_opportunities_total {}\n\
                            # HELP arbitrage_trades_total Total trades executed\n\
                            # TYPE arbitrage_trades_total counter\n\
                            arbitrage_trades_total {}\n\
                            # HELP arbitrage_profit_usd Total profit in USD\n\
                            # TYPE arbitrage_profit_usd gauge\n\
                            arbitrage_profit_usd {}\n",
                            stats.opportunities_found,
                            stats.trades_executed,
                            stats.total_profit_usd
                        )
                    }
                }));
            
            let listener = tokio::net::TcpListener::bind("0.0.0.0:9090")
                .await
                .unwrap();
            
            info!("📈 Metrics server running on :9090");
            axum::serve(listener, app).await.unwrap();
        })
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("arbitrage_bot=info")
        .init();
    
    // Load config from environment
    dotenv::dotenv().ok();
    
    let config = Config {
        execute_trades: std::env::var("EXECUTE_TRADES")
            .unwrap_or_else(|_| "false".to_string()) == "true",
        use_flash_loans: std::env::var("USE_FLASH_LOANS")
            .unwrap_or_else(|_| "true".to_string()) == "true",
        min_profit_usd: std::env::var("MIN_PROFIT_USD")
            .unwrap_or_else(|_| "1.0".to_string())
            .parse()?,
        ..Default::default()
    };
    
    let bot = ArbitrageBot::new(config).await?;
    bot.run().await?;
    
    Ok(())
}