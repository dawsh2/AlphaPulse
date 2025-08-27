---
task_id: AUDIT-007
status: TODO
priority: MEDIUM
estimated_hours: 3
assigned_branch: feat/architecture-validation-tests
assignee: TBD
created: 2025-08-26
completed: null
depends_on:
  - AUDIT-002  # Need codec integration to validate
  - AUDIT-003  # Need plugin architecture to validate
blocks: []
scope:
  - "tests/architecture_validation/"  # New test directory
  - "tests/Cargo.toml"  # Test dependencies
---

# AUDIT-007: Architecture Validation Tests

## Git Worktree Setup (REQUIRED)
```bash
# Create worktree for this task
git worktree add -b feat/architecture-validation-tests ../audit-007-worktree
cd ../audit-007-worktree
```

## Status
**Status**: TODO
**Priority**: MEDIUM
**Worktree**: `../audit-007-worktree` (Branch: `feat/architecture-validation-tests`)
**Estimated**: 3 hours

## Problem Statement
Need automated tests to prevent architectural regressions and ensure the refactored system maintains its design principles. These tests validate that services properly use the intended architecture patterns.

## Acceptance Criteria
- [ ] Create architecture validation test suite
- [ ] Verify all services use alphapulse_codec (no protocol duplication)
- [ ] Check correct crate dependencies across workspace
- [ ] Validate adapter plugin interface compliance
- [ ] Ensure typed IDs used consistently
- [ ] Tests run in CI/CD pipeline
- [ ] Clear failure messages guide developers

## Test Categories

### 1. Dependency Validation
```rust
#[test]
fn test_services_use_correct_codec() {
    // Verify all services depend on alphapulse_codec
    // Fail if services have inline protocol parsing
}

#[test]
fn test_no_protocol_duplication() {
    // Scan for duplicated TLV parsing logic
    // Ensure single source of truth in codec
}
```

### 2. Plugin Interface Compliance
```rust
#[test]
fn test_adapters_implement_trait() {
    // Verify all adapters implement Adapter trait
    // Check method signatures match interface
}

#[test]
fn test_adapter_plugin_structure() {
    // Validate directory structure for plugins
    // Ensure proper separation of concerns
}
```

### 3. Typed ID Usage Verification
```rust
#[test]
fn test_typed_ids_used_consistently() {
    // Verify no raw u64 IDs in financial code
    // Ensure proper typed ID usage patterns
}
```

### 4. Code Quality Checks
```rust
#[test]
fn test_no_hardcoded_values() {
    // Scan for hardcoded addresses, keys, etc.
    // Ensure configuration-driven approach
}
```

## Implementation Steps
1. **Create Test Infrastructure**
   - Create `tests/architecture_validation/` directory
   - Set up test dependencies and utilities
   - Create shared test helpers

2. **Dependency Validation Tests**
   - Parse Cargo.toml files to verify dependencies
   - Scan source code for protocol usage patterns
   - Check for codec import consistency

3. **Plugin Architecture Tests**
   - Validate adapter trait implementations
   - Check plugin directory structure
   - Verify common module usage

4. **Code Pattern Analysis**
   - Scan for typed ID usage vs raw primitives
   - Check for protocol logic duplication
   - Validate configuration usage patterns

5. **CI Integration**
   - Add tests to CI pipeline
   - Configure failure notifications
   - Document test purposes and fixes

## Test Structure
```
tests/architecture_validation/
├── Cargo.toml         # Test-specific dependencies
├── src/
│   ├── lib.rs         # Test utilities and helpers
│   ├── dependency_validation.rs  # Dependency checking tests
│   ├── plugin_compliance.rs      # Plugin interface tests
│   ├── typed_id_usage.rs         # ID usage validation
│   └── code_quality.rs          # General code quality checks
└── fixtures/          # Test data and examples
```

## Files to Create/Modify
- `tests/architecture_validation/Cargo.toml` - Test dependencies
- `tests/architecture_validation/src/lib.rs` - Test utilities
- `tests/architecture_validation/src/dependency_validation.rs` - Dependency tests
- `tests/architecture_validation/src/plugin_compliance.rs` - Plugin tests
- `tests/architecture_validation/src/typed_id_usage.rs` - ID usage tests
- `tests/architecture_validation/src/code_quality.rs` - Quality checks
- `.github/workflows/architecture-tests.yml` - CI integration

## Success Criteria
- All architectural principles validated by automated tests
- Tests catch common regression patterns
- Clear failure messages guide developers to fixes
- Tests run fast enough for regular CI execution
- Architecture compliance is enforced, not just documented

## Validation Patterns
The tests will check for:
- ✅ Services use `alphapulse_codec` consistently
- ✅ No duplicated protocol parsing logic
- ✅ Adapters implement the Adapter trait properly
- ✅ Typed IDs used instead of raw primitives
- ✅ Configuration used instead of hardcoded values
- ✅ Proper module organization and boundaries