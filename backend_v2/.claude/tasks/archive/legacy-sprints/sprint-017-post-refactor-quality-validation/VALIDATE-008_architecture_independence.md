---
task_id: S015-VALIDATE-008
status: TODO  ← CHANGE TO "IN_PROGRESS" WHEN STARTING, THEN "COMPLETE" WHEN FINISHED!
priority: HIGH
estimated_hours: 4
assigned_branch: validate/architecture-independence
assignee: TBD
created: 2025-08-27
completed: null
# Dependencies: End-to-end validation must complete first
depends_on: [S015-VALIDATE-007]
# Blocks: Code quality validation
blocks: [S015-VALIDATE-009]
# Scope: Project structure and dependency validation
scope: ["torq/network/", "torq/services/", "torq/relays/", "torq/libs/", "Cargo.toml"]
---

# VALIDATE-008: Architecture Independence & Extractability Validation

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
The refactored architecture must maintain strict component boundaries to enable future extraction of the network/ component into an independent Mycelium repository. We must validate that dependency directions are correct, boundaries are clean, and the network/ crate is truly self-contained.

**Critical Requirements**:
- network/ (torq_network crate) must be independently extractable
- No accidental upstream dependencies in network/
- Generic interfaces suitable for other applications
- Self-contained test suites that run independently
- Clear one-way dependency flow: services/ → relays/ → network/ → libs/

## Acceptance Criteria
- [ ] Dependency direction validation: Strict one-way flow enforced
- [ ] Network/ isolation: Zero dependencies on services/ or relays/
- [ ] Generic interfaces: network/ APIs are application-agnostic
- [ ] Self-contained tests: network/ tests run independently
- [ ] Extractability verification: network/ can compile as standalone crate
- [ ] Cargo.toml validation: Dependency graph matches architectural design
- [ ] Module boundaries: No cross-boundary private access
- [ ] Interface stability: APIs suitable for external consumption
- [ ] Documentation completeness: network/ is self-documenting
- [ ] Future-proof design: Ready for eventual repository extraction

## Technical Approach
### Architecture Validation Focus Areas
**DEPENDENCY VALIDATION**:
- `torq/network/Cargo.toml` - Verify only libs/ dependencies
- `torq/services/Cargo.toml` - Can depend on relays/, network/, libs/
- `torq/relays/Cargo.toml` - Can depend on network/, libs/ only
- `torq/libs/*/Cargo.toml` - Minimal cross-lib dependencies

**EXTRACTABILITY REQUIREMENTS**:
1. **One-Way Dependency Flow**: services/ → relays/ → network/ → libs/
2. **Network Isolation**: No upstream dependencies (services/, relays/)
3. **Generic Interfaces**: APIs not Torq-specific
4. **Self-Contained**: Independent compilation and testing

### Validation Tools & Methods
- **cargo tree**: Dependency graph analysis
- **cargo check**: Independent compilation verification  
- **rg/grep**: Source code dependency scanning
- **cargo test**: Isolated test execution
- **cargo doc**: Self-documentation verification

### Implementation Steps - ARCHITECTURE VALIDATION ⚠️
**🚨 CRITICAL: Validate modular design and extractability!**

1. **SETUP**: Initialize architecture validation environment
   ```bash
   # Create validation worktree
   git worktree add -b validate/architecture-independence ../validate-008-worktree
   cd ../validate-008-worktree
   
   # Install dependency analysis tools
   cargo install cargo-depgraph
   cargo install cargo-modules
   ```

2. **DEPENDENCY DIRECTION VALIDATION**: Enforce one-way flow
   ```bash
   # Validate dependency graph structure
   cargo tree --package torq_network --depth 2
   cargo tree --package relays --depth 2  
   cargo tree --package services --depth 2
   
   # Check for forbidden upstream dependencies
   echo "=== DEPENDENCY ANALYSIS ===" > ARCH_VALIDATION.md
   cargo tree --package torq_network 2>&1 | tee -a ARCH_VALIDATION.md
   ```

3. **NETWORK ISOLATION VERIFICATION**: Zero upstream dependencies
   ```bash
   # Search for prohibited imports in network/
   rg "use.*services::" torq/network/src/ && echo "❌ VIOLATION: network uses services" || echo "✅ Clean: no services deps"
   rg "use.*relays::" torq/network/src/ && echo "❌ VIOLATION: network uses relays" || echo "✅ Clean: no relays deps"
   
   # Verify only allowed dependencies
   rg "use.*torq_" torq/network/src/ | grep -v -E "(codec|types)" && echo "❌ VIOLATION" || echo "✅ Clean deps"
   ```

4. **STANDALONE COMPILATION**: Network crate independence
   ```bash
   # Test network/ compiles independently
   cd torq/network/
   cargo check --all-features
   cargo test --all-features --lib
   
   # Document results
   echo "Network standalone compilation: $(cargo check 2>&1 && echo SUCCESS || echo FAILED)" >> ../../ARCH_VALIDATION.md
   ```

5. **INTERFACE GENERICITY**: Application-agnostic APIs
   ```bash
   # Search for Torq-specific hardcoded references
   rg -i "torq|torq" torq/network/src/ --type rust | grep -v "crate_name\|package" && echo "❌ App-specific code found" || echo "✅ Generic interfaces"
   
   # Check for business logic leakage
   rg "arbitrage|trading|strategy" torq/network/src/ && echo "❌ Business logic in network" || echo "✅ Pure transport layer"
   ```

6. **EXTRACTABILITY SIMULATION**: Mock independent extraction
   ```bash
   # Create temporary extraction simulation
   mkdir /tmp/network_extraction_test
   cp -r torq/network/* /tmp/network_extraction_test/
   cp torq/libs/codec /tmp/network_extraction_test/vendor/codec -r
   cp torq/libs/types /tmp/network_extraction_test/vendor/types -r
   
   # Test independent compilation
   cd /tmp/network_extraction_test
   # Update Cargo.toml to use vendor/ paths
   sed -i 's|path = "../libs/|path = "vendor/|g' Cargo.toml
   cargo check && echo "✅ Extraction successful" || echo "❌ Extraction failed"
   ```

7. **MODULE BOUNDARY VALIDATION**: Interface consistency
   ```bash
   # Generate module documentation
   cargo doc --package torq_network --no-deps --open
   
   # Check for private item leakage
   cargo clippy --package torq_network -- -W clippy::missing_docs_in_private_items
   
   # Verify clean public API
   cargo expand --package torq_network --lib | grep "pub " | head -20
   ```

8. **FUTURE-PROOFING VERIFICATION**: Repository extraction readiness
   ```bash
   # Check for hardcoded paths
   rg "\.\./" torq/network/ && echo "❌ Relative paths found" || echo "✅ Clean paths"
   
   # Verify self-contained configuration
   find torq/network/ -name "*.toml" -exec grep -H "path.*\.\." {} \; && echo "❌ External paths" || echo "✅ Self-contained"
   
   # Final validation summary
   echo "=== EXTRACTABILITY SUMMARY ===" >> ARCH_VALIDATION.md
   echo "Network crate ready for extraction: $(date)" >> ARCH_VALIDATION.md
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
