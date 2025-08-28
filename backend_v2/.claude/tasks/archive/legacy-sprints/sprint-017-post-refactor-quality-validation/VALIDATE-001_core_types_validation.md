---
task_id: S015-VALIDATE-001
status: TODO  ← CHANGE TO "IN_PROGRESS" WHEN STARTING, THEN "COMPLETE" WHEN FINISHED!
priority: CRITICAL
estimated_hours: 8
assigned_branch: validate/core-types-post-refactor
assignee: TBD
created: 2025-08-27
completed: null
# Dependencies: Major refactor must be completed first
depends_on: []
# Blocks: All subsequent validation tasks depend on this
blocks: [S015-VALIDATE-002, S015-VALIDATE-003]
# Scope: Core Protocol V2 types after refactor
scope: ["torq/libs/types/src/protocol/", "torq/libs/types/src/common/", "torq/libs/types/benches/"]
---

# VALIDATE-001: Core Types Validation Post-Refactor

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
**Worktree**: `../validate-001-worktree` (Branch: `validate/core-types-post-refactor`)
**Estimated**: 8 hours
**Phase**: 1 - Core Data Structures & Protocol

## Problem Statement
After the major refactor (backend_v2/ → torq/, libs/ restructuring), we must validate that all core Protocol V2 data structures maintain complete integrity. This includes TLV struct definitions, macro functionality, precision preservation, and zerocopy trait implementations.

**Critical Risk**: The refactor may have broken fundamental Protocol V2 components, leading to:
- TLV parsing failures
- Precision loss in financial calculations  
- Zerocopy trait violations
- Macro expansion errors
- Performance degradation

## Acceptance Criteria

### **Core Validation Requirements**
- [ ] All TLV struct definitions validate successfully after refactor
- [ ] All Protocol V2 macros expand correctly in new location
- [ ] Zero precision loss verified across all numeric types (18-decimal WETH, 6-decimal USDC, 8-decimal USD prices)
- [ ] Zerocopy traits function correctly (no unexpected allocations)
- [ ] All existing unit tests pass without modification
- [ ] Performance benchmarks meet baseline: TLV construction >1M operations/second
- [ ] Memory safety verification (no new unsafe code introduced)
- [ ] Fixed-point arithmetic precision boundaries validated
- [ ] Type-safe TLV builder functionality confirmed
- [ ] Hot path buffer implementations maintain zero-allocation guarantee

### **NEW: Rustdoc Documentation Validation**
- [ ] **Complete API Coverage**: All public TLV types have comprehensive rustdoc documentation
- [ ] **Usage Examples**: All major TLV types include working rustdoc examples
- [ ] **Cross-References**: TLV types properly cross-reference related builders and parsers
- [ ] **Module Documentation**: Core types modules have comprehensive `//!` documentation
- [ ] **Integration Guidance**: Documentation explains how types integrate with adapter/relay systems
- [ ] **Performance Documentation**: Performance characteristics documented for critical types
- [ ] **Precision Documentation**: Precision preservation guarantees documented with examples

## Technical Approach
### Core Files to Validate (Post-Refactor Locations)
**NEW PATHS** (after refactor):
- `torq/libs/types/src/protocol/tlv/types.rs` - TLV type definitions
- `torq/libs/types/src/protocol/tlv/macros.rs` - TLV construction macros
- `torq/libs/types/src/common/fixed_point.rs` - Precision arithmetic
- `torq/libs/types/src/common/identifiers.rs` - Core ID types
- `torq/libs/types/src/protocol/tlv/hot_path_buffers.rs` - Zero-allocation buffers
- `torq/libs/types/benches/*.rs` - Performance benchmarks

### Validation Strategy
1. **Static Analysis**: Verify all types compile without warnings
2. **Runtime Testing**: Execute comprehensive test suites
3. **Performance Validation**: Benchmark critical operations
4. **Memory Safety**: Verify no unsafe code additions
5. **Precision Testing**: Validate financial arithmetic

### Implementation Steps - VALIDATION FOCUSED ⚠️
**🚨 CRITICAL: This is validation, not new development - Tests verify existing functionality!**

1. **SETUP**: Initialize validation environment
   ```bash
   # Create validation worktree
   git worktree add -b validate/core-types-post-refactor ../validate-001-worktree
   cd ../validate-001-worktree
   
   # Verify new directory structure exists
   ls torq/libs/types/src/protocol/tlv/
   ls torq/libs/types/src/common/
   ```

2. **COMPILATION VALIDATION**: Ensure all types compile cleanly
   ```bash
   # Full workspace compilation
   cargo check --workspace
   cargo clippy --workspace -- -D warnings
   
   # Specific types package compilation  
   cargo build --package types --release
   ```

3. **EXISTING TEST EXECUTION**: Run all existing tests in new locations
   ```bash
   # Unit tests for core types
   cargo test --package types --lib
   
   # Integration tests
   cargo test --package types --test '*'
   
   # Benchmark compilation (don't run yet)
   cargo build --benches --package types
   ```

4. **PRECISION VALIDATION**: Critical financial arithmetic testing
   ```bash
   # Run precision-specific tests
   cargo test --package types precision_validation
   cargo test --package types fixed_point
   cargo test --package types token_precision
   ```

5. **MEMORY SAFETY VERIFICATION**: Ensure no unsafe additions
   ```bash
   # Search for any new unsafe blocks
   rg "unsafe" torq/libs/types/src/ --type rust
   
   # Verify zerocopy trait implementations
   cargo test --package types zero_copy
   ```

6. **PERFORMANCE BASELINE VALIDATION**: Ensure no performance regression
   ```bash
   # Run critical benchmarks
   cargo bench --package types --bench typed_id_performance
   cargo bench --package types --bench fixedvec_performance
   cargo bench --package types --bench message_builder_comparison
   
   # Performance must meet: >1M msg/s construction
   # Record results in TEST_RESULTS.md
   ```

7. **MACRO FUNCTIONALITY**: Verify TLV macros work correctly
   ```bash
   # Test macro expansion in new location
   cargo test --package types macro_syntax
   cargo test --package types tlv_macro
   cargo expand --package types | head -50  # Verify expansion
   ```

8. **CRITICAL PATH VALIDATION**: Hot path components
   ```bash
   # Zero-allocation buffer tests
   cargo test --package types hot_path_buffers
   cargo test --package types zero_allocation
   
   # Type-safe builder validation
   cargo test --package types builder_tests
   cargo test --package types type_safe
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

## Comprehensive Validation Execution
```bash
# PHASE 1: Compilation & Static Analysis
cargo check --package types
cargo clippy --package types -- -D warnings
cargo fmt --check --package types

# PHASE 2: Core Test Suites (All Must Pass)
cargo test --package types --lib                      # Unit tests
cargo test --package types --test integration         # Integration tests  
cargo test --package types precision_validation       # Precision critical
cargo test --package types tlv_parsing               # TLV core functionality
cargo test --package types instrument_id_bijection   # Bijective ID validation

# PHASE 3: Performance & Memory Validation
cargo bench --package types                          # All benchmarks
cargo test --package types zero_copy                 # Memory safety
cargo test --package types zero_allocation           # Hot path validation

# PHASE 4: Regression Detection
cargo test --workspace                               # Full workspace regression check

# PHASE 5: Critical Validation (Must Document Results)
echo "=== VALIDATION RESULTS ===" > TEST_RESULTS.md
cargo test --package types 2>&1 | tee -a TEST_RESULTS.md
cargo bench --package types 2>&1 | tee -a TEST_RESULTS.md
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
