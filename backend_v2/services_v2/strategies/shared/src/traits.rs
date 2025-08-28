//! Strategy traits and interfaces

use async_trait::async_trait;
use anyhow::Result;

/// Core strategy trait that all trading strategies must implement
#[async_trait]
pub trait Strategy: Send + Sync {
    /// Strategy name for identification
    fn name(&self) -> &'static str;
    
    /// Start the strategy
    async fn start(&mut self) -> Result<()>;
    
    /// Stop the strategy  
    async fn stop(&mut self) -> Result<()>;
    
    /// Get current strategy metrics
    fn metrics(&self) -> StrategyMetrics;
}

/// Basic strategy metrics
#[derive(Debug, Clone, Default)]
pub struct StrategyMetrics {
    pub messages_processed: u64,
    pub signals_generated: u64,
    pub trades_executed: u64,
    pub errors: u64,
}

/// Strategy configuration trait
pub trait StrategyConfig: Send + Sync + Clone {
    /// Validate configuration
    fn validate(&self) -> Result<()>;
}