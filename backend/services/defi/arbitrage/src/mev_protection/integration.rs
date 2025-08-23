use anyhow::Result;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{info, warn, debug};
use tokio::sync::RwLock;

use super::{
    ProductionMevProtection, MevDecision, GasTrend,
    MevLogger, MevLoggingConfig,
};
use super::production_mev::MarketContext;

/// Production MEV protection system with logging integration
/// 
/// This provides a clean interface for the arbitrage system to use production-grade
/// MEV protection with automatic logging and outcome tracking.
pub struct MevProtectionSystem {
    protection: Arc<RwLock<ProductionMevProtection>>,
    logger: Option<Arc<MevLogger>>,
    config: MevSystemConfig,
}

#[derive(Debug, Clone)]
pub struct MevSystemConfig {
    pub huff_enabled: bool,
    pub execution_speed_ms: u64,
    pub logging_enabled: bool,
    pub decision_timeout_ms: u64,
}

impl Default for MevSystemConfig {
    fn default() -> Self {
        Self {
            huff_enabled: true,
            execution_speed_ms: 150,    // Our typical execution speed
            logging_enabled: true,
            decision_timeout_ms: 1,     // 1ms decision requirement
        }
    }
}

impl MevProtectionSystem {
    /// Create new production MEV protection system
    pub async fn new(config: MevSystemConfig) -> Result<Self> {
        let protection = Arc::new(RwLock::new(
            ProductionMevProtection::new(config.execution_speed_ms)
        ));
        
        let logger = if config.logging_enabled {
            let logging_config = MevLoggingConfig::from_env();
            Some(Arc::new(MevLogger::new(
                &logging_config.redis_url,
                logging_config.postgres_url.as_deref(),
            ).await?))
        } else {
            None
        };

        info!("Production MEV protection initialized (Huff: {}, Speed: {}ms, Logging: {})",
              config.huff_enabled, config.execution_speed_ms, config.logging_enabled);

        Ok(Self {
            protection,
            logger,
            config,
        })
    }

    /// Make MEV protection decision with <1ms guarantee
    /// 
    /// This is the main entry point for arbitrage decisions.
    /// Returns true if MEV protection should be used.
    pub async fn should_use_protection(
        &self,
        profit_usd: f64,
        path_complexity: usize,
    ) -> bool {
        let start = SystemTime::now();

        // Fast decision using production system
        let decision = {
            let protection = self.protection.read().await;
            protection.should_use_protection(
                profit_usd,
                path_complexity,
                self.config.execution_speed_ms,
            )
        };

        // Log decision asynchronously (non-blocking)
        if let Some(logger) = &self.logger {
            let protection_clone = self.protection.clone();
            let logger_clone = logger.clone();
            let decision_clone = decision.clone();
            tokio::spawn(async move {
                let _ = Self::log_decision_async(protection_clone, logger_clone, decision_clone).await;
            });
        }

        // Performance monitoring
        let elapsed = start.elapsed().unwrap_or_default();
        if elapsed.as_millis() > self.config.decision_timeout_ms as u128 {
            warn!("MEV decision took {}ms (target: {}ms)", 
                  elapsed.as_millis(), self.config.decision_timeout_ms);
        }

        debug!("{}", decision.reasoning);
        decision.use_protection
    }

    /// Record trade outcome for system learning and calibration
    pub async fn record_outcome(
        &self,
        trade_id: &str,
        quoted_profit: f64,
        realized_profit: f64,
        used_protection: bool,
        protection_succeeded: bool,
        execution_time_ms: u64,
        block_number: Option<u64>,
    ) {
        // Calculate metrics for calibration
        let was_front_run = if used_protection {
            false // Protection was used, so no front-running
        } else {
            let extraction_rate = if quoted_profit > 0.0 {
                (quoted_profit - realized_profit) / quoted_profit
            } else {
                0.0
            };
            extraction_rate > 0.1 // Consider 10%+ extraction as front-running
        };

        // Update system calibration
        {
            let mut protection = self.protection.write().await;
            protection.record_outcome(
                0.5, // TODO: Store original threat_score with decision
                was_front_run,
                used_protection,
                if used_protection { Some(protection_succeeded) } else { None },
            );
        }

        // Log outcome asynchronously
        if let Some(logger) = &self.logger {
            let protection_clone = self.protection.clone();
            let logger_clone = logger.clone();
            let trade_id_clone = trade_id.to_string();
            tokio::spawn(async move {
                let _ = Self::log_outcome_async(
                    protection_clone,
                    logger_clone,
                    trade_id_clone,
                    quoted_profit,
                    realized_profit,
                    used_protection,
                    protection_succeeded,
                    execution_time_ms,
                    block_number,
                ).await;
            });
        }

        info!("Recorded MEV outcome: trade={}, quoted=${:.2}, realized=${:.2}, protected={}, success={}",
              trade_id, quoted_profit, realized_profit, used_protection, protection_succeeded);
    }

