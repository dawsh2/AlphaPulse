# 🔒 MANDATORY AGENT INSTRUCTIONS - ENFORCEMENT TEMPLATE

## ⛔ CRITICAL: BRANCH ENFORCEMENT

**YOU ARE STRICTLY FORBIDDEN FROM WORKING ON THE MAIN BRANCH**

### MANDATORY FIRST COMMANDS (COPY AND RUN):
```bash
# CHECK 1: Verify you're NOT on main
git branch --show-current

# If the output is "main", you MUST run:
git checkout -b [YOUR-ASSIGNED-BRANCH]

# CHECK 2: Confirm you're on correct branch
git branch --show-current
# Output MUST show: [YOUR-ASSIGNED-BRANCH]
# If not, STOP and fix before proceeding
```

## 🚫 FORBIDDEN ACTIONS

You MUST NOT:
- ❌ Run `git checkout main` (except to create feature branch)
- ❌ Run `git merge` into main
- ❌ Run `git push origin main`
- ❌ Modify any branch other than your assigned branch
- ❌ Create additional branches beyond your assigned one
- ❌ Close or merge Pull Requests

## ✅ REQUIRED ACTIONS

You MUST:
- ✅ Work ONLY in branch: `[YOUR-ASSIGNED-BRANCH]`
- ✅ Commit ONLY to your feature branch
- ✅ Push ONLY your feature branch
- ✅ Create a Pull Request for review
- ✅ Include test results in your PR description

## 📋 VERIFICATION CHECKLIST

Before starting work:
```bash
# Run this verification script:
echo "=== GIT SAFETY CHECK ==="
CURRENT_BRANCH=$(git branch --show-current)
if [ "$CURRENT_BRANCH" = "main" ]; then
    echo "❌ ERROR: You are on main branch!"
    echo "Run: git checkout -b [YOUR-ASSIGNED-BRANCH]"
    exit 1
else
    echo "✅ Safe: You are on branch: $CURRENT_BRANCH"
fi
```

## 🎯 YOUR TASK ASSIGNMENT

**Task ID**: [TASK-ID]
**Branch Name**: `[EXACT-BRANCH-NAME]`
**Task File**: `.claude/sprints/[SPRINT]/tasks/[TASK-FILE]`

### Task Execution Steps:
1. Read your complete task file
2. Verify you're on the correct branch (commands above)
3. Implement ONLY what's specified in the task
4. Commit to your branch with clear messages
5. Push your branch: `git push -u origin [YOUR-BRANCH]`
6. Report: "PR ready for review on branch [YOUR-BRANCH]"

## 🔄 COMMIT MESSAGE FORMAT

Use this format for ALL commits:
```
[type]([scope]): [description]

- [Detail 1]
- [Detail 2]

Task: [TASK-ID]
```

Types: feat, fix, test, docs, refactor, perf

## 📤 PULL REQUEST TEMPLATE

When creating your PR, use:
```markdown
## Task: [TASK-ID]
## Branch: [YOUR-BRANCH]

### Summary
[What you implemented]

### Changes
- [File 1]: [What changed]
- [File 2]: [What changed]

### Testing
```bash
[Test commands you ran]
[Test results]
```

### Checklist
- [ ] Working in correct branch
- [ ] All tests passing
- [ ] No commits to main
- [ ] Ready for review
```

## ⚠️ SAFETY REMINDERS

1. **NEVER** type `git push origin main`
2. **ALWAYS** verify branch before commits
3. **IF UNSURE** ask: "Which branch should I be on?"
4. **NO EXCEPTIONS** to these rules

## 🚨 ERROR RECOVERY

If you accidentally commit to main:
```bash
# STOP IMMEDIATELY and report:
"ERROR: I may have committed to main. 
Current branch: $(git branch --show-current)
Last commit: $(git log -1 --oneline)"

# Wait for instructions to fix
```

## 📊 COMPLIANCE TRACKING

Your compliance will be verified:
- Branch name matches assignment: ✓/✗
- Zero commits to main: ✓/✗
- PR created from correct branch: ✓/✗
- All work in assigned branch: ✓/✗

---

**FINAL REMINDER**: You are working on branch `[YOUR-BRANCH]`, NOT main. 
Any commits to main will be rejected and must be redone.

**ACKNOWLEDGE**: Type "I confirm I will work only in branch [YOUR-BRANCH]" before starting.