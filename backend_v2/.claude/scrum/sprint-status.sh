#!/bin/bash
# Simple Sprint-Specific Status Tool
# Usage: ./sprint-status.sh sprint-006

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TASK_DIR="$SCRIPT_DIR/../tasks"

# Source the kanban library
source "$SCRIPT_DIR/lib/kanban.sh"

# Colors (already defined in kanban.sh but redefined for local use)
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m'

# Extract YAML field from frontmatter
extract_yaml_field() {
    local file="$1"
    local field="$2"
    
    if [[ -f "$file" ]] && grep -q "^---" "$file"; then
        # Extract YAML section (between first and second ---)
        awk '/^---$/{if(++c==2) exit} c==1' "$file" | grep "^$field:" | cut -d: -f2- | sed 's/^[[:space:]]*//;s/[[:space:]]*$//'
    fi
}

# Extract task status (with fallback)
get_task_status() {
    local task_file="$1"
    
    # Try YAML first
    local status=$(extract_yaml_field "$task_file" "status")
    if [[ -n "$status" ]]; then
        echo "$status" | tr '[:lower:]' '[:upper:]'
        return
    fi
    
    # Fallback to markdown
    if [[ -f "$task_file" ]]; then
        status=$(grep -i "^\*\*Status\*\*:" "$task_file" 2>/dev/null || echo "Status: TODO")
        echo "$status" | sed 's/.*[Ss]tatus.*: *//' | sed 's/\*//g' | tr '[:lower:]' '[:upper:]'
    else
        echo "TODO"
    fi
}

# Extract task priority (with fallback)  
get_task_priority() {
    local task_file="$1"
    
    # Try YAML first
    local priority=$(extract_yaml_field "$task_file" "priority")
    if [[ -n "$priority" ]]; then
        echo "$priority" | tr '[:lower:]' '[:upper:]'
        return
    fi
    
    # Fallback to markdown
    if [[ -f "$task_file" ]]; then
        priority=$(grep "^\*\*Priority\*\*:" "$task_file" 2>/dev/null || echo "Priority: MEDIUM")
        echo "$priority" | sed 's/.*Priority.*: *//' | sed 's/\*//g' | tr '[:lower:]' '[:upper:]'
    else
        echo "MEDIUM"
    fi
}

