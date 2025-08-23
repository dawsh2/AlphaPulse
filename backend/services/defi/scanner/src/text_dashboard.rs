use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::interval;
use tokio::sync::broadcast;
use tracing::{info, error, debug};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use chrono;

use crate::{ArbitrageOpportunity, DashboardUpdate};

/// Text-based dashboard that consumes scanner events instead of recalculating
pub struct TextDashboard {
    /// Receiver for dashboard updates from OpportunityDetector
    dashboard_receiver: broadcast::Receiver<DashboardUpdate>,
    
    /// Current dashboard state (cached data from events)
    state: DashboardState,
}

/// Cached dashboard state built from scanner events
#[derive(Debug, Clone)]
struct DashboardState {
    /// Token address -> symbol mappings
    token_symbols: HashMap<String, String>,
    
    /// Pool groups by token pair (e.g., "WETH/USDC")
    pool_groups: HashMap<String, PoolGroupDisplay>,
    
    /// Recent opportunities (kept for display)
    recent_opportunities: Vec<ArbitrageOpportunity>,
    
    /// Last update time
    last_update: Instant,
}

/// Display information for a pool group (same token pair across DEXs)
#[derive(Debug, Clone)]
struct PoolGroupDisplay {
    token_pair: String,
    token0_symbol: String,
    token1_symbol: String,
    pool_count: usize,
    price_range: (Decimal, Decimal), // min, max
    max_spread_percent: Decimal,
    total_liquidity_usd: Option<Decimal>,
    best_opportunity: Option<ArbitrageOpportunity>,
    last_activity: Instant,
}

impl TextDashboard {
    pub fn new(dashboard_receiver: broadcast::Receiver<DashboardUpdate>) -> Self {
        Self {
            dashboard_receiver,
            state: DashboardState {
                token_symbols: HashMap::new(),
                pool_groups: HashMap::new(),
                recent_opportunities: Vec::new(),
                last_update: Instant::now(),
            },
        }
    }

    /// Start the dashboard event loop
    pub async fn start(&mut self) -> Result<()> {
        let mut display_interval = interval(Duration::from_secs(3));
        
        // Print initial header
        self.print_header();
        
        info!("📊 Dashboard consuming live scanner events...");
        info!("🔄 Updates every 3 seconds based on OpportunityDetector calculations");
        
        loop {
            tokio::select! {
                // Handle dashboard updates from OpportunityDetector
                Ok(update) = self.dashboard_receiver.recv() => {
                    self.handle_dashboard_update(update);
                }
                
                // Periodic display refresh
                _ = display_interval.tick() => {
                    self.display_current_state();
                }
            }
        }
    }

    /// Handle a dashboard update event from OpportunityDetector
    fn handle_dashboard_update(&mut self, update: DashboardUpdate) {
        match update {
            DashboardUpdate::NewOpportunity(opportunity) => {
                // Add to recent opportunities (keep last 20)
                self.state.recent_opportunities.push(opportunity.clone());
                if self.state.recent_opportunities.len() > 20 {
                    self.state.recent_opportunities.remove(0);
                }
                
                // Update the best opportunity for the relevant pool group
                let token_pair = format!("{}/{}", 
                    self.resolve_symbol(&opportunity.token_in),
                    self.resolve_symbol(&opportunity.token_out)
                );
                
                if let Some(group) = self.state.pool_groups.get_mut(&token_pair) {
                    // Update if this is better than current best
                    let is_better = group.best_opportunity.as_ref()
                        .map(|current| opportunity.net_profit_usd > current.net_profit_usd)
                        .unwrap_or(true);
                    
                    if is_better {
                        group.best_opportunity = Some(opportunity);
                        group.last_activity = Instant::now();
                    }
                }
                
                self.state.last_update = Instant::now();
            }
            
            DashboardUpdate::PoolGroupUpdate { 
                token_pair,
                token0_symbol,
                token1_symbol,
                pools,
                price_range,
                max_spread_percent,
                total_liquidity_usd,
                best_opportunity,
            } => {
                let group = PoolGroupDisplay {
                    token_pair: token_pair.clone(),
                    token0_symbol: token0_symbol.clone(),
                    token1_symbol: token1_symbol.clone(),
                    pool_count: pools.len(),
                    price_range,
                    max_spread_percent,
                    total_liquidity_usd,
                    best_opportunity,
                    last_activity: Instant::now(),
                };
                
                self.state.pool_groups.insert(token_pair, group);
                self.state.last_update = Instant::now();
            }
            
            DashboardUpdate::TokenSymbolResolved { address, symbol } => {
                self.state.token_symbols.insert(address, symbol);
                self.state.last_update = Instant::now();
            }
        }
    }

    /// Resolve token symbol from address using cached data
    fn resolve_symbol(&self, address: &str) -> String {
        self.state.token_symbols.get(address)
            .cloned()
            .unwrap_or_else(|| {
                // Fallback to truncated address
                if address.len() >= 10 {
                    format!("{}...{}", &address[0..6], &address[address.len()-4..])
                } else {
                    address.to_string()
                }
            })
    }

    /// Print dashboard header
    fn print_header(&self) {
        println!("\n🚀 AlphaPulse DeFi Scanner - Live Event Dashboard");
        println!("{}", "═".repeat(120));
        println!("Real-time display of OpportunityDetector calculations (no recalculation)");
        println!("{}", "═".repeat(120));
    }

