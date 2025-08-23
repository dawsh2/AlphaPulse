use anyhow::Result;
use std::time::{SystemTime, UNIX_EPOCH};

/// Validates arbitrage opportunities to prevent false positives
pub struct ArbitrageValidator {
    min_profit_usd: f64,
    max_price_age_seconds: u64,
    max_price_impact: f64,
}

impl ArbitrageValidator {
    pub fn new() -> Self {
        Self {
            min_profit_usd: 0.01,      // Minimum $0.01 profit after gas
            max_price_age_seconds: 60,  // Prices must be < 1 minute old
            max_price_impact: 0.05,     // Max 5% price impact allowed
        }
    }
    
    /// Validate an arbitrage opportunity
    pub fn validate(&self, opportunity: &ArbitrageOpportunity) -> ValidationResult {
        let mut issues = Vec::new();
        
        // Check profit exceeds gas
        if opportunity.net_profit_usd < self.min_profit_usd {
            issues.push(format!(
                "Insufficient profit: ${:.4} < ${:.4} minimum",
                opportunity.net_profit_usd, self.min_profit_usd
            ));
        }
        
        // Check price data freshness
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let age_seconds = now - opportunity.timestamp;
        if age_seconds > self.max_price_age_seconds {
            issues.push(format!(
                "Stale price data: {} seconds old (max {})",
                age_seconds, self.max_price_age_seconds
            ));
        }
        
        // Check liquidity is sufficient
        if opportunity.tradeable_liquidity_usd < opportunity.optimal_trade_size {
            issues.push(format!(
                "Insufficient liquidity: ${:.2} available < ${:.2} needed",
                opportunity.tradeable_liquidity_usd, opportunity.optimal_trade_size
            ));
        }
        
        // Check price impact
        if opportunity.total_price_impact > self.max_price_impact {
            issues.push(format!(
                "Excessive price impact: {:.2}% > {:.2}% max",
                opportunity.total_price_impact * 100.0, self.max_price_impact * 100.0
            ));
        }
        
        // Check if pools are actually different
        if opportunity.buy_pool == opportunity.sell_pool {
            issues.push("Same pool for buy and sell".to_string());
        }
        
        ValidationResult {
            is_valid: issues.is_empty(),
            issues,
            confidence_score: Self::calculate_confidence(opportunity),
        }
    }
    
    /// Calculate confidence score (0-1) for the opportunity
    fn calculate_confidence(opportunity: &ArbitrageOpportunity) -> f64 {
        let mut score = 1.0;
        
        // Reduce confidence for low profit margins
        let profit_margin = opportunity.net_profit_usd / opportunity.optimal_trade_size;
        if profit_margin < 0.001 {  // Less than 0.1% profit
            score *= 0.5;
        }
        
        // Reduce confidence for high price impact
        if opportunity.total_price_impact > 0.02 {  // More than 2% impact
            score *= 0.7;
        }
        
        // Reduce confidence for V3 pools (estimates less accurate)
        if opportunity.pool_types.contains("V3") {
            score *= 0.8;
        }
        
        // Reduce confidence for very small trades
        if opportunity.optimal_trade_size < 10.0 {  // Less than $10
            score *= 0.6;
        }
        
        score
    }
}

pub struct ArbitrageOpportunity {
    pub buy_pool: String,
    pub sell_pool: String,
    pub pool_types: String,  // e.g., "V2-V2" or "V2-V3"
    pub token_pair: String,
    pub buy_price: f64,
    pub sell_price: f64,
    pub spread_percent: f64,
    pub total_fees_percent: f64,
    pub optimal_trade_size: f64,
    pub gross_profit_usd: f64,
    pub gas_cost_usd: f64,
    pub net_profit_usd: f64,
    pub tradeable_liquidity_usd: f64,
    pub total_price_impact: f64,
    pub timestamp: u64,
}

pub struct ValidationResult {
    pub is_valid: bool,
    pub issues: Vec<String>,
    pub confidence_score: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_validate_profitable_opportunity() {
        let validator = ArbitrageValidator::new();
        let opportunity = ArbitrageOpportunity {
            buy_pool: "0x123...".to_string(),
            sell_pool: "0x456...".to_string(),
            pool_types: "V2-V2".to_string(),
            token_pair: "WMATIC/USDC".to_string(),
            buy_price: 0.65,
            sell_price: 0.66,
            spread_percent: 1.5,
            total_fees_percent: 0.6,
            optimal_trade_size: 100.0,
            gross_profit_usd: 1.0,
            gas_cost_usd: 0.003,
            net_profit_usd: 0.997,
            tradeable_liquidity_usd: 1000.0,
            total_price_impact: 0.01,
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        };
        
        let result = validator.validate(&opportunity);
        assert!(result.is_valid);
        assert!(result.confidence_score > 0.8);
    }
    
    #[test]
    fn test_reject_unprofitable_opportunity() {
        let validator = ArbitrageValidator::new();
        let opportunity = ArbitrageOpportunity {
            buy_pool: "0x123...".to_string(),
            sell_pool: "0x456...".to_string(),
            pool_types: "V2-V2".to_string(),
            token_pair: "WMATIC/USDC".to_string(),
            buy_price: 0.65,
            sell_price: 0.6501,
            spread_percent: 0.015,
            total_fees_percent: 0.6,
            optimal_trade_size: 10.0,
            gross_profit_usd: 0.001,
            gas_cost_usd: 0.003,
            net_profit_usd: -0.002,
            tradeable_liquidity_usd: 100.0,
            total_price_impact: 0.001,
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        };
        
        let result = validator.validate(&opportunity);
        assert!(!result.is_valid);
        assert!(result.issues.iter().any(|i| i.contains("Insufficient profit")));
    }
    
    #[test]
    fn test_reject_high_price_impact() {
        let validator = ArbitrageValidator::new();
        let mut opportunity = ArbitrageOpportunity {
            buy_pool: "0x123...".to_string(),
            sell_pool: "0x456...".to_string(),
            pool_types: "V2-V2".to_string(),
            token_pair: "WMATIC/USDC".to_string(),
            buy_price: 0.65,
            sell_price: 0.70,
            spread_percent: 7.7,
            total_fees_percent: 0.6,
            optimal_trade_size: 1000.0,
            gross_profit_usd: 50.0,
            gas_cost_usd: 0.003,
            net_profit_usd: 49.997,
            tradeable_liquidity_usd: 1000.0,
            total_price_impact: 0.1,  // 10% impact - too high!
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        };
        
        let result = validator.validate(&opportunity);
        assert!(!result.is_valid);
        assert!(result.issues.iter().any(|i| i.contains("Excessive price impact")));
    }
}