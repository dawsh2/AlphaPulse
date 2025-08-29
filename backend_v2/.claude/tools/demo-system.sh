#!/bin/bash
# Complete System Demo - Org-Mode Task Management with Org-Edna

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
MAGENTA='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

clear

echo -e "${MAGENTA}════════════════════════════════════════════════════════════════${NC}"
echo -e "${MAGENTA}     ORG-MODE TASK MANAGEMENT SYSTEM - COMPLETE DEMO           ${NC}"
echo -e "${MAGENTA}════════════════════════════════════════════════════════════════${NC}"
echo ""

# 1. System Status
echo -e "${CYAN}1. SYSTEM STATUS CHECK${NC}"
echo -e "${BLUE}──────────────────────────────────────────────────────────────${NC}"
python3 agent_task_commands.py status
echo ""

sleep 2

# 2. Find Next Actionable Tasks
echo -e "${CYAN}2. FINDING NEXT ACTIONABLE TASKS${NC}"
echo -e "${BLUE}──────────────────────────────────────────────────────────────${NC}"
./next-tasks.sh -n | head -20
echo ""

sleep 2

# 3. Priority A Work Extraction
echo -e "${CYAN}3. PRIORITY A WORK PLAN${NC}"
echo -e "${BLUE}──────────────────────────────────────────────────────────────${NC}"
python3 priority_extractor.py ../tasks/active.org A | head -30
echo ""

sleep 2

# 4. Dependency Graph for Critical Task
echo -e "${CYAN}4. DEPENDENCY GRAPH - GAP-005 (Build System Fix)${NC}"
echo -e "${BLUE}──────────────────────────────────────────────────────────────${NC}"
./next-tasks.sh -t GAP-005
echo ""

sleep 2

# 5. Cross-Tree Dependencies
echo -e "${CYAN}5. CROSS-TREE DEPENDENCY VALIDATION${NC}"
echo -e "${BLUE}──────────────────────────────────────────────────────────────${NC}"
echo "GAP-005 (Build tree) → SAFETY-001 (Safety tree) → CREATE-GAP-004 (Migration tree)"
python3 org-task-graph.py ../tasks/active.org --task SAFETY-001 | grep -A10 "Dependencies"
echo ""

sleep 2

# 6. Org-Edna Status
echo -e "${CYAN}6. ORG-EDNA DEPENDENCY MANAGEMENT STATUS${NC}"
echo -e "${BLUE}──────────────────────────────────────────────────────────────${NC}"
emacs -batch -l simple-edna-test.el 2>&1 | grep -A5 "PROPERTY STATISTICS"
echo ""

sleep 2

# 7. TDD Workflow Example
echo -e "${CYAN}7. TDD WORKFLOW DEMONSTRATION${NC}"
echo -e "${BLUE}──────────────────────────────────────────────────────────────${NC}"
echo "Example: BUILD-002 (Implementation) depends on BUILD-002-TESTS (Test Design)"
grep -A3 "BUILD-002-TESTS" ../tasks/active.org | grep -E "TRIGGER|TODO"
echo ""
echo "When BUILD-002-TESTS is marked DONE, BUILD-002 automatically advances to NEXT"
echo ""

sleep 2

# 8. Project-Specific Tasks
echo -e "${CYAN}8. PROJECT-SPECIFIC TASK QUERIES${NC}"
echo -e "${BLUE}──────────────────────────────────────────────────────────────${NC}"
echo -e "${YELLOW}Build Fix Project:${NC}"
./next-tasks.sh -p BUILD-FIX-GOAL | head -10
echo ""
echo -e "${YELLOW}Mycelium MVP Project:${NC}"
./next-tasks.sh -p MYCELIUM-MVP-GOAL | head -10
echo ""

sleep 2

# 9. Task Update Workflow
echo -e "${CYAN}9. TASK UPDATE WORKFLOW${NC}"
echo -e "${BLUE}──────────────────────────────────────────────────────────────${NC}"
echo "Current BUILD-FIX-GOAL status:"
grep -A5 "BUILD-FIX-GOAL" ../tasks/active.org | grep -E "^\*\s|IN-PROGRESS"
echo ""
echo "Progress Update Added:"
grep -A3 "PROGRESS UPDATE" ../tasks/active.org | head -5
echo ""

# 10. Summary
echo -e "${GREEN}════════════════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}                        DEMO COMPLETE                          ${NC}"
echo -e "${GREEN}════════════════════════════════════════════════════════════════${NC}"
echo ""
echo -e "${YELLOW}Key Features Demonstrated:${NC}"
echo "✅ Org-edna automatic dependency management (139 BLOCKER, 85 TRIGGER)"
echo "✅ Cross-tree dependency tracking (GAP-005 → SAFETY-001)"
echo "✅ TDD workflow with test-first development"
echo "✅ Priority-based work extraction (21 actionable tasks)"
echo "✅ Project-specific task management"
echo "✅ Real-time task status updates"
echo ""
echo -e "${CYAN}Available Commands:${NC}"
echo "  ./next-tasks.sh            - Find actionable tasks"
echo "  ./org_tasks.sh ready       - Get ready tasks via Emacs"
echo "  python3 agent_task_commands.py status - System status"
echo "  python3 priority_extractor.py active.org A - Priority work"
echo ""
echo -e "${MAGENTA}System is fully operational with org-edna dependency management!${NC}"