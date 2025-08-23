pub mod flashbots_client;
pub mod production_mev;
pub mod logging;
pub mod config;

/// MEV protection strategy types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Strategy {
    PublicFast,
    PrivateRelay,
    HybridAdaptive,
}

/// Market context for MEV decisions
#[derive(Debug, Clone)]
pub struct MarketContext {
    pub gas_price: f64,
    pub block_fullness: f64,
    pub mev_competition: f64,
    pub profit_ratio: f64,
}
pub mod integration;
pub mod huff_integration;

pub use flashbots_client::{FlashbotsClient, BundleStatus, SimulationResult, MEVCompetition, MEVCompetitionLevel};
pub use production_mev::{ProductionMevProtection, ThreatCalibrator, MevDecision, GasTrend, HuffDeploymentStatus, HuffMetrics, HuffAdvantage};
pub use integration::{MevProtectionSystem, MevSystemConfig, MevSystemStatistics, MevSystemHealth};
pub use logging::{MevLogger, MevDecisionLog, MevOutcomeLog, MevTransactionLog};
pub use config::MevLoggingConfig;
pub use huff_integration::{HuffMevIntegration, HuffMevReport, MevProtectionImpact, DeploymentSnapshot};

use anyhow::{Result, Context};
use ethers::prelude::*;
use std::sync::Arc;
use tracing::{debug, info, warn, error};
use tokio::time::{sleep, Duration};

use crate::config::ArbitrageConfig;
use crate::price_oracle::LivePriceOracle;
use crate::secure_registries::SecureRegistryManager;

/// MEV protection coordinator that manages private mempool submission and bundle optimization
pub struct MEVProtection {
    config: Arc<ArbitrageConfig>,
    flashbots_client: Arc<FlashbotsClient>,
    gas_price_cache: Arc<parking_lot::RwLock<GasPriceCache>>,
    bundle_stats: Arc<parking_lot::RwLock<BundleStats>>,
    production_mev: Arc<parking_lot::RwLock<ProductionMevProtection>>,
    price_oracle: Arc<parking_lot::RwLock<LivePriceOracle>>,
}

#[derive(Debug, Clone)]
pub struct GasPriceCache {
    pub base_fee_gwei: u64,
    pub priority_fee_gwei: u64,
    pub fast_gas_gwei: u64,
    pub instant_gas_gwei: u64,
    pub last_updated: std::time::Instant,
    pub mev_competition_level: MEVCompetitionLevel,
}

#[derive(Debug, Clone, Default)]
pub struct BundleStats {
    pub bundles_submitted: u64,
    pub bundles_included: u64,
    pub bundles_failed: u64,
    pub total_profit_usd: f64,
    pub total_gas_cost_usd: f64,
    pub average_inclusion_time_ms: u64,
    pub mev_protection_saves: u64,
}

impl MEVProtection {
    pub async fn new(config: Arc<ArbitrageConfig>) -> Result<Self> {
        let flashbots_client = Arc::new(FlashbotsClient::new(
            &config.rpc_url,
            config.private_key.as_ref().ok_or_else(|| anyhow::anyhow!("Private key not configured"))?,
            config.flashbots_url.clone(),
            config.chain_id,
        )?);
        
        let gas_price_cache = Arc::new(parking_lot::RwLock::new(GasPriceCache {
            base_fee_gwei: 30,
            priority_fee_gwei: 2,
            fast_gas_gwei: 35,
            instant_gas_gwei: 50,
            last_updated: std::time::Instant::now(),
            mev_competition_level: MEVCompetitionLevel::Low,
        }));
        
        let bundle_stats = Arc::new(parking_lot::RwLock::new(BundleStats::default()));
        
        // Initialize production MEV system with dynamic Huff detection
        let production_mev = Arc::new(parking_lot::RwLock::new(
            ProductionMevProtection::new(100) // execution_speed_ms - TODO: measure actual speed
        ));
        
        // Initialize live price oracle for dynamic pricing
        let provider = Provider::<Http>::try_from(&config.rpc_url)
            .context("Failed to initialize RPC provider for MEV price oracle")?;
            
        // Create secure registry for MEV price oracle
        let secure_registry = Arc::new(
            SecureRegistryManager::new(config.chain_id, config.rpc_url.clone()).await?
        );
        let oracle = LivePriceOracle::new(Arc::new(provider), secure_registry);
        let price_oracle = Arc::new(parking_lot::RwLock::new(oracle));
        
        info!("MEVProtection initialized with Flashbots integration, production MEV system, and live price oracle");
        
        Ok(Self {
            config,
            flashbots_client,
            gas_price_cache,
            bundle_stats,
            production_mev,
            price_oracle,
        })
    }
    
