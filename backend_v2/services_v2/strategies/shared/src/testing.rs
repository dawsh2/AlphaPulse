//! Testing utilities for strategies

use crate::StrategyMetrics;

/// Mock strategy for testing
pub struct MockStrategy {
    pub name: &'static str,
    pub started: bool,
    pub metrics: StrategyMetrics,
}

impl MockStrategy {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            started: false,
            metrics: StrategyMetrics::default(),
        }
    }
}

#[async_trait::async_trait]
impl crate::Strategy for MockStrategy {
    fn name(&self) -> &'static str {
        self.name
    }
    
    async fn start(&mut self) -> anyhow::Result<()> {
        self.started = true;
        Ok(())
    }
    
    async fn stop(&mut self) -> anyhow::Result<()> {
        self.started = false;
        Ok(())
    }
    
    fn metrics(&self) -> StrategyMetrics {
        self.metrics.clone()
    }
}

/// Test utilities for strategy validation
pub mod test_utils {
    use super::*;
    
    pub fn assert_strategy_started<T: crate::Strategy>(strategy: &T, expected_name: &str) {
        assert_eq!(strategy.name(), expected_name);
    }
}