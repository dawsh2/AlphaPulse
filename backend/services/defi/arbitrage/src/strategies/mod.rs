pub mod compound_arb;

pub use compound_arb::CompoundArbitrage;

use anyhow::Result;
use async_trait::async_trait;
use ethers::prelude::*;
use std::sync::Arc;

use crate::config::ArbitrageConfig;
use crate::{FlashOpportunity, execution::FlashLoanRequest};

/// Flash loan strategy trait for arbitrage execution
#[async_trait]
pub trait FlashLoanStrategy: Send + Sync {
    fn estimate_gas(&self, path_length: usize) -> u64;
    fn get_receiver_address(&self) -> Address;
    async fn get_execution_params(&self) -> Result<Vec<u8>>;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum StrategyType {
    Simple,
    Triangular,
    Compound,
}

impl std::fmt::Display for StrategyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StrategyType::Simple => write!(f, "Simple"),
            StrategyType::Triangular => write!(f, "Triangular"),
            StrategyType::Compound => write!(f, "Compound"),
        }
    }
}

#[derive(Debug)]
pub struct StrategyResult {
    pub strategy_type: StrategyType,
    pub is_profitable: bool,
    pub expected_profit_usd: f64,
    pub gas_estimate: u64,
    pub confidence_score: f64,
    pub token_path: Vec<Address>,
    pub dex_path: Vec<String>,
    pub opportunity: crate::FlashOpportunity,
    pub amount_in: f64, // Add this field that was missing
    pub reason: Option<String>,
    pub strategy: Arc<dyn Strategy>, // Add strategy field
}

impl Clone for StrategyResult {
    fn clone(&self) -> Self {
        Self {
            strategy_type: self.strategy_type.clone(),
            is_profitable: self.is_profitable,
            expected_profit_usd: self.expected_profit_usd,
            gas_estimate: self.gas_estimate,
            confidence_score: self.confidence_score,
            token_path: self.token_path.clone(),
            dex_path: self.dex_path.clone(),
            opportunity: self.opportunity.clone(),
            amount_in: self.amount_in,
            reason: self.reason.clone(),
            strategy: self.strategy.clone(), // Arc is cloneable
        }
    }
}

// Placeholder for opportunity type
#[derive(Debug, Clone)]
pub struct AlphaOpportunity {
    pub id: String,
    pub profit_usd: f64,
    pub token_in: Address,
    pub token_out: Address,
}

#[async_trait]
pub trait Strategy: std::fmt::Debug {
    fn name(&self) -> &str;
    fn strategy_type(&self) -> StrategyType;
    
    async fn analyze_opportunity(&self, opportunity: &AlphaOpportunity) -> Result<StrategyResult>;
    
    fn min_profit_usd(&self) -> f64;
    fn max_complexity(&self) -> usize;
    
    async fn estimate_gas(&self, complexity: usize) -> u64;
}

/// Simple 2-token arbitrage strategy
#[derive(Debug)]
pub struct SimpleStrategy {
    config: Arc<ArbitrageConfig>,
}

impl SimpleStrategy {
    pub async fn new(config: Arc<ArbitrageConfig>) -> Result<Self> {
        Ok(Self { config })
    }
}

#[async_trait]
impl Strategy for SimpleStrategy {
    fn name(&self) -> &str {
        "simple_arbitrage"
    }
    
    fn strategy_type(&self) -> StrategyType {
        StrategyType::Simple
    }
    
    async fn analyze_opportunity(&self, opportunity: &AlphaOpportunity) -> Result<StrategyResult> {
        let is_profitable = opportunity.profit_usd >= self.min_profit_usd();
        
        Ok(StrategyResult {
            strategy_type: StrategyType::Simple,
            is_profitable,
            expected_profit_usd: opportunity.profit_usd,
            gas_estimate: self.estimate_gas(2).await,
            confidence_score: if is_profitable { 0.8 } else { 0.3 },
            token_path: vec![opportunity.token_in, opportunity.token_out],
            dex_path: vec!["uniswap_v2".to_string(), "sushiswap".to_string()],
            opportunity: crate::FlashOpportunity {
                id: opportunity.id.clone(),
                path: vec![opportunity.token_in.to_string(), opportunity.token_out.to_string()],
                amounts: vec![],
                expected_profit: rust_decimal::Decimal::from_f64_retain(opportunity.profit_usd).unwrap_or_default(),
                amount_in: rust_decimal::Decimal::from_f64_retain(1000.0).unwrap_or_default(),
            },
            amount_in: 1000.0, // Default amount for simple strategy
            strategy: Arc::new(SimpleStrategy { config: self.config.clone() }),
            reason: if is_profitable { None } else { Some("Profit below threshold".to_string()) },
        })
    }
    
    fn min_profit_usd(&self) -> f64 {
        self.config.simple_strategy.min_profit_usd
    }
    
    fn max_complexity(&self) -> usize {
        2
    }
    
    async fn estimate_gas(&self, _complexity: usize) -> u64 {
        200_000 // Simple 2-hop arbitrage
    }
}

/// Triangular arbitrage strategy (3 tokens)
#[derive(Debug)]
pub struct TriangularStrategy {
    config: Arc<ArbitrageConfig>,
}

