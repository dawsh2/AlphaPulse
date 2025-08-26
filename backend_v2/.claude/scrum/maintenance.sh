#!/bin/bash
# Sprint System Maintenance Script
# Run weekly to prevent cruft accumulation

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TASK_DIR="$SCRIPT_DIR/../tasks"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}🧹 AlphaPulse Sprint Maintenance${NC}"
echo "=================================="
echo "Date: $(date +%Y-%m-%d)"
echo ""

# 1. Archive completed sprints
echo -e "${BLUE}1. Checking for completed sprints...${NC}"
"$SCRIPT_DIR/task-manager.sh" auto-archive
echo ""

# 2. Check for stale tasks
echo -e "${BLUE}2. Checking for stale IN_PROGRESS tasks...${NC}"
STALE_TASKS=$(find "$TASK_DIR" -name "*.md" -mtime +7 -exec grep -l "status: IN_PROGRESS\|Status: IN_PROGRESS" {} \; 2>/dev/null || true)
if [ -n "$STALE_TASKS" ]; then
    echo -e "${YELLOW}⚠️  Found stale IN_PROGRESS tasks (>7 days old):${NC}"
    echo "$STALE_TASKS"
else
    echo -e "${GREEN}✅ No stale tasks found${NC}"
fi
echo ""

# 3. Check for abandoned sprints
echo -e "${BLUE}3. Checking for abandoned sprints...${NC}"
ABANDONED_COUNT=0
for sprint_dir in "$TASK_DIR"/sprint-*/; do
    if [[ -d "$sprint_dir" ]]; then
        SPRINT_NAME=$(basename "$sprint_dir")
        # Check last modification time of sprint
        LAST_MOD=$(find "$sprint_dir" -type f -name "*.md" -exec stat -f "%m" {} \; 2>/dev/null | sort -n | tail -1 || echo "0")
        CURRENT_TIME=$(date +%s)
        DAYS_OLD=$(( (CURRENT_TIME - LAST_MOD) / 86400 ))
        
        if [[ $DAYS_OLD -gt 30 ]]; then
            echo -e "${YELLOW}⚠️  Sprint $SPRINT_NAME hasn't been modified in $DAYS_OLD days${NC}"
            ((ABANDONED_COUNT++))
        fi
    fi
done

if [[ $ABANDONED_COUNT -eq 0 ]]; then
    echo -e "${GREEN}✅ No abandoned sprints${NC}"
fi
echo ""

# 4. Check for format compliance
echo -e "${BLUE}4. Checking task format compliance...${NC}"
NON_COMPLIANT=0
TOTAL_TASKS=0
for task_file in "$TASK_DIR"/sprint-*/TASK-*.md; do
    if [[ -f "$task_file" ]]; then
        ((TOTAL_TASKS++))
        if ! grep -q "^status:\|^\*\*Status\*\*:" "$task_file"; then
            echo -e "${YELLOW}⚠️  Non-standard format: $(basename "$(dirname "$task_file")")/$(basename "$task_file")${NC}"
            ((NON_COMPLIANT++))
        fi
    fi
done

if [[ $TOTAL_TASKS -gt 0 ]]; then
    COMPLIANCE_RATE=$(( (TOTAL_TASKS - NON_COMPLIANT) * 100 / TOTAL_TASKS ))
    if [[ $COMPLIANCE_RATE -eq 100 ]]; then
        echo -e "${GREEN}✅ Format compliance: 100%${NC}"
    else
        echo -e "${YELLOW}📊 Format compliance: $COMPLIANCE_RATE% ($NON_COMPLIANT/$TOTAL_TASKS non-compliant)${NC}"
    fi
else
    echo "No tasks found to check"
fi
echo ""

# 5. Check for stale branches
echo -e "${BLUE}5. Checking for stale branches...${NC}"
if command -v git &> /dev/null; then
    # Get branches older than 30 days
    OLD_BRANCHES=$(git for-each-ref --format='%(refname:short) %(committerdate:iso8601)' refs/heads/ | \
        awk -v date="$(date -d '30 days ago' '+%Y-%m-%d' 2>/dev/null || date -v-30d '+%Y-%m-%d')" '$2 < date && $1 != "main" {print $1}')
    
    if [ -n "$OLD_BRANCHES" ]; then
        echo -e "${YELLOW}⚠️  Found branches older than 30 days:${NC}"
        echo "$OLD_BRANCHES"
        echo ""
        echo "To delete merged branches:"
        echo "  git branch --merged main | grep -v main | xargs -r git branch -d"
    else
        echo -e "${GREEN}✅ No stale branches${NC}"
    fi
else
    echo "⚠️  Git not available"
fi
echo ""

