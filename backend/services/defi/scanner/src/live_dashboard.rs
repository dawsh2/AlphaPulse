use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::interval;
use crossterm::{
    cursor, execute, style,
    terminal::{self, ClearType},
    ExecutableCommand,
};
use tracing::{debug, error};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use dashmap::DashMap;
use parking_lot::RwLock;

use crate::{PoolInfo, SimpleArbitrageOpportunity, PriceCalculator};

/// Flash animation for pool updates
#[derive(Debug, Clone)]
pub struct FlashState {
    last_update: Instant,
    flash_type: FlashType,
}

#[derive(Debug, Clone)]
pub enum FlashType {
    SwapEvent,
    PoolUpdate,
}

/// Pool group containing all pools for a specific token pair
#[derive(Debug, Clone)]
struct PoolGroup {
    token0: String,
    token1: String,
    pools: Vec<PoolDisplayInfo>,
    max_spread: Decimal,
    best_opportunity: Option<SimpleArbitrageOpportunity>,
    last_activity: Option<Instant>,
}

/// Display information for a single pool
#[derive(Debug, Clone)]
struct PoolDisplayInfo {
    pool_info: PoolInfo,
    price: Option<Decimal>,
    liquidity_usd: Option<Decimal>,
    slippage_1k: Option<Decimal>, // Slippage for $1k trade
    flash_state: Option<FlashState>,
}

/// Live terminal dashboard for DeFi scanner
pub struct LiveDashboard {
    pools: Arc<DashMap<String, PoolInfo>>,
    price_calculator: Arc<PriceCalculator>,
    pool_groups: Arc<RwLock<HashMap<String, PoolGroup>>>,
    flash_states: Arc<DashMap<String, FlashState>>,
    last_refresh: Arc<RwLock<Instant>>,
    stdout: std::io::Stdout,
}

impl LiveDashboard {
    pub fn new(
        pools: Arc<DashMap<String, PoolInfo>>,
        price_calculator: Arc<PriceCalculator>,
    ) -> Result<Self> {
        let mut stdout = std::io::stdout();
        
        // Enable raw mode and hide cursor
        terminal::enable_raw_mode()?;
        execute!(stdout, cursor::Hide, terminal::Clear(ClearType::All))?;
        
        Ok(Self {
            pools,
            price_calculator,
            pool_groups: Arc::new(RwLock::new(HashMap::new())),
            flash_states: Arc::new(DashMap::new()),
            last_refresh: Arc::new(RwLock::new(Instant::now())),
            stdout,
        })
    }
    
    /// Start the live dashboard with periodic updates
    pub async fn start(&mut self) -> Result<()> {
        let mut refresh_interval = interval(Duration::from_millis(500)); // 2 FPS
        
        // Initial setup
        self.setup_terminal()?;
        
        loop {
            refresh_interval.tick().await;
            
            if let Err(e) = self.update_display().await {
                error!("Dashboard update failed: {}", e);
            }
            
            // Check for exit conditions (Ctrl+C handled by main)
            if let Ok(true) = crossterm::event::poll(Duration::from_millis(0)) {
                if let Ok(crossterm::event::Event::Key(key_event)) = crossterm::event::read() {
                    if key_event.code == crossterm::event::KeyCode::Char('q') {
                        break;
                    }
                }
            }
        }
        
        self.cleanup_terminal()?;
        Ok(())
    }
    
    /// Trigger a flash animation for pool activity
    pub fn flash_pool(&self, pool_address: &str, flash_type: FlashType) {
        let flash_state = FlashState {
            last_update: Instant::now(),
            flash_type,
        };
        self.flash_states.insert(pool_address.to_string(), flash_state);
    }
    
    /// Setup terminal for dashboard display
    fn setup_terminal(&mut self) -> Result<()> {
        execute!(
            self.stdout,
            terminal::Clear(ClearType::All),
            cursor::MoveTo(0, 0),
            style::Print("🚀 AlphaPulse DeFi Scanner - Live Dashboard\n"),
            style::Print("═".repeat(120)),
            style::Print("\n"),
        )?;
        Ok(())
    }
    
    /// Update the live display
    async fn update_display(&mut self) -> Result<()> {
        let now = Instant::now();
        
        // Update pool groups from current pool data
        self.update_pool_groups().await?;
        
        // Clear screen and redraw
        execute!(
            self.stdout,
            cursor::MoveTo(0, 3), // Start after header
            terminal::Clear(ClearType::FromCursorDown),
        )?;
        
        // Draw pool groups sorted by best opportunity
        self.draw_pool_groups().await?;
        
        // Draw footer with stats
        self.draw_footer().await?;
        
        *self.last_refresh.write() = now;
        Ok(())
    }
    
