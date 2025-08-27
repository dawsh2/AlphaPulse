---
task_id: S013.1-GAP001
status: COMPLETE
priority: CRITICAL
estimated_hours: 4
assigned_branch: fix/missing-tlv-types
assignee: Claude
created: 2025-08-26
completed: 2025-08-27
# Dependencies: task IDs that must be COMPLETE before this can start
depends_on: ["AUDIT-002", "AUDIT-003", "AUDIT-004"]
# Blocks: task IDs that cannot start until this is COMPLETE
blocks: ["S013.1-GAP002", "S013.1-GAP003"]
# Scope: files/directories this task modifies (for conflict detection)
scope: ["libs/types/src/protocol/tlv/*.rs", "libs/types/src/common/*.rs"]
---

# GAP-001: Implement Missing TLV Types (QuoteTLV, InvalidationReason)

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
**Worktree**: `../gap001-worktree` (Branch: `fix/missing-tlv-types`)
**Estimated**: 4 hours

## Problem Statement
Sprint 013 code review revealed critical missing TLV types that are preventing proper order book updates and state management. Two specific types are missing:

1. **QuoteTLV**: Referenced in adapter code but not implemented, causing compilation errors
2. **InvalidationReason**: Needed for proper state invalidation tracking, currently using stub implementations

Without these types:
- Binary targets fail to compile due to missing imports
- Order book updates lose quote data, creating incomplete market views
- State invalidation cannot properly track why states became invalid
- Risk of phantom arbitrage opportunities due to stale data

## Acceptance Criteria
- [ ] QuoteTLV struct implemented with proper TLV type number and serialization
- [ ] InvalidationReason enum implemented with all necessary variants
- [ ] Both types integrate with existing TLV parsing and building infrastructure
- [ ] All binary targets compile successfully (no more import errors)
- [ ] TLV type registry updated with new type numbers
- [ ] Unit tests written and passing for both new types
- [ ] Integration tests verify types work in full message parsing
- [ ] No performance regression in TLV parsing/building benchmarks
- [ ] Documentation updated in TLV type registry
- [ ] Zero data loss when using new types in order book updates

## Technical Approach
### Files to Modify
- `libs/types/src/protocol/tlv/types.rs` - Add new TLV type numbers for QuoteTLV and InvalidationReason
- `libs/types/src/protocol/tlv/market_data.rs` - Implement QuoteTLV struct with zerocopy traits
- `libs/types/src/protocol/tlv/system.rs` - Implement InvalidationReason enum
- `libs/types/src/protocol/tlv/mod.rs` - Export new types and update parsing logic
- `libs/types/src/protocol/tlv/parser.rs` - Add parsing support for new TLV types
- `libs/types/src/protocol/tlv/builder.rs` - Add building support for new TLV types
- `libs/types/tests/tlv_parsing.rs` - Add comprehensive tests for new types

### Implementation Steps - TDD REQUIRED ⚠️
**🚨 CRITICAL: Follow Test-Driven Development - Write Tests FIRST!**

1. **RED**: Write failing tests for QuoteTLV
   ```bash
   # Add to libs/types/tests/tlv_parsing.rs
   cargo test test_quote_tlv_parsing  # Should FAIL (no implementation yet)
   cargo test test_quote_tlv_building  # Should FAIL
   ```

2. **GREEN**: Implement QuoteTLV struct
   ```bash
   # Implement in libs/types/src/protocol/tlv/market_data.rs
   cargo test test_quote_tlv_parsing  # Should PASS
   ```

3. **RED**: Write failing tests for InvalidationReason
   ```bash
   cargo test test_invalidation_reason_serialization  # Should FAIL
   ```

4. **GREEN**: Implement InvalidationReason enum
   ```bash
   # Implement in libs/types/src/protocol/tlv/system.rs
   cargo test test_invalidation_reason_serialization  # Should PASS
   ```

5. **RED**: Write integration tests for both types
   ```bash
   cargo test test_tlv_message_with_new_types  # Should FAIL
   ```

6. **GREEN**: Update TLV parsing/building infrastructure
   ```bash
   cargo test test_tlv_message_with_new_types  # Should PASS
   ```

7. **REFACTOR**: Optimize and clean up implementation
   ```bash
   cargo test --package libs/types  # All tests should PASS
   cargo bench tlv_parsing  # No performance regression
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
