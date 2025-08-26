# AlphaPulse Self-Organizing Task Dependency System

## Overview

The Self-Organizing Task Dependency System transforms AlphaPulse's project management from rigid sprint-based planning to a flexible, Just-In-Time (JIT) task queue system. This prevents dependency conflicts and ensures optimal task execution order.

## Key Features

### 1. JIT Task Queue
Instead of rigid sprints, developers pull tasks from a continuously updated queue of ready work:
- Tasks become available the moment their dependencies are satisfied
- No idle time waiting for sprint boundaries
- Cross-sprint parallelization enabled

### 2. Automatic Dependency Detection
- File-level scope conflict detection
- Circular dependency prevention
- Bottleneck identification
- Critical path analysis

### 3. Visual Dependency Tracking
- Graphviz-powered dependency graphs
- Sprint timeline visualization
- Execution order planning

## System Components

### Task Metadata (YAML Frontmatter)

Every task file now includes structured metadata:

```yaml
---
task_id: S010-T001
status: TODO  # TODO, IN_PROGRESS, COMPLETE, BLOCKED
priority: CRITICAL  # CRITICAL, HIGH, MEDIUM, LOW
depends_on:  # Tasks that must complete before this can start
  - S013-T002
  - S013-T003
blocks:  # Tasks that cannot start until this completes
  - S006-T001
  - S007-T001
scope:  # Files/directories this task modifies
  - "libs/alphapulse_codec/src/"
  - "protocol_v2/src/identifiers/"
---
```

### Core Scripts

#### 1. **yaml_parser.py**
Robust YAML parsing and task metadata manipulation.

**Key Functions:**
- `parse_task_file()`: Extract metadata from task files
- `find_ready_tasks()`: Identify tasks with satisfied dependencies
- `check_scope_conflicts()`: Detect file-level conflicts
- `validate_dependencies()`: Check for cycles and missing deps

#### 2. **migrate_tasks.py**
Migrate existing tasks to new format with dependency metadata.

**Usage:**
```bash
# Migrate a single task interactively
./migrate_tasks.py task path/to/task.md

# Migrate entire sprint
./migrate_tasks.py sprint sprint-010-codec-separation

# Migrate critical sprints automatically
./migrate_tasks.py critical

# Migrate all sprints
./migrate_tasks.py all
```

#### 3. **dependency_analyzer.py**
Advanced dependency analysis and visualization.

**Features:**
- Generate Graphviz dependency graphs
- Find critical path through project
- Identify bottleneck tasks
- Suggest parallel execution groups
- Analyze scope impact

#### 4. **task-manager.sh** (Enhanced)
Main interface with new dependency-aware commands.

## New Commands

### Core Workflow Commands

#### `task-manager.sh next`
Show JIT task queue - all tasks with satisfied dependencies, sorted by priority.

```bash
$ ./task-manager.sh next
🎯 JIT Task Queue (Ready to Start)
========================================
Tasks with all dependencies satisfied:

  🔴 CODEC-001 - CODEC-001_create_codec_foundation.md
  🟡 TEST-001 - TEST-001_unit_test_framework.md
  🔵 CTRL-001 - CTRL-001_main_orchestrator.md

💡 Pick any task above to start working!
```

#### `task-manager.sh validate-plan`
Check entire project for circular dependencies and validation issues.

```bash
$ ./task-manager.sh validate-plan
🔍 Validating Project Dependencies
====================================
{
  "valid": true,
  "has_cycles": false,
  "missing_dependencies": {},
  "total_tasks": 47,
  "tasks_with_dependencies": 23
}

✅ All dependencies valid! No circular dependencies detected.
```

#### `task-manager.sh find-conflicts <task-file>`
Check if a task's scope conflicts with in-progress work.

```bash
$ ./task-manager.sh find-conflicts sprint-010/CODEC-001.md
🔍 Checking Scope Conflicts
============================
Scope conflicts detected:
  - S007-T002: relays/src/common/mod.rs, libs/types/src/messages.rs
```

### Visualization Commands

#### `task-manager.sh graph`
Generate dependency visualization graph.

```bash
$ ./task-manager.sh graph
🗺️ Generating Dependency Graph
================================
✅ Dependency graph generated:
  DOT file: .claude/tasks/dependencies.dot
  PNG file: .claude/tasks/dependencies.png

Open the PNG file to visualize task dependencies.
```

The graph shows:
- Tasks grouped by sprint (subgraphs)
- Color coding by status (green=complete, yellow=in-progress, white=todo)
- Shape coding by priority (octagon=critical, diamond=high, box=normal)
- Arrows showing dependency relationships

### Analysis Commands

#### `dependency_analyzer.py critical-path`
Find the longest dependency chain (critical path).

```bash
$ ./dependency_analyzer.py critical-path
Critical Dependency Path:
  1. S013-T001
  2. S010-T001
  3. S010-T002
  4. S006-T001
  5. S007-T002
```

#### `dependency_analyzer.py bottlenecks`
Identify tasks blocking the most other work.

```bash
$ ./dependency_analyzer.py bottlenecks
Top Bottleneck Tasks:
  CODEC-001: blocks 15 tasks [TODO]
  S013-T001: blocks 12 tasks [IN_PROGRESS]
  S010-T002: blocks 8 tasks [TODO]
```

#### `dependency_analyzer.py parallel`
Suggest tasks that can be done in parallel.

