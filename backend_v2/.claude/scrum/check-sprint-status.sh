#!/bin/bash
# Automated Sprint Status Verification
# Detects if work has been done but status hasn't been updated

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TASKS_DIR="$SCRIPT_DIR/../tasks"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "🔍 Checking sprint task status consistency..."

# Find all active sprints (not archived)
for sprint_dir in "$TASKS_DIR"/sprint-*; do
    if [[ -d "$sprint_dir" ]]; then
        sprint_name=$(basename "$sprint_dir")
        
        # Skip archived sprints
        if [[ -d "$TASKS_DIR/archive/$sprint_name" ]]; then
            continue
        fi
        
        echo -e "\n📋 Checking: $sprint_name"
        
        # Check each task file
        for task_file in "$sprint_dir"/TASK-*.md; do
            if [[ -f "$task_file" ]]; then
                task_name=$(basename "$task_file" .md)
                
                # Extract current status
                current_status=$(grep "^status:" "$task_file" | head -1 | cut -d: -f2 | xargs)
                
                # Check if file has been recently modified (heuristic for work being done)
                if [[ $(find "$task_file" -mtime -1 -print) ]]; then
                    if [[ "$current_status" == "TODO" ]]; then
                        echo -e "${YELLOW}⚠️  $task_name: Recently modified but still TODO${NC}"
                        echo -e "   ${YELLOW}Reminder: Update status to IN_PROGRESS or COMPLETE${NC}"
                    elif [[ "$current_status" == "IN_PROGRESS" ]]; then
                        echo -e "${GREEN}✅ $task_name: IN_PROGRESS (active work)${NC}"
                    elif [[ "$current_status" == "COMPLETE" ]]; then
                        echo -e "${GREEN}✅ $task_name: COMPLETE${NC}"
                    elif [[ "$current_status" == "BLOCKED" ]]; then
                        echo -e "${YELLOW}🚫 $task_name: BLOCKED${NC}"
                    else
                        echo -e "${RED}❌ $task_name: Unknown status '$current_status'${NC}"
                    fi
                fi
            fi
        done
    fi
done

echo -e "\n💡 Status update reminder:"
echo "   - When starting: TODO → IN_PROGRESS"
echo "   - When finished: IN_PROGRESS → COMPLETE"
echo "   - Verify with: task-manager.sh status"