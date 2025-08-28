---
task_id: S015-VALIDATE-007
status: TODO  ← CHANGE TO "IN_PROGRESS" WHEN STARTING, THEN "COMPLETE" WHEN FINISHED!
priority: CRITICAL
estimated_hours: 8
assigned_branch: validate/e2e-semantic-equality
assignee: TBD
created: 2025-08-27
completed: null
# Dependencies: All Phase 2 component validations must complete first
depends_on: [S015-VALIDATE-004, S015-VALIDATE-005, S015-VALIDATE-006]
# Blocks: Phase 3 system-wide consistency validation
blocks: [S015-VALIDATE-008]
# Scope: Full end-to-end pipeline validation
scope: ["torq/services/adapters/", "torq/relays/", "torq/libs/", "torq/tests/e2e/"]
---

# VALIDATE-007: End-to-End Semantic Equality Testing

**🚨 CRITICAL**: Update status to COMPLETE when finished!

## 🔴 CRITICAL INSTRUCTIONS

### 0. 📋 MARK AS IN-PROGRESS IMMEDIATELY
**⚠️ FIRST ACTION: Change status when you start work!**
```yaml
# Edit the YAML frontmatter above:
status: TODO → status: IN_PROGRESS

# This makes the kanban board show you're working on it!
```

### 1. Git Worktree Setup (REQUIRED)
```bash
# NEVER use git checkout - it changes all sessions!
# ALWAYS use git worktree for isolated development:
git worktree add -b fix/specific-issue-name ../task-xxx-worktree
cd ../task-xxx-worktree

# Verify you're in the correct worktree:
git branch --show-current  # Should show: fix/specific-issue-name
pwd  # Should show: ../task-xxx-worktree

# NEVER work directly in main repository!
```

### 2. 🧪 TEST-DRIVEN DEVELOPMENT MANDATORY
**⚠️ AGENTS: You MUST write tests BEFORE implementation code!**
```bash
# WORKFLOW: RED → GREEN → REFACTOR
# 1. Write failing test first
# 2. Implement minimal code to pass
# 3. Refactor while keeping tests green
# 4. Repeat for next feature

# DO NOT write implementation without tests first!
```

## Status
**Status**: TODO (⚠️ CHANGE TO IN_PROGRESS WHEN YOU START!)
**Priority**: CRITICAL
**Worktree**: `../task-xxx-worktree` (Branch: `fix/specific-issue-name`)
**Estimated**: 3 hours

## Problem Statement
After the major refactor, we must validate that the entire message pipeline preserves semantic equality end-to-end. Data flowing from JSON input through TLV encoding, binary relay transmission, and back to JSON output must be semantically identical with zero precision loss.

**Critical Risk**: The refactor may have introduced subtle data corruption that only appears in full pipeline flows:
- Precision loss during JSON→TLV→Binary→TLV→JSON conversion
- Semantic changes in data interpretation
- InstrumentId corruption during round-trips
- Timestamp precision truncation
- Financial calculation errors

## Acceptance Criteria
- [ ] Full pipeline semantic equality: JSON→TLV→Binary→TLV→JSON produces identical results
- [ ] Zero precision loss across all numeric types (WETH 18-decimal, USDC 6-decimal, USD 8-decimal)
- [ ] InstrumentId bijection preserved through complete round-trips
- [ ] Timestamp precision maintained (nanosecond accuracy preserved)
- [ ] All exchange-specific data formats handled correctly
- [ ] Market data semantic equality (prices, volumes, timestamps)
- [ ] Signal data semantic equality (arbitrage opportunities, risk metrics)
- [ ] Execution data semantic equality (trade confirmations, gas prices)
- [ ] Performance maintained: >1M msg/s end-to-end throughput
- [ ] Error propagation maintains full context through pipeline

## Technical Approach
### End-to-End Pipeline Components (Post-Refactor Locations)
**FULL PIPELINE PATH**:
1. **Input**: `torq/services/adapters/` - Exchange JSON → TLV conversion
2. **Relay**: `torq/relays/` - TLV → Binary message forwarding  
3. **Output**: Consumer logic - Binary → TLV → JSON conversion
4. **Validation**: `torq/tests/e2e/` - Semantic equality verification

### Key Validation Points
- **Adapter Input**: JSON parsing and TLV construction
- **Relay Transport**: Binary message integrity  
- **Consumer Output**: TLV parsing and JSON reconstruction
- **Round-Trip**: Input JSON ≡ Output JSON (semantic equality)

### Semantic Equality Definition
Two JSON objects are semantically equal if:
- All numerical values preserve precision (no 1.99999999 vs 2.0)
- All timestamps maintain nanosecond accuracy
- All InstrumentIds reconstruct identically
- All nested objects maintain structure and values
- Floating point comparisons use appropriate epsilon

