// Use local ArbitrageOpportunity type - protocol has ArbitrageOpportunityMessage
use alphapulse_protocol::ArbitrageOpportunityMessage;
use anyhow::{Context, Result};
use ethers::prelude::*;
use parking_lot::RwLock;
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

pub mod unix_socket_simple;

use crate::price_oracle::{LivePriceOracle, PriceManager};
use crate::safety_circuit_breaker::{SafetyCircuitBreaker, SafetyConfig};
use crate::secure_registries::SecureRegistryManager;
use crate::oracle::{PriceOracle, price_oracle::OracleConfig};

/// Arbitrage opportunity structure
#[derive(Debug, Clone)]
pub struct ArbitrageOpportunity {
    pub id: String,
    pub timestamp_ns: u64,
    pub timestamp: u64,  // For compatibility
    pub path: Vec<String>,
    pub token_path: Vec<Address>,
    pub dex_path: Vec<String>,
    pub amounts: Vec<U256>,
    pub expected_profit: i64,
    pub estimated_profit_usd: f64,
    pub profit_usd: f64,  // For compatibility
    pub profit_ratio: f64,
    pub gas_estimate: u64,
    pub net_profit_usd: f64,
    pub required_capital: U256,
    pub complexity_score: f64,
}

/// Flash loan opportunity structure
#[derive(Debug, Clone)]
pub struct FlashOpportunity {
    pub id: String,
    pub path: Vec<String>,
    pub amounts: Vec<U256>,
    pub expected_profit: Decimal,
    pub amount_in: Decimal,
}

/// Simplified opportunity structure for validation
#[derive(Debug, Clone)]
pub struct SimpleOpportunity {
    pub id: String,
    pub path: Vec<String>,
    pub amounts: Vec<U256>,
    pub expected_profit: i64,
}

pub mod config;
pub mod execution;
pub mod mev_protection;
pub mod strategies;
// pub mod huff_deployment_example; // Commented out due to compilation issues - example code
pub mod price_oracle;
pub mod safety_circuit_breaker;
pub mod dex_integration;
pub mod amm_math;
pub mod multi_hop_validator;
pub mod trade_optimizer;
pub mod liquidity_analyzer;
pub mod secure_registries;
pub mod fallback_handler;
pub mod testing;
pub mod monitoring;
pub mod simple_tests;
pub mod oracle;

pub use config::ArbitrageConfig;
pub use execution::{ExecutionEngine, ExecutionMode, ExecutionResult};
pub use strategies::{Strategy, StrategyType, StrategyResult};

/// Main arbitrage engine that orchestrates all components
pub struct ArbitrageEngine {
    config: Arc<ArbitrageConfig>,
    execution_engine: Arc<ExecutionEngine>,
    strategies: HashMap<StrategyType, Box<dyn Strategy + Send + Sync>>,
    metrics: Arc<RwLock<ArbitrageMetrics>>,
    price_manager: Arc<RwLock<PriceManager>>,
    safety_breaker: Arc<RwLock<SafetyCircuitBreaker>>,
    oracle: Arc<PriceOracle>,
}

#[derive(Debug, Clone, Default)]
pub struct ArbitrageMetrics {
    pub opportunities_received: u64,
    pub opportunities_simulated: u64,
    pub opportunities_executed: u64,
    pub opportunities_analyzed: usize,
    pub successful_trades: usize,
    pub total_profit_usd: f64,
    pub total_gas_cost_usd: f64,
    pub total_gas_used: u64,
    pub total_gas_estimated: u64,
    pub total_volume_usd: f64,
    pub success_rate: f64,
    pub compound_arbitrage_count: u64,
    pub flash_loan_count: u64,
    pub capital_mode_count: u64,
    pub execution_times_ms: Vec<u64>,
    pub active_positions: usize,
    pub pending_transactions: usize,
    pub blocks_processed: usize,
    pub mev_blocks_detected: usize,
    pub peak_balance: f64,
    pub current_balance: f64,
}

#[derive(Debug, Clone)]
pub struct ProcessedOpportunity {
    pub original: ArbitrageOpportunity,
    pub strategy_type: StrategyType,
    pub execution_mode: ExecutionMode,
    pub simulated_profit_usd: f64,
    pub confidence_score: f64,
    pub token_path: Vec<Address>,
    pub dex_path: Vec<String>,
    pub gas_estimate: u64,
    pub processing_time_ms: u64,
}

