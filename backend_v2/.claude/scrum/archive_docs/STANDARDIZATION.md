# Sprint & Task Standardization Guide

## Overview
This document defines the **mandatory** format standards for all sprints and tasks in the Torq project. Adherence to these standards enables automated tracking, archiving, and management.

## 🎯 Why Standardization Matters

1. **Automation**: Scripts can reliably parse and update task status
2. **Consistency**: All developers follow the same workflow
3. **Transparency**: Git history shows clear progress
4. **Quality**: Three-gate verification prevents incomplete work
5. **Scalability**: System works for 1 or 10 developers

## 📋 Required Files Per Sprint

Every sprint directory MUST contain:

```
.claude/tasks/sprint-XXX-name/
├── SPRINT_PLAN.md       # Sprint goals and timeline (from template)
├── TASK-001_*.md        # Individual tasks (from template)
├── TASK-002_*.md
├── TEST_RESULTS.md      # Created when tests complete (from template)
└── README.md            # Sprint-specific instructions
```

## 🔧 Quick Start Commands

### Create New Sprint
```bash
# Use the automated creator
./.claude/scrum/create-sprint.sh 007 "feature-name" "Description of sprint"

# Or manually
mkdir -p .claude/tasks/sprint-007-feature-name
cp .claude/scrum/templates/* .claude/tasks/sprint-007-feature-name/
```

### Start a Task
```bash
# 1. Read the task
cat .claude/tasks/sprint-007/TASK-001_description.md

# 2. Checkout branch (NEVER use main!)
git checkout -b fix/task-001-issue

# 3. Update status in task file
# Change: status: TODO → status: IN_PROGRESS
```

### Complete a Task
```bash
# 1. Run tests
cargo test

# 2. Update task file
# Change: status: IN_PROGRESS → status: COMPLETE
# Change: completed: null → completed: 2025-01-27

# 3. Create PR
gh pr create --title "Sprint 007: Task 001 - Description"
```

### Check Sprint Status
```bash
# Overall status
./.claude/scrum/task-manager.sh status

# Sprint-specific check
./.claude/tasks/sprint-007/check-status.sh

# Ready for archive?
./.claude/scrum/task-manager.sh check-complete sprint-007
```

## 📐 Format Specifications

### Task Status Format (MUST use one of these)

#### Option 1: YAML Frontmatter (Preferred)
```yaml
---
status: TODO
priority: CRITICAL
---
```

#### Option 2: Markdown Bold
```markdown
**Status**: TODO
**Priority**: CRITICAL
```

### Valid Status Values
- `TODO` - Not started
- `IN_PROGRESS` - Currently working
- `COMPLETE` - Finished successfully
- `BLOCKED` - Cannot proceed

### Valid Priority Values
- `CRITICAL` - Immediate action required
- `HIGH` - Should be done soon
- `MEDIUM` - Normal priority
- `LOW` - Nice to have

### Branch Naming Convention
- `fix/description` - Bug fixes
- `feat/description` - New features
- `perf/description` - Performance
- `test/description` - Tests
- `docs/description` - Documentation

## 🚫 Common Mistakes to Avoid

### ❌ DON'T: Work on main branch
```bash
# WRONG
git checkout main
vim file.rs
git commit -m "Fixed issue"
```

### ✅ DO: Use feature branches
```bash
# CORRECT
git checkout -b fix/specific-issue
vim file.rs
git commit -m "fix: resolve specific issue"
git push origin fix/specific-issue
```

### ❌ DON'T: Skip status updates
```markdown
# File shows TODO but you're done
status: TODO  # WRONG - not updated!
```

### ✅ DO: Keep status current
```markdown
# Update immediately when complete
status: COMPLETE  # CORRECT
completed: 2025-01-27
```

### ❌ DON'T: Mark complete without tests
```bash
# No TEST_RESULTS.md created
# Task marked COMPLETE anyway  # WRONG!
```

### ✅ DO: Document test results
```bash
cargo test > test_output.txt
# Create TEST_RESULTS.md from template
# Then mark task COMPLETE
```

## 🔄 Complete Workflow Example

```bash
# Day 1: Sprint Creation
./create-sprint.sh 008 "performance-optimization" "Improve hot path performance"
cd .claude/tasks/sprint-008-performance-optimization
vim SPRINT_PLAN.md  # Define goals

# Day 2: Task Creation
cp TASK-001_rename_me.md TASK-001_optimize-checksum.md
vim TASK-001_optimize-checksum.md  # Fill in details

# Day 3: Development
git checkout -b perf/optimize-checksum
# Update task: status: TODO → IN_PROGRESS
# ... do work ...
cargo test
# Update task: status: IN_PROGRESS → COMPLETE

# Day 4: Testing & PR
cp ../../scrum/templates/TEST_RESULTS.md .
vim TEST_RESULTS.md  # Add test output
git push origin perf/optimize-checksum
gh pr create

# Day 5: Merge & Archive
# After PR approved and merged
../../scrum/task-manager.sh check-complete sprint-008
# If all gates pass, auto-archives
```

## 🤖 Automation Integration

The standardized format enables:

1. **task-manager.sh** - Parses status/priority fields
2. **Git hooks** - Detects sprint completion on merge
3. **GitHub Actions** - Auto-archives completed sprints
4. **CI/CD** - Validates format compliance

## 📊 Verification Commands

```bash
# Verify format compliance
grep -E "status:|Status:" .claude/tasks/sprint-*/TASK-*.md

# Check for main branch commits (should be empty)
git log main --format="%an: %s" --since="last week" | grep -v "Merge"

# Validate TEST_RESULTS.md exists for completed sprints
find .claude/tasks -name "TEST_RESULTS.md" -path "*/sprint-*"
```

## 🎓 Training New Developers

1. **First Day**: Read this guide + templates
2. **First Task**: Use templates exactly as-is
3. **First PR**: Must follow branch conventions
4. **First Sprint**: Shadow experienced developer

## 📝 Checklist for Sprint Leaders

- [ ] Sprint created from template
- [ ] All tasks have clear acceptance criteria
- [ ] Branch names specified in each task
- [ ] Dependencies documented
- [ ] TEST_RESULTS.md template ready
- [ ] Team knows the standards

## 🚀 Benefits of Following Standards

1. **Predictable Automation**: Scripts always work
2. **Clear History**: Git log tells the story
3. **Quality Gates**: Nothing ships incomplete
4. **Easy Onboarding**: New devs follow templates
5. **Reduced Meetings**: Status visible in files

## ⚠️ Enforcement

- **Pre-commit hooks**: Validate format
- **CI checks**: Block incorrectly formatted PRs
- **task-manager.sh**: Won't recognize non-standard formats
- **Auto-archive**: Only works with proper format

Follow these standards and the system works smoothly. Ignore them and you'll be manually updating everything!