#!/bin/bash
# Quick helper to find NEXT actionable tasks

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
ACTIVE_ORG="$SCRIPT_DIR/../tasks/active.org"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

if [ "$1" == "--help" ] || [ "$1" == "-h" ]; then
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  --next, -n           Show all actionable tasks (default)"
    echo "  --project ID, -p ID  Show tasks for specific project"
    echo "  --task ID, -t ID     Show dependency graph for task"
    echo "  --graph ID, -g ID    Generate Graphviz for task/project"
    echo "  --summary, -s        Show task summary statistics"
    echo ""
    echo "Examples:"
    echo "  $0                          # Show all NEXT tasks"
    echo "  $0 -p DOC-SYSTEM-GOAL       # Show tasks for Documentation project"
    echo "  $0 -t BUILD-002             # Show dependency graph for BUILD-002"
    echo "  $0 -g MYCELIUM-MVP-GOAL     # Generate Graphviz output"
    exit 0
fi

# Default to showing NEXT actions
if [ $# -eq 0 ]; then
    echo -e "${GREEN}🎯 Finding NEXT Actionable Tasks...${NC}\n"
    python3 "$SCRIPT_DIR/org-task-graph.py" "$ACTIVE_ORG" --next
elif [ "$1" == "--next" ] || [ "$1" == "-n" ]; then
    shift
    echo -e "${GREEN}🎯 Finding NEXT Actionable Tasks...${NC}\n"
    python3 "$SCRIPT_DIR/org-task-graph.py" "$ACTIVE_ORG" --next "$@"
elif [ "$1" == "--project" ] || [ "$1" == "-p" ]; then
    shift
    PROJECT_ID="$1"
    echo -e "${BLUE}📦 Finding NEXT Tasks for Project: $PROJECT_ID${NC}\n"
    python3 "$SCRIPT_DIR/org-task-graph.py" "$ACTIVE_ORG" --next --project "$PROJECT_ID"
elif [ "$1" == "--task" ] || [ "$1" == "-t" ]; then
    shift
    TASK_ID="$1"
    echo -e "${YELLOW}📊 Extracting Dependency Graph for: $TASK_ID${NC}\n"
    python3 "$SCRIPT_DIR/org-task-graph.py" "$ACTIVE_ORG" --task "$TASK_ID"
elif [ "$1" == "--graph" ] || [ "$1" == "-g" ]; then
    shift
    TASK_ID="$1"
    OUTPUT_FILE="/tmp/${TASK_ID}-graph.dot"
    python3 "$SCRIPT_DIR/org-task-graph.py" "$ACTIVE_ORG" --task "$TASK_ID" --graph > "$OUTPUT_FILE"
    echo -e "${GREEN}✅ Graph saved to: $OUTPUT_FILE${NC}"
    echo ""
    echo "To visualize, run:"
    echo "  dot -Tpng $OUTPUT_FILE -o ${TASK_ID}.png"
    echo "  open ${TASK_ID}.png"
elif [ "$1" == "--summary" ] || [ "$1" == "-s" ]; then
    echo -e "${BLUE}📈 Task Summary Statistics${NC}\n"
    python3 "$SCRIPT_DIR/org-task-graph.py" "$ACTIVE_ORG"
else
    python3 "$SCRIPT_DIR/org-task-graph.py" "$ACTIVE_ORG" "$@"
fi