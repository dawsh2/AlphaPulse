#!/bin/bash

# Sprint Initialization Script - Creates structure with enforcement
# Usage: ./init_sprint.sh "SPRINT-NAME" "Sprint Description"

set -e

SPRINT_NAME=${1:-"UNNAMED-SPRINT"}
SPRINT_DESC=${2:-"Sprint objectives to be defined"}
SPRINT_DATE=$(date +%Y-%m-%d)
SPRINT_DIR=".claude/sprints/${SPRINT_DATE}-${SPRINT_NAME}"

echo "🚀 Initializing Sprint: $SPRINT_NAME"
echo "📅 Date: $SPRINT_DATE"
echo "📁 Location: $SPRINT_DIR"

# Create directory structure
mkdir -p "$SPRINT_DIR/tasks"
mkdir -p "$SPRINT_DIR/reviews"
mkdir -p "$SPRINT_DIR/branches"

# Create SPRINT_PLAN.md with enforcement rules
cat > "$SPRINT_DIR/SPRINT_PLAN.md" << EOF
# Sprint: $SPRINT_NAME
*Initialized: $SPRINT_DATE*
*Description: $SPRINT_DESC*

## 🔒 ENFORCEMENT RULES (MANDATORY)

### Git Branch Protection
1. **NO agent may commit to main branch**
2. **ALL work must be in feature branches**
3. **EVERY change requires PR review**
4. **ONLY project owner merges to main**