    /// Update pool groups from current pool data
    async fn update_pool_groups(&self) -> Result<()> {
        let mut groups: HashMap<String, PoolGroup> = HashMap::new();
        
        // Process all pools
        for pool_entry in self.pools.iter() {
            let pool = pool_entry.value().clone();
            let pool_key = format!("{}_{}", pool.token0, pool.token1);
            
            // Calculate current metrics
            let price = self.calculate_pool_price(&pool).await;
            let liquidity_usd = self.estimate_liquidity_usd(&pool).await;
            let slippage_1k = self.calculate_slippage(&pool, dec!(1000)).await;
            
            // Get flash state
            let flash_state = self.flash_states.get(&pool.address).map(|f| f.clone());
            
            let display_info = PoolDisplayInfo {
                pool_info: pool.clone(),
                price,
                liquidity_usd,
                slippage_1k,
                flash_state: flash_state.clone(),
            };
            
            // Add to group
            let group = groups.entry(pool_key.clone()).or_insert_with(|| PoolGroup {
                token0: pool.token0.clone(),
                token1: pool.token1.clone(),
                pools: Vec::new(),
                max_spread: Decimal::ZERO,
                best_opportunity: None,
                last_activity: None,
            });
            
            group.pools.push(display_info);
            
            // Update last activity if pool has recent flash
            if let Some(flash) = &flash_state {
                if Instant::now().duration_since(flash.last_update) < Duration::from_secs(2) {
                    group.last_activity = Some(flash.last_update);
                }
            }
        }
        
        // Calculate spreads and opportunities for each group
        for group in groups.values_mut() {
            self.calculate_group_metrics(group).await?;
        }
        
        *self.pool_groups.write() = groups;
        Ok(())
    }
    
    /// Calculate metrics for a pool group (spread, best opportunity)
    async fn calculate_group_metrics(&self, group: &mut PoolGroup) -> Result<()> {
        if group.pools.len() < 2 {
            return Ok(());
        }
        
        let mut min_price = Decimal::MAX;
        let mut max_price = Decimal::ZERO;
        let mut opportunities = Vec::new();
        
        // Find price range and calculate opportunities
        for pool in &group.pools {
            if let Some(price) = pool.price {
                min_price = min_price.min(price);
                max_price = max_price.max(price);
            }
        }
        
        // Calculate spread percentage
        if min_price > Decimal::ZERO && max_price > min_price {
            group.max_spread = ((max_price - min_price) / min_price) * dec!(100);
        }
        
        // Calculate arbitrage opportunities between all pool pairs
        for (i, buy_pool) in group.pools.iter().enumerate() {
            for (j, sell_pool) in group.pools.iter().enumerate() {
                if i != j {
                    if let Some(opportunity) = self.calculate_opportunity(
                        &buy_pool.pool_info,
                        &sell_pool.pool_info,
                    ).await {
                        opportunities.push(opportunity);
                    }
                }
            }
        }
        
        // Find best opportunity (highest profit, even if negative)
        group.best_opportunity = opportunities.into_iter()
            .max_by(|a, b| a.expected_profit_usd.cmp(&b.expected_profit_usd));
        
        Ok(())
    }
    
    /// Calculate pool price (token1 per token0)
    async fn calculate_pool_price(&self, pool: &PoolInfo) -> Option<Decimal> {
        if pool.reserve0 > Decimal::ZERO && pool.reserve1 > Decimal::ZERO {
            Some(pool.reserve1 / pool.reserve0)
        } else {
            None
        }
    }
    
    /// Estimate pool liquidity in USD
    async fn estimate_liquidity_usd(&self, pool: &PoolInfo) -> Option<Decimal> {
        // Simplified estimate: assume reserve1 is USD-pegged or multiply by estimated USD price
        // For now, use reserve1 as a proxy for USD liquidity
        if pool.reserve1 > Decimal::ZERO {
            Some(pool.reserve1 * dec!(2)) // Total liquidity ≈ 2x one side
        } else {
            None
        }
    }
    
