# Sprint Archiving System

## Overview
The Torq task management system now includes automated sprint archiving that ensures sprints are only archived when truly complete: tasks done, tests passing, and PR merged.

## Completion Criteria
A sprint is considered complete and ready for archiving when ALL of the following are met:

1. ✅ **All tasks marked COMPLETE/DONE** - Every task file shows completed status
2. ✅ **Tests verified passing** - TEST_RESULTS.md confirms all tests pass
3. ✅ **PR merged to main** - Git history shows the sprint PR was merged

## Manual Commands

### Check if a sprint is ready for archiving:
```bash
./.claude/scrum/task-manager.sh check-complete sprint-003-data-integrity
```

### Archive a specific sprint:
```bash
./.claude/scrum/task-manager.sh archive-sprint sprint-003-data-integrity
```

### Check and archive all completed sprints:
```bash
./.claude/scrum/task-manager.sh auto-archive
```

### Force archive (skip checks - use with caution):
```bash
./.claude/scrum/task-manager.sh archive-sprint sprint-003-data-integrity --force
```

## Automated Archiving

### 1. Local Git Hook
When you merge a PR locally, the post-merge hook automatically:
- Detects the sprint number from the merge
- Checks completion criteria
- Archives if all criteria are met

Location: `.git/hooks/post-merge`

### 2. GitHub Actions
When a PR is merged on GitHub:
- Workflow triggers on PR close
- Extracts sprint number from PR title
- Runs completion checks
- Archives and commits changes if ready

Location: `.github/workflows/sprint-archive.yml`

### 3. CI/CD Integration
For other CI/CD systems:
```bash
# After PR merge, call:
./.claude/scrum/ci-archive-hook.sh "Sprint 003 merged"

# Or check all sprints:
./.claude/scrum/ci-archive-hook.sh
```

## Archive Structure

Archived sprints are moved to:
```
.claude/tasks/archive/
├── sprint-001-initial-setup/
│   ├── ARCHIVED.md           # Auto-generated summary
│   ├── TEST_RESULTS.md       # Test verification
│   └── [all task files]
└── sprint-003-data-integrity/
    └── ...
```

## TEST_RESULTS.md Format

For automatic test verification, create a `TEST_RESULTS.md` file in the sprint directory with one of these patterns:

```markdown
# Test Results

✅ All tests passing
```

Or:
```markdown
Test Status: PASS
All tests completed successfully.
```

## Troubleshooting

### Sprint not archiving automatically?
Check the three criteria:
```bash
./.claude/scrum/task-manager.sh check-complete sprint-name
```

### Need to archive without PR merge?
Use the force flag (only for exceptional cases):
```bash
./.claude/scrum/task-manager.sh archive-sprint sprint-name --force
```

### Archive was premature?
Move the sprint back from archive:
```bash
mv .claude/tasks/archive/sprint-003-data-integrity .claude/tasks/
```

## Best Practices

1. **Always create TEST_RESULTS.md** when tests complete
2. **Use sprint numbers in PR titles** (e.g., "Sprint 003: Fix data integrity")
3. **Mark tasks as COMPLETE** as you finish them, not in batch
4. **Don't force archive** unless absolutely necessary
5. **Review archive summary** to ensure nothing was missed

## Status Field Format

For proper detection, use one of these formats in task files:

```markdown
**Status**: COMPLETE
```

Or:
```markdown
Status: DONE
```

Or in YAML frontmatter (recommended):
```yaml
---
status: COMPLETE
priority: CRITICAL
completed: 2025-01-27
---
```