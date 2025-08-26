# SAFETY-001: Re-enable Profitability Guards

## Task Overview
**Sprint**: 003-data-integrity
**Priority**: HIGH - PREVENTS FINANCIAL LOSSES
**Estimate**: 4 hours
**Status**: TODO
**Blocker**: System can execute unprofitable trades!

## Problem
Profitability validation is commented out, allowing the system to execute trades that lose money. Mock prices ($1 per token) make every trade look profitable.

## File Location
`services_v2/strategies/flash_arbitrage/src/detector.rs`

## Current State (DANGEROUS)
```rust
// Lines ~200-250: Guards DISABLED!
fn check_arbitrage_opportunity_native(&self, pool_a: &Pool, pool_b: &Pool) {
    // if profit_usd < self.config.min_profit_usd {
    //     return None; // COMMENTED OUT!
    // }

    // Using mock prices
    let token_price = 1.0; // $1 per token - WRONG!
}
```

## Required Implementation

### Step 1: Re-enable Guards
```rust
fn check_arbitrage_opportunity_native(
    &self,
    pool_a: &Pool,
    pool_b: &Pool,
    market_data: &MarketDataState,
) -> Option<ArbitrageOpportunity> {
    // Calculate real profit
    let profit_native = output_amount - input_amount;
    let token_price_usd = market_data.get_price(&token_address)?;
    let profit_usd = profit_native * token_price_usd;

    // CRITICAL: Enforce minimum profit
    if profit_usd < self.config.min_profit_usd {
        debug!("Rejecting opportunity: profit ${} below minimum ${}",
               profit_usd, self.config.min_profit_usd);
        return None;
    }

    // CRITICAL: Check gas costs
    let gas_cost_usd = self.estimate_gas_cost_usd(market_data);
    if gas_cost_usd > self.config.max_gas_cost_usd {
        debug!("Rejecting opportunity: gas ${} above maximum ${}",
               gas_cost_usd, self.config.max_gas_cost_usd);
        return None;
    }

    // CRITICAL: Net profit check
    let net_profit_usd = profit_usd - gas_cost_usd;
    if net_profit_usd < self.config.min_net_profit_usd {
        debug!("Rejecting opportunity: net profit ${} below minimum",
               net_profit_usd);
        return None;
    }

    Some(opportunity)
}
```

### Step 2: Add Configuration
```rust
#[derive(Debug, Clone)]
pub struct ArbitrageConfig {
    pub min_profit_usd: Decimal,      // e.g., 10.0
    pub max_gas_cost_usd: Decimal,    // e.g., 50.0
    pub min_net_profit_usd: Decimal,  // e.g., 5.0
    pub slippage_tolerance: Decimal,  // e.g., 0.005 (0.5%)
}

impl Default for ArbitrageConfig {
    fn default() -> Self {
        Self {
            min_profit_usd: Decimal::from(10),
            max_gas_cost_usd: Decimal::from(50),
            min_net_profit_usd: Decimal::from(5),
            slippage_tolerance: Decimal::from_str("0.005").unwrap(),
        }
    }
}
```

### Step 3: Get Real Prices
```rust
// Remove ALL mock prices
fn get_token_price_usd(&self, token: &Address) -> Decimal {
    // NOT THIS:
    // return Decimal::from(1); // $1 per token - WRONG!

    // THIS:
    self.market_data_state
        .get_latest_price(token)
        .unwrap_or_else(|| {
            error!("No price for token {:?}", token);
            Decimal::ZERO // Better to reject than use fake price
        })
}
```

## TDD Test Cases

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unprofitable_opportunity_rejected() {
        let detector = ArbitrageDetector::new(ArbitrageConfig {
            min_profit_usd: Decimal::from(10),
            ..Default::default()
        });

        let opportunity = detector.check_arbitrage_opportunity_native(
            &pool_with_small_spread(), // Only $5 profit
            &other_pool(),
            &market_data,
        );

        assert!(opportunity.is_none(), "Should reject unprofitable trade");
    }

    #[test]
    fn test_high_gas_rejected() {
        let detector = ArbitrageDetector::new(ArbitrageConfig {
            max_gas_cost_usd: Decimal::from(50),
            ..Default::default()
        });

        // Simulate high gas prices
        let mut market_data = create_test_market_data();
        market_data.set_gas_price(parse_units("200", "gwei")); // Very high

        let opportunity = detector.check_arbitrage_opportunity_native(
            &profitable_pool(),
            &other_pool(),
            &market_data,
        );

        assert!(opportunity.is_none(), "Should reject high gas cost");
    }

    #[test]
    fn test_net_profit_check() {
        let detector = ArbitrageDetector::new(ArbitrageConfig {
            min_net_profit_usd: Decimal::from(5),
            ..Default::default()
        });

        // $10 profit - $6 gas = $4 net (below minimum)
        let opportunity = create_opportunity_with_profit_and_gas(10.0, 6.0);
        let validated = detector.validate_opportunity(opportunity);

        assert!(validated.is_none(), "Should reject low net profit");
    }

    #[test]
    fn test_no_mock_prices() {
        let detector = ArbitrageDetector::new_with_market_data(real_market_data());

        // Should use real prices, not $1
        let weth_price = detector.get_token_price_usd(&WETH_ADDRESS);
        assert!(weth_price > Decimal::from(1000), "WETH should be >$1000");

        let usdc_price = detector.get_token_price_usd(&USDC_ADDRESS);
        assert!(usdc_price > Decimal::from_str("0.99").unwrap());
        assert!(usdc_price < Decimal::from_str("1.01").unwrap());
    }
}
```

## Configuration File
```toml
# config/arbitrage.toml
[profitability]
min_profit_usd = 10.0
max_gas_cost_usd = 50.0
min_net_profit_usd = 5.0
slippage_tolerance = 0.005

[safety]
max_position_size_usd = 100000.0
max_daily_trades = 1000
circuit_breaker_loss_usd = 1000.0
```

## Validation Checklist
- [ ] All profitability checks uncommented
- [ ] Real token prices from market data
- [ ] Gas costs calculated from network state
- [ ] Configuration loaded from file/env
- [ ] Logging for rejected opportunities
- [ ] Tests verify guards work
- [ ] No hardcoded $1 prices anywhere

## Why This is Critical
Without these guards, the system will:
1. Execute trades that lose money
2. Burn gas on unprofitable opportunities
3. Potentially drain capital on bad trades
4. Violate user trust by losing their funds

## Definition of Done
- All guards re-enabled and working
- Real market prices used throughout
- Configuration externalized
- Comprehensive test coverage
- Logs show rejected unprofitable trades
- Zero mock prices in codebase
