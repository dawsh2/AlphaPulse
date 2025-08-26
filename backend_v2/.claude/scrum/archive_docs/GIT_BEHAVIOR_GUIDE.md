# Git Behavior in Multi-Terminal Development

## 🌍 Critical Understanding: Shared Git State

**IMPORTANT DISCOVERY**: Git working directory state is shared across ALL terminal tabs in the same repository.

## 📊 What This Means

### Shared Across All Terminals
- ✅ **Current branch** - All terminals see the same active branch
- ✅ **File changes** - Modified files visible in all terminals
- ✅ **Git status** - Same status in every terminal
- ✅ **Working directory** - Shared staging area and commit history
- ✅ **Branch switches** - `git checkout` affects all terminals immediately

### NOT Shared (Terminal-Specific)
- ❌ **Command history** - Each terminal has separate bash history
- ❌ **Environment variables** - Each terminal's env is independent
- ❌ **Current directory** - Each terminal can be in different folders
- ❌ **Running processes** - Background jobs stay in their terminal

## 🎯 Implications for Our Framework

### Advantages
1. **Real-Time Monitoring** - You can observe agent work from any terminal
2. **Shared Context** - No confusion about current state
3. **Easy Coordination** - Simple branch management
4. **Immediate Feedback** - Catch issues as they happen

### Challenges
1. **No Isolation** - Agent branch changes affect everyone
2. **Coordination Required** - Must manage branch switching carefully
3. **Enhanced Safety Needed** - Mistakes have immediate global impact

## 🚀 Optimal Workflow Pattern

### Agent Assignment Strategy
```bash
# Pattern: Sequential Agent Work (Safest)

# Agent A starts
git checkout main
git pull origin main
# Assign: Terminal 2 to Agent A
# Agent A: git checkout -b fix/task-a

# After Agent A completes PR
gh pr merge [PR-NUMBER] --squash
git checkout main

# Agent B starts
# Assign: Terminal 3 to Agent B
# Agent B: git checkout -b fix/task-b
```

### Monitoring Setup
```bash
# Terminal 1: Your monitoring terminal
watch -n 5 'echo "=== GIT STATUS ===" && git status && echo && echo "=== RECENT COMMITS ===" && git log --oneline -5'

# This gives you live updates on any agent's progress
```

### Emergency Recovery
```bash
# If agent makes mistake on main
git stash                    # Save any good changes
git reset --hard origin/main # Nuclear reset
git clean -fd               # Remove untracked files

# Or more surgical approach
git revert [BAD-COMMIT]     # Undo specific commit
git push origin main        # Push fix
```

## 🔒 Enhanced Safety Protocols

### For Agents
1. **ALWAYS verify branch** before any git command
2. **NEVER switch branches** without permission
3. **CREATE PR immediately** after completing work
4. **REPORT completion** and wait for further instructions

### For Project Lead (You)
1. **Monitor continuously** during agent work
2. **Switch back to main** immediately after each PR merge
3. **Verify clean state** before assigning next agent
4. **Keep one terminal** on main for oversight

## 📋 Git State Checklist

### Before Each Agent Session
- [ ] Current branch: `main`
- [ ] Working tree clean: `git status`
- [ ] Up to date: `git pull origin main`
- [ ] No uncommitted changes: `git diff --cached`

### During Agent Work
- [ ] Monitor progress: `git status` in other terminal
- [ ] Check their commits: `git log --oneline [agent-branch]`
- [ ] Verify they stay in their branch: `git branch --show-current`

### After Agent Completes
- [ ] Review PR: `gh pr view [PR-NUMBER]`
- [ ] Test locally: `gh pr checkout [PR-NUMBER]`
- [ ] Merge if approved: `gh pr merge [PR-NUMBER] --squash`
- [ ] Return to main: `git checkout main`
- [ ] Verify clean state: `git status`

## 🎉 Benefits of Understanding This

1. **Better Monitoring** - You see everything in real-time
2. **Faster Feedback** - Catch issues immediately
3. **Simpler Coordination** - One branch state to manage
4. **Enhanced Control** - You control all branch switching
5. **Immediate Context** - Always know current state

## ⚠️ Red Flags to Watch For

- Agent switches to `main` branch unexpectedly
- Commits appear on `main` without PR
- Working tree becomes dirty while agent works
- Branch switches happen without your knowledge
- Multiple agents trying to work simultaneously

## 💡 Best Practices

### Agent Instructions Should Include:
```bash
# MANDATORY first commands for every agent:
echo "Current branch (affects ALL terminals): $(git branch --show-current)"
git checkout -b [YOUR-ASSIGNED-BRANCH]
echo "✅ Switched all terminals to: $(git branch --show-current)"
```

### Your Monitoring Commands:
```bash
# Live monitoring
git status
git log --oneline --all --graph -10
git diff --name-only

# Before/after comparisons
git diff main..[agent-branch]
git log main..[agent-branch] --oneline
```

This shared state is actually a **feature**, not a bug - it enables better oversight and coordination when properly managed!
