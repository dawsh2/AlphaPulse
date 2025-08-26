# Task Template: Test-Driven Development

## Test-First Task Structure

Every task should include detailed test specifications BEFORE implementation requirements.

```markdown
# Task [TASK-ID]: [Description]
*Branch: `[exact-branch-name]`*
*Test-First Development Required*

## 🔒 Git Enforcement
[Standard enforcement section]

## 🧪 TEST SPECIFICATION (WRITE THESE FIRST)

### Required Test Cases
You MUST write these tests BEFORE any implementation:

#### Unit Tests to Create
1. **Test: `test_[specific_behavior]`**
   ```rust
   #[test]
   fn test_[specific_behavior]() {
       // Given: [initial conditions]
       // When: [action performed]
       // Then: [expected result]
       assert_eq!(actual, expected);
   }
   ```

2. **Test: `test_[error_condition]`**
   ```rust
   #[test]
   fn test_[error_condition]() {
       // Given: [error condition setup]
       // When: [action that should fail]
       // Then: [expected error]
       assert!(result.is_err());
   }
   ```

#### Integration Tests to Create
1. **Test: `test_[integration_scenario]`**
   - Setup: [required components]
   - Action: [end-to-end operation]
   - Validation: [complete workflow]

#### Real Data Tests to Create
1. **Test: `test_with_real_[data_type]`**
   - Data source: [specific exchange/feed]
   - Duration: [test duration]
   - Success criteria: [measurable outcomes]

### Test Files to Create/Modify
- `src/[module]/tests.rs` - Unit tests
- `tests/integration/[feature].rs` - Integration tests
- `tests/real_data/[component].rs` - Real data validation

## 📋 Test-First Development Workflow

### Phase 1: Write Failing Tests (First commit)
```bash
# Create test files with failing tests
git add src/[module]/tests.rs
git commit -m "test: add failing tests for [feature] - TDD red phase"

# Verify tests fail
cargo test [new-test-name]
# MUST show failures - proves tests written first
```

### Phase 2: Minimal Implementation (Second commit)
```bash
# Write just enough code to make tests pass
git add src/[module]/[implementation].rs
git commit -m "feat: minimal implementation for [feature] - TDD green phase"

# Verify tests now pass
cargo test [new-test-name]
# MUST show all green
```

### Phase 3: Refactor & Optimize (Third commit)
```bash
# Improve implementation while keeping tests green
git add src/[module]/[implementation].rs
git commit -m "refactor: optimize [feature] implementation - TDD refactor phase"

# Verify tests still pass
cargo test [new-test-name]
# MUST remain green
```

## 🎯 Acceptance Criteria (Test-Driven)
- [ ] All specified unit tests written and initially failing
- [ ] All specified integration tests written and initially failing
- [ ] Minimal implementation makes all tests pass
- [ ] Refactored implementation maintains test success
- [ ] Real data tests validate production behavior
- [ ] Performance tests confirm targets met

## 📝 Implementation Guidance
[Regular implementation details here - but tests come FIRST]

## 🧪 Real Data Testing Requirements
[Specific real data sources and validation criteria]

## ✅ TDD Checklist for PR
- [ ] Failing tests committed first (red phase evidence)
- [ ] Implementation committed second (green phase evidence)
- [ ] Refactored version committed third (refactor phase evidence)
- [ ] All tests pass in final state
- [ ] Real data validation completed
- [ ] Performance targets maintained
```

## Benefits of Test-First Approach

### Quality Assurance
- **Design Clarity**: Tests force clear specification of expected behavior
- **Edge Case Coverage**: Error conditions defined upfront
- **Regression Prevention**: Comprehensive test suite prevents breakage

### Development Velocity
- **Faster Debugging**: Test failures pinpoint exact issues
- **Confident Refactoring**: Green tests enable safe optimization
- **Parallel Development**: Clear interfaces allow team parallelization

### Production Readiness
- **Real Data Validation**: Tests ensure production compatibility
- **Performance Verification**: Benchmarks built into development cycle
- **Documentation**: Tests serve as executable specifications

## Test-First Examples by Component Type

### Pool Cache Integration
```rust
// Write THIS test first (fails initially)
#[test]
fn test_pool_cache_integration() {
    let collector = UnifiedPolygonCollector::new(test_config()).await?;
    let unknown_pool = H160::random();

    // Should trigger background discovery
    let result = collector.process_swap_event_for_pool(unknown_pool).await;

    // Initially fails - no integration yet
    assert!(result.is_ok());
    assert!(collector.pool_cache.get(&unknown_pool).await.is_some());
}
```

### Precision Fix
```rust
// Write THIS test first (fails initially)
#[test]
fn test_precision_preserved_in_signal() {
    let opportunity = ArbitrageOpportunity {
        expected_profit_cents: 12345, // $123.45
        // ... other fields
    };

    let signal = SignalOutput::from_opportunity(&opportunity)?;

    // Initially fails - precision lost in conversion
    assert_eq!(signal.expected_profit_q8, 12345 * 1_000_000);
    assert!(signal.expected_profit_q8 > 0); // Must be profitable
}
```

### Performance Optimization
```rust
// Write THIS test first (establishes baseline)
#[test]
fn test_checksum_sampling_performance() {
    let messages = generate_test_messages(10_000);
    let consumer = RelayConsumer::new(sampling_config());

    let start = Instant::now();
    for msg in messages {
        consumer.parse_header_with_sampling(&msg)?;
    }
    let duration = start.elapsed();

    // Initially may fail - performance not optimized yet
    assert!(duration.as_micros() < 35 * 10_000); // <35μs per message
}
```

This approach ensures every feature is fully tested before implementation begins!
