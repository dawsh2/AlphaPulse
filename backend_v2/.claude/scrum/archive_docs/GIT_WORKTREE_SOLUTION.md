# Git Worktree Solution for Shared Terminal State

## Problem Identified
When using `git checkout` in one terminal, it affects ALL terminals in the same session because they share the same git working directory. This broke our atomic development workflow where agents were supposed to work on separate branches.

## Root Cause
- All terminal tabs/windows point to the same `.git` directory
- `git checkout` changes the working tree for the entire directory
- Shared state prevents parallel development on different branches

## Solution: Git Worktrees

### What Are Git Worktrees?
Git worktrees allow multiple working directories for a single repository, each checked out to different branches.

### Setup Commands

```bash
# Create a worktree for a specific task
git worktree add ../alphapulse-precision-001 -b precision-001-implementation

# Create worktree from existing branch
git worktree add ../alphapulse-execution-001 execution-001-implementation

# List all worktrees
git worktree list

# Remove completed worktree
git worktree remove ../alphapulse-precision-001
```

### Recommended Workflow

#### For Atomic Development
```bash
# Main development stays in primary repo
cd /Users/daws/alphapulse/backend_v2  # Main branch for coordination

# Create isolated worktree for each task
git worktree add ../alphapulse-task-work -b precision-001-fix

# Work in isolation
cd ../alphapulse-task-work
# ... do TDD development ...
git commit -m "feat: implement fixed-point signal conversion"

# Push and create PR
git push -u origin precision-001-fix
gh pr create --title "PRECISION-001: Fix signal precision loss"

# After merge, clean up
cd ../alphapulse/backend_v2
git worktree remove ../alphapulse-task-work
git branch -d precision-001-fix  # if merged
```

#### For Parallel Agent Development
```bash
# Agent 1: Works on precision fix
git worktree add ../alphapulse-precision ../task-precision-001
cd ../alphapulse-precision

# Agent 2: Works on execution logic
git worktree add ../alphapulse-execution ../task-execution-001
cd ../alphapulse-execution

# Agent 3: Reviews and coordinates from main
cd /Users/daws/alphapulse/backend_v2  # stays on main
```

### Directory Structure
```
/Users/daws/
├── alphapulse/backend_v2/          # Main repo (coordination, main branch)
├── alphapulse-precision-001/       # Worktree for PRECISION-001 task
├── alphapulse-execution-001/       # Worktree for EXECUTION-001 task
└── alphapulse-testing/             # Worktree for testing tasks
```

### Benefits
1. **True Isolation** - Each worktree has independent working directory
2. **Shared Git History** - All worktrees share same .git database
3. **No Conflicts** - Different terminals can work on different branches
4. **Easy Cleanup** - Remove worktree when task complete
5. **Atomic Commits** - Each task develops in isolation until PR ready

### Integration with Task Manager

Update the task manager to use worktrees:

```bash
# Modified start_task function
start_task() {
    task_id="$1"
    worktree_dir="../alphapulse-${task_id,,}"
    branch_name="${task_id,,}-implementation"

    echo "Creating isolated worktree for $task_id"
    git worktree add "$worktree_dir" -b "$branch_name"

    echo "Switch to: cd $worktree_dir"
    echo "Work in isolation, then push and create PR"
}
```

### Cleanup Commands
```bash
# List all worktrees
git worktree list

# Remove specific worktree (after task completion)
git worktree remove ../alphapulse-precision-001

# Prune stale worktree references
git worktree prune

# Force remove if needed
git worktree remove --force ../alphapulse-precision-001
```

### Best Practices
1. **One Task Per Worktree** - Keep worktrees focused on single tasks
2. **Clean Up After Merge** - Remove worktrees when PRs are merged
3. **Consistent Naming** - Use task ID in worktree directory names
4. **Main for Coordination** - Keep main repo for coordination and reviews
5. **Push Early** - Push to remote branches to avoid data loss

### Recovery from Current State
```bash
# Current state: all terminals affected by shared checkout
# Solution: Create worktrees for active development

# For immediate PRECISION-001 work:
git worktree add ../alphapulse-precision-work -b precision-001-actual-implementation
cd ../alphapulse-precision-work

# Now this terminal is isolated and can work independently
# Other terminals can stay on main or create their own worktrees
```

This solves the shared git state problem and enables true atomic development!
