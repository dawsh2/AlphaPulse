use anyhow::Result;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use tokio::time::{interval, Duration};
use tokio::sync::{mpsc, broadcast};
use tracing::{info, debug, warn, error};
use parking_lot::RwLock;
use dashmap::DashMap;
use rust_decimal::{Decimal, prelude::ToPrimitive};
use rust_decimal_macros::dec;

use crate::{
    ArbitrageOpportunity,
    PoolMonitor,
    PoolInfo,
    PriceCalculator,
    config::ScannerConfig,
    DashboardUpdate,
};

/// Events that trigger targeted arbitrage scanning
#[derive(Debug, Clone)]
pub enum ScanTrigger {
    SwapEvent { pool_hash: u64, token0_hash: u64, token1_hash: u64 },
    PoolUpdate { pool_hash: u64 },
}

/// Detects arbitrage opportunities across monitored DEXs
pub struct OpportunityDetector {
    config: ScannerConfig,
    pool_monitor: Arc<PoolMonitor>,
    price_calculator: Arc<PriceCalculator>,
    opportunities: Arc<DashMap<String, ArbitrageOpportunity>>,
    relay_sender: tokio::sync::mpsc::UnboundedSender<Vec<u8>>,
    last_scan_time: Arc<AtomicU64>,
    scan_receiver: Option<mpsc::UnboundedReceiver<ScanTrigger>>,
    scan_sender: mpsc::UnboundedSender<ScanTrigger>,
    
    // Dashboard event broadcasting
    dashboard_sender: broadcast::Sender<DashboardUpdate>,
    token_symbol_cache: Arc<RwLock<std::collections::HashMap<String, String>>>,
}

impl OpportunityDetector {
    pub async fn new(
        config: &ScannerConfig,
        pool_monitor: Arc<PoolMonitor>,
    ) -> Result<Self> {
        let price_calculator = Arc::new(PriceCalculator::new(config));
        let opportunities = Arc::new(DashMap::new());

        // Connect to SignalRelay via Unix socket for sending arbitrage opportunities
        let (relay_sender, relay_receiver) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();
        
        // Start signal relay writer task
        tokio::spawn(async move {
            Self::signal_relay_writer(relay_receiver).await;
        });

        // Create scan trigger channel
        let (scan_sender, scan_receiver) = mpsc::unbounded_channel();

        // Create dashboard event broadcast channel
        let (dashboard_sender, _) = broadcast::channel(1000);

        Ok(Self {
            config: config.clone(),
            pool_monitor,
            price_calculator,
            opportunities,
            relay_sender,
            last_scan_time: Arc::new(AtomicU64::new(0)),
            scan_receiver: Some(scan_receiver),
            scan_sender,
            dashboard_sender,
            token_symbol_cache: Arc::new(RwLock::new(std::collections::HashMap::new())),
        })
    }

    /// Create OpportunityDetector with Huff gas estimation enabled
    pub async fn with_huff_estimator(
        config: &ScannerConfig,
        pool_monitor: Arc<PoolMonitor>,
        huff_contract_address: ethers::types::Address,
        bot_address: ethers::types::Address,
    ) -> Result<Self> {
        let price_calculator = Arc::new(PriceCalculator::with_huff_estimator(
            config,
            huff_contract_address,
            bot_address,
        )?);
        let opportunities = Arc::new(DashMap::new());

        // Connect to SignalRelay via Unix socket for sending arbitrage opportunities
        let (relay_sender, relay_receiver) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();
        
        // Start signal relay writer task
        tokio::spawn(async move {
            Self::signal_relay_writer(relay_receiver).await;
        });

        // Create scan trigger channel
        let (scan_sender, scan_receiver) = mpsc::unbounded_channel();

        // Create dashboard event broadcast channel
        let (dashboard_sender, _) = broadcast::channel(1000);

        Ok(Self {
            config: config.clone(),
            pool_monitor,
            price_calculator,
            opportunities,
            relay_sender,
            last_scan_time: Arc::new(AtomicU64::new(0)),
            scan_receiver: Some(scan_receiver),
            scan_sender,
            dashboard_sender,
            token_symbol_cache: Arc::new(RwLock::new(std::collections::HashMap::new())),
        })
    }

    /// Get the scan sender for external components to trigger targeted scans
    pub fn get_scan_sender(&self) -> mpsc::UnboundedSender<ScanTrigger> {
        self.scan_sender.clone()
    }

    /// Get a dashboard update receiver for consuming opportunity/pool events
    pub fn subscribe_dashboard_updates(&self) -> broadcast::Receiver<DashboardUpdate> {
        self.dashboard_sender.subscribe()
    }

    /// Resolve token symbol from address, using cache first
    async fn resolve_token_symbol(&self, address: &str) -> String {
        // Check cache first
        {
            let cache = self.token_symbol_cache.read();
            if let Some(symbol) = cache.get(address) {
                return symbol.clone();
            }
        }

        // Try to resolve using token registry (bijective ID system)
        let symbol = match self.pool_monitor.token_registry().get_token_info(address).await {
            Ok(token_info) => token_info.symbol,
            Err(_) => "UNKNOWN".to_string(),
        };
        
        if symbol != "UNKNOWN" {
            // Cache the resolved symbol
            self.token_symbol_cache.write().insert(address.to_string(), symbol.clone());
            
            // Emit dashboard event for resolved symbol
            if let Err(e) = self.dashboard_sender.send(DashboardUpdate::TokenSymbolResolved {
                address: address.to_string(),
                symbol: symbol.clone(),
            }) {
                debug!("Failed to send token symbol update: {}", e);
            }
            
            symbol
        } else {
            // Return truncated address as fallback
            if address.len() >= 10 {
                format!("{}...{}", &address[0..6], &address[address.len()-4..])
            } else {
                address.to_string()
            }
        }
    }