impl ArbitrageEngine {
    pub async fn new(config: ArbitrageConfig) -> Result<Self> {
        let config = Arc::new(config);
        
        // Initialize SECURE registry manager (eliminates ALL hardcoded addresses + honeypot protection)
        let secure_registry = Arc::new(
            SecureRegistryManager::new(config.chain_id, config.rpc_url.clone()).await
                .context("Failed to initialize secure registry manager")?
        );
        
        // Initialize live price oracle system with registry
        let provider = Provider::<Http>::try_from(&config.rpc_url)
            .context("Failed to initialize RPC provider for price oracle")?;
        let provider = Arc::new(provider);
        let old_oracle = LivePriceOracle::new(provider.clone(), secure_registry.clone());
        let price_manager = Arc::new(RwLock::new(PriceManager::new(old_oracle)));
        
        // Initialize new unified oracle system
        let oracle_config = OracleConfig::default();
        let oracle = Arc::new(PriceOracle::new(provider.clone(), oracle_config).await
            .context("Failed to initialize unified oracle")?);
        
        // Initialize production safety circuit breaker
        let provider_for_safety = Provider::<Http>::try_from(&config.rpc_url)
            .context("Failed to initialize RPC provider for safety circuit")?;
        let oracle_for_safety = LivePriceOracle::new(
            Arc::new(provider_for_safety),
            secure_registry.clone()
        );
        let safety_config = SafetyConfig::default();
        let safety_breaker = Arc::new(RwLock::new(
            SafetyCircuitBreaker::new(safety_config, Arc::new(RwLock::new(oracle_for_safety)))
        ));
        
        // Initialize execution engine
        let execution_engine = Arc::new(
            ExecutionEngine::new(config.clone()).await
                .context("Failed to initialize execution engine")?
        );
        
        // Initialize strategies
        let mut strategies: HashMap<StrategyType, Box<dyn Strategy + Send + Sync>> = HashMap::new();
        
        // Add simple arbitrage strategy
        strategies.insert(
            StrategyType::Simple,
            Box::new(strategies::SimpleStrategy::new(config.clone()).await?)
        );
        
        // Add triangular arbitrage strategy
        strategies.insert(
            StrategyType::Triangular,
            Box::new(strategies::TriangularStrategy::new(config.clone()).await?)
        );
        
        // Add compound arbitrage strategy (key differentiator)
        if config.compound_enabled {
            strategies.insert(
                StrategyType::Compound,
                Box::new(strategies::CompoundStrategy::new(config.clone()).await?)
            );
        }
        
        let metrics = Arc::new(RwLock::new(ArbitrageMetrics::default()));
        
        info!("ArbitrageEngine initialized with {} strategies", strategies.len());
        
        Ok(Self {
            config,
            execution_engine,
            strategies,
            metrics,
            price_manager,
            safety_breaker,
            oracle,
        })
    }
    
