# 📋 Torq Sprint Management

Simple, sustainable task management using templates and automation.

## 🚀 Quick Start

```bash
# 1. Create a sprint (generates from templates)
./create-sprint.sh 10 "feature-name" "Sprint description"

# 2. Check status
./task-manager.sh status

# 3. Weekly maintenance
./maintenance.sh
```

## 📐 Core Workflow

### 1. Sprint Creation
```bash
./create-sprint.sh [number] [name] [description]
# Creates: .claude/tasks/sprint-XXX-name/
# With: SPRINT_PLAN.md, TASK-001_template.md, README.md
```

### 2. Task Management
Tasks use standardized format for automation:
```yaml
---
status: TODO         # TODO|IN_PROGRESS|COMPLETE|BLOCKED
priority: CRITICAL   # CRITICAL|HIGH|MEDIUM|LOW
assigned_branch: fix/issue-name
---
```

**Status Flow**: TODO → IN_PROGRESS → COMPLETE
- **Agents must** mark IN_PROGRESS when starting work  
- **Agents must** mark COMPLETE when finished
- **Critical**: Update YAML frontmatter in task files, not just TodoWrite

### Quick Status Commands
```bash
# Load helpful shortcuts
source ./scrum/status-shortcuts.sh

# Check all sprint status
sprint-status

# Visual kanban board
sprint-kanban

# Get next priority task
sprint-next

# Status update reminder
mark-done
```

### 3. Sprint Dependencies
Manage dependencies to prevent out-of-order work and conflicts:

```bash
# Check dependencies before starting sprint
./task-manager.sh check-deps sprint-007-generic-relay-refactor

# View all sprint relationships
./task-manager.sh deps
```

**SPRINT_PLAN.md Dependency Format**:
```markdown
### Sprint Dependencies
**Depends On**: 
- [x] Sprint 002: Code cleanup completed
- [ ] Sprint 004: Transport layer ready

**Provides For**:
- Sprint 008: Uses refactored relay system
- Sprint 009: Benefits from performance improvements

**✅ Can Run Concurrently With**:
- Sprint 005: Different architectural layers

**⚠️ Conflicts With**:
- Sprint 006: Both modify same core files
```

**Dependency Rules**:
- **Prerequisites**: Required sprints must be COMPLETE
- **Conflicts**: Conflicting sprints cannot be IN_PROGRESS simultaneously  
- **Loose chronology**: Dependencies guide order but allow flexibility

### 4. Three-Gate Completion
Sprints auto-archive when ALL gates pass:
1. ✅ All tasks marked COMPLETE
2. ✅ TEST_RESULTS.md shows passing
3. ✅ PR merged to main

## 📁 Structure

```
.claude/
├── scrum/
│   ├── create-sprint.sh      # Sprint creator
│   ├── task-manager.sh       # Status tracker
│   ├── maintenance.sh        # Health checker
│   ├── update-agent-docs.sh  # Doc updater
│   └── templates/            # Sprint/task templates
│       ├── SPRINT_PLAN.md
│       ├── TASK_TEMPLATE.md
│       └── TEST_RESULTS.md
└── tasks/
    ├── sprint-XXX-name/      # Active sprints
    └── archive/              # Completed sprints
```

## 🔧 Key Commands

### task-manager.sh
```bash
./task-manager.sh status        # Current sprint overview
./task-manager.sh kanban        # Visual kanban board
                                 # 🔴 = Unmodified from template
                                 # 🟡 = Work in progress  
                                 # 🟢 = Complete/archived
./task-manager.sh next          # Highest priority task
./task-manager.sh scan          # All tasks across sprints
./task-manager.sh auto-archive  # Archive completed sprints
```

### maintenance.sh
```bash
./maintenance.sh  # Weekly health check
# - Archives completed sprints
# - Finds stale tasks
# - Checks format compliance
# - Updates documentation
```

## 📝 Templates

Templates are self-contained with instructions:

- **SPRINT_PLAN.md**: Goals, tasks, dependencies
- **TASK_TEMPLATE.md**: Problem, solution, testing, git workflow
- **TEST_RESULTS.md**: Test outcomes for completion gate

Copy from `templates/` or use `create-sprint.sh`.

## 🧪 Testing Standards

See [TESTING_STANDARDS.md](TESTING_STANDARDS.md) for comprehensive testing pyramid:
- Unit tests (required)
- Integration tests (when applicable)
- E2E tests (critical paths only)

## 📊 Live Task Tracking

Use `./task-manager.sh status` for current priorities and dynamic task tracking.

## 🔄 Automation

- **Git hooks**: Auto-archive on PR merge (`.git/hooks/post-merge`)
- **GitHub Actions**: Sprint archiving (`.github/workflows/sprint-archive.yml`)
- **Weekly maintenance**: Prevents cruft accumulation

## ⚠️ Important Rules

1. **Never work on main** - Always use feature branches
2. **Update status immediately** - Keep tasks current
3. **Tests before complete** - No TEST_RESULTS.md = not done
4. **Use templates** - Don't create tasks manually
5. **Run maintenance weekly** - Prevents decay

## 🎯 That's It!

The workflow is intentionally simple:
1. Create sprints from templates
2. Update task status as you work
3. System auto-archives when done
4. Run maintenance weekly

Templates have all the instructions. Scripts handle the automation.