    /// Calculate slippage for a given trade size
    async fn calculate_slippage(&self, pool: &PoolInfo, trade_size_usd: Decimal) -> Option<Decimal> {
        // Simplified slippage calculation for AMM
        if pool.reserve0 > Decimal::ZERO && pool.reserve1 > Decimal::ZERO {
            let k = pool.reserve0 * pool.reserve1;
            let trade_amount = trade_size_usd / (pool.reserve1 / pool.reserve0); // Convert USD to token0
            
            if trade_amount < pool.reserve0 {
                let new_reserve0 = pool.reserve0 + trade_amount;
                let new_reserve1 = k / new_reserve0;
                let amount_out = pool.reserve1 - new_reserve1;
                let expected_out = trade_amount * (pool.reserve1 / pool.reserve0);
                
                if expected_out > Decimal::ZERO {
                    let slippage = ((expected_out - amount_out) / expected_out) * dec!(100);
                    Some(slippage.max(Decimal::ZERO))
                } else {
                    None
                }
            } else {
                Some(dec!(100)) // 100% slippage if trade > reserves
            }
        } else {
            None
        }
    }
    
    /// Calculate arbitrage opportunity between two pools
    async fn calculate_opportunity(&self, buy_pool: &PoolInfo, sell_pool: &PoolInfo) -> Option<SimpleArbitrageOpportunity> {
        // Use the price calculator to get a realistic opportunity
        // For now, use a simplified calculation since get_quote requires token symbols
        if buy_pool.reserve0 > Decimal::ZERO && buy_pool.reserve1 > Decimal::ZERO &&
           sell_pool.reserve0 > Decimal::ZERO && sell_pool.reserve1 > Decimal::ZERO {
            
            let buy_price = buy_pool.reserve1 / buy_pool.reserve0;
            let sell_price = sell_pool.reserve1 / sell_pool.reserve0;
            
            if sell_price > buy_price {
                let trade_size = dec!(1000);
                let profit_percentage = (sell_price - buy_price) / buy_price;
                let gross_profit = trade_size * profit_percentage;
                let gas_cost = dec!(10); // Simplified gas cost estimate
                let net_profit = gross_profit - gas_cost;
                
                return Some(SimpleArbitrageOpportunity {
                    buy_pool: buy_pool.clone(),
                    sell_pool: sell_pool.clone(),
                    buy_amount_in: trade_size,
                    expected_profit_usd: net_profit,
                    gas_cost_usd: gas_cost,
                    confidence_score: if net_profit > Decimal::ZERO { 0.8 } else { 0.3 },
                });
            }
        }
        None
    }
    
    /// Draw all pool groups to terminal
    async fn draw_pool_groups(&mut self) -> Result<()> {
        let groups = self.pool_groups.read().clone();
        let mut sorted_groups: Vec<_> = groups.values().collect();
        
        // Sort by best opportunity descending (highest profit first, even if negative)
        sorted_groups.sort_by(|a, b| {
            let a_profit = a.best_opportunity.as_ref()
                .map(|o| o.expected_profit_usd)
                .unwrap_or(Decimal::MIN);
            let b_profit = b.best_opportunity.as_ref()
                .map(|o| o.expected_profit_usd)
                .unwrap_or(Decimal::MIN);
            b_profit.cmp(&a_profit)
        });
        
        // Header
        execute!(
            self.stdout,
            style::Print(format!(
                "{:<20} {:<8} {:<12} {:<10} {:<12} {:<15} {:<12} {:<8}\n",
                "PAIR", "POOLS", "SPREAD%", "BEST_OPP", "LIQUIDITY", "PRICE_RANGE", "AVG_SLIP", "ACTIVITY"
            )),
            style::Print("─".repeat(120)),
            style::Print("\n"),
        )?;
        
        // Draw each group
        for group in sorted_groups.iter().take(25) { // Limit to screen space
            self.draw_pool_group(group).await?;
        }
        
        Ok(())
    }
    
