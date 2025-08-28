# Torq Scrum Framework
*Version 1.0 - Reusable Development Methodology*

## 🎯 Framework Overview

This is our standard development framework where:
1. **Scrum Leader** (me) breaks down work into atomic tasks
2. **Specialist Agents** work in isolated git branches
3. **You** review and merge PRs
4. **No agent ever touches main directly**

## 📁 Standard Directory Structure

```
.claude/
├── scrum/
│   ├── FRAMEWORK.md           # This file - the methodology
│   ├── AGENT_TEMPLATE.md       # Reusable agent instructions
│   └── SPRINT_TEMPLATE.md      # Sprint initialization template
├── sprints/
│   ├── 2025-01-CRITICAL/       # Current sprint
│   │   ├── SPRINT_PLAN.md      # Sprint overview
│   │   ├── tasks/              # Individual task files
│   │   └── STATUS.md           # Live progress tracking
│   └── archive/                # Completed sprints
└── roadmap.md                  # Product roadmap

```

## 🔒 Enforcement Mechanisms

**🌍 CRITICAL**: Git state is shared across ALL terminals. When any agent switches branches, every terminal in the repository shows that branch immediately. This makes enforcement even more critical.

### 1. Agent Instruction Template (ENFORCED)

Every agent receives this MANDATORY preamble with shared state warnings:

```markdown
# AGENT INSTRUCTIONS - MANDATORY COMPLIANCE

You are a specialist agent working on task [TASK-ID].

## GIT WORKFLOW REQUIREMENTS (MANDATORY)
1. You MUST work in your assigned branch: [BRANCH-NAME]
2. You MUST NOT merge to main
3. You MUST NOT modify other branches
4. You MUST create a PR for review
5. You MUST follow the task specification exactly

## VERIFICATION COMMANDS (RUN FIRST)
```bash
# MANDATORY: Verify you're NOT on main
git branch --show-current
# If output is "main", STOP and checkout your branch

# MANDATORY: Create your feature branch
git checkout main
git pull origin main
git checkout -b [BRANCH-NAME]

# MANDATORY: Confirm correct branch
git branch --show-current
# Must show: [BRANCH-NAME]
```

## YOUR TASK
[Task specification here]

## COMPLETION REQUIREMENTS
- [ ] All changes in feature branch
- [ ] Tests passing
- [ ] PR created
- [ ] NO direct commits to main

REMINDER: Any commits to main will be rejected. Work ONLY in [BRANCH-NAME].
```

### 2. Task File Structure (ENFORCED)

Every task file MUST include:

```markdown
# Task [TASK-ID]: [Description]
*Branch: `[EXACT-BRANCH-NAME]`* ← ENFORCED
*NEVER MERGE TO MAIN* ← REMINDER

## Git Safety Check (RUN FIRST)
```bash
# COPY AND RUN THESE COMMANDS:
git branch --show-current
# MUST NOT show "main"
# If it shows "main", run:
git checkout -b [EXACT-BRANCH-NAME]
```
[Rest of task...]
```

### 3. Sprint Initialization Script

Create a script that sets up sprints with proper structure:

```bash
#!/bin/bash
# init_sprint.sh - Initialize new sprint with enforcement

SPRINT_NAME=$1
SPRINT_DIR=".claude/sprints/$SPRINT_NAME"

# Create sprint structure
mkdir -p "$SPRINT_DIR/tasks"
mkdir -p "$SPRINT_DIR/reviews"

# Create sprint plan with enforcement rules
cat > "$SPRINT_DIR/SPRINT_PLAN.md" << 'EOF'
# Sprint: $SPRINT_NAME

## ⚠️ CRITICAL RULES
1. NO agent may commit to main
2. ALL work requires PR review
3. Branches MUST match task assignments
4. Reviews are MANDATORY before merge

## Enforcement Checklist
- [ ] All agents received AGENT_TEMPLATE instructions
- [ ] All tasks include branch verification
- [ ] PR templates configured
- [ ] Review gates enabled
EOF

echo "Sprint $SPRINT_NAME initialized with enforcement rules"
```

## 🎭 Role Definitions

### Scrum Leader (My Responsibilities)
1. **Task Decomposition**: Break epics into <4 hour tasks
2. **Dependency Mapping**: Identify task relationships
3. **Branch Assignment**: Assign unique branch per task
4. **Progress Tracking**: Monitor PR status
5. **Conflict Resolution**: Coordinate when branches conflict
6. **Quality Gates**: Define acceptance criteria

### Agent Responsibilities (ENFORCED)
1. **Branch Isolation**: Work ONLY in assigned branch
2. **Task Focus**: Complete ONLY assigned task
3. **PR Creation**: MUST create PR for review
4. **No Main Access**: NEVER merge to main
5. **Test Coverage**: Include tests in PR