### Compliance Verification
\`\`\`bash
# Run this to verify no direct main commits:
git log main --format="%an: %s" --since="$SPRINT_DATE" | grep -v "Merge pull request"
\`\`\`

## 📊 Sprint Goals
- [ ] Define specific objectives
- [ ] Set success metrics
- [ ] Establish timeline

## 🎯 Task Overview
| Task ID | Branch | Agent | Status | Priority |
|---------|--------|-------|--------|----------|
| | | | Not Started | |

## 📈 Progress Tracking
- Total Tasks: 0
- Completed: 0
- In Progress: 0
- Blocked: 0

## ⚠️ Blockers & Issues
None identified yet.

## 🔄 Daily Standup Notes
### $SPRINT_DATE
- Sprint initialized
- Awaiting task definitions
EOF

# Create STATUS.md for live tracking
cat > "$SPRINT_DIR/STATUS.md" << EOF
# Sprint Status: $SPRINT_NAME
*Last Updated: $(date +"%Y-%m-%d %H:%M")*

## 🚦 Overall Status: NOT STARTED

## 📊 Metrics
- Velocity: 0/0 tasks
- PRs Open: 0
- PRs Merged: 0
- Branch Compliance: ✅ 100%

## 🔄 Active Tasks
| Task | Agent | Branch | Status | PR |
|------|-------|--------|--------|-----|
| - | - | - | - | - |

## ✅ Completed Tasks
None yet.

## ⚠️ Attention Required
None yet.
EOF

# Create DEPENDENCY_GRAPH.md
cat > "$SPRINT_DIR/DEPENDENCY_GRAPH.md" << EOF
# Task Dependencies: $SPRINT_NAME

## 🔄 Dependency Graph
\`\`\`mermaid
graph TD
    Start[Sprint Start]
    End[Sprint Complete]
    Start --> End
\`\`\`

## 📋 Dependency Matrix
| Task | Depends On | Blocks | Can Start |
|------|------------|--------|-----------|
| - | - | - | - |

## 🚀 Parallel Execution Groups
### Group 1 (No Dependencies)
- Tasks that can start immediately

### Group 2 (After Group 1)
- Tasks depending on Group 1

## 🔴 Critical Path
The longest chain of dependent tasks:
1. Not yet defined
EOF

# Create agent instruction file
cat > "$SPRINT_DIR/AGENT_INSTRUCTIONS.md" << EOF
# Agent Instructions for Sprint: $SPRINT_NAME

## ⚠️ MANDATORY COMPLIANCE

Every agent MUST read and acknowledge:
1. \`.claude/scrum/AGENT_TEMPLATE.md\` - Enforcement rules
2. Your specific task file in \`tasks/\`
3. These sprint-specific instructions

## 🔒 Branch Verification Script

**EVERY AGENT MUST RUN THIS FIRST:**
\`\`\`bash
#!/bin/bash
# verify_branch.sh - Run before starting work

CURRENT_BRANCH=\$(git branch --show-current)
echo "==================================="
echo "GIT BRANCH VERIFICATION"
echo "==================================="
echo "Current branch: \$CURRENT_BRANCH"

if [ "\$CURRENT_BRANCH" = "main" ]; then
    echo "❌ ERROR: You are on main branch!"
    echo "❌ This is FORBIDDEN!"
    echo "➡️  Create your feature branch now:"
    echo "   git checkout -b [your-assigned-branch]"
    exit 1
else
    echo "✅ Good: You are on feature branch: \$CURRENT_BRANCH"
    echo "✅ You may proceed with your task"
fi
\`\`\`

## 📋 Task Assignment Process
1. Read your task file completely
2. Run the verification script above
3. Checkout your assigned branch
4. Work ONLY in that branch
5. Create PR when complete

## 🚫 Forbidden Commands
Never run these:
- \`git push origin main\`
- \`git merge main\` (into main)
- \`gh pr merge\`

## ✅ Required Commands
Always use these:
- \`git push origin [your-branch]\`
- \`gh pr create\`
- \`git commit -m "type(scope): message"\`
EOF

# Create task template
cat > "$SPRINT_DIR/tasks/TASK_TEMPLATE.md" << EOF
# Task [TASK-ID]: [Description]
*Sprint: $SPRINT_NAME*
*Branch: \`fix/[descriptive-name]\`*
*Estimated: [1-4] hours*

## ⛔ GIT ENFORCEMENT
**NEVER WORK ON MAIN BRANCH**

Run this FIRST:
\`\`\`bash
git branch --show-current
# If it shows "main", immediately run:
git checkout -b fix/[descriptive-name]
\`\`\`

## 📋 Context
[Why this task exists]

## 🎯 Acceptance Criteria
- [ ] Specific measurable outcome
- [ ] Test coverage included
- [ ] Performance targets met

## 🔧 Implementation Details
[Technical approach]

## 🧪 Testing
\`\`\`bash
# Commands to validate solution
\`\`\`

## 📤 PR Checklist
- [ ] Working in feature branch
- [ ] All tests passing
- [ ] No commits to main
- [ ] PR created with template
EOF

# Create branch tracking file
cat > "$SPRINT_DIR/branches/TRACKING.md" << EOF
# Branch Tracking: $SPRINT_NAME

## 🌳 Active Branches
| Branch | Task | Agent | Created | PR | Status |
|--------|------|-------|---------|-----|--------|
| main | - | - | - | - | Protected |

## 📋 Branch Naming Convention
- \`fix/[description]\` - Bug fixes
- \`feat/[description]\` - New features  
- \`perf/[description]\` - Performance
- \`test/[description]\` - Test additions

## 🔒 Protection Rules
- main: Protected, requires PR review
- feature branches: No restrictions
- Automatic deletion after PR merge
EOF

# Create executable verification script
cat > "$SPRINT_DIR/verify_compliance.sh" << EOF
#!/bin/bash
# Verify sprint compliance with enforcement rules

echo "🔍 Sprint Compliance Check: $SPRINT_NAME"
echo "========================================="

# Check for direct commits to main
echo "Checking for direct main commits..."
MAIN_COMMITS=\$(git log main --format="%an: %s" --since="$SPRINT_DATE" | grep -v "Merge pull request" | wc -l)
if [ "\$MAIN_COMMITS" -eq 0 ]; then
    echo "✅ No direct commits to main"
else
    echo "❌ Found \$MAIN_COMMITS direct commits to main!"
    git log main --format="%an: %s" --since="$SPRINT_DATE" | grep -v "Merge pull request"
fi

# Check for feature branches
echo ""
echo "Checking feature branches..."
FEATURE_BRANCHES=\$(git branch -r | grep -E "origin/(fix|feat|perf|test)/" | wc -l)
echo "📊 Found \$FEATURE_BRANCHES feature branches"

# Check for open PRs
echo ""
echo "Checking pull requests..."
if command -v gh &> /dev/null; then
    PR_COUNT=\$(gh pr list --limit 100 | wc -l)
    echo "📊 Found \$PR_COUNT open PRs"
else
    echo "⚠️  GitHub CLI not installed, skipping PR check"
fi

echo ""
echo "========================================="
echo "Compliance check complete!"
EOF

chmod +x "$SPRINT_DIR/verify_compliance.sh"

# Final summary
echo ""
echo "✅ Sprint initialization complete!"
echo ""
echo "📁 Created structure:"
echo "   $SPRINT_DIR/"
echo "   ├── SPRINT_PLAN.md (goals & rules)"
echo "   ├── STATUS.md (live tracking)"
echo "   ├── DEPENDENCY_GRAPH.md (task relationships)"
echo "   ├── AGENT_INSTRUCTIONS.md (enforcement)"
echo "   ├── tasks/ (individual task files)"
echo "   ├── reviews/ (PR reviews)"
echo "   ├── branches/ (branch tracking)"
echo "   └── verify_compliance.sh (validation script)"
echo ""
echo "📋 Next steps:"
echo "1. Define sprint goals in SPRINT_PLAN.md"
echo "2. Create task files in tasks/ directory"
echo "3. Assign tasks to agents with enforcement template"
echo "4. Run verify_compliance.sh to check adherence"
echo ""
echo "🔒 Enforcement enabled: Agents cannot commit to main!"