# Show sprint status
show_sprint_status() {
    local sprint_id="$1"
    local sprint_dir=""
    
    # First try exact match
    if [[ -d "$TASK_DIR/${sprint_id}" ]]; then
        sprint_dir="$TASK_DIR/${sprint_id}"
    else
        # Try shorthand match (e.g., sprint-006 matches sprint-006-protocol-optimization)
        local matches=($(find "$TASK_DIR" -maxdepth 1 -type d -name "${sprint_id}*" 2>/dev/null))
        
        if [[ ${#matches[@]} -eq 1 ]]; then
            sprint_dir="${matches[0]}"
            echo -e "${CYAN}🔍 Found: $(basename "$sprint_dir")${NC}"
            echo ""
        elif [[ ${#matches[@]} -gt 1 ]]; then
            echo -e "${YELLOW}❓ Multiple matches found for '$sprint_id':${NC}"
            for match in "${matches[@]}"; do
                echo "  - $(basename "$match")"
            done
            echo ""
            echo "Please use the full name or a more specific pattern."
            return 1
        else
            echo -e "${RED}❌ Sprint directory not found: $sprint_id${NC}"
            echo -e "${CYAN}💡 Available sprints:${NC}"
            for dir in "$TASK_DIR"/sprint-*/; do
                [[ -d "$dir" ]] && echo "  - $(basename "$dir")"
            done
            return 1
        fi
    fi
    
    echo -e "${BLUE}📊 Detailed Status: $(basename "$sprint_dir")${NC}"
    echo "$(printf '=%.0s' {1..60})"
    echo ""
    
    # Sprint plan info
    local plan_file="$sprint_dir/SPRINT_PLAN.md"
    if [[ -f "$plan_file" ]]; then
        echo -e "${CYAN}📋 Sprint Overview:${NC}"
        local title=$(grep "^# " "$plan_file" | head -1 | sed 's/^# //')
        local duration=$(grep "^\*Sprint Duration:" "$plan_file" | sed 's/^\*Sprint Duration: //')
        local objective=$(grep "^\*Objective:" "$plan_file" | sed 's/^\*Objective: //')
        
        [[ -n "$title" ]] && echo "   Title:     $title"
        [[ -n "$duration" ]] && echo "   Duration:  $duration"
        [[ -n "$objective" ]] && echo "   Objective: $objective"
        echo ""
    fi
    
    # Collect task data for kanban display
    local task_items=()
    
    # Process task files
    for task_file in "$sprint_dir"/*.md; do
        [[ ! -f "$task_file" ]] && continue
        
        local filename=$(basename "$task_file")
        
        # Skip non-task files
        [[ "$filename" == "SPRINT_PLAN.md" ]] && continue
        [[ "$filename" == "README.md" ]] && continue
        [[ "$filename" == "STATUS.md" ]] && continue
        [[ "$filename" == *"rename_me.md" ]] && continue
        
        # Only process actual task files (pattern: XXX-NNN_description.md)
        if [[ "$filename" =~ ^[A-Z]+-[0-9]+ ]]; then
            local task_name=$(basename "$task_file" .md)
            local status=$(get_task_status "$task_file")
            local priority=$(get_task_priority "$task_file")
            
            # Normalize status values
            case "$status" in
                "COMPLETED"|"COMPLETE") status="COMPLETE" ;;
                "IN_PROGRESS") status="IN_PROGRESS" ;;
                "BLOCKED") status="BLOCKED" ;;
                *) status="TODO" ;;
            esac
            
            # Add to items array (name|status|priority format)
            task_items+=("${task_name}|${status}|${priority}")
        fi
    done
    
    # Draw kanban board using library
    echo -e "${YELLOW}📋 Task Kanban Board:${NC}"
    echo ""
    
    if [[ ${#task_items[@]} -gt 0 ]]; then
        draw_kanban "Task" "${task_items[@]}"
        echo ""
        
        # Calculate and display statistics
        local stats=$(calculate_stats "${task_items[@]}")
        draw_progress_summary "$stats"
        echo ""
    else
        echo -e "${YELLOW}📈 No tasks found in this sprint${NC}"
        echo ""
    fi
    
    # Show next recommended task
    if [[ $progress_tasks -eq 0 && $todo_tasks -gt 0 ]]; then
        echo -e "${GREEN}🎯 Next Recommended Task:${NC}"
        
        # Find highest priority TODO task
        local best_task=""
        local best_priority=999
        
        for task_file in "$sprint_dir"/*.md; do
            [[ ! -f "$task_file" ]] && continue
            local filename=$(basename "$task_file")
            [[ "$filename" =~ ^[A-Z]+-[0-9]+ ]] || continue
            [[ "$(get_task_status "$task_file")" == "TODO" ]] || continue
            
            local priority=$(get_task_priority "$task_file")
            local priority_val
            case "$priority" in
                "CRITICAL") priority_val=1 ;;
                "HIGH") priority_val=2 ;;
                "MEDIUM") priority_val=3 ;;
                "LOW") priority_val=4 ;;
                *) priority_val=5 ;;
            esac
            
            if [[ $priority_val -lt $best_priority ]]; then
                best_priority=$priority_val
                best_task="$task_file"
            fi
        done
        
        if [[ -n "$best_task" ]]; then
            local task_name=$(basename "$best_task" .md)
            local priority=$(get_task_priority "$best_task")
            local branch=$(extract_yaml_field "$best_task" "branch")
            
            echo "   📋 Task:     $task_name [$priority]"
            [[ -n "$branch" ]] && echo "   🌿 Branch:   $branch"
            echo "   📁 File:     $best_task"
            echo ""
            echo "   🚀 To start: Edit the file and change 'status: TODO' to 'status: IN_PROGRESS'"
        fi
    elif [[ $progress_tasks -gt 0 ]]; then
        echo -e "${YELLOW}🎯 Currently Active Tasks:${NC}"
        
        for task_file in "$sprint_dir"/*.md; do
            [[ ! -f "$task_file" ]] && continue
            local filename=$(basename "$task_file")
            [[ "$filename" =~ ^[A-Z]+-[0-9]+ ]] || continue
            [[ "$(get_task_status "$task_file")" == "IN_PROGRESS" ]] || continue
            
            local task_name=$(basename "$task_file" .md)
            local branch=$(extract_yaml_field "$task_file" "branch")
            
            echo "   🟡 Task:     $task_name"
            [[ -n "$branch" ]] && echo "      🌿 Branch:   $branch"
            echo "      📁 File:     $task_file"
        done
    fi
}

# Main execution
if [[ -z "$1" ]]; then
    echo "Usage: $0 <sprint-id>"
    echo ""
    echo "Examples:"
    echo "  $0 sprint-006                      # Shorthand"
    echo "  $0 sprint-006-protocol-optimization # Full name"
    echo ""
    echo "Available sprints:"
    for dir in "$TASK_DIR"/sprint-*/; do
        [[ -d "$dir" ]] && echo "  - $(basename "$dir")"
    done
    exit 1
fi

show_sprint_status "$1"