### Your Responsibilities (Review & Merge)
1. **PR Review**: Examine code quality
2. **Test Validation**: Ensure tests pass
3. **Merge Decision**: Approve or request changes
4. **Main Protection**: You're the ONLY one who merges to main

## 📋 Task Breakdown Template

When I create tasks, each follows this structure:

```markdown
# Task Component Checklist
- [ ] Atomic scope (<4 hours work)
- [ ] Clear acceptance criteria
- [ ] Unique branch name
- [ ] No dependencies OR clearly stated
- [ ] Test requirements defined
- [ ] Performance targets specified

# Task Metadata
- ID: [COMPONENT-###]
- Branch: fix/[descriptive-name]
- Agent Type: [Specialist Role]
- Dependencies: [None | List]
- Priority: [🔴 Critical | 🟡 Major | 🟢 Minor]
```

## 🚀 Sprint Workflow

### 1. Sprint Planning (Scrum Leader)
```bash
# I create sprint structure
.claude/sprints/2025-01-FEATURE/
├── SPRINT_PLAN.md
├── tasks/
│   ├── TASK-001_description.md
│   ├── TASK-002_description.md
│   └── TASK-003_description.md
└── STATUS.md
```

### 2. Agent Assignment (You)
```bash
# Terminal 1
"Read .claude/sprints/2025-01-FEATURE/tasks/TASK-001_description.md
Follow AGENT_TEMPLATE.md enforcement rules"

# Terminal 2
"Read .claude/sprints/2025-01-FEATURE/tasks/TASK-002_description.md
Follow AGENT_TEMPLATE.md enforcement rules"
```

### 3. Progress Monitoring (Scrum Leader)
```bash
# I track in STATUS.md
| Task | Branch | Agent | PR | Status |
|------|--------|-------|----|---------|
| 001  | fix/cache | A1 | #101 | In Progress |
| 002  | fix/event | A2 | #102 | Review Ready |
| 003  | fix/perf  | A3 | - | Not Started |
```

### 4. Review & Merge (You)
```bash
gh pr list # See all PRs
gh pr review 101 --approve
gh pr merge 101 --squash
```

## 🔐 Enforcement Verification

### Pre-Sprint Checklist
- [ ] Framework document provided to all agents
- [ ] Branch protection enabled on main
- [ ] PR requirements configured
- [ ] Agent templates include git guards

### Per-Task Verification
```bash
# Verify agent is on correct branch
git rev-parse --abbrev-ref HEAD

# Verify no direct main commits
git log main..HEAD --author="Agent"

# Verify PR exists
gh pr list --author="Agent"
```

### Post-Sprint Validation
```bash
# All merges via PR
git log --merges --format="%s" | grep "^Merge pull request"

# No direct commits
git log --no-merges --format="%an" main | grep -v "Your Name"
```

## 📊 Success Metrics

Track framework effectiveness:

1. **Branch Compliance**: 100% of work in feature branches
2. **PR Coverage**: Every merge has associated PR
3. **Review Rate**: All PRs reviewed before merge
4. **Task Velocity**: Tasks completed per sprint
5. **Rework Rate**: Changes requested in review

## 🎯 Benefits of This Framework

1. **Quality Control**: Every line reviewed
2. **Parallel Development**: N agents, no conflicts
3. **Clear Ownership**: One agent, one branch, one task
4. **Audit Trail**: Complete history in PRs
5. **Rollback Safety**: Easy to revert PRs
6. **Learning**: Agents learn proper collaboration

## 🚨 Common Violations & Fixes

### Violation: Agent commits to main
**Fix**: Revert commit, move to branch
```bash
git revert [commit]
git cherry-pick [commit] # on feature branch
```

### Violation: Agent modifies wrong branch
**Fix**: Reset branch, create correct one
```bash
git reset --hard origin/[correct-branch]
git checkout -b [assigned-branch]
```

### Violation: No PR created
**Fix**: Create PR from existing branch
```bash
gh pr create --base main --head [branch]
```

## 📝 Reusable Components

### Sprint Initialization
```bash
./init_sprint.sh "2025-01-SPRINT-NAME"
```

### Task Creation
```bash
./create_task.sh "TASK-001" "fix/branch-name" "Task description"
```

### Agent Launch
```bash
./launch_agent.sh "TASK-001" "Terminal Title"
```

---

## 🎉 Framework Adoption

To adopt this framework for ALL development:

1. **Every feature** starts with sprint planning
2. **Every task** gets a unique branch
3. **Every agent** receives enforcement template
4. **Every change** goes through PR review
5. **Only you** merge to main

This is now our standard development methodology!