    pub async fn start(&mut self) -> Result<()> {
        info!("Starting purely event-driven opportunity detection...");
        info!("🎯 Only targeted scans when SwapEvents/PoolUpdates occur - no continuous polling!");

        // Log current state
        info!("📊 OpportunityDetector initialized with:");
        info!("   - Pool monitor: connected");
        info!("   - Scan receiver: {:?}", self.scan_receiver.is_some());
        info!("   - Dashboard sender: connected");
        
        // Take ownership of the receiver
        let mut scan_receiver = self.scan_receiver.take()
            .ok_or_else(|| anyhow::anyhow!("Scan receiver already taken"))?;
        
        info!("✅ Scan receiver obtained, ready to process triggers");

        // Periodic cleanup timer
        let mut cleanup_interval = interval(Duration::from_secs(30));

        loop {
            tokio::select! {
                // Handle targeted scan triggers
                Some(trigger) = scan_receiver.recv() => {
                    info!("📨 Received scan trigger: {:?}", trigger);
                    match trigger {
                        ScanTrigger::SwapEvent { pool_hash, token0_hash, token1_hash } => {
                            info!("🔥 SwapEvent trigger - scanning pool {:#x} token pair {:#x}/{:#x}", 
                                   pool_hash, token0_hash, token1_hash);
                            debug!("🎯 About to call scan_token_pair_arbitrage...");
                            let start = std::time::Instant::now();
                            if let Err(e) = self.scan_token_pair_arbitrage(pool_hash, token0_hash, token1_hash).await {
                                error!("Error in token pair scan: {}", e);
                            } else {
                                debug!("✅ scan_token_pair_arbitrage completed successfully in {:?}", start.elapsed());
                            }
                        }
                        ScanTrigger::PoolUpdate { pool_hash } => {
                            info!("🔥 PoolUpdate trigger - scanning affected opportunities for pool {:#x}", 
                                   pool_hash);
                            debug!("🎯 About to call scan_pool_affected_arbitrage...");
                            let start = std::time::Instant::now();
                            if let Err(e) = self.scan_pool_affected_arbitrage(pool_hash).await {
                                error!("Error in pool-affected scan: {}", e);
                            } else {
                                debug!("✅ scan_pool_affected_arbitrage completed successfully in {:?}", start.elapsed());
                            }
                        }
                    }
                }
                
                // Periodic cleanup of expired opportunities
                _ = cleanup_interval.tick() => {
                    self.cleanup_expired_opportunities();
                }
            }
        }
    }

    /// Scan arbitrage opportunities for a specific token pair (triggered by SwapEvent)
    pub async fn scan_token_pair_arbitrage(&self, pool_hash: u64, token0_hash: u64, token1_hash: u64) -> Result<()> {
        debug!("🎯 EXECUTING scan_token_pair_arbitrage for pool {:#x} token pair: {:#x}/{:#x}", pool_hash, token0_hash, token1_hash);
        
        // Get all pools from PoolMonitor (where live pool data with reserves is stored)
        let pools = self.pool_monitor.get_all_pools().await;
        
        if pools.is_empty() {
            debug!("❌ No pools available for targeted scan - pool collection is empty!");
            return Ok(());
        }
        
        info!("🎯 Triggered scan processing {} pools for potential arbitrage", pools.len());
        debug!("📊 Pool addresses: {:?}", pools.iter().map(|p| &p.address).collect::<Vec<_>>());
        
        // Group pools by token pair and emit PoolGroupUpdate for each group
        let mut symbol_pools: std::collections::HashMap<String, Vec<crate::PoolInfo>> = 
            std::collections::HashMap::new();

        for pool in &pools {
            // Group by token pair hash for same tokens across different DEXs  
            let symbol_hash = format!("{}_{}", pool.token0, pool.token1);
            symbol_pools.entry(symbol_hash).or_default().push(pool.clone());
        }

        // Process each token pair group for arbitrage opportunities
        for (symbol_hash, pools_for_symbol) in symbol_pools {
            self.detect_symbol_arbitrage(&symbol_hash, &pools_for_symbol).await?;
        }

        // Update last scan time
        let now_ns = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        self.last_scan_time.store(now_ns, Ordering::Relaxed);

        Ok(())
    }

    /// Scan arbitrage opportunities affected by a specific pool update (triggered by PoolUpdate)
    pub async fn scan_pool_affected_arbitrage(&self, pool_hash: u64) -> Result<()> {
        debug!("🎯 EXECUTING scan_pool_affected_arbitrage for pool: {:#x}", pool_hash);
        
        // Get all pools from PoolMonitor (where live pool data with reserves is stored)
        let pools = self.pool_monitor.get_all_pools().await;
        
        if pools.is_empty() {
            debug!("❌ No pools available for pool-affected scan - pool collection is empty!");
            return Ok(());
        }
        
        info!("🎯 Pool-affected scan processing {} pools", pools.len());
        debug!("📊 Pool addresses: {:?}", pools.iter().map(|p| &p.address).collect::<Vec<_>>());
        
        // Group pools by token pair and emit PoolGroupUpdate for each group
        let mut symbol_pools: std::collections::HashMap<String, Vec<crate::PoolInfo>> = 
            std::collections::HashMap::new();

        for pool in &pools {
            // Group by token pair hash for same tokens across different DEXs  
            let symbol_hash = format!("{}_{}", pool.token0, pool.token1);
            symbol_pools.entry(symbol_hash).or_default().push(pool.clone());
        }

        // Process each token pair group for arbitrage opportunities
        for (symbol_hash, pools_for_symbol) in symbol_pools {
            self.detect_symbol_arbitrage(&symbol_hash, &pools_for_symbol).await?;
        }

        Ok(())
    }

    async fn scan_for_opportunities(&self) -> Result<()> {
        // Get all pools from PoolMonitor (where live pool data with reserves is stored)
        let pools = self.pool_monitor.get_all_pools().await;
        
        if pools.is_empty() {
            debug!("No pools available for scanning - check relay data flow");
            return Ok(());
        }
        
        info!("Scanning {} pools for arbitrage opportunities", pools.len());

        // Group pools by their symbol hash (same token pair)
        let mut symbol_pools: std::collections::HashMap<String, Vec<PoolInfo>> = 
            std::collections::HashMap::new();

        for pool in &pools {
            // Group by token pair hash for same tokens across different DEXs
            let symbol_hash = format!("{}_{}", pool.token0, pool.token1);
            symbol_pools.entry(symbol_hash).or_default().push(pool.clone());
        }

        // Look for arbitrage opportunities within each token pair
        for (symbol_hash, pools_for_symbol) in symbol_pools {
            if pools_for_symbol.len() < 2 {
                continue; // Need at least 2 different prices for arbitrage
            }
            
            self.detect_symbol_arbitrage(&symbol_hash, &pools_for_symbol).await?;
        }

        debug!("Scanned {} pools, found {} active opportunities", 
               pools.len(), self.opportunities.len());
        Ok(())
    }
    