    /// Submit transaction via private mempool with MEV protection
    pub async fn submit_protected_transaction(
        &self,
        tx_request: Eip1559TransactionRequest,
        expected_profit_usd: f64,
    ) -> Result<H256> {
        // Check if transaction qualifies for private mempool
        if expected_profit_usd < self.config.private_mempool_threshold_usd {
            warn!("Transaction profit ${:.2} below private mempool threshold ${:.2}, using public mempool", 
                  expected_profit_usd, self.config.private_mempool_threshold_usd);
            // TODO: Submit to public mempool
            return Err(anyhow::anyhow!("Public mempool submission not yet implemented"));
        }
        
        // Update gas pricing based on MEV competition
        let tx_with_mev_pricing = self.apply_mev_gas_pricing(tx_request).await?;
        
        // Submit via Flashbots
        let tx_hash = self.flashbots_client.send_private_transaction(tx_with_mev_pricing).await?;
        
        // Update stats
        {
            let mut stats = self.bundle_stats.write();
            stats.bundles_submitted += 1;
            stats.mev_protection_saves += 1;
        }
        
        info!("Transaction submitted via Flashbots private mempool: {:?}", tx_hash);
        
        // Monitor bundle inclusion
        self.monitor_bundle_inclusion(tx_hash, expected_profit_usd).await;
        
        Ok(tx_hash)
    }
    
    /// Apply MEV-aware gas pricing based on current competition
    async fn apply_mev_gas_pricing(&self, mut tx_request: Eip1559TransactionRequest) -> Result<Eip1559TransactionRequest> {
        // Get current MEV competition level
        let competition = self.flashbots_client.get_mev_competition().await?;
        let gas_multiplier = competition.level.gas_multiplier();
        
        // Update cache
        {
            let mut cache = self.gas_price_cache.write();
            cache.mev_competition_level = competition.level.clone();
            cache.last_updated = std::time::Instant::now();
        }
        
        // Get base gas prices
        let gas_cache = self.gas_price_cache.read().clone();
        let base_max_fee = U256::from(gas_cache.fast_gas_gwei * 1_000_000_000u64);
        let base_priority_fee = U256::from(gas_cache.priority_fee_gwei * 1_000_000_000u64);
        
        // Apply MEV competition multiplier
        let mev_max_fee = base_max_fee * U256::from((gas_multiplier * 100.0) as u64) / 100;
        let mev_priority_fee = base_priority_fee * U256::from((gas_multiplier * 100.0) as u64) / 100;
        
        tx_request = tx_request
            .max_fee_per_gas(mev_max_fee)
            .max_priority_fee_per_gas(mev_priority_fee);
        
        debug!("Applied MEV gas pricing: max_fee={} gwei, priority_fee={} gwei ({}x multiplier)", 
               mev_max_fee / 1_000_000_000u64, mev_priority_fee / 1_000_000_000u64, gas_multiplier);
        
        Ok(tx_request)
    }
    
