---
task_id: TEST-003
status: TODO
priority: CRITICAL
estimated_hours: 4
assigned_branch: test/e2e-golden-path
assignee: TBD
created: 2025-01-27
completed: null
---

# TEST-003: E2E Golden Path Test (Full Pipeline)

## 🔴 CRITICAL INSTRUCTIONS
```bash
# BEFORE STARTING - VERIFY YOU'RE NOT ON MAIN:
git branch --show-current

# If you see "main", IMMEDIATELY run:
git checkout -b test/e2e-golden-path

# NEVER commit directly to main!
```

## Status
**Status**: TODO
**Priority**: CRITICAL
**Branch**: `test/e2e-golden-path`
**Estimated**: 4 hours

## Problem Statement
We lack end-to-end testing that validates the entire pipeline from market data input to arbitrage signal output. This test would have immediately caught the hardcoded $150 profit issue.

## Acceptance Criteria
- [ ] Complete pipeline test from data injection to signal output
- [ ] Uses deterministic test data with known expected results
- [ ] Verifies actual calculations, not hardcoded values
- [ ] Detects any hardcoded data in the pipeline
- [ ] Runs in <30 seconds
- [ ] Can be run in CI/CD

## Technical Approach

### Files to Create
- `tests/e2e/golden_path_test.rs` - Main E2E test
- `tests/e2e/test_system.rs` - Test system setup utilities
- `tests/e2e/test_data.rs` - Deterministic test data

### Implementation Steps

1. **Create test system wrapper**:
```rust
// tests/e2e/test_system.rs
pub struct TestSystem {
    market_relay: MarketDataRelay,
    signal_relay: SignalRelay,
    arbitrage_service: FlashArbitrageService,
    signal_receiver: SignalReceiver,
}

impl TestSystem {
    pub async fn start() -> Result<Self> {
        // Start all components with test config
        let market_relay = MarketDataRelay::start_test().await?;
        let signal_relay = SignalRelay::start_test().await?;
        let arbitrage_service = FlashArbitrageService::start_test().await?;
        
        // Connect signal receiver
        let signal_receiver = signal_relay.connect_receiver().await?;
        
        Ok(Self {
            market_relay,
            signal_relay,
            arbitrage_service,
            signal_receiver,
        })
    }
    
    pub async fn inject_market_data(&self, data: TestMarketData) {
        // Inject at entry point
        self.market_relay.inject(data.to_tlv()).await;
    }
    
    pub async fn await_signal(&mut self) -> ArbitrageSignal {
        // Wait for signal with timeout
        tokio::time::timeout(
            Duration::from_secs(5),
            self.signal_receiver.recv()
        ).await.expect("Signal timeout")
    }
}
```

2. **Create deterministic test data**:
```rust
// tests/e2e/test_data.rs
pub struct TestMarketData {
    pub pool_a_price: i64,  // WETH/USDC on DEX A
    pub pool_b_price: i64,  // WETH/USDC on DEX B
    pub gas_price: i64,
}

impl TestMarketData {
    pub fn profitable_arbitrage() -> Self {
        Self {
            pool_a_price: 3000_000000, // $3000/WETH (6 decimals)
            pool_b_price: 3050_000000, // $3050/WETH (6 decimals)
            gas_price: 20_000000000,   // 20 gwei
        }
    }
    
    pub fn expected_profit(&self) -> i64 {
        // Calculate expected profit
        let price_diff = self.pool_b_price - self.pool_a_price;
        let trade_size = 1_000000; // 1 WETH
        let gross_profit = price_diff * trade_size / 1_000000;
        
        // Subtract gas costs
        let gas_cost = self.calculate_gas_cost();
        gross_profit - gas_cost
    }
}
```

3. **Implement the golden path test**:
```rust
// tests/e2e/golden_path_test.rs
#[tokio::test]
async fn test_golden_path_arbitrage_detection() {
    // Given: Start the complete system
    let mut system = TestSystem::start().await.unwrap();
    
    // And: Prepare deterministic test data
    let test_data = TestMarketData::profitable_arbitrage();
    let expected_profit = test_data.expected_profit();
    
    // When: Inject market data at entry point
    system.inject_market_data(test_data).await;
    
    // Then: Await arbitrage signal
    let signal = system.await_signal().await;
    
    // CRITICAL ASSERTIONS - Would catch hardcoded $150!
    assert_eq!(
        signal.expected_profit, 
        expected_profit,
        "Profit must be calculated, not hardcoded!"
    );
    
    assert!(
        signal.expected_profit > 0,
        "Signal must be profitable after gas"
    );
    
    assert_eq!(
        signal.source_pool, "0x...", // Actual pool address
        "Pool addresses must be real, not 'Pool_A'"
    );
    
    // Verify no hardcoded values
    assert_ne!(signal.expected_profit, 150_00, "Detected hardcoded $150!");
    assert_ne!(signal.gas_cost, 2_50, "Detected hardcoded $2.50 gas!");
}

#[tokio::test]
async fn test_unprofitable_arbitrage_filtered() {
    let mut system = TestSystem::start().await.unwrap();
    
    // Inject unprofitable opportunity
    let test_data = TestMarketData {
        pool_a_price: 3000_000000,
        pool_b_price: 3001_000000, // Only $1 difference
        gas_price: 100_000000000,   // High gas
    };
    
    system.inject_market_data(test_data).await;
    
    // Should timeout - no signal for unprofitable
    let result = tokio::time::timeout(
        Duration::from_secs(2),
        system.await_signal()
    ).await;
    
    assert!(result.is_err(), "Unprofitable trades must be filtered");
}
```

## Testing Instructions
```bash
# Run the E2E test
cargo test --test golden_path_test

# Run with real timing
cargo test --test golden_path_test --release

# Debug output
RUST_LOG=debug cargo test --test golden_path_test -- --nocapture

# Run in CI mode
cargo test --test golden_path_test --no-fail-fast
```

## Git Workflow
```bash
# 1. Start on your branch
git checkout -b test/e2e-golden-path

# 2. Make changes and commit
git add tests/e2e/
git commit -m "test: add E2E golden path test to catch hardcoded data"

# 3. Push to your branch
git push origin test/e2e-golden-path

# 4. Create PR
gh pr create --title "TEST-003: E2E golden path test" \
             --body "Implements full pipeline testing that would catch hardcoded values"
```

## Completion Checklist
- [ ] Working on correct branch (not main)
- [ ] Test system wrapper created
- [ ] Deterministic test data defined
- [ ] Golden path test implemented
- [ ] Test catches hardcoded values
- [ ] Runs in <30 seconds
- [ ] PR created
- [ ] Updated task status to COMPLETE

## Notes
This is THE critical test that ensures our system calculates real values. It would have immediately failed with the hardcoded $150 profit!