    /// Display live price dashboard for all active pools and detect arbitrage opportunities
    async fn detect_symbol_arbitrage(
        &self,
        symbol_hash: &str, 
        pools: &[PoolInfo]
    ) -> Result<()> {
        // Always show price dashboard for monitoring, even with single pools
        self.display_price_dashboard(symbol_hash, pools).await?;
        
        // Emit pool group update for dashboard
        if !pools.is_empty() {
            let token0_symbol = self.resolve_token_symbol(&pools[0].token0).await;
            let token1_symbol = self.resolve_token_symbol(&pools[0].token1).await;
            
            // Calculate price range and spread
            let mut min_price = Decimal::MAX;
            let mut max_price = Decimal::ZERO;
            
            for pool in pools {
                if pool.reserve0 > Decimal::ZERO && pool.reserve1 > Decimal::ZERO {
                    let price = pool.reserve1 / pool.reserve0;
                    min_price = min_price.min(price);
                    max_price = max_price.max(price);
                }
            }
            
            let max_spread_percent = if min_price > Decimal::ZERO && max_price > min_price {
                ((max_price - min_price) / min_price) * dec!(100)
            } else {
                Decimal::ZERO
            };
            
            // Emit pool group update
            if let Err(e) = self.dashboard_sender.send(DashboardUpdate::PoolGroupUpdate {
                token_pair: format!("{}/{}", token0_symbol, token1_symbol),
                token0_symbol,
                token1_symbol,
                pools: pools.to_vec(),
                price_range: (min_price, max_price),
                max_spread_percent,
                total_liquidity_usd: None, // Could calculate if needed
                best_opportunity: None, // Will be set when opportunity is found
            }) {
                debug!("Failed to send pool group update: {}", e);
            }
        }
        
        if pools.len() < 2 {
            return Ok(());
        }
        
        // Find min and max prices using Decimal for precision
        let mut min_price_pool: Option<&PoolInfo> = None;
        let mut max_price_pool: Option<&PoolInfo> = None;
        let mut min_price = rust_decimal::Decimal::MAX;
        let mut max_price = rust_decimal::Decimal::ZERO;
        
        for pool in pools {
            // Skip pools with zero or invalid reserves
            if pool.reserve0 <= rust_decimal::Decimal::ZERO || pool.reserve1 <= rust_decimal::Decimal::ZERO {
                continue;
            }
            
            // Calculate price as reserve1/reserve0 (TOKEN1/TOKEN0) using Decimal arithmetic
            let price = pool.reserve1 / pool.reserve0;
            
            // Validate price is reasonable (between $0.0001 and $100,000)
            if price < rust_decimal::Decimal::new(1, 4) || price > rust_decimal::Decimal::new(100000, 0) {
                debug!("Skipping pool {} with unrealistic price: {}", pool.address, price);
                continue;
            }
            
            // Also skip if reserves are too small (likely fake data)
            if pool.reserve0 < rust_decimal::Decimal::new(100, 0) || 
               pool.reserve1 < rust_decimal::Decimal::new(100, 0) {
                debug!("Skipping pool {} with too small reserves: {}/{}", 
                       pool.address, pool.reserve0, pool.reserve1);
                continue;
            }
            
            if price < min_price {
                min_price = price;
                min_price_pool = Some(pool);
            }
            
            if price > max_price {
                max_price = price;
                max_price_pool = Some(pool);
            }
        }
        
        let min_pool = match min_price_pool { Some(p) => p, None => return Ok(()) };
        let max_pool = match max_price_pool { Some(p) => p, None => return Ok(()) };
        
        if min_price >= max_price {
            return Ok(()); // No price difference
        }
        
        // Calculate percentage difference using Decimal arithmetic
        let price_diff = max_price - min_price;
        let price_diff_pct = (price_diff / min_price) * rust_decimal::Decimal::new(100, 0);
        
        // Always display pricing info for monitoring, even if not profitable
        let gas_cost_preview = self.price_calculator.estimate_gas_cost(
            &min_pool.exchange,
            false,
            None,
            Some((&min_pool.token0, &min_pool.token1)),
            Some(rust_decimal::Decimal::new(10000, 2)), // $100 test amount
        ).await;
        
        // REMOVED: No minimum spread requirement - take any profitable trade
        // All trades evaluated for profitability after gas costs
        
        // Skip if price difference is unrealistically high (>50%)
        if price_diff_pct > rust_decimal::Decimal::new(50, 0) {
            println!("   ❌ Spread too large (max: 50%) - likely stale data\n");
            return Ok(());
        }
        
        // Calculate OPTIMAL trade size using closed-form AMM mathematics
        // This gives us the exact profit-maximizing amount, not just a test value
        let optimal_amount = if min_pool.reserve0 > rust_decimal::Decimal::ZERO && 
                                min_pool.reserve1 > rust_decimal::Decimal::ZERO &&
                                max_pool.reserve0 > rust_decimal::Decimal::ZERO && 
                                max_pool.reserve1 > rust_decimal::Decimal::ZERO {
            // Use closed-form solution for optimal V2 arbitrage
            // Buy from min_price pool, sell to max_price pool
            let min_fee_bps = (min_pool.fee * rust_decimal::Decimal::new(10000, 0)).to_u32().unwrap_or(30);
            let max_fee_bps = (max_pool.fee * rust_decimal::Decimal::new(10000, 0)).to_u32().unwrap_or(30);
            
            crate::amm_math::AmmMath::calculate_optimal_v2_arbitrage(
                min_pool.reserve0,  // reserve_in for buy pool
                min_pool.reserve1,  // reserve_out for buy pool
                min_fee_bps,
                max_pool.reserve1,  // reserve_in for sell pool (swapped)
                max_pool.reserve0,  // reserve_out for sell pool (swapped)
                max_fee_bps,
            ).unwrap_or(rust_decimal::Decimal::new(10000, 2)) // Fallback to $100 if calculation fails
        } else {
            rust_decimal::Decimal::new(10000, 2) // $100 fallback for missing reserves
        };
        
        let gas_cost_usd = self.price_calculator.estimate_gas_cost(
            &min_pool.exchange,
            false,
            None,
            Some((&min_pool.token0, &min_pool.token1)),
            Some(optimal_amount),
        ).await;
        
        // Calculate actual profit with OPTIMAL amount
        let tokens_bought = optimal_amount / min_price;
        let usd_received = tokens_bought * max_price;
        let gross_profit = usd_received - optimal_amount;
        let net_profit = gross_profit - gas_cost_usd;
        
        // Create opportunity for any positive price difference (zero minimum as requested)
        if gross_profit > rust_decimal::Decimal::ZERO {
            let opportunity = ArbitrageOpportunity {
                id: format!("arb_{}_{}", symbol_hash, chrono::Utc::now().timestamp_millis()),
                token_in: min_pool.token0.clone(),
                token_out: min_pool.token1.clone(),
                amount_in: optimal_amount,  // Use OPTIMAL amount, not test amount
                amount_out: usd_received,
                profit_usd: gross_profit,
                profit_percentage: price_diff_pct,
                buy_exchange: min_pool.exchange.clone(),
                sell_exchange: max_pool.exchange.clone(),
                buy_pool: min_pool.address.clone(),
                sell_pool: max_pool.address.clone(),
                gas_cost_estimate: gas_cost_usd,
                net_profit_usd: net_profit,
                timestamp: chrono::Utc::now().timestamp(),
                block_number: min_pool.block_number,
                confidence_score: 0.95, // High confidence for same-pair arbitrage
            };
            
            // Print opportunity to terminal (only if net profit is positive and reasonable)
            if net_profit > rust_decimal::Decimal::ZERO && net_profit < rust_decimal::Decimal::new(10000, 0) {
                println!("   ✅ PROFITABLE! Net: ${:.2}", net_profit.round_dp(2));
                println!("🚀 ARBITRAGE OPPORTUNITY DETECTED!");
                println!("   Token Pair: {} / {}", min_pool.token0, min_pool.token1);
                println!("   Buy at: ${} from {}", min_price.round_dp(6), min_pool.exchange);
                println!("   Sell at: ${} to {}", max_price.round_dp(6), max_pool.exchange);
                println!("   Spread: {}%", price_diff_pct.round_dp(2));
                println!("   Optimal Amount: ${}", optimal_amount.round_dp(2));
                println!("   Gross Profit: ${}", gross_profit.round_dp(2));
                println!("   Gas Cost: ${}", gas_cost_usd.round_dp(4));
                println!("   Net Profit: ${}", net_profit.round_dp(2));
                println!("───────────────────────────────────────");
            } else if gross_profit > rust_decimal::Decimal::ZERO {
                println!("   ⚠️  Gross profit: ${:.2} | Net loss: -${:.2} (gas too high)\n", 
                         gross_profit.round_dp(2), (gas_cost_usd - gross_profit).round_dp(2));
            } else {
                println!("   ❌ No arbitrage profit possible\n");
            }
            
            // Store opportunity
            self.opportunities.insert(opportunity.id.clone(), opportunity.clone());
            
            // Emit dashboard event for new opportunity
            if let Err(e) = self.dashboard_sender.send(DashboardUpdate::NewOpportunity(opportunity.clone())) {
                debug!("Failed to send dashboard update: {}", e);
            }
            
            // Send binary ArbitrageOpportunityMessage to relay for dashboard consumption
            // Send to SignalRelay
            if let Ok(binary_msg) = opportunity.to_binary_message() {
                if let Err(e) = self.relay_sender.send(binary_msg) {
                    warn!("Failed to broadcast arbitrage opportunity to signal relay: {}", e);
                    } else {
                        debug!("✅ Sent ArbitrageOpportunityMessage to relay: {} profit ${:.2}", 
                               opportunity.id, opportunity.net_profit_usd);
                    }
                } else {
                    warn!("Failed to serialize arbitrage opportunity to binary message");
                }
            } else {
                debug!("No relay sender configured - opportunity not sent to dashboard");
            }
        }
        
