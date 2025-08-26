# Task Template with Testing Requirements

---
task_id: [CATEGORY]-[NUMBER]
status: TODO
priority: [CRITICAL|HIGH|MEDIUM|LOW]
estimated_hours: [1-8]
branch: [branch-name]
dependencies: [list dependencies if any]
---

## Task Description
[Clear, concise description of what needs to be done]

## Definition of Done
- [ ] **Unit Tests**: All new functions have comprehensive unit tests (Layer 1)
- [ ] **Integration Tests**: Component interactions are tested (Layer 2) 
- [ ] **Property Tests**: Mathematical properties validated where applicable
- [ ] **Fuzz Tests**: Input validation and security testing for parsers
- [ ] **E2E Tests**: Golden path test added if changing core pipeline
- [ ] **Performance Tests**: Critical path latency verified (<35μs)
- [ ] **Precision Tests**: Financial calculations maintain exact precision
- [ ] All existing tests pass: `cargo test --workspace`
- [ ] Protocol V2 tests pass: `cargo test --package protocol_v2`
- [ ] Code coverage >80% on critical paths
- [ ] Documentation updated with examples
- [ ] Breaking changes documented
- [ ] Performance impact measured and acceptable

## Testing Strategy

### Layer 1: Unit Tests (Required)
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_[specific_behavior]() {
        // Given: Setup conditions
        let input = create_test_input();
        
        // When: Execute function
        let result = function_under_test(input);
        
        // Then: Assert expected outcome
        assert_eq!(result, expected_value);
    }
    
    #[test]
    fn test_[edge_case]() {
        // Test boundary conditions, error cases, etc.
    }
}
```

### Layer 2: Integration Tests (If applicable)
```rust
// In tests/integration/[module]_test.rs
#[tokio::test]
async fn test_[component_interaction]() {
    // Test how components work together
    let system = setup_test_system().await;
    let result = system.process_message(test_message).await;
    assert_eq!(result.status, "success");
}
```

### Layer 3: E2E Tests (For core pipeline changes)
```rust
// In tests/e2e/golden_path/[feature]_test.rs
#[tokio::test]
async fn test_[feature]_golden_path() {
    let framework = GoldenPathTestFramework::new().await;
    framework.inject_test_scenario(scenario).await;
    let result = framework.validate_output().await;
    assert!(result.profit > 0.0); // Would catch hardcoded values!
}
```

### Property-Based Tests (For financial calculations)
```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_[mathematical_property](
        input in valid_input_strategy()
    ) {
        let result = calculate_function(input);
        // Assert mathematical invariant
        prop_assert!(result >= 0.0, "Result should never be negative");
    }
}
```

### Fuzz Tests (For parsers and validation)
```rust
pub fn fuzz_[parser_function](data: &[u8]) -> Result<(), String> {
    match parse_function(data) {
        Ok(_) => Ok(()),
        Err(expected_error) => Ok(()), // Expected for malformed input
        Err(unexpected) => Err(format!("Unexpected error: {:?}", unexpected)),
    }
}
```

## Implementation Notes

### Critical Testing Guidelines
1. **No Mocks Ever**: Test with real connections and data
2. **Precision First**: Validate exact decimal calculations
3. **Performance Aware**: Test hot path latency requirements
4. **Security Focused**: Fuzz test all input validation
5. **Real Data**: Use actual exchange data when possible

### Test Data Management
- Use deterministic test data for reproducible results
- Capture real market data for replay testing
- Never hardcode expected profits or prices
- Test edge cases: zero values, maximum values, overflow conditions

### Performance Testing
- Measure critical path latency with `std::time::Instant`
- Assert performance requirements: `assert!(duration < Duration::from_micros(35))`
- Test throughput: >1M msg/s construction, >1.6M msg/s parsing
- Monitor memory usage and allocations

### Financial Testing Specifics
- Test with native token precision (18 decimals WETH, 6 USDC)
- Validate 8-decimal fixed-point USD prices
- Test arithmetic overflow/underflow conditions
- Verify no precision loss in conversions
- Test dust amount handling

## Common Test Patterns

### Protocol V2 Message Testing
```rust
use protocol_v2::{TLVMessageBuilder, parse_header, RelayDomain, SourceType};

#[test]
fn test_message_roundtrip() {
    let mut builder = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::Dashboard);
    builder.add_tlv(1, &test_data);
    let message = builder.build();
    
    let header = parse_header(&message).expect("Should parse header");
    assert_eq!(header.relay_domain, RelayDomain::MarketData);
}
```

### Arbitrage Calculation Testing
```rust
#[test]
fn test_arbitrage_profit_calculation() {
    let pool_a = create_pool_state(reserve0, reserve1, fee);
    let pool_b = create_pool_state(reserve0_b, reserve1_b, fee_b);
    
    let result = calculate_arbitrage_profit(&pool_a, &pool_b);
    
    // NEVER hardcode expected profit - calculate it
    let expected = calculate_expected_profit_manually(&pool_a, &pool_b);
    let tolerance = expected * 0.01; // 1% tolerance
    
    assert!((result.profit - expected).abs() < tolerance,
           "Profit calculation mismatch: got {}, expected {}", result.profit, expected);
}
```

### Relay Communication Testing
```rust
#[tokio::test]
async fn test_relay_message_routing() {
    let relay = start_test_relay().await;
    let consumer = connect_test_consumer(&relay).await;
    
    relay.send_message(test_tlv_message).await;
    
    let received = consumer.receive_message().await.expect("Should receive message");
    let parsed = parse_tlv_message(&received).expect("Should parse TLV");
    
    assert_eq!(parsed.message_type, expected_type);
}
```

## Test Organization
```
[module]/
├── src/
│   └── lib.rs
├── tests/
│   ├── unit/           # Layer 1: Fast, isolated tests
│   ├── integration/    # Layer 2: Component interaction  
│   └── e2e/           # Layer 3: Full pipeline tests
├── benches/           # Performance benchmarks
└── fuzz/             # Fuzz testing targets
```

## CI/CD Integration
- Tests run on every commit
- Performance regression detection
- Coverage reporting (target >80% on critical paths)
- Fuzz testing in extended CI runs
- E2E tests run on release branches

## Review Checklist
Before marking task as complete:
- [ ] All test categories implemented as appropriate
- [ ] Tests follow established patterns and naming
- [ ] No hardcoded values in financial calculations
- [ ] Performance requirements verified
- [ ] Error cases and edge conditions covered
- [ ] Documentation includes testing examples
- [ ] CI pipeline updated if needed

---

**Remember**: Quality over speed. Proper testing prevents production issues and builds confidence in the system's reliability.