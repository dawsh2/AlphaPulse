# Atomic Development Guide - Single Focus Pattern

## 🎯 New Development Philosophy: One Task, One Branch, One PR

After the foundation merge, we enforce **atomic development** - each branch focuses on exactly ONE issue with minimal changes.

## 🔒 Atomic Development Rules

### Rule 1: Single Responsibility
```bash
# GOOD: Focused task
git checkout -b fix/precision-loss-in-signals
# Focus: Only the float→integer conversion in signal output
# Changes: 1-2 files, 20-50 lines
# Scope: Crystal clear and testable

# BAD: Multiple concerns
git checkout -b fix/multiple-improvements
# Changes: 10 files, 500 lines
# Scope: Signal precision + performance + cleanup + new feature
```

### Rule 2: Minimal File Changes
- **Target**: 1-3 files maximum per PR
- **Limit**: <100 lines of changes total
- **Focus**: Single module/component only

### Rule 3: Clear Before/After
Every atomic task must have:
- **Before**: Clearly broken/suboptimal behavior
- **After**: Measurably improved behavior
- **Test**: Proof the change works

## 📋 Atomic Task Template

```markdown
# Atomic Task: [SINGLE-ISSUE-DESCRIPTION]
*Branch: `fix/specific-issue`*
*Files: 1-2 files maximum*
*Lines: <50 changes*

## 🎯 Single Focus
**ONLY**: [One specific problem to solve]
**NOT**: Any other improvements, cleanup, or additions

## 📊 Measurable Outcome
**Before**: [Specific current behavior]
**After**: [Specific improved behavior]
**Test**: [How to verify the change]

## 🔧 Minimal Implementation
**Files to Change**:
- `path/to/single/file.rs` - [specific change]

**Changes**:
- Replace line X with Y
- Add function Z
- Remove deprecated method A

## ✅ Atomic Acceptance Criteria
- [ ] Addresses ONLY the stated issue
- [ ] Changes <3 files
- [ ] <100 lines total changes
- [ ] Tests prove the specific improvement
- [ ] No scope creep or "while I'm here" additions
```

## 🚀 Atomic Workflow Pattern

### Step 1: Clean Start
```bash
# ALWAYS start from clean main
git checkout main
git pull origin main
git status  # MUST show "working tree clean"

# Any uncommitted changes = stop and investigate
```

### Step 2: Focused Branch Creation
```bash
# Branch name describes the SINGLE issue
git checkout -b fix/signal-float-precision
# NOT: fix/signal-improvements (too broad)
# NOT: fix/multiple-issues (violates atomic rule)
```

### Step 3: Laser-Focused Implementation
```bash
# Touch ONLY files needed for this specific issue
# Resist the urge to fix other things you notice
# "While I'm here" additions are FORBIDDEN

# Example: Fixing signal precision
# ONLY edit: services_v2/strategies/flash_arbitrage/src/signal_output.rs
# NOT: Also fix typos in other files
# NOT: Also improve error handling elsewhere
# NOT: Also add new features
```

### Step 4: Atomic Testing
```bash
# Test ONLY the specific change
cargo test signal_precision_test
cargo test signal_output

# NOT: Run entire test suite (save time)
# NOT: Test unrelated functionality
```

### Step 5: Atomic PR Creation
```bash
gh pr create --title "fix: signal precision loss in float conversion" \
  --body "Fixes #123 - Replace float arithmetic with integer math

Before: 0.12345 * 100000000 = 12344999 (precision loss)
After: 12345 * 1000000 = 12345000000 (exact)

Changes:
- Replace f64 calculation with i64 math
- Update tests for new precision

Files changed: 1
Lines changed: 23"
```

### Step 6: Immediate Return to Main
```bash
# After PR merged, immediately return to main
gh pr merge [PR-NUMBER] --squash
git checkout main
git pull origin main

# Ready for next atomic task
```

## 📊 Atomic Task Examples

### Perfect Atomic Tasks
```bash
# PRECISION-001: Signal float precision
Branch: fix/signal-float-precision
Files: signal_output.rs (1 file)
Lines: 15 changes
Focus: Replace f64 math with i64 math

# PERF-001: Checksum sampling
Branch: fix/checksum-sampling
Files: relay_consumer.rs (1 file)
Lines: 25 changes
Focus: Add sampling to reduce validation overhead

# CLEANUP-001: Remove unreachable pattern
Branch: fix/unreachable-pattern
Files: relay_consumer.rs (1 file)
Lines: 3 changes
Focus: Remove lines 440-442 redundant pattern match
```

### Anti-Patterns (Avoid)
```bash
# TOO BROAD
Branch: fix/signal-improvements
Files: 8 files
Lines: 300 changes
Issues: Precision + performance + refactoring + new features

# SCOPE CREEP
Branch: fix/precision
Files: signal_output.rs, error_handling.rs, config.rs, utils.rs
Lines: 150 changes
Issues: Started with precision, added error handling, config updates, utils refactoring
```

## 🎯 Benefits of Atomic Development

### Code Quality
- **Easier Review** - Reviewer can understand change in 5 minutes
- **Safer Merges** - Small changes have minimal risk
- **Clear History** - git log shows exact purpose of each change
- **Easy Rollback** - Can revert specific feature without side effects

### Development Velocity
- **Faster Feedback** - PRs reviewed and merged quickly
- **Parallel Work** - Multiple small tasks vs one big blocker
- **Reduced Conflicts** - Small changes rarely conflict
- **Quick Wins** - Regular sense of progress and completion

### Framework Benefits
- **Agent Focus** - Clear, bounded tasks for agents
- **Quality Control** - Easy to verify small changes
- **Risk Management** - Failures limited to single feature
- **Debugging** - Issues traced to specific atomic change

## 🚦 Quality Gates

### Before Creating Branch
- [ ] Task addresses exactly ONE issue
- [ ] Expected changes <100 lines
- [ ] Files to modify identified (≤3 files)
- [ ] Success criteria measurable

### Before Creating PR
- [ ] Only planned files changed
- [ ] No "while I'm here" additions
- [ ] Tests validate specific improvement
- [ ] Title clearly states single purpose

### Before Merging
- [ ] Change addresses stated issue completely
- [ ] No unrelated modifications
- [ ] Atomic and self-contained
- [ ] Safe to deploy independently

## 🔮 Future: Worktree + Atomic = Perfect

When we add git worktrees:
```bash
# Each agent gets atomic task in isolated worktree
Agent A: ../worktree-precision (fix/signal-precision)
Agent B: ../worktree-performance (fix/checksum-sampling)
Agent C: ../worktree-cleanup (fix/unreachable-pattern)

# All working in parallel on atomic, focused tasks
# No conflicts, no interference, maximum velocity
```

## 💡 Success Mantras

1. **One Problem, One Solution** - Resist feature creep
2. **Small and Safe** - Prefer 5 small PRs over 1 large PR
3. **Clear Purpose** - Anyone should understand the change instantly
4. **Testable Change** - Improvement must be measurable
5. **Clean History** - Each commit tells a clear story

This atomic approach ensures every change is **intentional, focused, and safe**!

---

*Atomic development creates a codebase where every change has a clear purpose and can be understood instantly.*