        Ok(())
    }
    
    /// DEPRECATED: Use price_calculator.estimate_gas_cost() instead for real Huff contract estimation
    #[allow(dead_code)]
    async fn calculate_huff_gas_cost(&self) -> rust_decimal::Decimal {
        // This function is deprecated - use PriceCalculator::estimate_gas_cost() for real Huff estimation
        self.price_calculator.estimate_gas_cost(
            "uniswap_v2",
            false,
            None,
            None,
            Some(Decimal::new(1000, 0)), // $1000 default amount
        ).await
    }

    async fn check_pair_arbitrage(
        &self,
        token_in: &str,
        token_out: &str,
        pools: &[&crate::PoolInfo],
    ) -> Result<()> {
        // Use dynamic test amount based on pool liquidity, not hardcoded $1000
        let test_amount = self.calculate_optimal_test_amount(pools).await?;

        // Get quotes from all pools
        let mut quotes = Vec::new();
        for pool in pools {
            if let Ok(quote) = self.price_calculator.get_quote(
                pool, token_in, token_out, test_amount
            ).await {
                quotes.push((pool, quote));
            }
        }

        if quotes.len() < 2 {
            return Ok(());
        }

        // Sort quotes by price (best buy price = lowest, best sell price = highest)
        quotes.sort_by(|a, b| a.1.price.partial_cmp(&b.1.price).unwrap());

        let best_buy = &quotes[0];
        let best_sell = &quotes[quotes.len() - 1];

        // Calculate potential profit
        let price_diff = best_sell.1.price - best_buy.1.price;
        let profit_percentage = (price_diff / best_buy.1.price) * Decimal::new(100, 0);

        if profit_percentage < self.config.arbitrage.min_profit_percentage {
            return Ok(());
        }

        let profit_usd = price_diff * test_amount;
        
        // Get real-time gas price and network conditions
        let current_gas_price = self.get_current_gas_price().await.unwrap_or(Decimal::new(30, 0));
        
        // Use real Huff gas measurements with optimal contract selection
        let estimated_gas_cost = self.price_calculator.estimate_gas_cost(
            &best_buy.0.exchange,
            false, // Simple trade for now
            Some(current_gas_price),  // Use real gas price
            Some((token_in, token_out)),  // Pass token pair for optimal contract selection
            Some(test_amount),  // Pass trade amount for accurate Huff estimation
        ).await;
        
        let net_profit = profit_usd - estimated_gas_cost;

        if net_profit < self.config.arbitrage.min_profit_usd {
            return Ok(());
        }

        // Create opportunity
        let opportunity = ArbitrageOpportunity {
            id: format!("{}_{}_{}_{}", 
                       token_in, token_out, 
                       best_buy.0.exchange, best_sell.0.exchange),
            token_in: token_in.to_string(),
            token_out: token_out.to_string(),
            amount_in: test_amount,
            amount_out: best_sell.1.amount_out,
            profit_usd,
            profit_percentage,
            buy_exchange: best_buy.0.exchange.clone(),
            sell_exchange: best_sell.0.exchange.clone(),
            buy_pool: best_buy.0.address.clone(),
            sell_pool: best_sell.0.address.clone(),
            gas_cost_estimate: estimated_gas_cost,
            net_profit_usd: net_profit,
            timestamp: chrono::Utc::now().timestamp(),
            block_number: self.get_current_block_number().await.unwrap_or(0), // Real block number
            confidence_score: self.calculate_confidence_score(&best_buy.1, &best_sell.1),
        };

        if opportunity.confidence_score < self.config.arbitrage.confidence_threshold {
            debug!("Opportunity {} rejected due to low confidence: {}", 
                   opportunity.id, opportunity.confidence_score);
            return Ok(());
        }

        // Calculate optimal trade size using closed-form solution
        let optimal_size = self.calculate_optimal_trade_size(&opportunity, &best_buy.1, &best_sell.1).await;
        
        println!("🚀 ARBITRAGE OPPORTUNITY DETECTED!");
        println!("═══════════════════════════════════════════════════════════════");
        println!("📊 PAIR: {} → {}", opportunity.token_in, opportunity.token_out);
        println!("───────────────────────────────────────────────────────────────");
        println!("💱 PRICES:");
        println!("   Buy:  {} @ ${:.6}", opportunity.buy_exchange, best_buy.1.price);
        println!("   Sell: {} @ ${:.6}", opportunity.sell_exchange, best_sell.1.price);
        println!("   Spread: {:.3}%", opportunity.profit_percentage);
        println!("───────────────────────────────────────────────────────────────");
        println!("📈 TRADE SIZING:");
        println!("   Optimal Size: ${:.2}", optimal_size);
        println!("   Buy Slippage:  {:.3}% @ ${:.2}", best_buy.1.slippage * dec!(100), opportunity.amount_in);
        println!("   Sell Slippage: {:.3}% @ ${:.2}", best_sell.1.slippage * dec!(100), opportunity.amount_out);
        println!("   Total Impact: {:.3}%", (best_buy.1.slippage + best_sell.1.slippage) * dec!(100));
        println!("───────────────────────────────────────────────────────────────");
        println!("💰 PROFITABILITY:");
        println!("   Gross Profit: ${:.2} ({:.2}%)", opportunity.profit_usd, opportunity.profit_percentage);
        println!("   Gas Cost:     ${:.2} (Huff optimized)", opportunity.gas_cost_estimate);
        println!("   Net Profit:   ${:.2}", opportunity.net_profit_usd);
        println!("───────────────────────────────────────────────────────────────");
        println!("🎯 EXECUTION:");
        println!("   Confidence: {:.1}%", opportunity.confidence_score * 100.0);
        println!("   Block: #{}", opportunity.block_number);
        println!("   Pools: {} → {}", &opportunity.buy_pool[..8], &opportunity.sell_pool[..8]);
        println!("═══════════════════════════════════════════════════════════════");
        
        info!("Found arbitrage opportunity: {} -> {} profit: ${:.2} ({:.2}%)",
              opportunity.buy_exchange, opportunity.sell_exchange,
              opportunity.net_profit_usd, opportunity.profit_percentage);

        // Store opportunity
        self.opportunities.insert(opportunity.id.clone(), opportunity.clone());

        // Broadcast opportunity to existing relay server
        // Convert to binary message and send to signal relay
        if let Ok(binary_msg) = opportunity.to_binary_message() {
            if let Err(e) = self.relay_sender.send(binary_msg) {
                    warn!("Failed to broadcast opportunity to relay: {}", e);
                }
            }
        }

        Ok(())
    }

    async fn check_cross_token_arbitrage(
        &self,
        token_in: &str,
        token_out: &str,
        all_pools: &[crate::PoolInfo],
    ) -> Result<()> {
        // Use dynamic test amount based on available pools
        let test_amount = self.calculate_optimal_test_amount(&all_pools.iter().collect::<Vec<_>>()).await?;
        
        // Get cross-token opportunities from price calculator
        let cross_tokens = self.price_calculator.detect_cross_token_opportunities(token_in, test_amount);
        
        if cross_tokens.is_empty() {
            return Ok(());
        }
        
        // For each cross-token, look for triangular arbitrage: token_in -> cross_token -> token_out
        for cross_token in cross_tokens {
            if let Some(opportunity) = self.check_triangular_path(
                token_in, &cross_token, token_out, test_amount, all_pools
            ).await? {
                info!("Found cross-token arbitrage: {} -> {} -> {} profit: ${:.2}",
                      token_in, cross_token, token_out, opportunity.net_profit_usd);
                
                // Store opportunity
                self.opportunities.insert(opportunity.id.clone(), opportunity.clone());
                
                // Broadcast to relay
                if let Ok(binary_msg) = opportunity.to_binary_message() {
                    if let Err(e) = self.relay_sender.send(binary_msg) {
                            warn!("Failed to broadcast cross-token opportunity to relay: {}", e);
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
    
    async fn check_triangular_path(
        &self,
        token_a: &str,
        token_b: &str,
        token_c: &str,
        amount_in: Decimal,
        all_pools: &[crate::PoolInfo],
    ) -> Result<Option<ArbitrageOpportunity>> {
        // Find pools for each leg of the triangular arbitrage
        let pool_ab = all_pools.iter().find(|p| 
            (p.token0 == token_a && p.token1 == token_b) || 
            (p.token0 == token_b && p.token1 == token_a)
        );
        
        let pool_bc = all_pools.iter().find(|p| 
            (p.token0 == token_b && p.token1 == token_c) || 
            (p.token0 == token_c && p.token1 == token_b)
        );
        
        if pool_ab.is_none() || pool_bc.is_none() {
            return Ok(None);
        }
        
        let pool_ab = pool_ab.unwrap();
        let pool_bc = pool_bc.unwrap();
        
        // Calculate quotes for each leg
        let quote_ab = self.price_calculator.get_quote(pool_ab, token_a, token_b, amount_in).await?;
        let quote_bc = self.price_calculator.get_quote(pool_bc, token_b, token_c, quote_ab.amount_out).await?;
        
        // Calculate profit vs direct path
        let final_amount = quote_bc.amount_out;
        let profit_usd = final_amount - amount_in; // Simplified assuming USD-pegged tokens
        
        if profit_usd <= Decimal::ZERO {
            return Ok(None);
        }
        
        let profit_percentage = (profit_usd / amount_in) * Decimal::new(100, 0);
        
        if profit_percentage < self.config.arbitrage.min_profit_percentage {
            return Ok(None);
        }
        
        // Enhanced gas cost for complex triangular trade using real Huff measurements
        let estimated_gas_cost = self.price_calculator.estimate_gas_cost(
            &pool_ab.exchange,
            true, // Complex trade
            None,
            Some((token_a, token_c)), // Pass token pair for optimal contract selection
            Some(amount_in), // Pass trade amount for accurate Huff estimation
        ).await;
        
        let net_profit = profit_usd - estimated_gas_cost;
        
        if net_profit < self.config.arbitrage.min_profit_usd {
            return Ok(None);
        }
        
        let opportunity = ArbitrageOpportunity {
            id: format!("triangular_{}_{}_{}_{}", 
                       token_a, token_b, token_c, chrono::Utc::now().timestamp()),
            token_in: token_a.to_string(),
            token_out: token_c.to_string(),
            amount_in,
            amount_out: final_amount,
            profit_usd,
            profit_percentage,
            buy_exchange: pool_ab.exchange.clone(),
            sell_exchange: pool_bc.exchange.clone(),
            buy_pool: pool_ab.address.clone(),
            sell_pool: pool_bc.address.clone(),
            gas_cost_estimate: estimated_gas_cost,
            net_profit_usd: net_profit,
            timestamp: chrono::Utc::now().timestamp(),
            block_number: 0,
            confidence_score: self.calculate_confidence_score(&quote_ab, &quote_bc) * 0.8, // Lower confidence for complex trades
        };
        
        if opportunity.confidence_score < self.config.arbitrage.confidence_threshold {
            return Ok(None);
        }
        
        Ok(Some(opportunity))
    }

    async fn calculate_optimal_trade_size(
        &self,
        opportunity: &ArbitrageOpportunity,
        buy_quote: &crate::PriceQuote,
        sell_quote: &crate::PriceQuote,
    ) -> Decimal {
        // Use closed-form solution from our AMM math
        // This would normally call amm_math::calculate_optimal_v2_arbitrage
        // For now, return the amount we tested with
        opportunity.amount_in
    }
    
    fn calculate_confidence_score(
        &self,
        buy_quote: &crate::PriceQuote,
        sell_quote: &crate::PriceQuote,
    ) -> f64 {
        // Simple confidence scoring based on slippage and liquidity
        let max_slippage = buy_quote.slippage.max(sell_quote.slippage);
        let slippage_score = if max_slippage > Decimal::new(1, 2) { // >1%
            0.5
        } else {
            1.0
        };

        // TODO: Add liquidity-based scoring
        let liquidity_score = 0.8; // Placeholder

        slippage_score * liquidity_score
    }

    fn cleanup_expired_opportunities(&self) {
        let now = chrono::Utc::now().timestamp();
        let timeout = self.config.arbitrage.opportunity_timeout_ms as i64 / 1000;

        self.opportunities.retain(|_, opportunity| {
            now - opportunity.timestamp < timeout
        });
    }

    pub fn get_active_opportunities(&self) -> Vec<ArbitrageOpportunity> {
        self.opportunities.iter().map(|entry| entry.value().clone()).collect()
    }

    async fn connect_to_relay(socket_path: &str) -> Option<tokio::sync::mpsc::UnboundedSender<[u8; 64]>> {
        // TODO: Implement actual Unix socket connection to existing relay_server
        // For now, return None to disable relay broadcasting
        // This would connect to /tmp/alphapulse/relay.sock and send binary messages
        debug!("Would connect to relay at: {}", socket_path);
        None
    }

    /// Calculate optimal test amount based on pool liquidity and gas costs
    async fn calculate_optimal_test_amount(&self, pools: &[&crate::PoolInfo]) -> Result<Decimal> {
        if pools.is_empty() {
            return Ok(Decimal::new(100, 0)); // Fallback to $100
        }

        // Calculate average pool liquidity from real reserve data
        let mut total_liquidity = Decimal::ZERO;
        let mut valid_pools = 0;
        let mut min_liquidity = Decimal::new(1000000, 0); // Start high

        for pool in pools {
            // Calculate pool liquidity from actual reserves (not hardcoded)
            let reserve_product = pool.reserve0 * pool.reserve1;
            if reserve_product > Decimal::ZERO {
                // Geometric mean of reserves as proxy for liquidity
                // Approximate liquidity as geometric mean (reserve_product^0.5)
                // Using simple approximation since rust_decimal doesn't have sqrt
                let pool_liquidity = if reserve_product > Decimal::ZERO {
                    // Simple approximation: use average of reserves as liquidity indicator
                    (pool.reserve0 + pool.reserve1) / dec!(2)
                } else {
                    Decimal::ZERO
                };
                total_liquidity += pool_liquidity;
                min_liquidity = min_liquidity.min(pool_liquidity);
                valid_pools += 1;
            }
        }

        if valid_pools == 0 {
            return Ok(Decimal::new(100, 0)); // Fallback to $100
        }

        let avg_liquidity = total_liquidity / Decimal::new(valid_pools, 0);
        
        // Calculate minimum profitable amount based on gas costs (using Huff gas savings)
        let current_gas_price = self.get_current_gas_price().await.unwrap_or(Decimal::new(30, 0));
        let gas_cost = self.price_calculator.estimate_gas_cost(
            "uniswap_v2", // Default exchange for estimation
            false,
            Some(current_gas_price),
            None, // Use default contract selection
            Some(Decimal::new(1000, 0)), // $1000 test amount for gas estimation
        ).await;
        
        // Minimum amount should be 10x gas cost for reasonable profit margin
        let min_profitable = gas_cost * Decimal::new(10, 0);
        
        // Use 0.3% of average pool liquidity as test amount (lower than 0.5% for better accuracy)
        let liquidity_based_amount = avg_liquidity * Decimal::new(3, 3); // 0.3%
        
        // Also consider 1% of minimum pool liquidity to avoid high slippage in smaller pools
        let min_pool_based = min_liquidity * Decimal::new(1, 2); // 1%
        
        // Take the maximum of profitable minimum and liquidity constraints
        let candidate_amount = liquidity_based_amount
            .max(min_profitable)
            .max(Decimal::new(50, 0)) // Absolute minimum $50
            .min(min_pool_based) // Don't exceed 1% of smallest pool
            .min(Decimal::new(25000, 0)); // Practical maximum $25,000
        
        debug!("Dynamic test amount calculation:");
        debug!("  Avg liquidity: ${:.2}", avg_liquidity);
        debug!("  Min pool liquidity: ${:.2}", min_liquidity);
        debug!("  Gas cost: ${:.4}", gas_cost);
        debug!("  Min profitable: ${:.2}", min_profitable);
        debug!("  Final amount: ${:.2}", candidate_amount);
        
        Ok(candidate_amount)
    }
    
    /// Calculate trade size that maximizes profit after gas costs (using real Huff measurements)
    async fn calculate_gas_efficient_size(
        &self,
        pools: &[&crate::PoolInfo],
        token_in: &str,
        token_out: &str,
    ) -> Result<Decimal> {
        let base_amount = self.calculate_optimal_test_amount(pools).await?;
        
        // Test multiple sizes to find the most gas-efficient
        let test_sizes = vec![
            base_amount * Decimal::new(5, 1),   // 0.5x
            base_amount,                                       // 1.0x
            base_amount * Decimal::new(2, 0),   // 2.0x
            base_amount * Decimal::new(5, 0),   // 5.0x
        ];
        
        let mut best_efficiency = 0.0f64;
        let mut best_size = base_amount;
        
        for size in test_sizes {
            // Calculate efficiency: profit_ratio / gas_cost_ratio
            let gas_cost = self.price_calculator.estimate_gas_cost(
                "uniswap_v2",
                false,
                None,
                Some((token_in, token_out)),
                Some(size), // Pass actual trade size for accurate gas estimation
            ).await;
            
            // Efficiency metric: expected profit per dollar of gas cost
            let efficiency = (size.to_f64().unwrap_or(0.0) * 0.01) / // Assume 1% profit
                             gas_cost.to_f64().unwrap_or(1.0).max(0.0001);
            
            if efficiency > best_efficiency {
                best_efficiency = efficiency;
                best_size = size;
            }
        }
        
        debug!("Gas-efficient size calculation: ${:.2} (efficiency: {:.2})", 
               best_size, best_efficiency);
        
        Ok(best_size)
    }

    /// Get real-time gas price from Unix socket instead of hardcoded estimates
    async fn get_current_gas_price(&self) -> Result<Decimal> {
        // TODO: Read StatusUpdate messages from Unix socket to get real gas prices
        // The exchange_collector sends real gas prices from block headers
        
        // For now, return a reasonable default but this should be replaced with
        // actual Unix socket reading of StatusUpdate messages
        Ok(Decimal::new(30, 0)) // 30 gwei placeholder
    }

    /// Get real-time block number from Unix socket
    async fn get_current_block_number(&self) -> Result<u64> {
        // TODO: Read StatusUpdate messages from Unix socket for real block numbers
        // The exchange_collector provides real-time block numbers
        
        Ok(0) // Placeholder - should be real block number
    }

    /// Process real-time Sync events from Unix socket relay (from scripts/arb)
    pub async fn start_realtime_sync_processing(&self) -> Result<()> {
        info!("Starting real-time Sync event processing from relay...");
        
        let (sync_tx, mut sync_rx) = tokio::sync::mpsc::unbounded_channel::<PoolInfo>();
        
        // TODO: Connect to existing relay Unix socket to receive Sync events
        // The exchange_collector should be sending DEX Sync events through the relay
        // We should listen for specific message types related to pool updates
        
        // For now, create a mock receiver that would be replaced with actual Unix socket listener
        tokio::spawn(async move {
            // This would be replaced with actual Unix socket reading from relay
            debug!("Mock Sync event receiver started (replace with Unix socket listener)");
            
            // Example of how we'd process incoming Sync events:
            while let Some(updated_pool) = sync_rx.recv().await {
                debug!("Received pool update from Sync event: {}", updated_pool.address);
                // Update local pool cache with real-time data
                // Trigger immediate arbitrage scan for affected tokens
            }
        });
        
        info!("Real-time Sync event processing started");
        Ok(())
    }

    /// Process incoming pool update from Sync events (from scripts/arb analysis)
    fn process_sync_pool_update(&self, updated_pool: &PoolInfo) -> Result<()> {
        debug!("Processing Sync event for pool {}", updated_pool.address);
        
        // Update pool monitor with real-time data
        // This would trigger immediate opportunity detection for affected token pairs
        // Key insight from scripts/arb: Sync events provide the freshest data for arbitrage
        
        // Check if this pool update creates immediate arbitrage opportunities
        let affected_tokens = vec![updated_pool.token0.clone(), updated_pool.token1.clone()];
        
        // TODO: Trigger focused arbitrage scan for these specific tokens
        // This is more efficient than full scan and captures opportunities faster
        
        info!("Pool {} updated via Sync event - tokens: {}/{}", 
              updated_pool.address, updated_pool.token0, updated_pool.token1);
        
        Ok(())
    }

    /// Display live terminal dashboard showing all active token pairs and their prices
    async fn display_price_dashboard(&self, symbol_hash: &str, pools: &[PoolInfo]) -> Result<()> {
        use std::collections::HashMap;
        
        if pools.is_empty() {
            return Ok(());
        }

        // Group pools by token pair for dashboard display
        let mut pair_prices: Vec<(String, rust_decimal::Decimal, String, rust_decimal::Decimal, rust_decimal::Decimal)> = Vec::new();
        
        for pool in pools {
            // Skip pools with invalid reserves
            if pool.reserve0 <= rust_decimal::Decimal::ZERO || pool.reserve1 <= rust_decimal::Decimal::ZERO {
                continue;
            }
            
            // Calculate price as reserve1/reserve0 (TOKEN1/TOKEN0)
            let price = pool.reserve1 / pool.reserve0;
            
            // Validate reasonable price range
            if price < rust_decimal::Decimal::new(1, 6) || price > rust_decimal::Decimal::new(1000000, 0) {
                continue;
            }
            
            let pair_name = format!("{}/{}", 
                pool.token0.get(0..6).unwrap_or(&pool.token0), 
                pool.token1.get(0..6).unwrap_or(&pool.token1)
            );
            
            pair_prices.push((
                pair_name.clone(),
                price,
                pool.exchange.clone(),
                pool.reserve0,
                pool.reserve1
            ));
        }

        if pair_prices.is_empty() {
            return Ok(());
        }

        // Clear previous output and show dashboard header
        println!("\x1B[H\x1B[J"); // Clear screen
        println!("═══════════════════════════════════════════════════════════════");
        println!("🚀 AlphaPulse Live DeFi Price Dashboard");
        println!("═══════════════════════════════════════════════════════════════");
        println!();

        // Sort by price for better display
        pair_prices.sort_by(|a, b| a.1.cmp(&b.1));

        // Group by token pair name and show comparative analysis
        let mut current_pair = String::new();
        let mut pair_pools: Vec<&(String, rust_decimal::Decimal, String, rust_decimal::Decimal, rust_decimal::Decimal)> = Vec::new();

        for price_entry in &pair_prices {
            if price_entry.0 != current_pair {
                // Display previous pair's analysis
                if !pair_pools.is_empty() {
                    self.display_pair_analysis(&current_pair, &pair_pools).await?;
                    pair_pools.clear();
                }
                current_pair = price_entry.0.clone();
            }
            pair_pools.push(price_entry);
        }

        // Display final pair
        if !pair_pools.is_empty() {
            self.display_pair_analysis(&current_pair, &pair_pools).await?;
        }

        println!("═══════════════════════════════════════════════════════════════");
        println!("📊 Monitoring {} active token pairs", 
                 pair_prices.iter().map(|p| &p.0).collect::<std::collections::HashSet<_>>().len());
        println!();

        Ok(())
    }

    /// Display analysis for a specific token pair across multiple pools
    async fn display_pair_analysis(&self, pair_name: &str, pools: &[&(String, rust_decimal::Decimal, String, rust_decimal::Decimal, rust_decimal::Decimal)]) -> Result<()> {
        if pools.is_empty() {
            return Ok(());
        }

        println!("📈 {} - {} pool(s)", pair_name, pools.len());
        
        for (i, (_, price, exchange, reserve0, reserve1)) in pools.iter().enumerate() {
            let liquidity_usd = reserve0 * rust_decimal::Decimal::new(2, 0); // Rough estimate
            println!("   {}. {}: ${:.6} (Liq: ${:.0}K)", 
                     i + 1, exchange, price.round_dp(6), 
                     (liquidity_usd / rust_decimal::Decimal::new(1000, 0)).round_dp(0));
        }

        // Show spread analysis if multiple pools
        if pools.len() > 1 {
            let min_price = pools.iter().map(|(_, price, _, _, _)| price).min().unwrap();
            let max_price = pools.iter().map(|(_, price, _, _, _)| price).max().unwrap();
            let spread_pct = ((max_price - min_price) / min_price) * rust_decimal::Decimal::new(100, 0);
            
            // Estimate gas cost for this pair
            let gas_cost = self.price_calculator.estimate_gas_cost(
                &pools[0].2, // Use first exchange for estimation
                false,
                None,
                None,
                Some(rust_decimal::Decimal::new(10000, 2)), // $100 test
            ).await;
            
            let min_profitable_spread = (gas_cost / rust_decimal::Decimal::new(10000, 2)) * rust_decimal::Decimal::new(100, 0);
            
            if spread_pct > rust_decimal::Decimal::ZERO {
                let status = if spread_pct > min_profitable_spread {
                    "🟢 PROFITABLE"
                } else if spread_pct > min_profitable_spread / rust_decimal::Decimal::new(2, 0) {
                    "🟡 MARGINAL"
                } else {
                    "🔴 TOO SMALL"
                };
                
                println!("   💰 Spread: {:.3}% | Gas: ${:.4} | Min: {:.3}% | {}", 
                         spread_pct.round_dp(3), gas_cost.round_dp(4), 
                         min_profitable_spread.round_dp(3), status);
            }
        }
        
        println!();
        Ok(())
    }

    /// Signal relay writer task - sends arbitrage opportunities to SignalRelay
    async fn signal_relay_writer(mut receiver: tokio::sync::mpsc::UnboundedReceiver<Vec<u8>>) {
        use tokio::net::UnixStream;
        use tokio::io::AsyncWriteExt;
        use alphapulse_protocol::SIGNAL_RELAY_PATH;
        
        info!("🚀 Starting signal relay writer for {}", SIGNAL_RELAY_PATH);
        
        let mut connection_count = 0;
        
        loop {
            connection_count += 1;
            info!("🔌 Attempting SignalRelay connection #{} to {}", connection_count, SIGNAL_RELAY_PATH);
            
            // Connect to SignalRelay Unix socket
            let mut stream = match UnixStream::connect(SIGNAL_RELAY_PATH).await {
                Ok(s) => {
                    info!("✅ Connected to SignalRelay (connection #{})", connection_count);
                    s
                }
                Err(e) => {
                    warn!("❌ Failed to connect to SignalRelay (attempt #{}): {}. Retrying in 5s...", connection_count, e);
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    continue;
                }
            };
            
            // Process messages
            loop {
                match receiver.recv().await {
                    Some(message) => {
                        match stream.write_all(&message).await {
                            Ok(_) => {
                                debug!("✅ Sent {}-byte arbitrage message to SignalRelay", message.len());
                            }
                            Err(e) => {
                                warn!("❌ Failed to write to SignalRelay (connection #{}): {}", connection_count, e);
                                break; // Connection lost, reconnect
                            }
                        }
                    }
                    None => {
                        info!("📨 Signal relay sender channel closed");
                        return;
                    }
                }
            }
            
            // Reconnect delay
            info!("⏳ Waiting 2 seconds before SignalRelay reconnection...");
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    }
}