    /// Process an arbitrage opportunity through the full pipeline
    pub async fn process_opportunity(&self, opportunity: ArbitrageOpportunity) -> Result<Option<ProcessedOpportunity>> {
        let start_time = SystemTime::now();
        
        // Update metrics
        {
            let mut metrics = self.metrics.write();
            metrics.opportunities_received += 1;
        }
        
        // Quick validation using live prices
        if !self.is_opportunity_valid(&opportunity).await {
            debug!("Opportunity failed validation: {:?}", opportunity);
            return Ok(None);
        }
        
        // Strategy selection
        let strategy_type = self.select_strategy(&opportunity).await?;
        let strategy = self.strategies.get(&strategy_type)
            .ok_or_else(|| anyhow::anyhow!("Strategy not found: {:?}", strategy_type))?;
        
        // Convert ArbitrageOpportunity to AlphaOpportunity
        let token_in = opportunity.path.first()
            .and_then(|s| s.parse::<Address>().ok())
            .unwrap_or_else(Address::zero);
        let token_out = opportunity.path.last()
            .and_then(|s| s.parse::<Address>().ok())
            .unwrap_or_else(Address::zero);
            
        let alpha_opportunity = crate::strategies::AlphaOpportunity {
            id: opportunity.id.clone(),
            profit_usd: opportunity.expected_profit as f64,
            token_in,
            token_out,
        };
        
        // Execute strategy analysis
        let strategy_result = strategy.analyze_opportunity(&alpha_opportunity).await?;
        
        if !strategy_result.is_profitable {
            debug!("Strategy analysis rejected opportunity: {}", strategy_result.reason.unwrap_or_default());
            return Ok(None);
        }
        
        // CRITICAL SAFETY CHECK - Production circuit breaker
        let gas_cost_usd = self.calculate_live_gas_cost_usd(strategy_result.gas_estimate).await
            .unwrap_or(strategy_result.expected_profit_usd * 0.1); // Conservative fallback
        
        let safety_check = {
            let mut safety_breaker = self.safety_breaker.write();
            safety_breaker.check_execution_safety(
                strategy_result.expected_profit_usd,
                gas_cost_usd,
                strategy_result.token_path.len()
            ).await?
        };
        
        if !safety_check.is_safe {
            warn!("🚨 SAFETY CIRCUIT BREAKER BLOCKED EXECUTION: {}", 
                  safety_check.blocked_reason.unwrap_or("Safety concerns".to_string()));
            // Record this as a blocked opportunity in metrics
            return Ok(None);
        }
        
        if !safety_check.warnings.is_empty() {
            warn!("Safety warnings for execution: {:?}", safety_check.warnings);
        }
        
        
        // Determine execution mode
        let execution_mode = self.determine_execution_mode(&strategy_result, None).await?;
        
        // Execute if profitable  
        let execution_result = self.execution_engine
            .execute_strategy(&strategy_result, execution_mode.clone()).await?;
        
        if execution_result.success {
            // Record successful execution in safety circuit breaker
            {
                let mut safety_breaker = self.safety_breaker.write();
                safety_breaker.record_success(execution_result.profit_usd).await;
            }
            
            let mut metrics = self.metrics.write();
            metrics.opportunities_executed += 1;
            metrics.total_profit_usd += execution_result.profit_usd;
            metrics.total_gas_cost_usd += execution_result.gas_cost_usd;
            
            // Update strategy-specific metrics
            match strategy_type {
                StrategyType::Compound => metrics.compound_arbitrage_count += 1,
                _ => {}
            }
            
            match execution_mode {
                ExecutionMode::FlashLoan => metrics.flash_loan_count += 1,
                ExecutionMode::Capital => metrics.capital_mode_count += 1,
            }
            
            // Update success rate
            metrics.success_rate = metrics.opportunities_executed as f64 / metrics.opportunities_received as f64;
            
            info!("✅ Arbitrage executed successfully: profit=${:.2}, gas=${:.2}", 
                  execution_result.profit_usd, execution_result.gas_cost_usd);
        } else {
            // Record execution failure in safety circuit breaker
            {
                let mut safety_breaker = self.safety_breaker.write();
                safety_breaker.record_failure("execution_failed").await;
            }
            
            error!("❌ Arbitrage execution failed");
        }
        
        let processing_time = start_time.elapsed()
            .unwrap_or_else(|_| Duration::from_millis(0))
            .as_millis() as u64;
        
        let processed = ProcessedOpportunity {
            original: opportunity,
            strategy_type,
            execution_mode,
            simulated_profit_usd: strategy_result.expected_profit_usd,
            confidence_score: 0.8,
            token_path: strategy_result.token_path.clone(),
            dex_path: strategy_result.dex_path.clone(),
            gas_estimate: strategy_result.gas_estimate,
            processing_time_ms: processing_time,
        };
        
        Ok(Some(processed))
    }
    
    /// Validate basic opportunity requirements
    async fn is_opportunity_valid(&self, opportunity: &ArbitrageOpportunity) -> bool {
        // Check age
        let now = SystemTime::now().duration_since(UNIX_EPOCH)
            .unwrap_or_default().as_nanos() as u64;
        let age_ms = (now - opportunity.timestamp_ns) / 1_000_000;
        
        if age_ms > self.config.max_opportunity_age_ms {
            return false;
        }
        
        // Check minimum profit
        if opportunity.estimated_profit_usd < self.config.min_profit_usd {
            return false;
        }
        
        // Check gas costs using LIVE MATIC price (not hardcoded $0.80)
        let estimated_gas_cost = match self.calculate_live_gas_cost_usd(opportunity.gas_estimate).await {
            Ok(cost) => cost,
            Err(e) => {
                warn!("Failed to get live gas cost, using fallback: {}", e);
                // Fallback calculation if price oracle fails
                opportunity.gas_estimate as f64 * self.config.gas_price_gwei * 1e-9 * 1.0 // Assume $1 MATIC as safe fallback
            }
        };
        
        if estimated_gas_cost > self.config.max_gas_cost_usd {
            return false;
        }
        
        true
    }
    
