use anyhow::Result;
use tracing::{info, error, warn};
use tracing_subscriber;
use tokio::signal;
use std::sync::Arc;

use defi_scanner::{
    OpportunityDetector,
    PoolMonitor,
    config::ScannerConfig,
    HuffGasEstimator,
    TextDashboard,
    PriceCalculator,
};
use ethers::types::Address;
use exchange_collector::{
    token_registry::TokenRegistry,
    pool_registry::PoolRegistry,
};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    info!("Starting AlphaPulse DeFi Scanner with realistic gas estimation...");
    info!("📊 Using comprehensive gas testing results:");
    info!("   - Simple V2 Arbitrage: 345,200 gas (~$0.008 at 30 gwei)");
    info!("   - Complex V3 Arbitrage: 415,200 gas (~$0.010 at 30 gwei)");  
    info!("   - Multi-hop Arbitrage: 478,100 gas (~$0.011 at 30 gwei)");
    info!("   - These values include full flash loan + swap + repayment costs");

    // Load configuration
    let config = ScannerConfig::from_env()?;
    info!("Loaded configuration for {} exchanges", config.exchanges.len());

    // Initialize registries for token and pool management
    let rpc_url = std::env::var("POLYGON_RPC_URL")
        .unwrap_or_else(|_| "https://polygon-mainnet.g.alchemy.com/v2/demo".to_string());
    
    info!("🔧 Initializing TokenRegistry with RPC: {}", rpc_url);
    let token_registry = Arc::new(TokenRegistry::new(rpc_url));
    
    info!("🔧 Initializing PoolRegistry");
    let pool_registry = Arc::new(PoolRegistry::new());
    
    // Preload common tokens to improve performance
    info!("⏳ Preloading common tokens...");
    token_registry.preload_common_tokens().await;
    let (token_count, _) = token_registry.cache_stats();
    info!("✅ TokenRegistry initialized with {} cached tokens", token_count);

    // Initialize components with realistic gas estimation and registries
    let pool_monitor = PoolMonitor::new(&config, token_registry.clone(), pool_registry.clone()).await?;
    
    // Initialize HuffGasEstimator with realistic values if contract address is configured
    let opportunity_detector = if let Ok(huff_contract) = std::env::var("HUFF_CONTRACT_ADDRESS") {
        let contract_address: Address = huff_contract.parse()
            .map_err(|e| anyhow::anyhow!("Invalid HUFF_CONTRACT_ADDRESS: {}", e))?;
        let bot_address: Address = std::env::var("BOT_ADDRESS")
            .unwrap_or_else(|_| "0x742d35Cc6634C0532925a3b8D9B5b7C3B5F6c8f7".to_string())
            .parse()
            .map_err(|e| anyhow::anyhow!("Invalid BOT_ADDRESS: {}", e))?;
            
        info!("Using HuffGasEstimator with contract: {:?}", contract_address);
        OpportunityDetector::with_huff_estimator(
            &config,
            pool_monitor.clone(),
            contract_address,
            bot_address,
        ).await?
    } else {
        warn!("HUFF_CONTRACT_ADDRESS not set, using fallback gas estimation");
        OpportunityDetector::new(&config, pool_monitor.clone()).await?
    };

    // Connect the opportunity detector's scan sender to the pool monitor
    let scan_sender = opportunity_detector.get_scan_sender();
    pool_monitor.set_scan_sender(scan_sender);

    // Initialize the SINGLE socket reader ONCE
    pool_monitor.initialize_socket_reader().await?;

    // Check if live dashboard should be enabled
    let enable_dashboard = std::env::var("ENABLE_DASHBOARD")
        .map(|v| v.to_lowercase() == "true" || v == "1")
        .unwrap_or(false);

    // Initialize text dashboard if requested
    let dashboard_handle = if enable_dashboard {
        info!("🖥️ Starting text dashboard...");
        
        // Get dashboard receiver from opportunity detector
        let dashboard_receiver = opportunity_detector.subscribe_dashboard_updates();
        let dashboard = TextDashboard::new(dashboard_receiver);
        
        Some(tokio::spawn(async move {
            let mut dashboard = dashboard;
            if let Err(e) = dashboard.start().await {
                error!("Text dashboard failed: {}", e);
            }
        }))
    } else {
        info!("💻 Running in console mode (set ENABLE_DASHBOARD=true for text dashboard)");
        None
    };

    // Start services
    let pool_handle = tokio::spawn(async move {
        if let Err(e) = pool_monitor.start().await {
            error!("Pool monitor failed: {}", e);
        }
    });

    let detector_handle = tokio::spawn(async move {
        let mut detector = opportunity_detector;
        if let Err(e) = detector.start().await {
            error!("Opportunity detector failed: {}", e);
        }
    });

    info!("DeFi Scanner started successfully");
    info!("Monitoring {} DEXs for arbitrage opportunities", config.exchanges.len());

    // Wait for shutdown signal
    signal::ctrl_c().await?;
    info!("Shutdown signal received, stopping services...");

    // Graceful shutdown
    pool_handle.abort();
    detector_handle.abort();
    if let Some(handle) = dashboard_handle {
        handle.abort();
    }

    info!("DeFi Scanner stopped");
    Ok(())
}