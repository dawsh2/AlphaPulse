# Sprint 999: test-validation

Test sprint to validate scrum processes and tooling

## Quick Start

1. **Review sprint plan**: 
   ```bash
   cat SPRINT_PLAN.md
   ```

2. **Create tasks from template**:
   ```bash
   cp TASK-001_rename_me.md TASK-001_actual_task_name.md
   vim TASK-001_actual_task_name.md
   ```

3. **Start work**:
   ```bash
   # Never work on main!
   git checkout -b feat/sprint-999-task-001
   ```

4. **Check status**:
   ```bash
   ../../scrum/task-manager.sh status
   ```

## Important Rules

- **NEVER commit to main branch**
- **Always update task status** (TODO → IN_PROGRESS → COMPLETE)
- **Create TEST_RESULTS.md** when tests pass
- **Use PR for all merges**

## Directory Structure
```
.
├── README.md           # This file
├── SPRINT_PLAN.md     # Sprint goals and timeline
├── TASK-001_*.md      # Individual task files
├── TASK-002_*.md
├── TEST_RESULTS.md    # Created when tests complete
└── [archived]         # Moved here when sprint completes
```
