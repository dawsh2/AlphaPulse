# Scrum Leader Agent - Current Practices Documentation

## Overview
This document captures the AlphaPulse scrum practices as implemented in January 2025. The system uses a file-based, git-trackable approach with automated sprint archiving.

## Current Sprint Management System

### 1. Directory Structure
```
.claude/
├── tasks/                       # Sprint management
│   ├── sprint-002-cleanup/      # Active sprints
│   ├── sprint-004-mycelium-runtime/
│   ├── sprint-005-mycelium-mvp/
│   ├── sprint-006-protocol-optimization/
│   └── archive/                 # Completed sprints
│       └── sprint-003-data-integrity/
├── scrum/                       # Management tools
│   ├── task-manager.sh          # Dynamic task tracking
│   ├── ci-archive-hook.sh       # CI integration
│   └── ARCHIVING.md             # Archive documentation
└── agents/
    └── scrum-leader.md          # Agent configuration
```

### 2. Sprint Lifecycle

#### Creation Phase
1. **Identify Need**: Based on roadmap or emergent issues
2. **Create Directory**: `sprint-XXX-descriptive-name/`
3. **Define Plan**: Create `SPRINT_PLAN.md` with:
   - Sprint duration (typically 5 days)
   - Objectives and success metrics
   - Task breakdown with time estimates
   - Risk mitigation strategies

4. **Generate Tasks**: Individual files per task
   - Format: `TASK-ID_description.md`
   - Contains: Status, Priority, Problem, Solution, Acceptance Criteria

#### Execution Phase
- **Status Tracking**: Via markdown field updates
- **Priority Management**: CRITICAL → HIGH → MEDIUM → LOW
- **Progress Monitoring**: `task-manager.sh status`
- **Next Task Selection**: `task-manager.sh next`

#### Completion Phase
**Three-Gate Verification System:**
1. ✅ **Tasks Complete**: All marked DONE/COMPLETE
2. ✅ **Tests Pass**: TEST_RESULTS.md confirms success
3. ✅ **PR Merged**: Git history shows merge to main

**Automatic Archiving**: Triggers via:
- Local git hook (`.git/hooks/post-merge`)
- GitHub Actions (`.github/workflows/sprint-archive.yml`)
- Manual command (`task-manager.sh auto-archive`)

### 3. Task Management Tool

#### Core Commands
```bash
# Status and navigation
./task-manager.sh status        # Current sprint overview
./task-manager.sh next          # Highest priority task
./task-manager.sh scan          # All tasks across sprints
./task-manager.sh list          # Active task list

# Sprint management
./task-manager.sh check-complete sprint-005  # Verify ready
./task-manager.sh archive-sprint sprint-005  # Manual archive
./task-manager.sh auto-archive              # Check all sprints
```

#### Implementation Details
- **Dynamic Detection**: Reads actual markdown files
- **Pattern Matching**: Searches for `Status:` and `Priority:` fields
- **Color Coding**: Visual indicators for priority/status
- **Git Integration**: Checks merge history for PR verification

### 4. Task File Formats

#### Current Standard
```markdown
# TASK-001: Clear Description

**Status**: TODO
**Priority**: CRITICAL
**Assignee**: TBD
**Created**: 2025-01-27

## Problem
What issue does this solve?

## Solution
How will we approach it?

## Acceptance Criteria
- [ ] Specific measurable outcome
- [ ] Tests pass
- [ ] No performance regression

## Technical Approach
Implementation details...
```

#### Future Enhancement (YAML)
```yaml
---
status: TODO
priority: CRITICAL
assignee: system
depends_on: [TASK-002, TASK-003]
created: 2025-01-27
completed: null
---
# Task content...
```

### 5. Automation Features

#### Git Hooks
**Location**: `.git/hooks/post-merge`
- Detects sprint number from merge commit
- Runs completion checks
- Archives if all criteria met