# 6. Check for missing TEST_RESULTS.md
echo -e "${BLUE}6. Checking for missing test results...${NC}"
MISSING_TESTS=0
for sprint_dir in "$TASK_DIR"/sprint-*/; do
    if [[ -d "$sprint_dir" ]]; then
        SPRINT_NAME=$(basename "$sprint_dir")
        # Check if sprint has completed tasks
        if grep -q "status: COMPLETE\|Status: COMPLETE" "$sprint_dir"/*.md 2>/dev/null; then
            # Check for TEST_RESULTS.md
            if [[ ! -f "$sprint_dir/TEST_RESULTS.md" ]]; then
                echo -e "${YELLOW}⚠️  $SPRINT_NAME has completed tasks but no TEST_RESULTS.md${NC}"
                ((MISSING_TESTS++))
            fi
        fi
    fi
done

if [[ $MISSING_TESTS -eq 0 ]]; then
    echo -e "${GREEN}✅ All completed sprints have test results${NC}"
fi
echo ""

# 7. Generate metrics summary
echo -e "${BLUE}📊 Sprint Metrics Summary${NC}"
echo "========================="

# Count sprints
ACTIVE_SPRINTS=$(find "$TASK_DIR" -maxdepth 1 -type d -name "sprint-*" | wc -l)
ARCHIVED_SPRINTS=$(find "$TASK_DIR/archive" -maxdepth 1 -type d -name "sprint-*" 2>/dev/null | wc -l || echo 0)

# Count tasks by status
TODO_COUNT=$(grep -r "status: TODO\|Status: TODO" "$TASK_DIR"/sprint-*/TASK-*.md 2>/dev/null | wc -l || echo 0)
IN_PROGRESS_COUNT=$(grep -r "status: IN_PROGRESS\|Status: IN_PROGRESS" "$TASK_DIR"/sprint-*/TASK-*.md 2>/dev/null | wc -l || echo 0)
COMPLETE_COUNT=$(grep -r "status: COMPLETE\|Status: COMPLETE" "$TASK_DIR"/sprint-*/TASK-*.md 2>/dev/null | wc -l || echo 0)
BLOCKED_COUNT=$(grep -r "status: BLOCKED\|Status: BLOCKED" "$TASK_DIR"/sprint-*/TASK-*.md 2>/dev/null | wc -l || echo 0)

echo "Active Sprints:    $ACTIVE_SPRINTS"
echo "Archived Sprints:  $ARCHIVED_SPRINTS"
echo ""
echo "Task Status:"
echo "  TODO:        $TODO_COUNT"
echo "  IN_PROGRESS: $IN_PROGRESS_COUNT"
echo "  COMPLETE:    $COMPLETE_COUNT"
echo "  BLOCKED:     $BLOCKED_COUNT"
echo ""

# Calculate health score
HEALTH_SCORE=100
HEALTH_ISSUES=""

if [[ $NON_COMPLIANT -gt 0 ]]; then
    HEALTH_SCORE=$((HEALTH_SCORE - 10))
    HEALTH_ISSUES="${HEALTH_ISSUES}\n  - Format compliance issues"
fi

if [[ -n "$STALE_TASKS" ]]; then
    HEALTH_SCORE=$((HEALTH_SCORE - 15))
    HEALTH_ISSUES="${HEALTH_ISSUES}\n  - Stale IN_PROGRESS tasks"
fi

if [[ $ABANDONED_COUNT -gt 0 ]]; then
    HEALTH_SCORE=$((HEALTH_SCORE - 20))
    HEALTH_ISSUES="${HEALTH_ISSUES}\n  - Abandoned sprints"
fi

if [[ $MISSING_TESTS -gt 0 ]]; then
    HEALTH_SCORE=$((HEALTH_SCORE - 15))
    HEALTH_ISSUES="${HEALTH_ISSUES}\n  - Missing test results"
fi

if [[ -n "$OLD_BRANCHES" ]]; then
    HEALTH_SCORE=$((HEALTH_SCORE - 10))
    HEALTH_ISSUES="${HEALTH_ISSUES}\n  - Stale git branches"
fi

echo -e "${BLUE}🏥 System Health Score: $HEALTH_SCORE/100${NC}"
if [[ $HEALTH_SCORE -eq 100 ]]; then
    echo -e "${GREEN}✅ System is healthy!${NC}"
elif [[ $HEALTH_SCORE -ge 80 ]]; then
    echo -e "${YELLOW}⚠️  Minor issues detected:${NC}"
    echo -e "$HEALTH_ISSUES"
else
    echo -e "${RED}🚨 Significant issues requiring attention:${NC}"
    echo -e "$HEALTH_ISSUES"
fi

echo ""
echo -e "${BLUE}📝 Recommendations${NC}"
echo "=================="

if [[ $HEALTH_SCORE -lt 100 ]]; then
    echo "1. Run weekly maintenance to prevent decay"
    [[ -n "$STALE_TASKS" ]] && echo "2. Review and update stale IN_PROGRESS tasks"
    [[ $ABANDONED_COUNT -gt 0 ]] && echo "3. Archive or resume abandoned sprints"
    [[ $NON_COMPLIANT -gt 0 ]] && echo "4. Update non-compliant tasks to standard format"
    [[ $MISSING_TESTS -gt 0 ]] && echo "5. Add TEST_RESULTS.md to completed sprints"
    [[ -n "$OLD_BRANCHES" ]] && echo "6. Clean up old git branches"
else
    echo "System is healthy - continue current practices!"
fi

echo ""

# 8. Update agent documentation
echo -e "${BLUE}8. Updating agent documentation...${NC}"
if [[ -f "$SCRIPT_DIR/update-agent-docs.sh" ]]; then
    "$SCRIPT_DIR/update-agent-docs.sh" > /dev/null 2>&1
    echo -e "${GREEN}✅ Agent documentation updated${NC}"
else
    echo -e "${YELLOW}⚠️  update-agent-docs.sh not found${NC}"
fi

echo ""
echo "Maintenance complete: $(date +%H:%M:%S)"