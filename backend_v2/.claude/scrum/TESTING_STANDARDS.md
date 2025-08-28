# Torq Testing Standards & Architecture

## 🎯 Testing Philosophy
**Test everything that could break. Test nothing that couldn't.**

Every feature MUST include tests at the appropriate level of the testing pyramid. Tests are not optional - they are part of the definition of done.

## 📐 Testing Pyramid Architecture

```
         /\
        /E2E\       5% - End-to-End Tests (5-10 tests)
       /______\      Slow, expensive, critical paths only
      /        \
     /Integration\  25% - Integration Tests (50+ tests)  
    /______________\  Component boundaries, real dependencies
   /                \
  /    Unit Tests    \ 70% - Unit Tests (200+ tests)
 /____________________\ Fast, isolated, comprehensive
```

## Layer 1: Unit Tests (Foundation)

### Purpose
Test individual functions and methods in complete isolation. These are your first line of defense against bugs.

### Characteristics
- **Speed**: <100ms per test
- **Isolation**: No external dependencies
- **Precision**: Pinpoint exact failures
- **Coverage**: >80% of business logic

### Implementation
```rust
// Location: In source files
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_calculate_arbitrage_profit() {
        // Given: Known market conditions
        let price_a = 3000_000000; // $3000
        let price_b = 3050_000000; // $3050
        let quantity = 1_000000;    // 1 WETH
        
        // When: Calculate profit
        let profit = calculate_profit(price_a, price_b, quantity);
        
        // Then: Verify calculation (NOT hardcoded!)
        assert_eq!(profit, 50_000000); // $50
        assert_ne!(profit, 150_000000); // Not hardcoded $150!
    }
    
    #[test]
    fn test_tlv_serialization_roundtrip() {
        let original = TradeTLV { price: 4500000000000, ..Default::default() };
        let bytes = original.as_bytes();
        let decoded = TradeTLV::from_bytes(&bytes).unwrap();
        assert_eq!(decoded.price, original.price); // No precision loss
    }
}
```

### Required Unit Tests
- All calculation functions
- All parsing/serialization
- All validation logic
- All state transitions
- All error conditions

## Layer 2: Integration Tests

### Purpose
Verify that components work together correctly with real (or realistic) dependencies.

### Characteristics
- **Speed**: <1s per test
- **Dependencies**: Real components, test databases
- **Scope**: Public APIs, service boundaries
- **Coverage**: Key interaction points

### Implementation
```rust
// Location: tests/ directory in crate
// File: tests/relay_integration.rs

#[tokio::test]
async fn test_relay_message_forwarding() {
    // Start real components
    let market_relay = MarketDataRelay::start_test().await;
    let signal_relay = SignalRelay::start_test().await;
    let consumer = signal_relay.connect().await;
    
    // Send real message through system
    let trade = create_test_trade();
    market_relay.publish(trade.to_tlv()).await;
    
    // Verify message received correctly
    let received = consumer.receive().await.unwrap();
    assert_eq!(received.trade_id, trade.trade_id);
}

#[tokio::test]  
async fn test_pool_cache_integration() {
    let cache = PoolCache::new();
    let collector = UnifiedPolygonCollector::new(cache.clone());
    
    // Unknown pool should trigger discovery
    let unknown = H160::from_str("0x...").unwrap();
    collector.process_swap(unknown).await.unwrap();
    
    // Verify pool was discovered and cached
    assert!(cache.get(unknown).await.is_some());
}
```

### Required Integration Tests
- Service-to-service communication
- Database operations
- External API interactions
- Message relay paths
- Cache interactions

## Layer 3: End-to-End Tests

### Purpose
Validate complete user scenarios from start to finish. These catch system-level issues.

### Characteristics  
- **Speed**: <30s per test
- **Scope**: Complete pipeline
- **Data**: Deterministic, controlled
- **Count**: Minimal (5-10 total)

### Implementation
```rust
// Location: tests/e2e/ in root
// File: tests/e2e/arbitrage_detection.rs

#[tokio::test]
async fn test_golden_path_arbitrage() {
    // Start ENTIRE system
    let system = TestSystem::start_all().await;
    
    // Inject known market data at entry
    let test_data = MarketData {
        dex_a_price: 3000_00,
        dex_b_price: 3050_00,
        gas_price: 20_gwei,
    };
    system.inject_at_entry(test_data).await;
    
    // Wait for output signal
    let signal = system.await_arbitrage_signal().await;
    
    // CRITICAL: Verify calculated, not hardcoded!
    let expected = calculate_expected_profit(test_data);
    assert_eq!(signal.profit, expected);
    assert_ne!(signal.profit, 150_00); // Catch hardcoding!
    assert!(signal.profit > 0);
}
```

### Required E2E Tests
- Happy path (profitable arbitrage)
- No opportunity (prices equal)
- Unprofitable (gas too high)
- High load scenario
- Recovery from disconnect

## Specialized Testing for Financial Systems