#### GitHub Actions
**Location**: `.github/workflows/sprint-archive.yml`
- Triggers on PR close
- Extracts sprint from PR title
- Commits archive changes

#### CI Integration
**Location**: `.claude/scrum/ci-archive-hook.sh`
- Callable from any CI system
- Supports sprint-specific or full scan

### 6. Archive Structure

When archived, sprints move to:
```
.claude/tasks/archive/sprint-XXX/
├── ARCHIVED.md          # Auto-generated summary
├── TEST_RESULTS.md      # Test verification
├── SPRINT_PLAN.md       # Original plan
└── [task files]         # All task files
```

Archive summary includes:
- Completion date
- All three-gate verifications
- List of completed tasks
- Archive location

## Workflow Evaluation

### ✅ What's Working Well

1. **Quality Gates**: Three-step verification prevents premature completion
2. **Transparency**: Everything in git, fully trackable
3. **Automation**: Reduces manual overhead significantly
4. **Flexibility**: Can quickly pivot priorities
5. **Simplicity**: No external tools needed
6. **History**: Complete archive for reference

### ⚠️ Current Limitations

1. **Manual Updates**: Status changes require file edits
2. **No Metrics**: Missing velocity/burndown tracking
3. **Single User**: Limited collaboration features
4. **No Dependencies**: Tasks don't show relationships
5. **Basic Reporting**: No dashboards or analytics

### 🎯 Is This a Good Workflow?

**Yes, for the current context:**
- ✅ Perfect for 1-2 developer teams
- ✅ Minimal overhead, maximum transparency
- ✅ Git-native, no tool lock-in
- ✅ Enforces quality through gates
- ✅ Self-documenting process

**Scaling Considerations:**
- For 3-5 developers: Add metrics and dependency tracking
- For 5+ developers: Consider Jira/Linear integration
- For distributed teams: Add async communication features

### Recommended Next Steps

1. **Immediate** (This Sprint):
   - Add YAML frontmatter to new tasks
   - Create sprint metrics script
   - Add dependency fields

2. **Short Term** (Next Month):
   - Auto-detect status from git commits
   - Generate burndown charts
   - Add retrospective templates

3. **Long Term** (Next Quarter):
   - Integration with external tools
   - Multi-user assignment tracking
   - Automated sprint planning from roadmap

## Integration with Scrum Leader Agent

The Scrum Leader agent leverages this system by:

1. **Sprint Creation**: Uses directory structure and templates
2. **Task Management**: Reads/updates markdown files
3. **Status Reporting**: Calls `task-manager.sh` commands
4. **Planning**: Analyzes archive for velocity metrics
5. **Coordination**: Ensures git branch compliance

Example agent workflow:
```bash
# Agent checks status
status=$(./.claude/scrum/task-manager.sh status)

# Identifies blockers
blocked_tasks=$(grep "BLOCKED" .claude/tasks/sprint-*/*)

# Suggests next priority
next_task=$(./.claude/scrum/task-manager.sh next)

# Creates new sprint when needed
if all_sprints_complete; then
  create_next_sprint_from_roadmap
fi
```

## Best Practices

### For Sprint Planning
1. Keep sprints to 5 days maximum
2. Include 20% buffer for discovered work
3. Define clear "Definition of Done"
4. Break tasks into 1-4 hour chunks

### For Task Management
1. Update status immediately when complete
2. Create TEST_RESULTS.md before marking done
3. Reference task IDs in commit messages
4. Use descriptive branch names

### For Collaboration
1. Run `task-manager.sh status` daily
2. Include sprint number in PR titles
3. Document blockers clearly
4. Keep tasks atomic and independent

## Conclusion

The current scrum practice is **well-suited** for AlphaPulse's needs:
- Simple yet effective
- Automated where it matters
- Quality-focused with verification gates
- Scalable with incremental improvements

The file-based approach provides excellent transparency and git integration, while the automation reduces overhead. Continue using this system while gradually enhancing based on team growth and needs.