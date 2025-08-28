# Org-Mode Task Management System Specification

## Overview

This specification defines the Torq task management system based on Org-mode files, enabling DAG-based parallel task execution and human-AI collaborative management.

## Task State Keywords

```org
#+TODO: TODO NEXT IN-PROGRESS WAITING | DONE CANCELLED
```

- **TODO**: Task defined but not started
- **NEXT**: Ready for immediate execution (no pending dependencies)
- **IN-PROGRESS**: Currently being worked on
- **WAITING**: Blocked on external factor
- **DONE**: Completed successfully
- **CANCELLED**: Won't be completed

## Task Structure

### Basic Task Format

```org
* TODO Task heading here                              :tag1:tag2:
  SCHEDULED: <2024-01-15 Mon>
  DEADLINE: <2024-01-20 Sat>
  :PROPERTIES:
  :ID:          UNIQUE-TASK-ID
  :CREATED:     [2024-01-15 Mon 10:00]
  :EFFORT:      6h
  :PRIORITY:    A
  :GOAL:        goal-identifier
  :DEPENDS:     TASK-ID-1 TASK-ID-2
  :BLOCKS:      TASK-ID-3 TASK-ID-4
  :PARALLEL_GROUP: group-name
  :END:
  
  Task description and context goes here.
  Can be multiple lines.
  
  - [ ] Subtask 1
  - [X] Subtask 2
  - [ ] Subtask 3
```

## Properties Specification

### Required Properties

- **:ID:** - Unique task identifier (e.g., `VALIDATE-001`, `FIX-CODEC-003`)
  - Format: `[CATEGORY]-[NUMBER]` or custom unique string
  - Must be unique across entire task system

### Optional Properties

- **:CREATED:** - Timestamp when task was created
- **:EFFORT:** - Estimated time (e.g., `2h`, `3d`, `1w`)
- **:PRIORITY:** - A (highest) through D (lowest)
- **:GOAL:** - Parent goal/sprint identifier
- **:DEPENDS:** - Space-separated list of task IDs that must complete first
- **:BLOCKS:** - Space-separated list of task IDs blocked by this task
- **:PARALLEL_GROUP:** - Tasks in same group can execute in parallel
- **:ASSIGNEE:** - Who's responsible (human or agent identifier)
- **:COMPLEXITY:** - S, M, L, XL for sizing
- **:RISK:** - LOW, MEDIUM, HIGH for risk assessment

## Tags Specification

### System Tags

- `:critical:` - Mission-critical task
- `:performance:` - Performance-related
- `:refactor:` - Code refactoring
- `:bug:` - Bug fix
- `:feature:` - New feature
- `:test:` - Testing-related
- `:docs:` - Documentation

### Domain Tags

- `:codec:` - Protocol/codec related
- `:relay:` - Relay system
- `:adapter:` - Exchange adapters
- `:strategy:` - Trading strategies

## Goal/Sprint Structure

```org
* GOAL Post-Refactor Quality Validation               :sprint:goal:
  :PROPERTIES:
  :ID:          SPRINT-015
  :TYPE:        goal
  :TIMELINE:    [2024-01-15 Mon]--[2024-01-25 Thu]
  :STATUS:      active
  :END:
  
  Goal description and success criteria.
  
** TODO Phase 1: Core validation                      :phase:
   :PROPERTIES:
   :ID:          PHASE-015-1
   :GOAL:        SPRINT-015
   :END:
   
*** TODO Validate TLV structures                      :critical:
    :PROPERTIES:
    :ID:          VALIDATE-001
    :GOAL:        SPRINT-015
    :DEPENDS:     
    :EFFORT:      6h
    :END:
```

## Dependency Rules

1. **Forward Dependencies** (`:DEPENDS:`):
   - Task cannot start until all DEPENDS tasks are DONE
   - Used for prerequisite relationships

2. **Reverse Dependencies** (`:BLOCKS:`):
   - Listed tasks cannot start until this task is DONE
   - Alternative way to express dependencies

3. **Parallel Groups** (`:PARALLEL_GROUP:`):
   - Tasks in same group have no inter-dependencies
   - Can be executed simultaneously

## File Organization

```
.claude/tasks/
├── active.org          # Currently active tasks
├── backlog.org         # Future tasks not yet scheduled
├── completed.org       # Archived completed tasks
├── goals.org           # High-level goals and sprints
└── templates/
    └── task.org        # Task templates
```

## JSON Export Format

The org_task_parser.py will export to this JSON structure:

```json
{
  "tasks": [
    {
      "id": "VALIDATE-001",
      "heading": "Validate TLV structures",
      "state": "TODO",
      "priority": "A",
      "tags": ["critical", "codec"],
      "properties": {
        "effort": "6h",
        "goal": "SPRINT-015",
        "depends": ["REFACTOR-001", "REFACTOR-002"],
        "blocks": [],
        "parallel_group": "validation"
      },
      "body": "Task description text...",
      "subtasks": [
        {"text": "Subtask 1", "done": false},
        {"text": "Subtask 2", "done": true}
      ],
      "scheduled": "2024-01-15T10:00:00Z",
      "deadline": "2024-01-20T17:00:00Z"
    }
  ],
  "metadata": {
    "total_tasks": 42,
    "todo_count": 15,
    "in_progress_count": 3,
    "done_count": 24,
    "parse_timestamp": "2024-01-15T10:30:00Z"
  }
}
```

## DAG Traversal Rules

1. **Ready Tasks** = Tasks where:
   - State is TODO or NEXT
   - All DEPENDS tasks are DONE
   - No WAITING status

2. **Parallel Execution** = Ready tasks that:
   - Have no dependencies between them
   - Or share same PARALLEL_GROUP

3. **Priority Ordering**:
   - Priority A > B > C > D
   - Within same priority: earliest DEADLINE first
   - Tiebreaker: lexicographic ID ordering

## Validation Rules

1. **ID Uniqueness**: No duplicate IDs across all org files
2. **Dependency Cycles**: No circular dependencies allowed
3. **Valid References**: All DEPENDS/BLOCKS IDs must exist
4. **State Consistency**: IN-PROGRESS tasks should have STARTED timestamp
5. **Effort Format**: Must match pattern like "2h", "3d", "1w"

## Example Queries

```python
# Get all ready tasks
ready = [t for t in tasks if t.is_ready()]

# Get tasks for specific goal
goal_tasks = [t for t in tasks if t.goal == "SPRINT-015"]

# Get critical path
critical = dag.get_critical_path("GOAL-001")

# Get parallel work sets
parallel_sets = dag.get_parallel_execution_sets()
```

## Migration from Current System

```bash
# Convert existing sprint files
python tools/migrate_to_org.py \
  --source .claude/tasks/sprint-015-*/ \
  --output .claude/tasks/active.org \
  --preserve-ids \
  --infer-dependencies
```

## Version

- **Specification Version**: 1.0.0
- **Date**: 2024-01-15
- **Status**: Draft

## Future Enhancements

- [ ] Recurring tasks support
- [ ] Task templates with variable substitution
- [ ] Time tracking integration
- [ ] Automated dependency inference
- [ ] Task estimation accuracy tracking