# Git Worktree Standard - AlphaPulse Development

## 🚫 **NEVER Use `git worktree add -b`**
Git branches change state for **ALL sessions**, breaking parallel development and AI agent workflows.

## ✅ **ALWAYS Use `git worktree`**
Worktrees create isolated development environments for each task.

### Standard Worktree Workflow

```bash
# 1. Create isolated worktree for task
git worktree add -b task-branch-name ../task-worktree-dir
cd ../task-worktree-dir

# 2. Verify isolation
git branch --show-current  # Should show: task-branch-name
pwd                         # Should show: ../task-worktree-dir

# 3. Work on task (completely isolated)
# ... make changes ...
git add -A
git commit -m "implement: task changes"

# 4. Push and create PR
git push origin task-branch-name
gh pr create --title "Task: Description" --body "Implementation details"

# 5. Clean up after PR merge
cd ../backend_v2
git worktree remove ../task-worktree-dir
git branch -D task-branch-name  # Optional: delete local branch
```

## Why Worktrees?

### ❌ Problems with `git worktree add -b`:
- Changes branch for **ALL** terminal sessions
- Breaks parallel development
- Causes conflicts with multiple AI agents
- Risk of accidental commits to wrong branch
- Context switching requires stashing/committing

### ✅ Benefits of `git worktree`:
- **Isolated Development**: Each task has its own directory
- **Parallel Work**: Multiple tasks can be developed simultaneously
- **Session Safety**: No global state changes
- **Clean Workspace**: No need to stash/commit when switching tasks
- **AI Agent Compatible**: Each agent can work in isolation
- **Safer**: Impossible to commit to wrong branch accidentally

## Template Updates

Both task templates now enforce worktree usage:
- `TASK_TEMPLATE.md`: Updated with worktree workflow
- `TASK_TEMPLATE_TESTING.md`: Added worktree setup section

## Migration Strategy

### For Existing Tasks:
- ✅ Sprint 013-014: Already using worktrees
- ⚠️  Sprint 010-012: Mixed usage - migrate as needed
- 🔄 Earlier sprints: Update to worktrees when working on them

### For New Tasks:
- **REQUIRED**: All new tasks must use worktree workflow
- Templates automatically include worktree setup
- Task validation should check for worktree usage

## Directory Structure

```
/Users/daws/alphapulse/
├── backend_v2/           # Main repository (NEVER work here directly)
├── task-001-worktree/    # Isolated worktree for TASK-001
├── task-002-worktree/    # Isolated worktree for TASK-002
├── sink-001-worktree/    # Isolated worktree for SINK-001
└── ...                   # One worktree per active task
```

## Best Practices

1. **Naming Convention**: `task-id-worktree` (e.g., `sink-001-worktree`)
2. **One Task Per Worktree**: Don't reuse worktrees for multiple tasks
3. **Clean Up**: Remove worktrees after PR merge
4. **Verification**: Always check `pwd` and `git branch --show-current`
5. **Never Main**: Never work directly in the main repository directory

## Emergency: Remove All Worktrees

```bash
# List all worktrees
git worktree list

# Remove specific worktree
git worktree remove ../task-worktree-name

# Remove all worktrees (nuclear option)
git worktree list --porcelain | grep worktree | cut -d' ' -f2 | xargs -I {} git worktree remove {}
```

This standard ensures safe, parallel development for both human developers and AI agents.
