# 📋 Scrum & Task Management Documentation

This directory contains all scrum framework and task management documentation maintained by the Scrum Leader agent.

## 🎯 Quick Start for New Agents

1. **Read First**: `FRAMEWORK.md` - Complete scrum methodology
2. **Current Tasks**: `TASK_MANAGEMENT.md` - What needs to be done NOW
3. **Use Tool**: `./task-manager.sh status` - Check sprint status

## 📁 Core Documents

### Task Management
- **`TASK_MANAGEMENT.md`** - Central dashboard of all active tasks, priorities, and status
- **`task-manager.sh`** - CLI tool for task workflow (`./task-manager.sh help`)
- **`SPRINT_RETROSPECTIVE.md`** - Analysis of what actually got done vs planned

### Development Workflow
- **`FRAMEWORK.md`** - Complete scrum framework and methodology
- **`ATOMIC_DEVELOPMENT_GUIDE.md`** - Single-focus development pattern (<100 lines per PR)
- **`TASK_TEMPLATE_TDD.md`** - Test-driven development template for all tasks
- **`PR_REVIEW_PROCESS.md`** - Pull request requirements and review checklist

### Git Workflow
- **`GIT_BEHAVIOR_GUIDE.md`** - Understanding shared git state behavior
- **`GIT_WORKTREE_SOLUTION.md`** - Solution for parallel development with git worktrees
- **`INITIAL_MERGE_STRATEGY.md`** - How we handled the foundation merge

### Agent Coordination
- **`AGENT_TEMPLATE.md`** - Template for agent task assignments
- **`SCRUM_LEADER_WORKFLOW.md`** - How the scrum leader coordinates work

### Validation Scripts
- **`validate_tdd_workflow.sh`** - Validates TDD cycle was followed
- **`test_validation_template.sh`** - Template for test validation
- **`init_sprint.sh`** - Initialize new sprint (if present)

## 🚀 How to Use This Framework

### For Task Execution
```bash
# 1. Check current status
./task-manager.sh status

# 2. See next priority
./task-manager.sh next

# 3. Start working on task
./task-manager.sh start PRECISION-001

# 4. Follow TDD workflow from TASK_TEMPLATE_TDD.md
# 5. Create PR following PR_REVIEW_PROCESS.md
# 6. Mark complete when merged
./task-manager.sh complete PRECISION-001
```

### For Parallel Development (Avoiding Git Conflicts)
```bash
# Use git worktrees as documented in GIT_WORKTREE_SOLUTION.md
git worktree add ../alphapulse-precision -b precision-001-fix
cd ../alphapulse-precision
# Work in isolation
```

## 📊 Current Sprint Status

**Objective**: Complete arbitrage strategy production deployment

### Critical Blockers (Must Complete)
- **PRECISION-001**: Fix signal precision loss (f64 → UsdFixedPoint8) - NOT STARTED
- **POOL-001**: Production validation of pool cache - FOUNDATION ONLY
- **EXECUTION-001**: Complete arbitrage execution - NOT STARTED
- **RISK-001**: Position sizing and risk management - NOT STARTED
- **MONITORING-001**: Production monitoring - NOT STARTED

See `TASK_MANAGEMENT.md` for full details and dependencies.

## 🔄 Process Flow

1. **Sprint Planning** → Define tasks in `TASK_MANAGEMENT.md`
2. **Task Assignment** → Use `task-manager.sh` to coordinate
3. **Development** → Follow `ATOMIC_DEVELOPMENT_GUIDE.md`
4. **Testing** → Use `TASK_TEMPLATE_TDD.md` for TDD
5. **Review** → Follow `PR_REVIEW_PROCESS.md`
6. **Retrospective** → Update `SPRINT_RETROSPECTIVE.md`

## 📈 Key Metrics

- **Definition of Done**: Code works, tests pass, PR merged, no warnings
- **Atomic Commits**: Single focus, <100 lines per PR
- **TDD Required**: Test first, implement second, refactor third
- **No Mocks**: Real market data and connections only

## 🔗 Related Documentation

- **Main Roadmap**: `.claude/roadmap.md` - Strategic objectives
- **Architecture**: `network/transport/MONOLITH_ARCHITECTURE.md` - Mycelium design
- **Task Details**: `.claude/tasks/pool-address-fix/` - Detailed breakdowns

## ⚠️ Important Notes

1. **Git State is Shared**: All terminals share same git checkout - use worktrees!
2. **Conservative Completion**: Don't mark tasks done until fully working
3. **Production Focus**: Everything should be production-ready from the start
4. **Breaking Changes OK**: This is greenfield - refactor freely

---

*This directory is maintained by the Scrum Leader agent. For questions about specific tasks, check `TASK_MANAGEMENT.md`. For process questions, see `FRAMEWORK.md`.*