    /// Update system from new block data (called every block)
    pub async fn update_from_block(&self, block_number: u64) -> Result<()> {
        let mut protection = self.protection.write().await;
        protection.update_from_block(block_number).await?;
        
        debug!("Updated MEV protection from block {}", block_number);
        Ok(())
    }

    /// Get current market context for monitoring
    pub async fn get_market_context(&self) -> MarketContext {
        let protection = self.protection.read().await;
        protection.get_market_context().clone()
    }

    /// Get system statistics for monitoring
    pub async fn get_statistics(&self) -> MevSystemStatistics {
        let context = self.get_market_context().await;
        
        MevSystemStatistics {
            current_gas_gwei: context.current_gas_gwei,
            your_break_even_usd: context.your_break_even_usd,
            mev_break_even_usd: context.mev_break_even_usd,
            estimated_competitors: context.estimated_competitors,
            using_huff: context.huff_gas_usage < context.solidity_gas_usage, // Derive from gas usage
            native_price_usd: context.native_price_usd,
            price_confidence: context.price_confidence,
            last_update_block: context.block_number,
        }
    }

    /// Perform system health check
    pub async fn health_check(&self) -> Result<MevSystemHealth> {
        // Test decision making speed
        let start = SystemTime::now();
        let _ = self.should_use_protection(50.0, 3).await;
        let decision_latency_ms = start.elapsed().unwrap_or_default().as_millis() as u64;

        // Check market context validity
        let context = self.get_market_context().await;
        let market_data_valid = context.mev_break_even_usd.is_finite() && 
                               context.your_break_even_usd.is_finite() &&
                               context.price_confidence > 0.0;

        // Check logging system if enabled
        let logging_healthy = if let Some(logger) = &self.logger {
            logger.get_recent_decisions(1).await.is_ok()
        } else {
            true // Not using logging
        };

        let overall_healthy = decision_latency_ms <= self.config.decision_timeout_ms && 
                             market_data_valid && 
                             logging_healthy;

        Ok(MevSystemHealth {
            decision_latency_ms,
            market_data_valid,
            logging_connected: logging_healthy,
            overall_healthy,
        })
    }

    /// Update market conditions (called when new market data is available)
    pub async fn update_market_conditions(
        &self,
        gas_gwei: f64,
        gas_trend: GasTrend,
        native_price_usd: f64,
        price_confidence: f64,
        block_fullness: f64,
    ) -> Result<()> {
        let mut protection = self.protection.write().await;
        
        // Update price oracle 
        protection.update_price_oracle(native_price_usd, price_confidence);
        
        // Update market context using the updater method
        protection.update_market_context_field(|ctx| {
            ctx.current_gas_gwei = gas_gwei;
            ctx.gas_trend = gas_trend;
            ctx.native_price_usd = native_price_usd;
            ctx.price_confidence = price_confidence;
            ctx.block_fullness = block_fullness;
        });
        
        // Recalculate break-evens
        protection.update_break_evens_public().await?;
        
        debug!("Updated market conditions: gas={:.1} gwei, price=${:.3}, confidence={:.2}",
               gas_gwei, native_price_usd, price_confidence);
        
        Ok(())
    }

    // Integration helpers for different trade types
    
    /// Simple arbitrage decision (2-token path)
    pub async fn simple_arbitrage_decision(&self, profit_usd: f64) -> bool {
        self.should_use_protection(profit_usd, 2).await
    }

    /// Complex arbitrage decision (multi-hop path)
    pub async fn complex_arbitrage_decision(&self, profit_usd: f64, hops: usize) -> bool {
        self.should_use_protection(profit_usd, hops).await
    }

    /// Flash loan arbitrage decision (includes flash loan complexity)
    pub async fn flash_loan_decision(&self, profit_usd: f64, underlying_complexity: usize) -> bool {
        // Flash loans add 1-2 hops of complexity
        self.should_use_protection(profit_usd, underlying_complexity + 2).await
    }

    // Private async logging methods
    async fn log_decision_async(
        protection: Arc<RwLock<ProductionMevProtection>>,
        logger: Arc<MevLogger>,
        decision: MevDecision,
    ) -> Result<()> {
        // Create market context for logging compatibility
        // Get the production MarketContext
        let production_context = {
            let protection = protection.read().await;
            protection.get_market_context().clone()
        };

        logger.log_decision(
            100.0, // profit_usd - TODO: pass from decision
            production_context.current_gas_gwei,
            production_context.native_price_usd,
            2.0,   // profit_ratio
            if decision.use_protection { 
                super::Strategy::PrivateRelay 
            } else { 
                super::Strategy::PublicFast 
            },
            decision.threat_probability,
            decision.expected_mev_loss,
            decision.protection_cost,
            &production_context,
            "production_decision",
            1,
            None,
        ).await?;

        Ok(())
    }