```bash
$ ./dependency_analyzer.py parallel
Tasks that can be done in parallel:
  Phase 1: CODEC-001, TEST-001, DOCS-001
  Phase 2: CODEC-002, TEST-002
  Phase 3: CODEC-003, CODEC-004, TEST-003
```

## Workflow Examples

### Starting a New Task

1. **Check ready tasks:**
```bash
$ ./task-manager.sh next
```

2. **Pick a task and check for conflicts:**
```bash
$ ./task-manager.sh find-conflicts sprint-010/CODEC-001.md
```

3. **Start work:**
```bash
$ git checkout -b refactor/codec-foundation
$ # Update status to IN_PROGRESS in task file
$ ./yaml_parser.py status sprint-010/CODEC-001.md IN_PROGRESS
```

4. **Complete task:**
```bash
$ # Update status to COMPLETE
$ ./yaml_parser.py status sprint-010/CODEC-001.md COMPLETE
$ git add -A
$ git commit -m "feat: complete CODEC-001"
```

### Adding a New Task

1. **Create task from template:**
```bash
$ cp .claude/scrum/templates/TASK_TEMPLATE.md \
     .claude/tasks/sprint-XXX/TASK-001_description.md
```

2. **Fill in metadata:**
```yaml
depends_on:
  - S010-T002  # Must wait for this
  - S007-T001  # And this
blocks:
  - S014-T001  # This can't start until we're done
scope:
  - "libs/alphapulse_codec/src/"
  - "relays/src/common/"
```

3. **Validate dependencies:**
```bash
$ ./task-manager.sh validate-plan
```

### Sprint Planning

1. **Check sprint timeline:**
```bash
$ ./dependency_analyzer.py timeline
Sprint Execution Timeline:

Phase 1:
  ⏳ sprint-013-architecture-audit (3 tasks)

Phase 2:
  ⏳ sprint-010-codec-separation (6 tasks)

Phase 3:
  ⏳ sprint-006-protocol-optimization (4 tasks)
  ⏳ sprint-007-generic-relay-refactor (6 tasks)
```

2. **Identify bottlenecks:**
```bash
$ ./dependency_analyzer.py bottlenecks
```

3. **Plan parallel work:**
```bash
$ ./dependency_analyzer.py parallel
```

## Migration Guide

### For Existing Tasks

1. **Run automatic migration for critical sprints:**
```bash
$ ./task-manager.sh migrate-critical
```

2. **Manually review and adjust:**
- Check suggested dependencies
- Verify scope declarations
- Add missing relationships

3. **Validate the migration:**
```bash
$ ./task-manager.sh validate-plan
```

### For New Tasks

Always include in YAML frontmatter:
- `depends_on`: List prerequisite task IDs
- `blocks`: List tasks that depend on this
- `scope`: List files/directories modified

## Best Practices

### 1. Dependency Declaration
- Be specific with task IDs (use full S010-T001 format)
- Include both direct and transitive dependencies when obvious
- Update `blocks` field to help others understand impact

### 2. Scope Management
- Use glob patterns for directories: `"libs/alphapulse_codec/src/*.rs"`
- Be comprehensive - include test files, configs, docs
- Check conflicts before starting work

### 3. Status Updates
- Change to IN_PROGRESS immediately when starting
- Update to COMPLETE before committing
- Use BLOCKED if waiting on external factors

### 4. Priority Setting
- CRITICAL: Blocks many tasks or on critical path
- HIGH: Important but not blocking
- MEDIUM: Standard priority
- LOW: Nice-to-have or cleanup

## Troubleshooting

### "No tasks ready" but work exists
```bash
# Check for circular dependencies
$ ./task-manager.sh validate-plan

# View bottlenecks
$ ./dependency_analyzer.py bottlenecks

# See what's blocking tasks
$ ./yaml_parser.py parse path/to/task.md | grep depends_on
```

### Circular dependency detected
```bash
# Find the cycle
$ ./dependency_analyzer.py tsort | tsort
# tsort will report the cycle

# Or check validation
$ ./task-manager.sh validate-plan
```

### Scope conflicts
```bash
# Check who's working on conflicting files
$ ./task-manager.sh find-conflicts my-task.md

# Find all tasks touching a file
$ ./dependency_analyzer.py scope "libs/types/src/protocol/"
```

## System Benefits

### 1. **Prevents Dependency Hell**
- No more out-of-order execution
- Automatic cycle detection
- Clear execution path

### 2. **Maximizes Parallelization**
- Identifies independent work
- Enables cross-sprint execution
- Reduces idle time

### 3. **Improves Visibility**
- Visual dependency graphs
- Clear bottleneck identification
- Real-time ready queue

### 4. **Enables True JIT Development**
- Pull model instead of push
- Work available instantly when unblocked
- No artificial sprint boundaries

## Future Enhancements

Potential improvements to the system:

1. **Git Integration**: Auto-detect scope from git diff
2. **PR Automation**: Generate PR descriptions from task metadata
3. **Time Estimation**: Critical path time calculations
4. **Team Assignment**: Multi-developer task allocation
5. **Progress Tracking**: Burndown charts from status changes
6. **Webhooks**: Notifications when tasks unblock

---

*The Self-Organizing Task Dependency System ensures that the right work happens at the right time, preventing the dependency conflicts that previously caused friction in the AlphaPulse development process.*