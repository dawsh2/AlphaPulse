---
task_id: S015-VALIDATE-002
status: TODO  ← CHANGE TO "IN_PROGRESS" WHEN STARTING, THEN "COMPLETE" WHEN FINISHED!
priority: CRITICAL
estimated_hours: 8
assigned_branch: validate/codec-post-refactor
assignee: TBD
created: 2025-08-27
completed: null
# Dependencies: Core types validation must complete first
depends_on: [S015-VALIDATE-001]
# Blocks: Round-trip testing depends on this
blocks: [S015-VALIDATE-003]
# Scope: Protocol V2 codec after libs separation
scope: ["torq/libs/codec/src/", "torq/libs/codec/tests/", "torq/libs/codec/benches/"]
---

# VALIDATE-002: Codec Validation Post-Refactor

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
After the libs/ restructuring that separated types (pure data) from codec (protocol logic), we must validate that all Protocol V2 codec functionality remains intact. This includes bijective InstrumentId operations, TLV message builders/parsers, and all protocol serialization/deserialization logic.

**Critical Risk**: The codec separation may have broken:
- Bijective InstrumentId construction/deconstruction
- TLV message parsing/building
- Protocol serialization round-trips
- Message header validation
- Error handling in codec operations

## Acceptance Criteria
- [ ] All InstrumentId bijective operations work correctly (construction ↔ deconstruction)
- [ ] TLV message builders create valid Protocol V2 messages
- [ ] TLV message parsers correctly extract all data
- [ ] Message header validation functions correctly
- [ ] Protocol error handling preserves all error information
- [ ] Codec performance meets baseline: >1.6M parsing operations/second
- [ ] All existing codec tests pass without modification
- [ ] No data corruption in serialization/deserialization
- [ ] Parser handles malformed messages safely
- [ ] Builder enforces Protocol V2 constraints correctly

## Technical Approach
### Core Codec Files to Validate (Post-Refactor Locations)
**NEW PATHS** (after libs/ separation):
- `torq/libs/codec/src/instrument_id.rs` - Bijective InstrumentId operations
- `torq/libs/codec/src/message_builder.rs` - TLV message construction
- `torq/libs/codec/src/parser.rs` - TLV message parsing
- `torq/libs/codec/src/tlv_types.rs` - Protocol codec logic
- `torq/libs/codec/src/error.rs` - Codec-specific error handling
- `torq/libs/codec/tests/` - All codec-specific tests
- `torq/libs/codec/benches/` - Performance benchmarks

### Validation Strategy
1. **Bijective Validation**: Ensure InstrumentId operations are reversible
2. **Codec Integrity**: Verify builders/parsers work correctly
3. **Error Handling**: Validate robust error propagation
4. **Performance Testing**: Ensure parsing speed maintained
5. **Protocol Compliance**: Verify strict Protocol V2 adherence

### Implementation Steps - CODEC VALIDATION ⚠️
**🚨 CRITICAL: This validates existing codec functionality after libs/ separation!**

1. **SETUP**: Initialize codec validation environment
   ```bash
   # Create validation worktree
   git worktree add -b validate/codec-post-refactor ../validate-002-worktree
   cd ../validate-002-worktree
   
   # Verify new codec structure exists
   ls torq/libs/codec/src/
   ls torq/libs/codec/tests/
   ```

2. **BIJECTIVE VALIDATION**: Critical InstrumentId testing
   ```bash
   # Property-based bijective testing
   cargo test --package codec instrument_id_bijection
   cargo test --package codec --test '*bijection*'
   
   # Verify all venue types work correctly
   cargo test --package codec venue_
   ```

3. **MESSAGE BUILDER VALIDATION**: TLV construction integrity
   ```bash
   # Builder functionality
   cargo test --package codec message_builder
   cargo test --package codec --test message_builder_integration
   
   # Verify Protocol V2 compliance
   cargo test --package codec protocol_v2_compliance
   ```

4. **PARSER VALIDATION**: Message parsing correctness
   ```bash
   # Core parsing functionality
   cargo test --package codec parser
   cargo test --package codec --test parser_integration
   
   # Malformed message handling
   cargo test --package codec malformed_message
   cargo test --package codec error_cases
   ```

5. **ROUND-TRIP INTEGRITY**: Builder → Parser validation
   ```bash
   # Full round-trip testing
   cargo test --package codec round_trip
   cargo test --package codec codec_tests
   
   # Ensure no data loss
   cargo test --package codec data_integrity
   ```

6. **PERFORMANCE VALIDATION**: Ensure parsing speed maintained
   ```bash
   # Critical performance benchmarks
   cargo bench --package codec --bench error_performance
   
   # Must maintain >1.6M parsing operations/second
   # Document baseline vs post-refactor performance
   ```

7. **ERROR HANDLING VALIDATION**: Robust error propagation
   ```bash
   # Error formatting and propagation
   cargo test --package codec error_formatting
   cargo test --package codec error_propagation
   
   # Ensure no error information lost
   cargo test --package codec error_preservation
   ```

8. **PROTOCOL COMPLIANCE**: Strict Protocol V2 validation
   ```bash
   # Message format validation
   cargo test --package codec message_format
   cargo test --package codec protocol_constraints
   
   # Header validation
   cargo test --package codec header_validation
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

## Comprehensive Codec Validation Execution
```bash
# PHASE 1: Static Analysis & Compilation
cargo check --package codec
cargo clippy --package codec -- -D warnings
cargo fmt --check --package codec

# PHASE 2: Critical Codec Functionality (All Must Pass)
cargo test --package codec --lib                     # Unit tests
cargo test --package codec --test codec_tests        # Integration tests
cargo test --package codec instrument_id_bijection   # Bijective critical
cargo test --package codec message_builder           # Builder validation
cargo test --package codec parser                    # Parser validation

# PHASE 3: Protocol & Error Validation  
cargo test --package codec error_formatting          # Error handling
cargo test --package codec protocol_constraints      # Protocol compliance
cargo test --package codec round_trip                # Data integrity

# PHASE 4: Performance Validation (Critical Benchmarks)
cargo bench --package codec --bench error_performance # >1.6M parsing/sec
cargo test --package codec --test performance_tests   # Performance regression

# PHASE 5: Comprehensive Validation (Document Everything)
echo "=== CODEC VALIDATION RESULTS ===" >> TEST_RESULTS.md
echo "Dependency: VALIDATE-001 must complete first" >> TEST_RESULTS.md
cargo test --package codec 2>&1 | tee -a TEST_RESULTS.md
cargo bench --package codec 2>&1 | tee -a TEST_RESULTS.md
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
