#!/bin/bash
# Create new sprint with standardized templates

set -e

# Get sprint details
SPRINT_NUMBER=${1:-"XXX"}
SPRINT_NAME=${2:-"unnamed"}
SPRINT_DESC=${3:-"Sprint description"}

# Pad sprint number with zeros
if [[ "$SPRINT_NUMBER" =~ ^[0-9]+$ ]]; then
    SPRINT_NUMBER_PADDED=$(printf "%03d" "$SPRINT_NUMBER")
else
    echo "Error: Sprint number must be numeric (got: $SPRINT_NUMBER)"
    exit 1
fi
# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

SPRINT_DIR=".claude/tasks/sprint-${SPRINT_NUMBER_PADDED}-${SPRINT_NAME}"

echo -e "${BLUE}🚀 Creating Sprint ${SPRINT_NUMBER_PADDED}: ${SPRINT_NAME}${NC}"
echo "=================================="

# Check if sprint already exists
if [ -d "$SPRINT_DIR" ]; then
    echo -e "${YELLOW}⚠️  Sprint directory already exists: $SPRINT_DIR${NC}"
    read -p "Overwrite? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
    rm -rf "$SPRINT_DIR"
fi

# Create directory structure
echo "Creating directory structure..."
mkdir -p "$SPRINT_DIR"

# Copy templates
echo "Copying templates..."
cp .claude/scrum/templates/SPRINT_PLAN.md "$SPRINT_DIR/"
cp .claude/scrum/templates/TASK_TEMPLATE.md "$SPRINT_DIR/TASK-001_rename_me.md"

# Update sprint plan with actual values
echo "Customizing sprint plan..."
START_DATE=$(date +%Y-%m-%d)
# Cross-platform date calculation (macOS first, then Linux)
if command -v gdate >/dev/null 2>&1; then
    # GNU date available (e.g., from coreutils on macOS)
    END_DATE=$(gdate -d "+5 days" +%Y-%m-%d)
elif date -v+5d +%Y-%m-%d >/dev/null 2>&1; then
    # macOS/BSD date
    END_DATE=$(date -v+5d +%Y-%m-%d)
else
    # Linux/GNU date
    END_DATE=$(date -d "+5 days" +%Y-%m-%d)
fi

# Update SPRINT_PLAN.md
sed -i.bak "s/Sprint XXX:/Sprint ${SPRINT_NUMBER_PADDED}:/" "$SPRINT_DIR/SPRINT_PLAN.md"
sed -i.bak "s/\[Sprint Name\]/${SPRINT_NAME}/" "$SPRINT_DIR/SPRINT_PLAN.md"
sed -i.bak "s/Start Date: YYYY-MM-DD/Start Date: ${START_DATE}/" "$SPRINT_DIR/SPRINT_PLAN.md"
sed -i.bak "s/End Date: YYYY-MM-DD/End Date: ${END_DATE}/" "$SPRINT_DIR/SPRINT_PLAN.md"
rm "$SPRINT_DIR/SPRINT_PLAN.md.bak"

# Update task template
sed -i.bak "s/YYYY-MM-DD/${START_DATE}/" "$SPRINT_DIR/TASK-001_rename_me.md"
rm "$SPRINT_DIR/TASK-001_rename_me.md.bak"

# Create README
cat > "$SPRINT_DIR/README.md" << EOF
# Sprint ${SPRINT_NUMBER_PADDED}: ${SPRINT_NAME}

${SPRINT_DESC}

## Quick Start

1. **Review sprint plan**: 
   \`\`\`bash
   cat SPRINT_PLAN.md
   \`\`\`

2. **Create tasks from template**:
   \`\`\`bash
   cp TASK-001_rename_me.md TASK-001_actual_task_name.md
   vim TASK-001_actual_task_name.md
   \`\`\`

3. **Start work**:
   \`\`\`bash
   # Never work on main!
   git checkout -b feat/sprint-${SPRINT_NUMBER_PADDED}-task-001
   \`\`\`

4. **Check status**:
   \`\`\`bash
   ../../scrum/task-manager.sh status
   \`\`\`

## Important Rules

- **NEVER commit to main branch**
- **Always update task status** (TODO → IN_PROGRESS → COMPLETE)
- **Create TEST_RESULTS.md** when tests pass
- **Use PR for all merges**

## Directory Structure
\`\`\`
.
├── README.md           # This file
├── SPRINT_PLAN.md     # Sprint goals and timeline
├── TASK-001_*.md      # Individual task files
├── TASK-002_*.md
├── TEST_RESULTS.md    # Created when tests complete
└── [archived]         # Moved here when sprint completes
\`\`\`
EOF

# Create initial status check script
cat > "$SPRINT_DIR/check-status.sh" << 'EOF'
#!/bin/bash
# Quick status check for this sprint

SPRINT_DIR=$(dirname "$0")
echo "📊 Sprint Status Check"
echo "====================="
echo ""

# Count task statuses
TODO_COUNT=$(grep -l "Status: TODO\|status: TODO" "$SPRINT_DIR"/TASK-*.md 2>/dev/null | wc -l)
IN_PROGRESS_COUNT=$(grep -l "Status: IN_PROGRESS\|status: IN_PROGRESS" "$SPRINT_DIR"/TASK-*.md 2>/dev/null | wc -l)
COMPLETE_COUNT=$(grep -l "Status: COMPLETE\|status: COMPLETE" "$SPRINT_DIR"/TASK-*.md 2>/dev/null | wc -l)

echo "📋 Task Summary:"
echo "  TODO:        $TODO_COUNT"
echo "  IN_PROGRESS: $IN_PROGRESS_COUNT"
echo "  COMPLETE:    $COMPLETE_COUNT"
echo ""

# Check for test results
if [ -f "$SPRINT_DIR/TEST_RESULTS.md" ]; then
    if grep -q "All tests passing" "$SPRINT_DIR/TEST_RESULTS.md"; then
        echo "✅ Tests: PASSING"
    else
        echo "❌ Tests: FAILING or INCOMPLETE"
    fi
else
    echo "⚠️  Tests: NOT RUN (no TEST_RESULTS.md)"
fi

echo ""
# Check git branch
CURRENT_BRANCH=$(git branch --show-current)
if [ "$CURRENT_BRANCH" = "main" ]; then
    echo "🔴 WARNING: You are on main branch!"
else
    echo "✅ Branch: $CURRENT_BRANCH"
fi
EOF

chmod +x "$SPRINT_DIR/check-status.sh"

# Success message
echo ""
echo -e "${GREEN}✅ Sprint created successfully!${NC}"
echo ""
echo "📁 Location: $SPRINT_DIR"
echo ""
echo "📋 Next Steps:"
echo "1. Edit sprint goals:"
echo "   vim $SPRINT_DIR/SPRINT_PLAN.md"
echo ""
echo "2. Create tasks from template:"
echo "   cp $SPRINT_DIR/TASK-001_rename_me.md $SPRINT_DIR/TASK-001_your_task.md"
echo ""
echo "3. Check sprint status:"
echo "   ./.claude/scrum/task-manager.sh status"
echo ""
echo "4. Quick status check:"
echo "   $SPRINT_DIR/check-status.sh"
echo ""
echo -e "${YELLOW}⚠️  Remember: NEVER work on main branch!${NC}"