### Implementation Steps - END-TO-END VALIDATION ⚠️
**🚨 CRITICAL: This validates the complete refactored pipeline!**

1. **SETUP**: Initialize end-to-end validation environment
   ```bash
   # Create validation worktree
   git worktree add -b validate/e2e-semantic-equality ../validate-007-worktree
   cd ../validate-007-worktree
   
   # Verify complete pipeline exists
   ls torq/services/adapters/
   ls torq/relays/
   ls torq/tests/e2e/
   ```

2. **BASELINE ESTABLISHMENT**: Capture pre-refactor semantic behavior
   ```bash
   # Run existing e2e tests to establish baselines
   cargo test --package tests --test e2e_collector_relay
   cargo test --package tests --test e2e_live_validation
   cargo test --package tests --test live_e2e_integration
   
   # Document baseline results
   echo "=== E2E BASELINE RESULTS ===" > E2E_BASELINE.md
   ```

3. **ADAPTER SEMANTIC VALIDATION**: JSON→TLV integrity
   ```bash
   # Test all exchange adapters
   cargo test --package polygon_adapter --test integration
   cargo test --package adapters --test e2e_pool_cache_validation
   
   # Precision preservation specific tests
   cargo test --package types precision_validation
   ```

4. **RELAY TRANSPORT VALIDATION**: TLV→Binary→TLV integrity
   ```bash
   # Binary message transport
   cargo test --package relays message_transport
   cargo test --package relays binary_forwarding
   
   # Message integrity during relay
   cargo test --package relays --test relay_integration
   ```

5. **CONSUMER RECONSTRUCTION**: Binary→TLV→JSON integrity
   ```bash
   # Consumer parsing validation
   cargo test --package strategies relay_consumer
   cargo test --package strategies --test integration/relay_consumer_integration
   
   # JSON reconstruction accuracy
   cargo test --package adapters json_reconstruction
   ```

6. **FULL PIPELINE VALIDATION**: Complete semantic equality testing
   ```bash
   # End-to-end golden path testing
   cargo test --package tests --test e2e_golden_path
   
   # Real market data validation (if available)
   cargo test --package tests --test live_polygon_dex
   cargo test --package tests --test pool_events_live
   ```

7. **SEMANTIC EQUALITY VERIFICATION**: Deep equality comparison
   ```bash
   # Custom semantic equality tests
   cargo test --package tests deep_equality_validation
   cargo test --package tests semantic_equality
   
   # Precision boundary testing
   cargo test --package types precision_boundary
   ```

8. **PERFORMANCE VALIDATION**: End-to-end throughput
   ```bash
   # Full pipeline performance testing
   cargo test --package tests --test performance --release
   
   # Must maintain >1M msg/s end-to-end throughput
   # Document in TEST_RESULTS.md
   ```

9. **ERROR PROPAGATION**: End-to-end error handling
   ```bash
   # Error context preservation
   cargo test --package tests error_propagation_e2e
   
   # Malformed input handling
   cargo test --package adapters malformed_input
   ```

## Testing Requirements - Rust Convention

### 🏗️ Both Unit AND Integration Tests Required

Following idiomatic Rust testing practices, implement **both** test types:

#### 1. Unit Tests (REQUIRED) - White-Box Testing
**Location**: Inside `src/` files in `#[cfg(test)] mod tests {}` blocks
**Access**: Can test private and public functions
**Purpose**: Test internal algorithms, edge cases, private function logic

```rust
// Add to: src/[module].rs (same file as the code)
fn internal_helper(data: &Data) -> bool {
    // Private function - only unit tests can access
    data.is_valid() && data.size > 0
}

pub fn public_api(data: Data) -> Result<ProcessedData, Error> {
    if internal_helper(&data) {
        Ok(ProcessedData::from(data))
    } else {
        Err(Error::Invalid)
    }
}

#[cfg(test)]
mod tests {
    use super::*; // Import everything from parent module

    #[test]
    fn test_internal_helper() {
        // We CAN test private functions in unit tests!
        let valid_data = Data { size: 10, ..Default::default() };
        assert!(internal_helper(&valid_data));

        let invalid_data = Data { size: 0, ..Default::default() };
        assert!(!internal_helper(&invalid_data));
    }

    #[test]
    fn test_public_api_success() {
        let data = Data { size: 10, ..Default::default() };
        let result = public_api(data);
        assert!(result.is_ok());
    }

    #[test]
    fn test_public_api_edge_cases() {
        // Test edge cases and error conditions
        let invalid_data = Data { size: 0, ..Default::default() };
        let result = public_api(invalid_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_precision_preservation() {
        // Critical for financial data
        let original_price = 4500000000000i64; // $45,000.00
        let data = Data { price: original_price, ..Default::default() };
        let processed = public_api(data).unwrap();
        assert_eq!(processed.price, original_price); // No precision loss!
    }
}
```

