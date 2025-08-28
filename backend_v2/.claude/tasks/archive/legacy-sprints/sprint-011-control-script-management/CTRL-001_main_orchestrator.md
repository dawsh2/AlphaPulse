---
task_id: CTRL-001
status: COMPLETE
priority: CRITICAL
estimated_hours: 3
assigned_branch: feat/control-script-orchestrator
assignee: TBD
created: 2025-08-26
completed: 2025-08-27
depends_on:
  - CODEC-002  # Need protocol refactoring complete
  - TASK-002   # Need relay refactor complete
blocks: []
scope:
  - "manage.sh"  # Main orchestrator script
  - "scripts/start_*.sh"  # Integration with existing scripts
  - ".claude/scrum/task-manager.sh"  # Integration with task management
---

# CTRL-001: Main Orchestrator Script (manage.sh)

## 🔴 CRITICAL INSTRUCTIONS

### 0. 📋 MARK AS IN-PROGRESS IMMEDIATELY
**⚠️ FIRST ACTION: Change status when you start work!**
```yaml
# Edit the YAML frontmatter above:
status: TODO → status: IN_PROGRESS

# This makes the kanban board show you're working on it!
```

### 1. Git Branch Safety
```bash
# BEFORE STARTING - VERIFY YOU'RE NOT ON MAIN:
git branch --show-current

# If you see "main", IMMEDIATELY run:
git worktree add -b feat/control-script-orchestrator

# NEVER commit directly to main!
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
**Branch**: `feat/control-script-orchestrator`
**Estimated**: 3 hours

## Problem Statement
Torq developers currently struggle with complex, scattered service management. Starting the system requires running multiple commands in sequence, tracking PIDs manually, and managing logs from different locations. This creates friction for both development and deployment.

## Acceptance Criteria
- [ ] Single `manage.sh` script provides all core commands (up/down/status/logs/restart)
- [ ] Script auto-creates required directories (logs/, .pids/)
- [ ] Command validation with helpful usage text
- [ ] Delegated execution to lib/ scripts for modularity
- [ ] Error handling with clear user feedback
- [ ] Script is executable and has proper shebang
- [ ] Works from any directory location
- [ ] Unit tests for command parsing logic
- [ ] Integration test for full script workflow

## Technical Approach
### Files to Create
- `scripts/manage.sh` - Main orchestrator script
- `scripts/lib/` - Directory for implementation scripts
- `logs/` - Centralized log directory (auto-created)
- `.pids/` - Process ID tracking directory (auto-created)

### Implementation Steps - TDD REQUIRED ⚠️
**🚨 CRITICAL: Follow Test-Driven Development - Write Tests FIRST!**

1. **RED**: Write failing tests first (before any implementation)
   ```bash
   # Create test file first
   cat > test_manage.sh << 'EOF'
   #!/bin/bash
   # Test the manage.sh script behavior
   
   test_command_validation() {
       # Test invalid command
       output=$(./scripts/manage.sh invalid 2>&1)
       if [[ $output == *"Usage:"* ]]; then
           echo "✅ Invalid command test passed"
       else
           echo "❌ Invalid command test failed"
           return 1
       fi
   }
   
   test_directory_creation() {
       # Test that directories are created
       rm -rf logs .pids
       ./scripts/manage.sh status > /dev/null 2>&1
       if [[ -d "logs" && -d ".pids" ]]; then
           echo "✅ Directory creation test passed"
       else
           echo "❌ Directory creation test failed"
           return 1
       fi
   }
   
   # Run tests
   test_command_validation
   test_directory_creation
   EOF
   
   chmod +x test_manage.sh
   ./test_manage.sh  # Should FAIL initially
   ```

2. **GREEN**: Implement minimal code to make tests pass
   ```bash
   # Create basic manage.sh structure
   cat > scripts/manage.sh << 'EOF'
   #!/bin/bash
   set -e
   
   SCRIPTS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" &> /dev/null && pwd)"
   PROJECT_ROOT="$(dirname "$SCRIPTS_DIR")"
   LIB_DIR="$SCRIPTS_DIR/lib"
   
   # Ensure directories exist
   mkdir -p "$PROJECT_ROOT/logs"
   mkdir -p "$PROJECT_ROOT/.pids"
   
   case "$1" in
     up|down|status|logs|restart)
       echo "Command: $1" # Placeholder
       ;;
     *)
       echo "Usage: $0 {up|down|status|logs|restart}"
       exit 1
       ;;
   esac
   EOF
   
   chmod +x scripts/manage.sh
   ./test_manage.sh  # Should PASS now
   ```

3. **REFACTOR**: Add proper command delegation
   ```bash
   # Add delegation to lib scripts
   case "$1" in
     up)
       echo "Starting system..."
       "$LIB_DIR/startup.sh"
       ;;
     down)
       echo "Stopping system..."
       "$LIB_DIR/shutdown.sh"
       ;;
     status)
       "$LIB_DIR/status.sh"
       ;;
     logs)
       "$LIB_DIR/logs.sh"
       ;;
     restart)
       echo "Restarting system..."
       "$LIB_DIR/shutdown.sh"
       "$LIB_DIR/startup.sh"
       ;;
     *)
       echo "Usage: $0 {up|down|status|logs|restart}"
       exit 1
       ;;
   esac
   ```

4. **REPEAT**: Add error handling and validation tests

## Script Architecture
```bash
#!/bin/bash
set -e # Exit on any error

# Path resolution
SCRIPTS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" &> /dev/null && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPTS_DIR")"
LIB_DIR="$SCRIPTS_DIR/lib"