    /// Display current dashboard state
    fn display_current_state(&self) {
        // Clear previous output
        print!("\x1B[2J\x1B[1;1H"); // Clear screen and move cursor to top
        
        self.print_header();
        
        // Current time and stats
        println!("\n⏰ Last Update: {} | Data Age: {:.1}s", 
                 chrono::Utc::now().format("%H:%M:%S"),
                 self.state.last_update.elapsed().as_secs_f64());
        
        // System status
        self.display_system_status();
        
        // Pool groups table
        self.display_pool_groups();
        
        // Recent opportunities
        self.display_recent_opportunities();
        
        println!("\n{}", "═".repeat(120));
        println!("📊 Data sourced directly from OpportunityDetector - 100% consistent calculations");
    }

    /// Display system status
    fn display_system_status(&self) {
        let total_groups = self.state.pool_groups.len();
        let total_pools: usize = self.state.pool_groups.values()
            .map(|g| g.pool_count)
            .sum();
        
        let profitable_opportunities = self.state.pool_groups.values()
            .filter_map(|g| g.best_opportunity.as_ref())
            .filter(|o| o.net_profit_usd > Decimal::ZERO)
            .count();
        
        let recent_opportunities = self.state.recent_opportunities.len();
        
        println!("📊 SYSTEM STATUS:");
        println!("   Total Pools: {} | Token Pairs: {} | Profitable Opps: {} | Recent Activity: {}", 
                 total_pools, total_groups, profitable_opportunities, recent_opportunities);
        
        if total_pools == 0 {
            println!("   🔄 Waiting for pool data from OpportunityDetector...");
            println!("   📡 Ensure scanner services are running and processing events");
        }
        println!();
    }

    /// Display pool groups in a properly formatted table
    fn display_pool_groups(&self) {
        if self.state.pool_groups.is_empty() {
            println!("⏳ No pool groups available yet - waiting for scanner events...");
            return;
        }

        // Sort by best profit (descending)
        let mut sorted_groups: Vec<_> = self.state.pool_groups.values().collect();
        sorted_groups.sort_by(|a, b| {
            let a_profit = a.best_opportunity.as_ref()
                .map(|o| o.net_profit_usd)
                .unwrap_or(Decimal::MIN);
            let b_profit = b.best_opportunity.as_ref()
                .map(|o| o.net_profit_usd)
                .unwrap_or(Decimal::MIN);
            b_profit.cmp(&a_profit)
        });

        // Header with proper spacing
        println!("🔥 LIVE ARBITRAGE OPPORTUNITIES (Scanner Data):");
        println!("{:<20} {:<6} {:<10} {:<12} {:<12} {:<15} {:<10}", 
                 "TOKEN PAIR", "POOLS", "SPREAD%", "PROFIT", "PRICE RANGE", "LAST UPDATE", "STATUS");
        println!("{}", "─".repeat(100));

        // Display top opportunities with proper alignment
        for (i, group) in sorted_groups.iter().take(15).enumerate() {
            let rank = format!("{}.", i + 1);
            let pair = format!("{:<19}", group.token_pair);
            let pools = format!("{:<6}", group.pool_count);
            let spread = format!("{:<10.3}%", group.max_spread_percent);
            
            let (profit, status) = if let Some(opp) = &group.best_opportunity {
                let profit_str = format!("${:<11.2}", opp.net_profit_usd);
                let status = if opp.net_profit_usd > Decimal::ZERO {
                    "🟢 PROFIT"
                } else if opp.net_profit_usd > dec!(-5) {
                    "🟡 BREAK-EVEN"
                } else {
                    "🔴 LOSS"
                };
                (profit_str, status)
            } else {
                ("N/A".to_string(), "⚫ NO DATA")
            };
            
            let price_range = if group.price_range.0 != Decimal::MAX {
                format!("{:.4}-{:.4}", group.price_range.0, group.price_range.1)
            } else {
                "N/A".to_string()
            };
            
            let last_update = format!("{:.0}s", group.last_activity.elapsed().as_secs());
            
            println!("{:<3}{} {} {} {} {:<15} {:<10} {}", 
                     rank, pair, pools, spread, profit, price_range, last_update, status);
        }
    }

    /// Display recent opportunities
    fn display_recent_opportunities(&self) {
        if self.state.recent_opportunities.is_empty() {
            return;
        }

        println!("\n🚨 RECENT OPPORTUNITIES:");
        println!("{:<15} {:<15} {:<10} {:<10} {:<10}", 
                 "TOKEN PAIR", "BUY EXCHANGE", "SELL EXCHANGE", "PROFIT", "TIME");
        println!("{}", "─".repeat(70));

        // Show last 5 opportunities
        for opp in self.state.recent_opportunities.iter().rev().take(5) {
            let token_pair = format!("{}/{}", 
                self.resolve_symbol(&opp.token_in),
                self.resolve_symbol(&opp.token_out)
            );
            let buy_exchange = format!("{:<15}", opp.buy_exchange);
            let sell_exchange = format!("{:<15}", opp.sell_exchange);
            let profit = format!("${:<9.2}", opp.net_profit_usd);
            let time = format!("{}s ago", 
                (chrono::Utc::now().timestamp() - opp.timestamp).max(0));

            println!("{:<15} {} {} {} {}", 
                     token_pair, buy_exchange, sell_exchange, profit, time);
        }
    }
}