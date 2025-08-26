#!/bin/bash
# AlphaPulse Task Manager
# Coordinates atomic development workflow for production deployment

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TASK_DIR="$SCRIPT_DIR/../tasks"
SCRUM_DIR="$SCRIPT_DIR"
ROADMAP="$SCRIPT_DIR/../roadmap.md"
TASK_MGMT="$SCRIPT_DIR/TASK_MANAGEMENT.md"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

show_usage() {
    echo "Usage: $0 <command>"
    echo ""
    echo "Commands:"
    echo "  status     - Show current sprint status"
    echo "  next       - Show next priority task"
    echo "  start ID   - Start working on task ID"
    echo "  complete ID - Mark task as completed"
    echo "  list       - List all active tasks"
    echo "  help       - Show this help"
}

show_status() {
    echo -e "${BLUE}📊 AlphaPulse Production Sprint Status${NC}"
    echo "============================================="
    echo ""

    echo -e "${RED}🔴 EMERGENCY DATA INTEGRITY CRISIS:${NC}"
    echo "INTEGRITY-001: Fix hardcoded signal data (fake profits/venues)"
    echo "INTEGRITY-002: Remove protocol violations (type 255 abuse)"
    echo "SAFETY-001-NEW: Re-enable profitability guards"
    echo ""

    echo -e "${YELLOW}🟡 Critical Tasks (This Week):${NC}"
    echo "SAFETY-002: Complete detector implementation"
    echo "EVENTS-001: Process all DEX events (not just Swaps)"
    echo "EVENTS-002: Update PoolStateManager for liquidity"
    echo ""

    echo -e "${GREEN}✅ ARCHIVED (Completed 2025-08-26):${NC}"
    echo "TESTING-001, PERF-001, SAFETY-001, CAPITAL-001, LOGGING-001"
    echo ""

    echo -e "${GREEN}📈 Current Branch:${NC} $(git branch --show-current)"
    echo -e "${GREEN}📈 Last Commit:${NC} $(git log --oneline -1)"
}

show_next() {
    echo -e "${BLUE}🎯 Next Priority Task (Data Integrity Crisis)${NC}"
    echo "============================================="
    echo ""

    echo -e "${RED}🚨 HIGHEST PRIORITY - EMERGENCY DATA INTEGRITY:${NC}"
    echo "INTEGRITY-001: Fix hardcoded fake data in dashboard signals"
    echo "  - Remove hardcoded profit values and venue assignments"
    echo "  - Ensure all signals come from real arbitrage calculations"
    echo "  - Fix dashboard display to show authentic opportunity data"
    echo ""
    echo -e "${YELLOW}Branch:${NC} git checkout -b integrity-001-fix-fake-data"
    echo -e "${YELLOW}Location:${NC} services_v2/dashboard/websocket_server/src/"
    echo "  - message_converter.rs (likely contains hardcoded data)"
    echo "  - relay_consumer.rs (signal processing)"
    echo ""
    echo -e "${RED}CRITICAL IMPACT:${NC} System is currently lying to users about profits!"
    echo -e "${RED}MUST BE FIXED BEFORE ANY PRODUCTION DEPLOYMENT${NC}"
    echo ""
    echo -e "${YELLOW}To start:${NC} git checkout -b integrity-001-fix-fake-data"
}

start_task() {
    task_id="$1"
    if [[ -z "$task_id" ]]; then
        echo -e "${RED}Error: Task ID required${NC}"
        echo "Usage: $0 start TASK-001"
        exit 1
    fi

    # Check if on main branch
    current_branch=$(git branch --show-current)
    if [[ "$current_branch" != "main" ]]; then
        echo -e "${YELLOW}Warning: Not on main branch. Current: $current_branch${NC}"
        echo "Switch to main first? (y/n)"
        read -r response
        if [[ "$response" =~ ^[Yy]$ ]]; then
            git checkout main
            git pull origin main
        fi
    fi

    # Create branch name
    branch_name=$(echo "$task_id" | tr '[:upper:]' '[:lower:]' | sed 's/[^a-z0-9]/-/g')
    branch_name="${branch_name}-implementation"

    echo -e "${BLUE}🚀 Starting task: $task_id${NC}"
    echo "Creating branch: $branch_name"

    git checkout -b "$branch_name"

    echo ""
    echo -e "${GREEN}✅ Ready to work on $task_id${NC}"
    echo -e "${YELLOW}📋 Next steps:${NC}"
    echo "1. Follow TDD workflow: Write test first"
    echo "2. Check task details in: $TASK_DIR/pool-address-fix/"
    echo "3. Use atomic commits (<100 lines)"
    echo "4. Create PR when complete"

    # Show task template
    if [[ -f "$SCRUM_DIR/TASK_TEMPLATE_TDD.md" ]]; then
        echo ""
        echo -e "${BLUE}📖 TDD Template:${NC} $SCRUM_DIR/TASK_TEMPLATE_TDD.md"
    fi
}

complete_task() {
    task_id="$1"
    if [[ -z "$task_id" ]]; then
        echo -e "${RED}Error: Task ID required${NC}"
        echo "Usage: $0 complete TASK-001"
        exit 1
    fi

    echo -e "${GREEN}✅ Marking $task_id as completed${NC}"
    echo ""
    echo -e "${YELLOW}Completion Checklist:${NC}"
    echo "□ All tests passing"
    echo "□ Performance regression check passed"
    echo "□ Code review completed"
    echo "□ PR merged to main"
    echo ""
    echo "Update task status manually in $TASK_MGMT"
    echo "Update roadmap completion in $ROADMAP"
}

list_tasks() {
    echo -e "${BLUE}📋 All Active Tasks${NC}"
    echo "=================="
    echo ""

    echo -e "${RED}🔴 Critical:${NC}"
    grep -A 20 "CRITICAL: Production Blockers" $TASK_MGMT | grep "| \*\*" | head -5
    echo ""

    echo -e "${YELLOW}🟡 Quality:${NC}"
    grep -A 20 "Production Quality" $TASK_MGMT | grep "| \*\*" | head -5
}

# Main command handling
case "$1" in
    "status")
        show_status
        ;;
    "next")
        show_next
        ;;
    "start")
        start_task "$2"
        ;;
    "complete")
        complete_task "$2"
        ;;
    "list")
        list_tasks
        ;;
    "help"|"--help"|"-h")
        show_usage
        ;;
    *)
        echo -e "${RED}Error: Unknown command '$1'${NC}"
        echo ""
        show_usage
        exit 1
        ;;
esac