#### 2. Integration Tests (REQUIRED if multiple components) - Black-Box Testing
**Location**: In `tests/` directory (separate files)
**Access**: Only public API (what external users see)
**Purpose**: Test component interactions, full workflows

```rust
// Create: tests/integration_[feature].rs
use my_crate::{public_api, Data}; // Only import public items

#[test]
fn test_full_workflow_integration() {
    // This tests how external users would use our crate
    // We CANNOT call internal_helper() here - it's private!

    let data = Data::new(10);
    let result = public_api(data).unwrap();
    assert_eq!(result.status, ProcessingStatus::Complete);
}

#[test]
fn test_multi_component_integration() {
    // Test that different public components work together
    let processor = DataProcessor::new();
    let validator = DataValidator::new();

    let data = Data::new(10);
    let validated = validator.validate(data).unwrap();
    let processed = processor.process(validated).unwrap();

    assert!(processed.is_complete());
}
```

#### 3. E2E Tests (if critical system paths)
```rust
// Add to: tests/e2e/[feature]_test.rs
#[tokio::test]
async fn test_[feature]_end_to_end() {
    let system = start_test_system().await;
    system.execute_feature().await;
    assert_eq!(system.get_output(), expected);
}
```

### Testing Hierarchy Summary

| Test Type | Location | Access | Purpose | Example |
|-----------|----------|--------|---------|---------|
| **Unit Tests** | `src/module.rs` | Private + Public | Test algorithms, edge cases, internal state | `assert!(internal_helper(&data))` |
| **Integration Tests** | `tests/integration.rs` | Public only | Test workflows, component interaction | `public_api(data).unwrap()` |
| **E2E Tests** | `tests/e2e/` | Full system | Test complete system flows | Relay → Consumer communication |

## Test Execution
```bash
# Run unit tests for this feature
cargo test --package package_name --lib test_[feature]

# Run with coverage
cargo tarpaulin --packages package_name --lib

# Check for regressions
cargo test --workspace

# Verify no hardcoded values
grep -r "hardcoded_pattern" src/
```

## Git Workflow
```bash
# 1. Create worktree (already done in step 1)
git worktree add -b fix/specific-issue-name ../task-xxx-worktree
cd ../task-xxx-worktree

# 2. Make changes and commit
git add -A
git commit -m "fix: clear description of change"

# 3. Push to origin
git push origin fix/specific-issue-name

# 4. Create PR
gh pr create --title "Fix: Clear description" --body "Closes TASK-XXX"

# 5. Clean up worktree after PR merge
cd ../backend_v2  # Return to main repository
git worktree remove ../task-xxx-worktree
git branch -D fix/specific-issue-name  # Delete local branch if desired
```

## ✅ Before Marking Complete
- [ ] All acceptance criteria met
- [ ] Code committed in worktree
- [ ] Tests passing (if applicable)
- [ ] **UPDATE: Change `status: TODO` to `status: COMPLETE` in YAML frontmatter above**
- [ ] Run: `../../../scrum/task-manager.sh sprint-XXX` to verify status

## Completion Checklist
- [ ] **🚨 STEP 0: Changed status to IN_PROGRESS when starting** ← AGENTS MUST DO THIS!
- [ ] Working in correct worktree (not main repository)
- [ ] **🚨 TDD FOLLOWED: Tests written BEFORE implementation**
- [ ] All tests pass (unit + integration)
- [ ] All acceptance criteria met
- [ ] Code reviewed locally
- [ ] No performance regression verified
- [ ] PR created
- [ ] **🚨 STEP FINAL: Updated task status to COMPLETE** ← AGENTS MUST DO THIS!

## 📋 Sprint Task Workflow
1. Pick task from TODO status
2. **IMMEDIATELY**: Change status: TODO → IN_PROGRESS
3. Do the work
4. **BEFORE COMMITTING**: Change status: IN_PROGRESS → COMPLETE
5. Verify with: `task-manager.sh sprint-XXX`

## ⚠️ IMPORTANT: Status Updates Required
**When you START this task, you MUST:**
1. **IMMEDIATELY** change `status: TODO` to `status: IN_PROGRESS` in the YAML frontmatter above
2. This makes the kanban board show you're working on it

**When you FINISH this task, you MUST:**
1. Change `status: IN_PROGRESS` to `status: COMPLETE` in the YAML frontmatter above
2. This is NOT optional - the task-manager.sh depends on accurate status
3. If you forget, the task will show as incomplete forever
4. Update immediately after PR is merged, not before

**Status Flow: TODO → IN_PROGRESS → COMPLETE**

## Task Completion Protocol
- [ ] Technical work completed
- [ ] Code committed in worktree
- [ ] Tests passing (if applicable)
- [ ] **CRITICAL**: Update YAML status to COMPLETE
- [ ] Verify status with task manager script

## Notes
[Space for implementation notes, blockers, or discoveries]
