# Sprint 011: Control Script Pattern - System Management Orchestration

Implement unified control script pattern (manage.sh) for streamlined system management

## Quick Start

1. **Review sprint plan**: 
   ```bash
   cat SPRINT_PLAN.md
   ```

2. **Create tasks from template**:
   ```bash
   cp TASK-001_rename_me.md CTRL-002_service_startup_engine.md
   vim CTRL-002_service_startup_engine.md
   ```

3. **Start work**:
   ```bash
   # Never work on main!
   git checkout -b feat/sprint-011-task-002
   ```

4. **Check status**:
   ```bash
   ../../scrum/task-manager.sh sprint-011
   ```

## Important Rules

- **NEVER commit to main branch**
- **Always update task status** (TODO → IN_PROGRESS → COMPLETE)
- **Create TEST_RESULTS.md** when tests pass
- **Use PR for all merges**

## Directory Structure
```
.
├── README.md              # This file
├── SPRINT_PLAN.md        # Sprint goals and timeline
├── CTRL-001_*.md         # Main orchestrator task ✅
├── TASK-001_rename_me.md # Template for creating new tasks
├── TEST_RESULTS.md       # Created when tests complete
└── [other tasks]         # Copy template to create
```

## Sprint Objectives
Create a unified `./scripts/manage.sh` script that provides:
- `./scripts/manage.sh up` - Start entire system
- `./scripts/manage.sh down` - Stop all services gracefully  
- `./scripts/manage.sh status` - Check what's running
- `./scripts/manage.sh logs` - Stream all service logs
- `./scripts/manage.sh restart` - Full system restart

This will replace scattered script management with a single, intuitive interface.