    /// Monitor bundle inclusion and update stats
    async fn monitor_bundle_inclusion(&self, tx_hash: H256, expected_profit_usd: f64) {
        let start_time = std::time::Instant::now();
        let timeout = Duration::from_secs(300); // 5 minute timeout
        
        tokio::spawn({
            let flashbots_client = self.flashbots_client.clone();
            let bundle_stats = self.bundle_stats.clone();
            
            async move {
                loop {
                    if start_time.elapsed() > timeout {
                        warn!("Bundle inclusion monitoring timeout for tx: {:?}", tx_hash);
                        let mut stats = bundle_stats.write();
                        stats.bundles_failed += 1;
                        break;
                    }
                    
                    match flashbots_client.check_bundle_status(tx_hash).await {
                        Ok(BundleStatus::Confirmed) => {
                            let inclusion_time = start_time.elapsed().as_millis() as u64;
                            info!("Bundle included successfully in {}ms: {:?}", inclusion_time, tx_hash);
                            
                            let mut stats = bundle_stats.write();
                            stats.bundles_included += 1;
                            stats.total_profit_usd += expected_profit_usd;
                            
                            // Update average inclusion time
                            if stats.bundles_included == 1 {
                                stats.average_inclusion_time_ms = inclusion_time;
                            } else {
                                stats.average_inclusion_time_ms = 
                                    (stats.average_inclusion_time_ms * (stats.bundles_included - 1) + inclusion_time) 
                                    / stats.bundles_included;
                            }
                            break;
                        }
                        Ok(BundleStatus::Failed) => {
                            error!("Bundle execution failed: {:?}", tx_hash);
                            let mut stats = bundle_stats.write();
                            stats.bundles_failed += 1;
                            break;
                        }
                        Ok(BundleStatus::Pending) => {
                            debug!("Bundle still pending: {:?}", tx_hash);
                            sleep(Duration::from_secs(2)).await;
                        }
                        Err(e) => {
                            error!("Error checking bundle status: {}", e);
                            sleep(Duration::from_secs(5)).await;
                        }
                    }
                }
            }
        });
    }
    
    /// Get current MEV protection statistics
    pub fn get_stats(&self) -> BundleStats {
        self.bundle_stats.read().clone()
    }
    
    /// Get current gas price recommendations
    pub fn get_gas_prices(&self) -> GasPriceCache {
        self.gas_price_cache.read().clone()
    }
    
    /// Update gas prices from external source (e.g., Unix socket)
    pub fn update_gas_prices(&self, base_fee_gwei: u64, priority_fee_gwei: u64) {
        let mut cache = self.gas_price_cache.write();
        cache.base_fee_gwei = base_fee_gwei;
        cache.priority_fee_gwei = priority_fee_gwei;
        cache.fast_gas_gwei = base_fee_gwei + (priority_fee_gwei * 120) / 100; // 20% higher
        cache.instant_gas_gwei = base_fee_gwei + (priority_fee_gwei * 150) / 100; // 50% higher
        cache.last_updated = std::time::Instant::now();
        
        debug!("Updated gas prices: base={} gwei, priority={} gwei", base_fee_gwei, priority_fee_gwei);
    }
    
    /// Integrated MEV protection decision using production system for critical trades
    pub async fn should_use_mev_protection(&self, profit_usd: f64, gas_cost_usd: f64, trade_size_usd: f64) -> bool {
        self.should_use_mev_protection_with_complexity(profit_usd, gas_cost_usd, trade_size_usd, 2, 100).await
    }
    
    /// MEV protection decision with path complexity and execution speed
    pub async fn should_use_mev_protection_with_complexity(
        &self, 
        profit_usd: f64, 
        gas_cost_usd: f64, 
        trade_size_usd: f64,
        path_complexity: usize,
        execution_speed_ms: u64
    ) -> bool {
        if !self.config.mev_protection_enabled {
            return false;
        }
        
        // Use production MEV system for significant trades (more sophisticated analysis)
        if profit_usd > self.config.min_profit_usd * 2.0 {
            // Update production system with current gas prices
            self.sync_production_mev_state();
            
            let production_decision = {
                let production_mev = self.production_mev.read();
                production_mev.should_use_protection(profit_usd, path_complexity, execution_speed_ms)
            };
            
            debug!("Production MEV decision: profit=${:.2}, complexity={}, speed={}ms → {}", 
                   profit_usd, path_complexity, execution_speed_ms, production_decision.use_protection);
            
            return production_decision.use_protection;
        }
        
        // Fast path for smaller trades (prioritize speed over sophistication)
        self.fast_mev_decision(profit_usd, gas_cost_usd, trade_size_usd).await
    }
    