    /// Select the best strategy for an opportunity
    async fn select_strategy(&self, opportunity: &ArbitrageOpportunity) -> Result<StrategyType> {
        // Strategy selection logic based on opportunity characteristics
        
        // Check if compound arbitrage is enabled and profitable
        if self.config.compound_enabled && 
           opportunity.estimated_profit_usd >= self.config.min_compound_profit_usd {
            return Ok(StrategyType::Compound);
        }
        
        // Check token pair characteristics
        let token_count = self.estimate_token_path_length(opportunity).await?;
        
        match token_count {
            2 => Ok(StrategyType::Simple),
            3 => Ok(StrategyType::Triangular),
            4..=15 if self.config.compound_enabled => Ok(StrategyType::Compound),
            _ => Ok(StrategyType::Simple), // Fallback
        }
    }
    
    /// Estimate token path length for strategy selection
    async fn estimate_token_path_length(&self, _opportunity: &ArbitrageOpportunity) -> Result<usize> {
        // Simplified logic - in production this would analyze the actual token graph
        // For now, assume most opportunities are simple 2-token arbitrage
        Ok(2)
    }
    
    /// Determine execution mode based on strategy and simulation results
    async fn determine_execution_mode(
        &self,
        strategy_result: &StrategyResult,
        _simulation_result: Option<()>,
    ) -> Result<ExecutionMode> {
        // Default to flash loan mode
        let mut mode = ExecutionMode::FlashLoan;
        
        // Fall back to capital mode if:
        // 1. Flash loans disabled in config
        // 2. Trade size too small for flash loan fees
        // 3. High confidence in capital mode execution
        
        if !self.config.flash_loans_enabled {
            mode = ExecutionMode::Capital;
        } else if strategy_result.expected_profit_usd < self.config.min_flash_loan_profit_usd {
            mode = ExecutionMode::Capital;
        }
        
        Ok(mode)
    }
    
    /// Calculate gas cost in USD using live MATIC price
    async fn calculate_live_gas_cost_usd(&self, gas_estimate: u64) -> Result<f64> {
        let price_manager = self.price_manager.read();
        let oracle = price_manager.get_oracle();
        
        // Get live MATIC price 
        let matic_price = oracle.get_live_matic_price().await
            .context("Failed to get live MATIC price")?;
        
        // Get live gas prices
        let gas_prices = oracle.get_live_gas_prices().await
            .context("Failed to get live gas prices")?;
        
        // Calculate gas cost in USD using live data
        let gas_cost_matic = (gas_estimate as f64) * gas_prices.fast * 1e-9;
        let gas_cost_usd = gas_cost_matic * matic_price;
        
        debug!("Live gas cost calculation: gas={}, gas_price={:.1} gwei, matic_price=${:.4}, cost=${:.4}", 
               gas_estimate, gas_prices.fast, matic_price, gas_cost_usd);
        
        Ok(gas_cost_usd)
    }
    
    /// Get current metrics
    pub fn get_metrics(&self) -> ArbitrageMetrics {
        self.metrics.read().clone()
    }
    
    /// Start the arbitrage engine with opportunity stream
    pub async fn run(&self, mut opportunity_receiver: mpsc::Receiver<ArbitrageOpportunity>) -> Result<()> {
        info!("ArbitrageEngine starting...");
        
        while let Some(opportunity) = opportunity_receiver.recv().await {
            if let Err(e) = self.process_opportunity(opportunity).await {
                error!("Failed to process opportunity: {}", e);
            }
        }
        
        warn!("ArbitrageEngine stopping - opportunity stream closed");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;
    
    #[tokio::test]
    async fn test_arbitrage_engine_creation() {
        let config = ArbitrageConfig::default();
        let result = ArbitrageEngine::new(config).await;
        assert!(result.is_ok());
    }
}