    async fn log_outcome_async(
        protection: Arc<RwLock<ProductionMevProtection>>,
        logger: Arc<MevLogger>,
        trade_id: String,
        quoted_profit: f64,
        realized_profit: f64,
        used_protection: bool,
        protection_succeeded: bool,
        execution_time_ms: u64,
        block_number: Option<u64>,
    ) -> Result<()> {
        // Get the production MarketContext
        let production_context = {
            let protection = protection.read().await;
            protection.get_market_context().clone()
        };

        logger.log_outcome(
            "",
            &trade_id,
            quoted_profit,
            realized_profit,
            used_protection,
            protection_succeeded,
            production_context.current_gas_gwei,
            production_context.native_price_usd,
            &production_context,
            execution_time_ms,
            block_number,
        ).await?;

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct MevSystemStatistics {
    pub current_gas_gwei: f64,
    pub your_break_even_usd: f64,
    pub mev_break_even_usd: f64,
    pub estimated_competitors: u32,
    pub using_huff: bool,
    pub native_price_usd: f64,
    pub price_confidence: f64,
    pub last_update_block: u64,
}

#[derive(Debug, Clone)]
pub struct MevSystemHealth {
    pub decision_latency_ms: u64,
    pub market_data_valid: bool,
    pub logging_connected: bool,
    pub overall_healthy: bool,
}

/// Example usage of the production MEV protection system
#[cfg(test)]
mod example {
    use super::*;
    
    #[tokio::test]
    async fn example_production_mev_usage() -> Result<()> {
        // Initialize production MEV protection
        let config = MevSystemConfig::default();
        let mev_system = MevProtectionSystem::new(config).await?;

        // Update market conditions
        mev_system.update_market_conditions(
            35.0,                    // gas_gwei
            GasTrend::Stable,        // gas_trend
            1.0, // Native price (will be fetched from oracle in production)
            0.9,                     // price_confidence
            0.6,                     // block_fullness
        ).await?;

        // Simulate block updates
        for block in 12345670..12345675 {
            mev_system.update_from_block(block).await?;
        }

        // Example arbitrage opportunities
        let test_cases = vec![
            ("Small simple trade", 15.0, 2),
            ("Medium complex trade", 45.0, 4),
            ("Large simple trade", 120.0, 2),
            ("Complex compound trade", 75.0, 8),
        ];

        println!("🎯 Production MEV Protection Decisions:\n");
        for (description, profit_usd, complexity) in test_cases {
            let should_protect = mev_system.should_use_protection(profit_usd, complexity).await;
            
            println!("{}:", description);
            println!("  Profit: ${:.0}, Complexity: {} hops", profit_usd, complexity);
            println!("  Decision: {}", if should_protect { "🛡️  PROTECT" } else { "⚡ PUBLIC" });
            
            // Record outcome
            mev_system.record_outcome(
                &format!("trade_{}", profit_usd as u32),
                profit_usd,
                if should_protect { profit_usd * 0.95 } else { profit_usd * 0.8 }, // MEV extracted if public
                should_protect,
                true, // Protection succeeded if used
                150,  // execution_time_ms
                Some(12345678),
            ).await;
            
            println!();
        }

        // System statistics
        let stats = mev_system.get_statistics().await;
        println!("📊 System Statistics:");
        println!("  Gas: {:.1} gwei", stats.current_gas_gwei);
        println!("  Your break-even: ${:.2}", stats.your_break_even_usd);
        println!("  MEV break-even: ${:.2}", stats.mev_break_even_usd);
        println!("  Competitors: {}", stats.estimated_competitors);
        println!("  Using Huff: {}", stats.using_huff);
        println!("  Native price: ${:.3} (confidence: {:.1}%)", 
                 stats.native_price_usd, stats.price_confidence * 100.0);

        // Health check
        let health = mev_system.health_check().await?;
        println!("\n🏥 System Health:");
        println!("  Decision latency: {}ms", health.decision_latency_ms);
        println!("  Market data valid: {}", health.market_data_valid);
        println!("  Logging connected: {}", health.logging_connected);
        println!("  Overall healthy: {}", health.overall_healthy);

        // Verify performance requirement
        assert!(health.decision_latency_ms <= 1, "Decision latency must be ≤1ms");
        assert!(health.overall_healthy, "System must be healthy");

        Ok(())
    }
}