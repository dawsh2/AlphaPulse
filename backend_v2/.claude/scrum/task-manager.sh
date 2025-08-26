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

    echo -e "${RED}🔴 CRITICAL Production Blockers:${NC}"
    grep -A 10 "CRITICAL: Production Blockers" $TASK_MGMT | grep "| \*\*" | head -5
    echo ""

    echo -e "${YELLOW}🟡 Production Quality Tasks:${NC}"
    grep -A 10 "Production Quality" $TASK_MGMT | grep "| \*\*" | head -5
    echo ""

    echo -e "${GREEN}📈 Current Branch:${NC} $(git branch --show-current)"
    echo -e "${GREEN}📈 Last Commit:${NC} $(git log --oneline -1)"
}

show_next() {
    echo -e "${BLUE}🎯 Next Priority Task${NC}"
    echo "===================="
    echo ""

    # Find first pending critical task
    next_task=$(grep -A 10 "CRITICAL: Production Blockers" $TASK_MGMT | grep "⭕ Pending" | head -1)

    if [[ -n "$next_task" ]]; then
        task_id=$(echo "$next_task" | grep -o '\*\*[^*]*\*\*' | sed 's/\*//g')
        echo -e "${RED}CRITICAL:${NC} $task_id"
        echo ""

        # Check if detailed breakdown exists
        detail_file="$TASK_DIR/pool-address-fix/${task_id}_*.md"
        if ls $detail_file 2>/dev/null; then
            echo -e "${GREEN}📋 Detailed breakdown available:${NC}"
            ls $detail_file | head -1
            echo ""
        fi

        echo -e "${YELLOW}To start:${NC} .claude/task-manager.sh start $task_id"
    else
        echo -e "${GREEN}✅ All critical tasks complete! Check quality tasks.${NC}"
    fi
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