### Property-Based Testing
Test properties that must ALWAYS hold, with random inputs.

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn profit_never_negative_after_gas(
        price_a in 1i64..10_000_000_000i64,
        price_b in 1i64..10_000_000_000i64,
        gas in 1i64..1000i64,
    ) {
        let opportunity = detect_arbitrage(price_a, price_b, gas);
        if let Some(opp) = opportunity {
            prop_assert!(opp.profit_after_gas >= 0);
        }
    }
    
    #[test]
    fn precision_preserved_in_calculations(
        value in 0i64..i64::MAX/2,
    ) {
        let scaled = scale_to_decimals(value, 18);
        let unscaled = scale_from_decimals(scaled, 18);
        prop_assert_eq!(value, unscaled);
    }
}
```

### Fuzz Testing
Find panics and security issues with malformed input.

```rust
// fuzz/fuzz_targets/tlv_parser.rs
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Must never panic, even with garbage
    let _ = parse_tlv_message(data);
});
```

Run with:
```bash
cargo fuzz run tlv_parser -- -max_len=1024
```

### Market Replay Testing
Test with real historical data.

```rust
#[test]
fn test_replay_market_crash() {
    let replay_data = load_market_data("2024-01-15-flash-crash.json");
    let mut system = TestSystem::new();
    
    for event in replay_data {
        system.process(event);
        assert!(system.is_healthy());
    }
}
```

## Test Data Management

### Never Hardcode Values
```rust
// ❌ WRONG - Hardcoded test data
#[test]
fn test_bad() {
    let profit = 150.0; // Hardcoded!
    assert_eq!(calculate(), profit);
}

// ✅ CORRECT - Calculated test data
#[test] 
fn test_good() {
    let market_data = TestDataBuilder::profitable_scenario();
    let expected = calculate_expected(market_data);
    assert_eq!(calculate(market_data), expected);
}
```

### Use Test Builders
```rust
pub struct TestDataBuilder {
    price: i64,
    quantity: i64,
    timestamp: i64,
}

impl TestDataBuilder {
    pub fn profitable() -> MarketData {
        Self {
            price: 3000_000000,
            quantity: 1_000000,
            timestamp: 1234567890,
        }.build()
    }
}
```

## Coverage Requirements

### Minimum Coverage by Component
- Protocol parsing: >95%
- Financial calculations: >90%
- Business logic: >85%
- State management: >85%
- Utilities: >70%
- UI/formatting: >50%

### How to Measure
```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Run coverage
cargo tarpaulin --out Html --output-dir coverage

# Check specific package
cargo tarpaulin --packages protocol_v2 --lib
```

## CI/CD Requirements

### Pre-commit Hooks
```bash
#!/bin/bash
# .git/hooks/pre-commit

# Run fast unit tests
cargo test --lib --bins

# Check coverage didn't drop
cargo tarpaulin --print-summary
```

### PR Checks (GitHub Actions)
```yaml
name: Tests
on: [pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - name: Unit Tests
        run: cargo test --lib --bins
        
      - name: Integration Tests  
        run: cargo test --test '*'
        
      - name: Coverage Check
        run: |
          cargo tarpaulin --out Xml
          # Fail if <80%
          
      - name: E2E Smoke Test
        run: cargo test --test golden_path
```

### Main Branch Protection
- All tests must pass
- Coverage must not decrease
- At least one review required
- E2E tests run before merge

## Testing Anti-Patterns to Avoid

### ❌ Testing Implementation Details
```rust
// BAD: Tests private internals
#[test]
fn test_internal_state() {
    let obj = MyStruct::new();
    assert_eq!(obj.internal_counter, 0); // Private field!
}
```

### ❌ Slow Unit Tests
```rust
// BAD: Unit test makes network call
#[test]
fn test_fetch_price() {
    let price = fetch_from_api(); // Real network call!
    assert!(price > 0);
}
```

### ❌ Flaky Tests
```rust
// BAD: Time-dependent test
#[test]
fn test_timeout() {
    sleep(Duration::from_secs(1));
    assert!(now() > start + 1); // Might fail on slow CI
}
```

### ❌ No Assertion Tests
```rust
// BAD: Test that can't fail
#[test]
fn test_useless() {
    let _ = calculate_something();
    // No assertions!
}
```

## Test Organization

### Directory Structure
```
crate/
├── src/
│   └── lib.rs          # Unit tests here
├── tests/
│   ├── integration/    # Integration tests
│   └── common/         # Shared test utilities
└── benches/            # Performance tests

project_root/
└── tests/
    └── e2e/            # End-to-end tests
```

### Naming Conventions
- Unit tests: `test_[function]_[scenario]`
- Integration: `test_[feature]_integration`
- E2E: `test_[workflow]_e2e`
- Property: `prop_[invariant]`
- Benchmark: `bench_[operation]`

## Debugging Test Failures

### Run single test
```bash
cargo test test_specific_name
```

### Show output
```bash
cargo test -- --nocapture
```

### Run with logging
```bash
RUST_LOG=debug cargo test
```

### Run in release mode
```bash
cargo test --release
```

## Conclusion

Testing is not optional at Torq. Every PR must include appropriate tests. The testing pyramid ensures fast feedback while catching critical issues. Property-based and fuzz testing catch edge cases humans miss.

**Remember**: A test that could have caught the hardcoded $150 issue is worth 1000 lines of untested code.