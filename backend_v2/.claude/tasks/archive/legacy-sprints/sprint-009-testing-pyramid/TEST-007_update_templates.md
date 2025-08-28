---
task_id: TEST-007
status: COMPLETE
priority: HIGH
estimated_hours: 2
assigned_branch: test/update-templates
assignee: TBD
created: 2025-01-27
completed: null
---

# TEST-007: Update Task Templates with Testing Requirements

## 🔴 CRITICAL INSTRUCTIONS
```bash
# BEFORE STARTING - VERIFY YOU'RE NOT ON MAIN:
git branch --show-current

# If you see "main", IMMEDIATELY run:
git worktree add -b test/update-templates

# NEVER commit directly to main!
```

## Status
**Status**: COMPLETE
**Priority**: HIGH
**Branch**: `test/update-templates`
**Estimated**: 2 hours

## Problem Statement
Current task templates don't enforce the testing pyramid architecture. Every new feature should include unit, integration, and potentially E2E tests as part of the acceptance criteria.

## Acceptance Criteria
- [ ] Task template includes testing requirements section
- [ ] Sprint template includes test coverage metrics
- [ ] Testing pyramid architecture documented
- [ ] Test categories clearly defined
- [ ] Example test code in templates
- [ ] CI/CD requirements included

## Technical Approach

### Files to Modify
- `.claude/scrum/templates/TASK_TEMPLATE.md` - Add testing section
- `.claude/scrum/templates/SPRINT_PLAN.md` - Add coverage metrics
- `.claude/scrum/templates/TEST_RESULTS.md` - Enhance with pyramid metrics
- `.claude/scrum/TESTING_STANDARDS.md` - New comprehensive guide

### Implementation Steps

1. **Update TASK_TEMPLATE.md**:
```markdown
## Testing Requirements

### Unit Tests (Required)
```rust
// Location: src/[module]/mod.rs
#[cfg(test)]
mod tests {
    #[test]
    fn test_[feature]_[behavior]() {
        // Given-When-Then structure
    }
}
```
- [ ] Core logic has unit tests
- [ ] Edge cases covered
- [ ] Tests run in <100ms

### Integration Tests (If applicable)
```rust
// Location: tests/integration_[feature].rs
#[test]
fn test_[feature]_integration() {
    // Test with real components
}
```
- [ ] Component interactions tested
- [ ] Public API validated

### E2E Tests (For critical paths)
```rust
// Location: tests/e2e/[feature]_e2e.rs
#[test]
fn test_[feature]_end_to_end() {
    // Full pipeline validation
}
```
- [ ] Complete workflow tested
- [ ] No hardcoded test data

### Test Coverage Target
- Unit: >80% of new code
- Integration: Key interaction points
- E2E: Critical user paths only
```

2. **Create TESTING_STANDARDS.md**:
```markdown
# Testing Standards for Torq

## Testing Pyramid

### Layer 1: Unit Tests (Base - 70% of tests)
**Purpose**: Test individual functions in isolation
**Location**: `#[cfg(test)] mod tests` in source files
**Speed**: <100ms per test
**Count**: 200+ tests

**Required for**:
- All calculation functions
- All parsing functions
- All state transitions
- All validation logic

**Example**:
```rust
#[test]
fn test_calculate_profit() {
    assert_eq!(calculate_profit(100, 10), 90);
}
```

### Layer 2: Integration Tests (Middle - 25% of tests)
**Purpose**: Test component interactions
**Location**: `tests/` directory in crate
**Speed**: <1s per test
**Count**: 50+ tests

**Required for**:
- Service-to-service communication
- Database interactions
- External API calls
- Message passing

### Layer 3: E2E Tests (Top - 5% of tests)
**Purpose**: Validate complete user scenarios
**Location**: `tests/e2e/` in root
**Speed**: <30s per test
**Count**: 5-10 tests

**Required for**:
- Critical business flows
- User-facing features
- System integration points

## Specialized Testing

### Property-Based Testing
```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_profit_never_negative(
        price in 0i64..1_000_000_000,
        quantity in 0i64..1_000_000
    ) {
        let profit = calculate_profit(price, quantity);
        assert!(profit >= 0);
    }
}
```

### Fuzz Testing
```bash
cargo fuzz run parser_fuzz
```

### Performance Testing
```rust
#[bench]
fn bench_tlv_parsing(b: &mut Bencher) {
    b.iter(|| parse_tlv(&test_data));
}
```

## Test Data Management

### Never Hardcode
❌ **BAD**:
```rust
let profit = 150.0; // Hardcoded!
```

✅ **GOOD**:
```rust
let profit = calculate_from_market_data(data);
```

### Use Builders
```rust
let trade = TradeBuilder::new()
    .with_price(calculate_price())
    .with_quantity(100)
    .build();
```

## Coverage Requirements
- Critical paths: >80%
- Business logic: >90%
- Protocol code: >95%
- Utilities: >70%

## CI/CD Integration
All PRs must:
1. Pass unit tests
2. Pass integration tests
3. Maintain coverage thresholds
4. Pass E2E smoke tests
```

3. **Update agent documentation**:
```bash
# After template updates
./.claude/scrum/update-agent-docs.sh
```

## Testing Instructions
```bash
# Verify templates are valid markdown
cat .claude/scrum/templates/TASK_TEMPLATE.md

# Test template generation
./.claude/scrum/create-sprint.sh 999 "test" "test"

# Verify testing section appears
grep -A 20 "Testing Requirements" .claude/tasks/sprint-999-test/TASK-001_rename_me.md
```

## Git Workflow
```bash
# 1. Start on your branch
git worktree add -b test/update-templates

# 2. Make changes and commit
git add .claude/scrum/templates/
git commit -m "feat: add testing requirements to task templates"

# 3. Push to your branch
git push origin test/update-templates

# 4. Create PR
gh pr create --title "TEST-007: Update templates with testing requirements" \
             --body "Enforces testing pyramid in all new tasks"
```

## Completion Checklist
- [ ] Working on correct branch (not main)
- [ ] TASK_TEMPLATE.md updated
- [ ] SPRINT_PLAN.md updated
- [ ] TEST_RESULTS.md enhanced
- [ ] TESTING_STANDARDS.md created
- [ ] Agent docs updated
- [ ] Templates tested
- [ ] PR created
- [ ] Updated task status to COMPLETE

## Notes
This ensures every future task includes proper testing from the start. No more features without tests!