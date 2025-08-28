# How to Use the Task Delegation System

## 🚀 Quick Start for Task Distribution

**⚠️ CRITICAL**: All terminals share the SAME git state. When any agent switches branches, ALL terminals see that branch immediately.

### Step 1: Open New Terminal for Each Agent
```bash
# Terminal 1 - Your Monitoring Terminal (STAYS ON MAIN)
cd /Users/daws/torq/backend_v2
git checkout main  # Keep one terminal on main for oversight
git status         # Monitor agent changes in real-time

# Terminal 2 - Agent A
cd /Users/daws/torq/backend_v2
# Agent will: git checkout -b fix/pool-cache-integration
cat .claude/tasks/pool-address-fix/POOL-001_cache_structure.md

# Terminal 3 - Agent B (after Agent A finishes)
cd /Users/daws/torq/backend_v2
# Agent will: git checkout -b fix/signal-precision-loss
cat .claude/tasks/pool-address-fix/PRECISION-001_signal_output.md

# NOTE: All terminals will show whichever branch is currently active!
```

### 🌍 Shared State Implications
- When Agent A runs `git checkout -b fix/branch`, ALL terminals switch to that branch
- You can monitor their progress from any terminal with `git status`, `git diff`
- After agent creates PR, you switch back: `git checkout main` (affects all terminals)

### Step 2: Agents Work Independently
Each agent will:
1. Read their task file
2. Create their feature branch
3. Implement the solution
4. Commit and push changes
5. Create a pull request

### Step 3: Monitor Progress
```bash
# Check all branches
git branch -a | grep fix/

# See branch visualization
git log --graph --pretty=oneline --abbrev-commit --all

# Check specific agent progress
git log --oneline fix/pool-cache-structure
```

### Step 4: Review Pull Requests
```bash
# List all PRs (if using GitHub CLI)
gh pr list

# Review specific PR
gh pr view [PR-NUMBER]

# Check out PR locally for testing
gh pr checkout [PR-NUMBER]
```

## 📊 Task Monitoring Commands

### Check Agent Progress
```bash
# See what each agent is working on
git log --oneline --author="Agent" --since="1 day ago"

# Check uncommitted work in branches
git checkout fix/pool-cache-structure
git status
git diff
```

### Visualize Branch Relationships
```bash
# Beautiful branch visualization
git log --graph --pretty=format:'%Cred%h%Creset -%C(yellow)%d%Creset %s %Cgreen(%cr) %C(bold blue)<%an>%Creset' --abbrev-commit --all

# Simpler view
git log --oneline --graph --all --decorate
```

## 🔄 Merging Workflow

### Phase 1: Independent Merges
```bash
# These can merge in any order
git checkout main
git pull origin main

# Merge POOL-001
gh pr merge [POOL-001-PR] --squash

# Merge POOL-002
gh pr merge [POOL-002-PR] --squash
```

### Phase 2: Dependent Merges
```bash
# After Phase 1 is complete
git pull origin main

# Now merge dependent tasks
gh pr merge [POOL-003-PR] --squash  # Depends on POOL-001
gh pr merge [POOL-004-PR] --squash  # Depends on POOL-001
```

## 🎯 Example Agent Prompts

### For Starting an Agent on a Task
```
Please read the task specification in .claude/tasks/pool-address-fix/POOL-001_cache_structure.md and implement it following all the git workflow instructions provided. Create the branch, implement the solution, and prepare the pull request as specified.
```

### For Checking Agent Status
```
What is the current status of your task implementation? Have you created the branch and what files have you modified so far?
```

### For Coordinating Between Agents
```
POOL-001 has been merged to main. Please pull the latest main and rebase your branch to include the new cache structure for your implementation.
```

## 📈 Success Metrics

Track the effectiveness of distributed development:

```bash
# Time from task assignment to PR
git log --format='%ai' -1 fix/pool-cache-structure  # Branch creation
gh pr view [PR-NUMBER] --json createdAt  # PR creation

# Code quality metrics
cargo clippy --all-targets -- -D warnings
cargo test --workspace
cargo fmt --all -- --check

# Merge success rate
gh pr list --state merged --limit 10
```

## 🚨 Common Issues & Solutions

### Merge Conflicts
```bash
# Agent should:
git checkout main
git pull origin main
git checkout fix/[their-branch]
git rebase main
# Resolve conflicts
git add .
git rebase --continue
git push --force-with-lease
```

### Dependency Not Ready
If POOL-003 needs POOL-001 but it's not merged:
```bash
# Agent can cherry-pick or wait
git checkout fix/discovery-queue
git cherry-pick [POOL-001-commits]
# Or simply implement interface and update later
```

### Agent Needs Clarification
Agents should add questions to their PR:
```markdown
## ❓ Questions for Review
1. Should the cache size be configurable via environment variable?
2. Is 10ms timeout acceptable for RPC calls?
3. Should we log at INFO or DEBUG level for discoveries?
```

## 🎊 Completion Checklist

When all tasks are done:
- [ ] All 6 PRs merged to main
- [ ] Integration tests passing
- [ ] No performance regressions
- [ ] Documentation updated
- [ ] Old placeholder code removed
- [ ] Celebration! 🎉

## 💡 Pro Tips

1. **Parallel Development**: Start all independent agents simultaneously
2. **Clear Communication**: Use PR descriptions for async communication
3. **Atomic Commits**: Each commit should be meaningful and complete
4. **Test Locally**: Agents should test before pushing
5. **Document Decisions**: Add notes about trade-offs in code comments

---

## Task Assignment Script

```bash
#!/bin/bash
# assign_tasks.sh - Open terminals with tasks

# Terminal 1
osascript -e 'tell app "Terminal" to do script "cd /Users/daws/torq/backend_v2 && echo \"Task: POOL-001 Cache Structure\" && cat .claude/tasks/pool-address-fix/POOL-001_cache_structure.md"'

# Terminal 2
osascript -e 'tell app "Terminal" to do script "cd /Users/daws/torq/backend_v2 && echo \"Task: POOL-002 Event Extraction\" && cat .claude/tasks/pool-address-fix/POOL-002_event_extraction.md"'

# Terminal 3
osascript -e 'tell app "Terminal" to do script "cd /Users/daws/torq/backend_v2 && echo \"Task: POOL-003 Discovery Queue\" && cat .claude/tasks/pool-address-fix/POOL-003_discovery_queue.md"'
```

Make it executable: `chmod +x assign_tasks.sh`

---

*This distributed approach teaches proper git workflow while parallelizing development!*