# Ensure support directories exist
mkdir -p "$PROJECT_ROOT/logs"
mkdir -p "$PROJECT_ROOT/.pids"

# Command dispatch
case "$1" in
  up)
    echo "Starting Torq system..."
    "$LIB_DIR/startup.sh"
    ;;
  down)
    echo "Stopping Torq system..."
    "$LIB_DIR/shutdown.sh"
    ;;
  status)
    "$LIB_DIR/status.sh"
    ;;
  logs)
    "$LIB_DIR/logs.sh"
    ;;
  restart)
    echo "Restarting Torq system..."
    "$LIB_DIR/shutdown.sh"
    sleep 2  # Allow shutdown to complete
    "$LIB_DIR/startup.sh"
    ;;
  *)
    echo "Usage: $0 {up|down|status|logs|restart}"
    echo ""
    echo "Commands:"
    echo "  up      - Start all Torq services"
    echo "  down    - Stop all services gracefully"
    echo "  status  - Show status of all services"
    echo "  logs    - Stream logs from all services"
    echo "  restart - Stop and start all services"
    exit 1
    ;;
esac
```

## Testing Requirements - Shell Script Testing

### Unit Tests (Command Validation)
```bash
#!/bin/bash
# tests/test_manage_unit.sh

test_invalid_command() {
    output=$(./scripts/manage.sh invalid_command 2>&1)
    [[ $output == *"Usage:"* ]] || return 1
    echo "✅ Invalid command handling works"
}

test_directory_creation() {
    # Clean slate
    rm -rf logs .pids
    
    # Run any command to trigger directory creation
    ./scripts/manage.sh status > /dev/null 2>&1 || true
    
    [[ -d "logs" && -d ".pids" ]] || return 1
    echo "✅ Directory auto-creation works"
}

test_script_executable() {
    [[ -x "scripts/manage.sh" ]] || return 1
    echo "✅ Script is executable"
}

# Run all unit tests
test_invalid_command
test_directory_creation  
test_script_executable
echo "All unit tests passed!"
```

### Integration Tests (Full Workflow)
```bash
#!/bin/bash
# tests/test_manage_integration.sh

test_command_delegation() {
    # Mock lib scripts for testing
    mkdir -p scripts/lib
    
    cat > scripts/lib/status.sh << 'EOF'
#!/bin/bash
echo "Mock status output"
EOF
    chmod +x scripts/lib/status.sh
    
    # Test delegation works
    output=$(./scripts/manage.sh status)
    [[ $output == *"Mock status"* ]] || return 1
    echo "✅ Command delegation works"
}

test_path_independence() {
    # Test script works from different directories
    cd /tmp
    output=$(/Users/daws/torq/backend_v2/scripts/manage.sh status 2>&1)
    cd - > /dev/null
    
    # Should not fail due to path issues
    [[ $? -eq 0 ]] || return 1
    echo "✅ Path independence works"
}

test_command_delegation
test_path_independence
echo "All integration tests passed!"
```

## Error Handling Requirements
- Graceful handling of missing lib/ scripts
- Clear error messages for permission issues
- Validation that required directories can be created
- Proper exit codes for automation use

## Git Workflow
```bash
# 1. Start on your branch
git worktree add -b feat/control-script-orchestrator

# 2. Create test files first (TDD)
mkdir -p tests
# Create unit tests (as shown above)
# Create integration tests

# 3. Verify tests fail initially
./tests/test_manage_unit.sh        # Should fail
./tests/test_manage_integration.sh # Should fail

# 4. Implement manage.sh to make tests pass

# 5. Commit when tests pass
git add -A
git commit -m "feat: implement main orchestrator script (manage.sh)

- Add command dispatch for up/down/status/logs/restart
- Auto-create logs/ and .pids/ directories
- Include error handling and usage help
- Add unit and integration tests
- Script works from any directory"

# 6. Push and create PR
git push origin feat/control-script-orchestrator
gh pr create --title "feat: Main Control Script Orchestrator" --body "Implements CTRL-001: Core manage.sh dispatcher script"
```

## Completion Checklist
- [ ] **🚨 STEP 0: Changed status to IN_PROGRESS when starting** ← AGENTS MUST DO THIS!
- [ ] Working on correct branch (not main)
- [ ] **🚨 TDD FOLLOWED: Tests written BEFORE implementation**
- [ ] manage.sh script created and executable
- [ ] All five commands supported (up/down/status/logs/restart)
- [ ] Directory auto-creation works
- [ ] Command validation and help text included
- [ ] Error handling implemented
- [ ] Unit tests pass
- [ ] Integration tests pass
- [ ] Works from any directory location
- [ ] PR created and reviewed
- [ ] **🚨 STEP FINAL: Updated task status to COMPLETE** ← AGENTS MUST DO THIS!

## ⚠️ IMPORTANT: Status Updates Required
**When you START this task, you MUST:**
1. **IMMEDIATELY** change `status: TODO` to `status: IN_PROGRESS` in the YAML frontmatter above
2. This makes the kanban board show you're working on it

**When you FINISH this task, you MUST:**
1. Change `status: IN_PROGRESS` to `status: COMPLETE` in the YAML frontmatter above  
2. This is NOT optional - the task-manager.sh depends on accurate status
3. Update immediately after PR is merged, not before

**Status Flow: TODO → IN_PROGRESS → COMPLETE**

## Notes
This is the foundation script that all other control scripts depend on. Focus on robust command parsing and error handling since this will be the primary user interface.