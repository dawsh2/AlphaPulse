---
task_id: AUDIT-005
status: COMPLETE
priority: MEDIUM
estimated_hours: 3
assigned_branch: feat/manage-control-script
assignee: Claude
created: 2025-08-26
completed: 2025-08-27
depends_on: []  # Independent of other AUDIT tasks
blocks: []
scope:
  - "scripts/manage.sh"  # Main control script
  - "scripts/lib/"  # Internal script library
---

# AUDIT-005: Create manage.sh Control Script

## Git Worktree Setup (REQUIRED)
```bash
# Create worktree for this task
git worktree add -b feat/manage-control-script ../audit-005-worktree
cd ../audit-005-worktree
```

## Status
**Status**: TODO
**Priority**: MEDIUM
**Worktree**: `../audit-005-worktree` (Branch: `feat/manage-control-script`)
**Estimated**: 3 hours

## Problem Statement
Torq currently lacks a unified management interface for system lifecycle. Starting and stopping the system requires knowledge of multiple scripts and manual process management.

## Acceptance Criteria
- [ ] Create main `manage.sh` dispatcher script
- [ ] Implement `up`, `down`, `status`, `logs`, `restart` commands
- [ ] Move existing scripts to `lib/` subdirectory
- [ ] Add PID tracking for process management
- [ ] Auto-create required directories (logs/, .pids/)
- [ ] Script works from any directory location
- [ ] Comprehensive error handling and user feedback

## Target Script Structure
```
scripts/
├── manage.sh          # Main control script
└── lib/               # Internal scripts
    ├── startup.sh     # System startup logic
    ├── shutdown.sh    # Graceful shutdown
    ├── status.sh      # Process status checking
    └── logs.sh        # Log aggregation and viewing
```

## Command Interface
```bash
# Basic lifecycle management
./scripts/manage.sh up      # Start all Torq services
./scripts/manage.sh down    # Stop all services gracefully
./scripts/manage.sh restart # Stop and start all services
./scripts/manage.sh status  # Show status of all services
./scripts/manage.sh logs    # Stream logs from all services

# Usage help
./scripts/manage.sh         # Show usage information
./scripts/manage.sh --help  # Detailed help
```

## Implementation Steps
1. **Create Directory Structure**
   - Create `scripts/lib/` directory
   - Set up module organization

2. **Build Main Dispatcher**
   - Create `manage.sh` with command parsing
   - Implement command validation and help
   - Add path resolution for any-directory execution

3. **Implement Subcommands**
   - `startup.sh`: Service startup logic with PID tracking
   - `shutdown.sh`: Graceful shutdown with timeout handling
   - `status.sh`: Process status checking and reporting
   - `logs.sh`: Log aggregation and streaming

4. **Add Process Management**
   - PID file creation and tracking
   - Service health monitoring
   - Automatic directory creation (logs/, .pids/)

5. **Error Handling**
   - Comprehensive error messages
   - Graceful fallbacks for missing dependencies
   - User-friendly feedback

## Files to Create/Modify
- `scripts/manage.sh` - Main control script
- `scripts/lib/startup.sh` - System startup logic
- `scripts/lib/shutdown.sh` - Graceful shutdown
- `scripts/lib/status.sh` - Process status checking
- `scripts/lib/logs.sh` - Log viewing and streaming
- Move existing utility scripts to `lib/` as needed

## Success Criteria
- Single command starts entire Torq system
- Clean shutdown stops all processes gracefully
- Status command shows health of all services
- Log streaming provides unified view of system activity
- Scripts work reliably from any directory
- Clear error messages guide users when issues occur

## Integration Points
- Works with existing service startup scripts
- Compatible with current logging setup
- Integrates with PID management for monitoring
- Provides foundation for Docker/systemd integration