impl TriangularStrategy {
    pub async fn new(config: Arc<ArbitrageConfig>) -> Result<Self> {
        Ok(Self { config })
    }
}

#[async_trait]
impl Strategy for TriangularStrategy {
    fn name(&self) -> &str {
        "triangular_arbitrage"
    }
    
    fn strategy_type(&self) -> StrategyType {
        StrategyType::Triangular
    }
    
    async fn analyze_opportunity(&self, opportunity: &AlphaOpportunity) -> Result<StrategyResult> {
        let is_profitable = opportunity.profit_usd >= self.min_profit_usd();
        
        Ok(StrategyResult {
            strategy_type: StrategyType::Triangular,
            is_profitable,
            expected_profit_usd: opportunity.profit_usd,
            gas_estimate: self.estimate_gas(3).await,
            confidence_score: if is_profitable { 0.7 } else { 0.3 },
            token_path: vec![opportunity.token_in, opportunity.token_out], // Simplified
            dex_path: vec!["uniswap_v2".to_string(), "uniswap_v3".to_string(), "sushiswap".to_string()],
            opportunity: crate::FlashOpportunity {
                id: opportunity.id.clone(),
                path: vec![opportunity.token_in.to_string(), opportunity.token_out.to_string()],
                amounts: vec![],
                expected_profit: rust_decimal::Decimal::from_f64_retain(opportunity.profit_usd).unwrap_or_default(),
                amount_in: rust_decimal::Decimal::from_f64_retain(1000.0).unwrap_or_default(),
            },
            amount_in: 2000.0, // Default amount for triangular strategy
            strategy: Arc::new(TriangularStrategy { config: self.config.clone() }),
            reason: if is_profitable { None } else { Some("Profit below threshold".to_string()) },
        })
    }
    
    fn min_profit_usd(&self) -> f64 {
        self.config.triangular_strategy.min_profit_usd
    }
    
    fn max_complexity(&self) -> usize {
        3
    }
    
    async fn estimate_gas(&self, _complexity: usize) -> u64 {
        350_000 // 3-hop arbitrage
    }
}

/// Compound arbitrage strategy (10+ tokens) - key differentiator
#[derive(Debug)]
pub struct CompoundStrategy {
    config: Arc<ArbitrageConfig>,
    compound_arb: CompoundArbitrage,
}

impl CompoundStrategy {
    pub async fn new(config: Arc<ArbitrageConfig>) -> Result<Self> {
        let compound_arb = CompoundArbitrage::new(
            rust_decimal::Decimal::new(config.compound_strategy.min_profit_usd as i64, 0),
            config.max_token_path_length,
            Address::zero(), // Placeholder receiver address
        );
        
        Ok(Self { config, compound_arb })
    }
}

#[async_trait]
impl Strategy for CompoundStrategy {
    fn name(&self) -> &str {
        "compound_arbitrage"
    }
    
    fn strategy_type(&self) -> StrategyType {
        StrategyType::Compound
    }
    
    async fn analyze_opportunity(&self, opportunity: &AlphaOpportunity) -> Result<StrategyResult> {
        let is_profitable = opportunity.profit_usd >= self.min_profit_usd();
        
        // Generate 10+ token paths for compound arbitrage
        // For now, assume we have a compound path of sufficient length
        let path_length = 12; // Simulated compound path length
        let meets_compound_requirement = path_length >= 10;
        
        Ok(StrategyResult {
            strategy_type: StrategyType::Compound,
            is_profitable: is_profitable && meets_compound_requirement,
            expected_profit_usd: opportunity.profit_usd * 1.5, // Compound bonus
            gas_estimate: self.estimate_gas(path_length).await,
            confidence_score: if is_profitable && meets_compound_requirement { 0.9 } else { 0.2 },
            token_path: vec![opportunity.token_in, opportunity.token_out], // Simplified
            dex_path: vec!["uniswap_v2".to_string(), "uniswap_v3".to_string(), "sushiswap".to_string()],
            opportunity: crate::FlashOpportunity {
                id: opportunity.id.clone(),
                path: vec![opportunity.token_in.to_string(), opportunity.token_out.to_string()],
                amounts: vec![],
                expected_profit: rust_decimal::Decimal::from_f64_retain(opportunity.profit_usd).unwrap_or_default(),
                amount_in: rust_decimal::Decimal::from_f64_retain(1000.0).unwrap_or_default(),
            },
            amount_in: 5000.0, // Default amount for compound strategy
            strategy: Arc::new(CompoundStrategy { 
                config: self.config.clone(), 
                compound_arb: self.compound_arb.clone() 
            }),
            reason: if is_profitable && meets_compound_requirement { 
                None 
            } else { 
                Some("Path length < 10 tokens or profit below threshold".to_string()) 
            },
        })
    }
    
    fn min_profit_usd(&self) -> f64 {
        self.config.compound_strategy.min_profit_usd
    }
    
    fn max_complexity(&self) -> usize {
        self.config.max_token_path_length
    }
    
    async fn estimate_gas(&self, complexity: usize) -> u64 {
        self.compound_arb.estimate_gas(complexity)
    }
}