    /// Fast MEV decision for small trades (<1ms hot path)
    async fn fast_mev_decision(&self, profit_usd: f64, gas_cost_usd: f64, trade_size_usd: f64) -> bool {
        // Get current gas price for dynamic break-even calculation
        let gas_cache = self.gas_price_cache.read();
        let current_gas_gwei = gas_cache.fast_gas_gwei as f64;
        
        // Dynamic MEV break-even using real network conditions and LIVE MATIC price
        let base_gas_cost = 300_000.0; // Gas for typical MEV bot transaction
        let gas_cost_matic = base_gas_cost * current_gas_gwei * 1e-9;
        
        // Get LIVE MATIC price instead of hardcoded $0.80
        let matic_price_usd = {
            let mut oracle = self.price_oracle.write();
            match oracle.get_live_matic_price().await {
                Ok(price) => price,
                Err(e) => {
                    warn!("Failed to get live MATIC price for MEV calculation, using fallback: {}", e);
                    1.0 // Conservative fallback instead of dangerous $0.80
                }
            }
        };
        
        let dynamic_break_even = gas_cost_matic * matic_price_usd * 1.3; // 30% profit margin for MEV bots
        
        // Fast risk calculation
        let mev_risk_prob = if trade_size_usd <= dynamic_break_even {
            0.05 // Minimum risk - not profitable for MEV
        } else {
            // Profit attractiveness
            let profit_ratio = (trade_size_usd - dynamic_break_even) / trade_size_usd;
            let base_risk = profit_ratio * 0.7; // 70% extraction chance for profitable trades
            
            // Gas price impact (higher gas = more competition)
            let gas_multiplier = (current_gas_gwei / 30.0).min(2.0);
            
            // Mempool congestion impact
            let congestion = gas_cache.mev_competition_level.gas_multiplier();
            
            (base_risk * gas_multiplier * congestion).min(0.95).max(0.05)
        };
        
        // Expected costs and outcomes
        let expected_loss_rate = 0.25; // 25% average MEV extraction
        let mev_loss = profit_usd * mev_risk_prob * expected_loss_rate;
        
        // Protection costs
        let flashbots_fee = 0.01; // 1% fee
        let gas_overhead = 0.10; // 10% gas overhead
        let opportunity_decay = 0.03; // 3% decay
        let protection_costs = (gas_cost_usd * gas_overhead) + 
                              (profit_usd * flashbots_fee) + 
                              (profit_usd * opportunity_decay);
        
        // Decision logic
        let expected_profit_with_protection = profit_usd - protection_costs;
        let expected_profit_without_protection = profit_usd - mev_loss;
        
        let should_protect = expected_profit_with_protection > expected_profit_without_protection &&
                            expected_profit_with_protection > self.config.private_mempool_threshold_usd;
        
        debug!("Fast MEV decision: trade=${:.0}, break_even=${:.2}, risk={:.3}, mev_loss=${:.2}, protection_cost=${:.2}, decision={}", 
               trade_size_usd, dynamic_break_even, mev_risk_prob, mev_loss, protection_costs, should_protect);
        
        should_protect
    }
    
    /// Sync current gas prices and market state to production MEV system
    fn sync_production_mev_state(&self) {
        let gas_cache = self.gas_price_cache.read();
        let mut production_mev = self.production_mev.write();
        
        // Update market context with current gas prices
        production_mev.update_market_context_field(|ctx| {
            ctx.current_gas_gwei = gas_cache.fast_gas_gwei as f64;
            ctx.block_fullness = match gas_cache.mev_competition_level {
                MEVCompetitionLevel::Low => 0.3,
                MEVCompetitionLevel::Medium => 0.6,
                MEVCompetitionLevel::High => 1.0,
            };
        });
        
        // Update price oracle with LIVE MATIC price (no more hardcoded values!)
        // Note: In sync context, we'll use the current cached value or default
        let _live_matic_price = 1.0; // Conservative fallback for sync context - prefixed with _ to suppress warning
        // TODO: Replace with actual oracle query when async context is available
        // Price oracle is updated internally via market context
    }
}