# INTEGRITY-001: Fix Hardcoded Signal Data

## Task Overview
**Sprint**: 003-data-integrity
**Priority**: CRITICAL - PRODUCTION EMERGENCY
**Estimate**: 4 hours
**Status**: COMPLETE
**Blocker**: Dashboard showing completely fake data to users

## Problem
The `send_arbitrage_analysis()` function sends hardcoded mock data instead of real arbitrage opportunities. Users see fabricated profits, venues, and tokens.

## File Location
`services_v2/strategies/flash_arbitrage/src/signal_output.rs` (lines 159-256)

## Current State (UNACCEPTABLE)
```rust
// Line 159-256: Complete fabrication
gas_cost_usd: 2.50,  // Hardcoded!
venues: vec!["Uniswap V3".to_string(), "SushiSwap V2".to_string()], // Fake!
tokens: vec!["WETH".to_string(), "USDC".to_string()], // Made up!
```

## Required Implementation

### Step 1: Use Real Opportunity Data
```rust
// BEFORE: Lies
let signal = ArbitrageSignalTLV {
    gas_cost_usd: 2.50, // FAKE
    profit_usd: 150.0,  // FAKE
    ...
};

// AFTER: Truth
let signal = ArbitrageSignalTLV {
    gas_cost_usd: opportunity.estimated_gas_cost_usd,
    profit_usd: opportunity.profit_usd,
    capital_required_usd: opportunity.capital_required_usd,
    ...
};
```

### Step 2: Map Pool Addresses to Venues
```rust
// Use PoolCache to get real venue names
let venue_a = pool_cache.get_pool_info(opportunity.pool_a_address)
    .map(|info| info.venue_name())
    .unwrap_or("Unknown");

let venue_b = pool_cache.get_pool_info(opportunity.pool_b_address)
    .map(|info| info.venue_name())
    .unwrap_or("Unknown");
```

### Step 3: Extract Real Token Symbols
```rust
// Get actual tokens from opportunity
let tokens = vec![
    opportunity.token_in_symbol.clone(),
    opportunity.token_out_symbol.clone(),
];
```

### Step 4: Calculate Real Gas Costs
```rust
// Use current network conditions
let gas_price = get_current_gas_price().await?;
let estimated_gas = ARBITRAGE_GAS_ESTIMATE; // ~300k for flash loan
let gas_cost_usd = (gas_price * estimated_gas) * eth_price_usd;
```

## TDD Test Cases

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_hardcoded_values() {
        let opportunity = create_test_opportunity();
        let signal = create_arbitrage_signal(&opportunity);

        // Signal must match opportunity
        assert_eq!(signal.profit_usd, opportunity.profit_usd);
        assert_eq!(signal.capital_required_usd, opportunity.capital_required_usd);
        assert!(signal.gas_cost_usd > 0.0 && signal.gas_cost_usd < 1000.0);
    }

    #[test]
    fn test_real_venue_mapping() {
        let opportunity = create_opportunity_with_pools(
            UNISWAP_V3_POOL_ADDRESS,
            SUSHISWAP_POOL_ADDRESS
        );
        let signal = create_arbitrage_signal(&opportunity);

        assert!(signal.venues.contains(&"Uniswap V3".to_string()));
        assert!(signal.venues.contains(&"SushiSwap".to_string()));
    }

    #[test]
    fn test_real_token_extraction() {
        let opportunity = ArbitrageOpportunity {
            token_in_symbol: "WETH".to_string(),
            token_out_symbol: "USDC".to_string(),
            ..Default::default()
        };
        let signal = create_arbitrage_signal(&opportunity);

        assert_eq!(signal.tokens, vec!["WETH", "USDC"]);
    }
}
```

## Validation Checklist
- [ ] No hardcoded numeric values
- [ ] No hardcoded venue names
- [ ] No hardcoded token symbols
- [ ] Gas cost calculated from network state
- [ ] All data sourced from ArbitrageOpportunity
- [ ] PoolCache used for venue resolution
- [ ] Tests verify no mock data

## Emergency Fix Priority
This MUST be fixed immediately. Users are making decisions based on completely false data. This is a breach of trust and potentially legally problematic.

## Definition of Done
- All hardcoded values removed
- Signal data matches actual opportunity data
- Tests prove no mock data remains
- Dashboard displays real arbitrage information
- Code review confirms no deception