    /// Draw a single pool group
    async fn draw_pool_group(&mut self, group: &PoolGroup) -> Result<()> {
        let pair_name = format!("{}/{}", group.token0, group.token1);
        let pool_count = group.pools.len();
        let spread = format!("{:.3}%", group.max_spread);
        
        let best_opp = group.best_opportunity.as_ref()
            .map(|o| format!("${:.2}", o.expected_profit_usd))
            .unwrap_or_else(|| "N/A".to_string());
        
        let total_liquidity: Decimal = group.pools.iter()
            .filter_map(|p| p.liquidity_usd)
            .sum();
        let liquidity_str = if total_liquidity > Decimal::ZERO {
            format!("${:.0}K", total_liquidity / dec!(1000))
        } else {
            "N/A".to_string()
        };
        
        // Price range
        let prices: Vec<_> = group.pools.iter().filter_map(|p| p.price).collect();
        let price_range = if !prices.is_empty() {
            let min_p = prices.iter().min().unwrap();
            let max_p = prices.iter().max().unwrap();
            format!("{:.6}-{:.6}", min_p, max_p)
        } else {
            "N/A".to_string()
        };
        
        // Average slippage
        let slippages: Vec<_> = group.pools.iter().filter_map(|p| p.slippage_1k).collect();
        let avg_slippage = if !slippages.is_empty() {
            let avg: Decimal = slippages.iter().sum::<Decimal>() / Decimal::from(slippages.len());
            format!("{:.2}%", avg)
        } else {
            "N/A".to_string()
        };
        
        // Activity indicator
        let activity = if let Some(last_activity) = group.last_activity {
            let elapsed = Instant::now().duration_since(last_activity);
            if elapsed < Duration::from_millis(500) {
                "🔥 LIVE"
            } else if elapsed < Duration::from_secs(2) {
                "⚡ RECENT"
            } else {
                "💤 IDLE"
            }
        } else {
            "💤 IDLE"
        };
        
        // Color coding for profit
        let profit_color = if let Some(opp) = &group.best_opportunity {
            if opp.expected_profit_usd > Decimal::ZERO {
                style::Color::Green
            } else if opp.expected_profit_usd > dec!(-5) {
                style::Color::Yellow
            } else {
                style::Color::Red
            }
        } else {
            style::Color::White
        };
        
        execute!(
            self.stdout,
            style::SetForegroundColor(profit_color),
            style::Print(format!(
                "{:<20} {:<8} {:<12} {:<10} {:<12} {:<15} {:<12} {:<8}\n",
                pair_name, pool_count, spread, best_opp, liquidity_str, price_range, avg_slippage, activity
            )),
            style::ResetColor,
        )?;
        
        // Draw individual pools for this group (indented)
        for pool in &group.pools {
            self.draw_individual_pool(pool).await?;
        }
        
        Ok(())
    }
    
    /// Draw individual pool details
    async fn draw_individual_pool(&mut self, pool: &PoolDisplayInfo) -> Result<()> {
        let pool_addr = &pool.pool_info.address[0..10]; // Truncated address
        let exchange = &pool.pool_info.exchange;
        let price_str = pool.price
            .map(|p| format!("{:.6}", p))
            .unwrap_or_else(|| "N/A".to_string());
        let liquidity_str = pool.liquidity_usd
            .map(|l| format!("${:.0}K", l / dec!(1000)))
            .unwrap_or_else(|| "N/A".to_string());
        let slippage_str = pool.slippage_1k
            .map(|s| format!("{:.2}%", s))
            .unwrap_or_else(|| "N/A".to_string());
        
        // Flash animation
        let (flash_color, flash_symbol) = if let Some(flash) = &pool.flash_state {
            let elapsed = Instant::now().duration_since(flash.last_update);
            if elapsed < Duration::from_millis(200) {
                match flash.flash_type {
                    FlashType::SwapEvent => (style::Color::Cyan, "🔄"),
                    FlashType::PoolUpdate => (style::Color::Magenta, "📊"),
                }
            } else if elapsed < Duration::from_millis(500) {
                (style::Color::DarkGrey, "·")
            } else {
                (style::Color::Reset, " ")
            }
        } else {
            (style::Color::Reset, " ")
        };
        
        execute!(
            self.stdout,
            style::SetForegroundColor(flash_color),
            style::Print(format!(
                "  {} {:<10} {:<15} {:<12} {:<12} {:<12}\n",
                flash_symbol, pool_addr, exchange, price_str, liquidity_str, slippage_str
            )),
            style::ResetColor,
        )?;
        
        Ok(())
    }
    
    /// Draw footer with system stats
    async fn draw_footer(&mut self) -> Result<()> {
        let total_pools = self.pools.len();
        let total_groups = self.pool_groups.read().len();
        let active_flashes = self.flash_states.len();
        let last_refresh = *self.last_refresh.read();
        let uptime = last_refresh.elapsed();
        
        execute!(
            self.stdout,
            style::Print("\n"),
            style::Print("─".repeat(120)),
            style::Print("\n"),
            style::Print(format!(
                "Pools: {} | Groups: {} | Active: {} | Uptime: {:?} | Press 'q' to quit",
                total_pools, total_groups, active_flashes, uptime
            )),
        )?;
        
        Ok(())
    }
    
    /// Cleanup terminal on exit
    fn cleanup_terminal(&mut self) -> Result<()> {
        execute!(
            self.stdout,
            style::ResetColor,
            cursor::Show,
            terminal::Clear(ClearType::All),
        )?;
        terminal::disable_raw_mode()?;
        Ok(())
    }
}

impl Drop for LiveDashboard {
    fn drop(&mut self) {
        let _ = self.cleanup_terminal();
    }
}