#!/bin/bash
# Torq Dynamic Task Manager
# Reads from actual task files to provide current status

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TASK_DIR="$SCRIPT_DIR/../tasks"
SCRUM_DIR="$SCRIPT_DIR"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

show_usage() {
    echo "Usage: $0 <command> [args]"
    echo ""
    echo "Commands:"
    echo "  status          - Show current sprint status (dynamic)"
    echo "  sprint-XXX      - Show detailed status for specific sprint"
    echo "  kanban          - Show tasks in visual kanban board with colors"
    echo "  next            - Show ready tasks with satisfied dependencies (JIT queue)"
    echo "  list            - List all active tasks across all sprints"
    echo "  sprints         - Show all available sprints"
    echo "  scan            - Scan task files and show current state"
    echo "  deps            - Show sprint dependencies and conflicts"
    echo "  check-deps <sprint-name> - Validate dependencies before starting sprint"
    echo "  check-complete <sprint-name> - Check if sprint is ready for archiving"
    echo "  archive-sprint <sprint-name> - Archive a completed sprint"
    echo "  auto-archive    - Check all sprints and archive completed ones"
    echo "  validate-plan   - Check entire project for circular dependencies"
    echo "  find-conflicts <file> - Find scope conflicts for a task file"
    echo "  graph           - Generate dependency visualization graph"
    echo "  migrate-critical - Migrate critical sprints to new format"
    echo "  lint <file|dir>  - Validate task metadata format"
    echo "  lint-all        - Validate all task files"
    echo "  health          - Generate task metadata health report"
    echo "  help            - Show this help"
    echo ""
    echo "Examples:"
    echo "  $0 status           # Show all sprint progress"
    echo "  $0 sprint-006       # Show detailed Sprint 006 status"
    echo "  $0 kanban           # Show visual kanban board"
    echo "  $0 next             # Get JIT ready task queue"
    echo "  $0 validate-plan    # Check for circular dependencies"
    echo "  $0 graph            # Generate dependency graph"
}

# Extract task status from markdown files
get_task_status() {
    local task_file="$1"
    if [[ -f "$task_file" ]]; then
        # First try YAML frontmatter
        if grep -q "^---" "$task_file"; then
            status=$(sed -n '/^---/,/^---/p' "$task_file" | grep "^status:" | cut -d: -f2 | tr -d ' ')
            if [[ -n "$status" ]]; then
                echo "$status" | tr '[:lower:]' '[:upper:]'
                return
            fi
        fi
        
        # Then try markdown format (multiple patterns)
        status=$(grep -i "^\*\*Status\*\*:" "$task_file" 2>/dev/null || \
                 grep -i "^Status:" "$task_file" 2>/dev/null || \
                 grep -i "\*\*Status\*\*:" "$task_file" 2>/dev/null | head -1 || \
                 echo "Status: TODO")
        echo "$status" | sed 's/.*[Ss]tatus.*: *//' | sed 's/\*//g' | tr '[:lower:]' '[:upper:]'
    else
        echo "FILE_NOT_FOUND"
    fi
}

# Extract priority from markdown files
get_task_priority() {
    local task_file="$1"
    if [[ -f "$task_file" ]]; then
        # First try YAML frontmatter
        if grep -q "^---" "$task_file"; then
            priority=$(sed -n '/^---/,/^---/p' "$task_file" | grep "^priority:" | cut -d: -f2 | tr -d ' ')
            if [[ -n "$priority" ]]; then
                echo "$priority" | tr '[:lower:]' '[:upper:]'
                return
            fi
        fi
        
        # Then try markdown format
        priority=$(grep "^\*\*Priority\*\*:" "$task_file" 2>/dev/null || grep "^Priority:" "$task_file" 2>/dev/null || echo "Priority: MEDIUM")
        echo "$priority" | sed 's/.*Priority.*: *//' | sed 's/\*//g' | tr '[:lower:]' '[:upper:]'
    else
        echo "UNKNOWN"
    fi
}

# Extract task description from filename and content
get_task_description() {
    local task_file="$1"
    local filename=$(basename "$task_file" .md)
    local task_id=$(echo "$filename" | cut -d'_' -f1)
    local description=""
    
    if [[ -f "$task_file" ]]; then
        # Try to get first line that looks like a title
        description=$(head -3 "$task_file" | grep "^#" | head -1 | sed 's/^#* *//' | sed 's/^[A-Z0-9-]*: *//')
    fi
    
    if [[ -z "$description" ]]; then
        # Fallback to filename parsing
        description=$(echo "$filename" | cut -d'_' -f2- | tr '_' ' ')
    fi
    
    echo "$task_id: $description"
}

# Get priority color (bash 3 compatible)
get_priority_color() {
    case "$1" in
        "CRITICAL") echo "${RED}🔴 CRITICAL${NC}" ;;
        "HIGH") echo "${YELLOW}🟡 HIGH${NC}" ;;
        "MEDIUM") echo "${CYAN}🔵 MEDIUM${NC}" ;;
        "LOW") echo "${GREEN}🟢 LOW${NC}" ;;
        *) echo "$1" ;;
    esac
}

# Get status color (bash 3 compatible)
get_status_color() {
    case "$1" in
        "TODO") echo "${RED}TODO${NC}" ;;
        "IN_PROGRESS") echo "${YELLOW}IN PROGRESS${NC}" ;;
        "DONE") echo "${GREEN}DONE${NC}" ;;
        "COMPLETE") echo "${GREEN}COMPLETE${NC}" ;;
        "ARCHIVED") echo "${PURPLE}ARCHIVED${NC}" ;;
        "BLOCKED") echo "${RED}BLOCKED${NC}" ;;
        *) echo "$1" ;;
    esac
}

# Analyze task file modification state
get_task_modification_state() {
    local task_file="$1"
    local sprint_dir=$(dirname "$task_file")
    local filename=$(basename "$task_file")
    
    # Check if it's an unmodified template
    if [[ "$filename" == *"rename_me"* ]] || [[ "$filename" == *"template"* ]]; then
        echo "TEMPLATE"
        return
    fi
    
    # Check file size and content modification from template
    local file_size=$(wc -c < "$task_file" 2>/dev/null || echo "0")
    local template_size=$(wc -c < "$SCRUM_DIR/templates/TASK_TEMPLATE.md" 2>/dev/null || echo "1000")
    
    # If file is very close to template size, it's probably unmodified
    local size_diff=$((file_size - template_size))
    if [[ $size_diff -lt 100 ]] && [[ $size_diff -gt -100 ]]; then
        # Check if it has real content vs template content
        if grep -q "Clear Task Description\|Problem Statement.*Clear description\|TASK-XXX" "$task_file"; then
            echo "UNMODIFIED"
            return
        fi
    fi
    
    # Check if file has been significantly worked on
    local status=$(get_task_status "$task_file")
    if [[ "$status" == "COMPLETE" ]] || [[ "$status" == "DONE" ]] || [[ "$status" == "ARCHIVED" ]]; then
        echo "COMPLETE"
    elif [[ "$status" == "IN_PROGRESS" ]]; then
        echo "IN_PROGRESS"
    elif [[ "$status" == "BLOCKED" ]]; then
        echo "BLOCKED"
    else
        # Check if file has substantial modifications
        local content_lines=$(grep -v "^#\|^$\|^---\|^\s*$" "$task_file" | wc -l)
        if [[ $content_lines -gt 50 ]]; then
            echo "IN_PROGRESS"
        else
            echo "UNMODIFIED"
        fi
    fi
}

# Get kanban color based on modification state
get_kanban_color() {
    case "$1" in
        "TEMPLATE") echo "${RED}🔴 TEMPLATE${NC}" ;;
        "UNMODIFIED") echo "${RED}🔴 UNMODIFIED${NC}" ;;
        "IN_PROGRESS") echo "${YELLOW}🟡 IN_PROGRESS${NC}" ;;
        "BLOCKED") echo "${YELLOW}🟡 BLOCKED${NC}" ;;
        "COMPLETE") echo "${GREEN}🟢 COMPLETE${NC}" ;;
        *) echo "${RED}🔴 $1${NC}" ;;
    esac
}

# Scan all task files dynamically
scan_tasks() {
    echo -e "${BLUE}🔍 Dynamic Task Scan${NC}"
    echo "==================="
    echo ""
    
    # Scan active sprints (exclude archive)
    for sprint_dir in "$TASK_DIR"/sprint-*/; do
        if [[ -d "$sprint_dir" ]]; then
            sprint_name=$(basename "$sprint_dir")
            if [[ "$sprint_name" != *"archive"* ]]; then
                echo -e "${BLUE}📋 $sprint_name${NC}"
                echo "$(echo "$sprint_name" | sed 's/./─/g')──"
                
                # Find all task files in this sprint
                task_found=false
                for task_file in "$sprint_dir"*.md; do
                    local filename=$(basename "$task_file")
                    # Exclude sprint plan, readme, test results, and template files
                    if [[ -f "$task_file" ]] && [[ "$filename" != "SPRINT_PLAN.md" ]] && [[ "$filename" != "README.md" ]] && [[ "$filename" != "TEST_RESULTS.md" ]] && [[ "$filename" != *"rename_me"* ]] && [[ "$filename" =~ ^[A-Z]+-[0-9]+ ]]; then
                        task_found=true
                        local description=$(get_task_description "$task_file")
                        local status=$(get_task_status "$task_file")
                        local priority=$(get_task_priority "$task_file")
                        
                        # Color code by priority and status
                        local priority_display=$(get_priority_color "$priority")
                        local status_display=$(get_status_color "$status")
                        
                        printf "  %-30s %s [%s]\n" "$description" "$status_display" "$priority_display"
                    fi
                done
                
                if [[ "$task_found" == false ]]; then
                    echo "  No tasks found"
                fi
                echo ""
            fi
        fi
    done
}

# Truncate text with ellipses if too long
truncate_text() {
    local text="$1"
    local max_length="$2"
    if [[ ${#text} -gt $max_length ]]; then
        echo "${text:0:$((max_length-3))}..."
    else
        echo "$text"
    fi
}

# Show visual kanban board
show_kanban() {
    echo -e "${BLUE}📋 Torq Sprint Kanban Board${NC}"
    echo "===================================="
    echo ""
    echo -e "Sprint status: ${RED}🔴 Not Started${NC} | ${YELLOW}🟡 In Progress${NC} | ${GREEN}🟢 Complete${NC}"
    echo ""
    
    # Header with status columns and proper grid structure
    printf "│ %-22s │ %12s │ %12s │ %12s │\n" "Sprint" "TODO" "IN_PROGRESS" "COMPLETE"
    printf "├────────────────────────┼──────────────┼──────────────┼──────────────┤\n"
    
    # Process each sprint
    for sprint_dir in "$TASK_DIR"/sprint-*/; do
        if [[ -d "$sprint_dir" ]]; then
            local sprint_name=$(basename "$sprint_dir")
            if [[ "$sprint_name" != *"archive"* ]]; then
                local short_name=$(echo "$sprint_name" | sed 's/sprint-//')
                
                # Calculate sprint status
                local todo_count=0
                local progress_count=0
                local complete_count=0
                local total_count=0
                
                for task_file in "$sprint_dir"*.md; do
                    local filename=$(basename "$task_file")
                    # Exclude sprint plan, readme, test results, and template files
                    if [[ -f "$task_file" ]] && [[ "$filename" != "SPRINT_PLAN.md" ]] && [[ "$filename" != "README.md" ]] && [[ "$filename" != "TEST_RESULTS.md" ]] && [[ "$filename" != *"rename_me"* ]] && [[ "$filename" =~ ^[A-Z]+-[0-9]+ ]]; then
                        local status=$(get_task_status "$task_file")
                        ((total_count++))
                        
                        case "$status" in
                            "TODO"|"UNKNOWN") ((todo_count++)) ;;
                            "IN_PROGRESS"|"BLOCKED") ((progress_count++)) ;;
                            "COMPLETE"|"DONE"|"ARCHIVED") ((complete_count++)) ;;
                        esac
                    fi
                done
                
                # Determine which column gets the sprint
                local todo_circle=""
                local progress_circle=""
                local complete_circle=""
                
                if [[ $complete_count -eq $total_count ]] && [[ $total_count -gt 0 ]]; then
                    # All tasks complete
                    complete_circle="🟢"
                elif [[ $complete_count -gt 0 ]] || [[ $progress_count -gt 0 ]]; then
                    # Mixed status: some complete/in-progress, some todo
                    progress_circle="🟡"
                else
                    # All tasks are TODO (not started)
                    todo_circle="🔴"
                fi
                
                # Show sprint row with proper grid structure and truncation
                local truncated_name=$(truncate_text "$short_name" 22)
                
                # Create centered cells (14 chars wide, emoji in center)
                local todo_cell="      $(printf '%-8s' "$todo_circle")"
                local progress_cell="      $(printf '%-8s' "$progress_circle")"
                local complete_cell="      $(printf '%-8s' "$complete_circle")"
                
                printf "│ %-22s │ %-12s │ %-12s │ %-12s │\n" \
                    "$truncated_name" "${todo_cell:0:12}" "${progress_cell:0:12}" "${complete_cell:0:12}"
            fi
        fi
    done
    
    # Bottom border of table
    printf "└────────────────────────┴──────────────┴──────────────┴──────────────┘\n"
    
    # Summary
    echo ""
    echo -e "${BLUE}📊 Sprint Summary${NC}"
    echo "──────────────────"
    
    local not_started=0
    local in_progress=0
    local completed=0
    
    for sprint_dir in "$TASK_DIR"/sprint-*/; do
        if [[ -d "$sprint_dir" ]] && [[ "$(basename "$sprint_dir")" != *"archive"* ]]; then
            local todo_count=0
            local progress_count=0
            local complete_count=0
            local total_count=0
            
            for task_file in "$sprint_dir"*.md; do
                if [[ -f "$task_file" ]] && [[ $(basename "$task_file") != "SPRINT_PLAN.md" ]] && [[ $(basename "$task_file") != "README.md" ]] && [[ $(basename "$task_file") != "TEST_RESULTS.md" ]] && [[ $(basename "$task_file") != *"rename_me"* ]]; then
                    local status=$(get_task_status "$task_file")
                    ((total_count++))
                    
                    case "$status" in
                        "TODO"|"UNKNOWN") ((todo_count++)) ;;
                        "IN_PROGRESS"|"BLOCKED") ((progress_count++)) ;;
                        "COMPLETE"|"DONE"|"ARCHIVED") ((complete_count++)) ;;
                    esac
                fi
            done
            
            # Categorize sprint
            if [[ $complete_count -eq $total_count ]] && [[ $total_count -gt 0 ]]; then
                ((completed++))
            elif [[ $complete_count -gt 0 ]] || [[ $progress_count -gt 0 ]]; then
                ((in_progress++))
            else
                ((not_started++))
            fi
        fi
    done
    
    echo -e "${RED}🔴 Not Started:${NC} $not_started sprints"
    echo -e "${YELLOW}🟡 In Progress:${NC} $in_progress sprints"  
    echo -e "${GREEN}🟢 Complete:${NC} $completed sprints"
    echo ""
    echo "Total active sprints: $((not_started + in_progress + completed))"
}

show_current_status() {
    echo -e "${BLUE}📊 Torq Current Sprint Status (Dynamic)${NC}"
    echo "=============================================="
    echo ""
    
    # Show critical tasks across all sprints
    echo -e "${RED}🚨 CRITICAL PRIORITY TASKS${NC}"
    echo "──────────────────────────"
    
    for sprint_dir in "$TASK_DIR"/sprint-*/; do
        if [[ -d "$sprint_dir" ]]; then
            sprint_name=$(basename "$sprint_dir")
            if [[ "$sprint_name" != *"archive"* ]]; then
                for task_file in "$sprint_dir"*.md; do
                    local filename=$(basename "$task_file")
                    # Exclude sprint plan, readme, test results, and template files
                    if [[ -f "$task_file" ]] && [[ "$filename" != "SPRINT_PLAN.md" ]] && [[ "$filename" != "README.md" ]] && [[ "$filename" != "TEST_RESULTS.md" ]] && [[ "$filename" != *"rename_me"* ]] && [[ "$filename" =~ ^[A-Z]+-[0-9]+ ]]; then
                        local priority=$(get_task_priority "$task_file")
                        local status=$(get_task_status "$task_file")
                        
                        if [[ "$priority" == *"CRITICAL"* ]] && [[ "$status" != *"DONE"* ]] && [[ "$status" != *"ARCHIVED"* ]] && [[ "$status" != *"COMPLETE"* ]]; then
                            local description=$(get_task_description "$task_file")
                            printf "  %-40s [%s] (%s)\n" "$description" "$status" "$sprint_name"
                        fi
                    fi
                done
            fi
        fi
    done
    
    echo ""
    echo -e "${GREEN}📈 System Info${NC}"
    echo "──────────────"
    echo -e "Current Branch: $(git branch --show-current)"
    echo -e "Last Commit:    $(git log --oneline -1)"
    echo -e "Task Directory: $(realpath "$TASK_DIR")"
}

show_next_task() {
    echo -e "${BLUE}🎯 JIT Task Queue (Ready to Start)${NC}"
    echo "========================================="
    echo ""
    echo -e "${CYAN}Tasks with all dependencies satisfied:${NC}"
    echo ""
    
    # Use Python YAML parser to find ready tasks
    local ready_tasks=$(python3 "$SCRIPT_DIR/yaml_parser.py" ready 2>/dev/null)
    
    if [[ -z "$ready_tasks" ]] || [[ "$ready_tasks" == "" ]]; then
        echo -e "${GREEN}✅ No tasks ready! Either all dependencies unsatisfied or all work in progress/complete.${NC}"
        echo ""
        echo "Run '$0 validate-plan' to check for dependency issues."
    else
        echo "$ready_tasks" | while IFS= read -r task; do
            if [[ -n "$task" ]]; then
                # Extract task ID and priority from output
                local task_id=$(echo "$task" | cut -d: -f1)
                local filename=$(echo "$task" | cut -d: -f2 | tr -d ' ')
                local priority=$(echo "$task" | grep -o '\[.*\]' | tr -d '[]')
                
                # Color code by priority
                case "$priority" in
                    "CRITICAL") echo -e "  ${RED}🔴 $task_id${NC} - $filename" ;;
                    "HIGH") echo -e "  ${YELLOW}🟡 $task_id${NC} - $filename" ;;
                    "MEDIUM") echo -e "  ${CYAN}🔵 $task_id${NC} - $filename" ;;
                    "LOW") echo -e "  ${GREEN}🟢 $task_id${NC} - $filename" ;;
                    *) echo -e "  $task" ;;
                esac
            fi
        done
        
        echo ""
        echo -e "${GREEN}💡 Pick any task above to start working!${NC}"
        echo -e "Remember to update status to IN_PROGRESS when you begin."
    fi
}

list_sprints() {
    echo -e "${BLUE}📚 Available Sprints${NC}"
    echo "==================="
    echo ""
    
    for sprint_dir in "$TASK_DIR"/*/; do
        if [[ -d "$sprint_dir" ]]; then
            sprint_name=$(basename "$sprint_dir")
            task_count=$(find "$sprint_dir" -name "*.md" -not -name "SPRINT_PLAN.md" -not -name "README.md" -not -name "TEST_RESULTS.md" -not -name "*rename_me*" | wc -l)
            
            if [[ "$sprint_name" == "archive" ]]; then
                echo -e "${PURPLE}📦 $sprint_name${NC} ($task_count archived sprints)"
            else
                echo -e "${CYAN}📋 $sprint_name${NC} ($task_count tasks)"
            fi
        fi
    done
}

# Check if all tasks in a sprint are complete
all_tasks_complete() {
    local sprint_dir="$1"
    local incomplete_count=0
    
    for task_file in "$sprint_dir"*.md; do
        local filename=$(basename "$task_file")
        if [[ -f "$task_file" ]] && [[ "$filename" != "SPRINT_PLAN.md" ]] && [[ "$filename" != "README.md" ]] && [[ "$filename" != "TEST_RESULTS.md" ]] && [[ "$filename" != *"rename_me"* ]] && [[ "$filename" =~ ^[A-Z]+-[0-9]+ ]]; then
            local status=$(get_task_status "$task_file")
            if [[ "$status" != "COMPLETE" ]] && [[ "$status" != "DONE" ]] && [[ "$status" != "ARCHIVED" ]]; then
                echo "  ⚠️  Task not complete: $(basename "$task_file" .md) [$status]"
                ((incomplete_count++))
            fi
        fi
    done
    
    if [[ $incomplete_count -eq 0 ]]; then
        return 0  # All complete
    else
        return 1  # Some incomplete
    fi
}

# Check if tests are passing for sprint
tests_passing() {
    local sprint_dir="$1"
    local test_results_file="$sprint_dir/TEST_RESULTS.md"
    
    if [[ -f "$test_results_file" ]]; then
        # Look for test status in file
        if grep -q "All tests.*passing\|✅.*[Tt]ests.*pass\|PASS\|SUCCESS" "$test_results_file"; then
            return 0  # Tests passing
        else
            echo "  ⚠️  Tests not verified as passing in TEST_RESULTS.md"
            return 1
        fi
    else
        # No test results file - check for test indicators in sprint plan
        if [[ -f "$sprint_dir/SPRINT_PLAN.md" ]]; then
            if grep -q "No tests required\|Testing not applicable" "$sprint_dir/SPRINT_PLAN.md"; then
                return 0  # No tests required
            fi
        fi
        echo "  ⚠️  No TEST_RESULTS.md found"
        return 1
    fi
}

# Check if PR is merged for sprint
pr_merged() {
    local sprint_name="$1"
    local sprint_number=$(echo "$sprint_name" | grep -o '[0-9]*')
    
    # Check various PR patterns in git log
    if git log --oneline --grep="sprint-${sprint_number}\|Sprint ${sprint_number}\|sprint_${sprint_number}" main 2>/dev/null | grep -q .; then
        return 0  # PR merged
    else
        # Also check if current branch has been merged
        local branch_patterns=("sprint-${sprint_number}" "sprint_${sprint_number}" "sprint${sprint_number}")
        for pattern in "${branch_patterns[@]}"; do
            if git branch --merged main 2>/dev/null | grep -q "$pattern"; then
                return 0  # Branch merged
            fi
        done
        
        echo "  ⚠️  No merged PR found for sprint $sprint_number"
        return 1
    fi
}

# Create archive summary
create_archive_summary() {
    local sprint_name="$1"
    local archive_dir="$TASK_DIR/archive/$sprint_name"
    local summary_file="$archive_dir/ARCHIVED.md"
    
    cat > "$summary_file" << EOF
# Archived Sprint: $sprint_name

**Archived Date**: $(date +"%Y-%m-%d")
**Archived By**: Automated archiving system

## Sprint Summary
This sprint has been automatically archived after meeting all completion criteria:
- ✅ All tasks marked as COMPLETE/DONE
- ✅ Tests verified as passing
- ✅ PR merged to main branch

## Completed Tasks
EOF
    
    # List all completed tasks
    for task_file in "$archive_dir"*.md; do
        if [[ -f "$task_file" ]] && [[ $(basename "$task_file") != "SPRINT_PLAN.md" ]] && [[ $(basename "$task_file") != "README.md" ]] && [[ $(basename "$task_file") != "ARCHIVED.md" ]]; then
            local description=$(get_task_description "$task_file")
            echo "- $description" >> "$summary_file"
        fi
    done
    
    echo "" >> "$summary_file"
    echo "## Archive Location" >> "$summary_file"
    echo "This sprint is archived at: \`$archive_dir\`" >> "$summary_file"
}

# Check if a sprint is ready for archiving
check_sprint_complete() {
    local sprint_name="$1"
    local sprint_dir="$TASK_DIR/$sprint_name/"
    
    if [[ ! -d "$sprint_dir" ]]; then
        echo -e "${RED}❌ Sprint directory not found: $sprint_name${NC}"
        return 1
    fi
    
    echo -e "${BLUE}🔍 Checking Sprint: $sprint_name${NC}"
    echo "======================================"
    echo ""
    
    local all_checks_pass=true
    
    # Check 1: All tasks complete
    echo "1️⃣  Checking task completion..."
    if all_tasks_complete "$sprint_dir"; then
        echo -e "   ${GREEN}✅ All tasks complete${NC}"
    else
        echo -e "   ${RED}❌ Some tasks incomplete${NC}"
        all_checks_pass=false
    fi
    echo ""
    
    # Check 2: Tests passing
    echo "2️⃣  Checking test results..."
    if tests_passing "$sprint_dir"; then
        echo -e "   ${GREEN}✅ Tests passing${NC}"
    else
        echo -e "   ${YELLOW}⚠️  Tests not verified${NC}"
        all_checks_pass=false
    fi
    echo ""
    
    # Check 3: PR merged
    echo "3️⃣  Checking PR status..."
    if pr_merged "$sprint_name"; then
        echo -e "   ${GREEN}✅ PR merged to main${NC}"
    else
        echo -e "   ${YELLOW}⚠️  PR not merged${NC}"
        all_checks_pass=false
    fi
    echo ""
    
    # Summary
    if [[ "$all_checks_pass" == true ]]; then
        echo -e "${GREEN}✅ Sprint $sprint_name is ready for archiving!${NC}"
        echo -e "   Run: $0 archive-sprint $sprint_name"
        return 0
    else
        echo -e "${YELLOW}⚠️  Sprint $sprint_name is not ready for archiving${NC}"
        echo -e "   Complete the above items before archiving"
        return 1
    fi
}

# Archive a completed sprint
archive_sprint() {
    local sprint_name="$1"
    local force_flag="$2"
    local sprint_dir="$TASK_DIR/$sprint_name/"
    local archive_dir="$TASK_DIR/archive/"
    
    
    if [[ ! -d "$sprint_dir" ]]; then
        echo -e "${RED}❌ Sprint directory not found: $sprint_name${NC}"
        return 1
    fi
    
    echo -e "${BLUE}📦 Archiving Sprint: $sprint_name${NC}"
    echo "======================================"
    
    # Verify sprint is ready
    if check_sprint_complete "$sprint_name" > /dev/null 2>&1; then
        echo -e "${GREEN}✅ Sprint passes all checks, proceeding with archive...${NC}"
    else
        echo ""
        echo -e "${YELLOW}⚠️  Running completion checks...${NC}"
        check_sprint_complete "$sprint_name" || true  # Don't exit on failure due to set -e
        echo ""
        
        if [[ "$force_flag" == "--force" ]]; then
            echo -e "${YELLOW}⚠️  Force flag detected, proceeding with archive despite incomplete checks...${NC}"
            echo ""
        else
            echo -e "${RED}❌ Sprint not ready for archiving. Use --force to override.${NC}"
            return 1
        fi
    fi
    
    # Create archive directory if needed
    mkdir -p "$archive_dir"
    
    # Move sprint to archive
    echo "Moving sprint to archive..."
    mv "$sprint_dir" "$archive_dir"
    
    # Create archive summary
    echo "Creating archive summary..."
    create_archive_summary "$sprint_name"
    
    echo ""
    echo -e "${GREEN}✅ Sprint $sprint_name successfully archived!${NC}"
    echo -e "   Location: $archive_dir$sprint_name/"
    
    return 0
}

# Auto-archive all completed sprints
auto_archive() {
    echo -e "${BLUE}🤖 Auto-Archive: Checking all sprints${NC}"
    echo "======================================="
    echo ""
    
    local archived_count=0
    local checked_count=0
    
    for sprint_dir in "$TASK_DIR"/sprint-*/; do
        if [[ -d "$sprint_dir" ]]; then
            sprint_name=$(basename "$sprint_dir")
            if [[ "$sprint_name" != *"archive"* ]]; then
                ((checked_count++))
                echo -e "${CYAN}Checking $sprint_name...${NC}"
                
                if check_sprint_complete "$sprint_name" > /dev/null 2>&1; then
                    echo -e "  ${GREEN}✅ Ready for archiving${NC}"
                    if archive_sprint "$sprint_name"; then
                        ((archived_count++))
                    fi
                else
                    echo -e "  ${YELLOW}⏳ Not ready yet${NC}"
                fi
                echo ""
            fi
        fi
    done
    
    echo ""
    echo -e "${BLUE}📊 Summary${NC}"
    echo "─────────"
    echo "Sprints checked: $checked_count"
    echo "Sprints archived: $archived_count"
    
    if [[ $archived_count -gt 0 ]]; then
        echo ""
        echo -e "${GREEN}✅ Successfully archived $archived_count sprint(s)${NC}"
    else
        echo ""
        echo -e "${YELLOW}ℹ️  No sprints ready for archiving${NC}"
    fi
}

# Extract dependencies from SPRINT_PLAN.md
get_sprint_dependencies() {
    local sprint_name="$1"
    local sprint_dir="$TASK_DIR/$sprint_name/"
    local plan_file="$sprint_dir/SPRINT_PLAN.md"
    
    if [[ ! -f "$plan_file" ]]; then
        echo "No SPRINT_PLAN.md found for $sprint_name"
        return 1
    fi
    
    # Extract dependencies section
    sed -n '/^### Sprint Dependencies/,/^###/p' "$plan_file" | grep -E "^- \[.\] Sprint" | sed 's/^- \[.\] //'
}

# Check if sprint dependencies are satisfied
check_sprint_dependencies() {
    local sprint_name="$1"
    local sprint_dir="$TASK_DIR/$sprint_name/"
    local plan_file="$sprint_dir/SPRINT_PLAN.md"
    
    echo -e "${BLUE}🔗 Dependency Check: $sprint_name${NC}"
    echo "====================================="
    
    if [[ ! -f "$plan_file" ]]; then
        echo -e "${YELLOW}⚠️  No SPRINT_PLAN.md found - cannot verify dependencies${NC}"
        return 1
    fi
    
    local has_blockers=false
    
    # Check "Depends On" requirements
    echo "📋 Checking Prerequisites..."
    local depends_on=$(sed -n '/^\*\*Depends On\*\*:/,/^\*\*Provides For\*\*:/p' "$plan_file" | grep -E "^- \[.\] Sprint" | head -20)
    
    if [[ -n "$depends_on" ]]; then
        while IFS= read -r line; do
            local dep_sprint=$(echo "$line" | sed 's/^- \[.\] Sprint \([^:]*\).*/sprint-\1/')
            local dep_status=""
            
            # Check if dependency sprint exists and is complete
            if [[ -d "$TASK_DIR/$dep_sprint" ]]; then
                # Check if all tasks in dependency sprint are complete
                local incomplete_tasks=0
                for task_file in "$TASK_DIR/$dep_sprint"*.md; do
                    if [[ -f "$task_file" ]] && [[ $(basename "$task_file") =~ ^[A-Z]+-[0-9]+ ]]; then
                        local status=$(get_task_status "$task_file")
                        if [[ "$status" != "COMPLETE" ]] && [[ "$status" != "DONE" ]]; then
                            ((incomplete_tasks++))
                        fi
                    fi
                done
                
                if [[ $incomplete_tasks -eq 0 ]]; then
                    echo -e "  ✅ $dep_sprint - Complete"
                else
                    echo -e "  ❌ $dep_sprint - $incomplete_tasks tasks incomplete"
                    has_blockers=true
                fi
            else
                echo -e "  ⚠️  $dep_sprint - Sprint not found"
            fi
        done <<< "$depends_on"
    else
        echo -e "  ℹ️  No sprint dependencies declared"
    fi
    
    echo ""
    
    # Check for conflicts
    echo "⚠️  Checking Conflicts..."
    local conflicts=$(sed -n '/^\*\*⚠️ Conflicts With\*\*:/,/^###/p' "$plan_file" | grep -E "^- Sprint" | head -10)
    
    if [[ -n "$conflicts" ]]; then
        while IFS= read -r line; do
            local conflict_sprint=$(echo "$line" | sed 's/^- Sprint \([0-9][0-9]*\).*/sprint-\1/')
            
            if [[ -d "$TASK_DIR/$conflict_sprint" ]]; then
                # Check if conflicting sprint is currently in progress
                local in_progress_tasks=0
                for task_file in "$TASK_DIR/$conflict_sprint"*.md; do
                    if [[ -f "$task_file" ]] && [[ $(basename "$task_file") =~ ^[A-Z]+-[0-9]+ ]]; then
                        local status=$(get_task_status "$task_file")
                        if [[ "$status" == "IN_PROGRESS" ]]; then
                            ((in_progress_tasks++))
                        fi
                    fi
                done
                
                if [[ $in_progress_tasks -gt 0 ]]; then
                    echo -e "  🚨 CONFLICT: $conflict_sprint has $in_progress_tasks tasks IN_PROGRESS"
                    has_blockers=true
                else
                    echo -e "  ✅ $conflict_sprint - Not currently active"
                fi
            fi
        done <<< "$conflicts"
    else
        echo -e "  ℹ️  No conflicts declared"
    fi
    
    echo ""
    
    # Summary
    if [[ "$has_blockers" == false ]]; then
        echo -e "${GREEN}✅ Sprint $sprint_name is ready to start!${NC}"
        echo -e "   All dependencies satisfied, no conflicts detected"
        return 0
    else
        echo -e "${RED}❌ Sprint $sprint_name has unresolved blockers${NC}"
        echo -e "   Resolve the above issues before starting"
        return 1
    fi
}

# Show dependency overview across all sprints
show_dependencies() {
    echo -e "${BLUE}🔗 Sprint Dependencies Overview${NC}"
    echo "================================="
    echo ""
    
    for sprint_dir in "$TASK_DIR"/sprint-*/; do
        if [[ -d "$sprint_dir" ]]; then
            local sprint_name=$(basename "$sprint_dir")
            if [[ "$sprint_name" != *"archive"* ]]; then
                local plan_file="$sprint_dir/SPRINT_PLAN.md"
                
                echo -e "${CYAN}📋 $sprint_name${NC}"
                echo "$(echo "$sprint_name" | sed 's/./─/g')──"
                
                if [[ -f "$plan_file" ]]; then
                    # Show dependencies
                    local depends_on=$(sed -n '/^\*\*Depends On\*\*:/,/^\*\*Provides For\*\*:/p' "$plan_file" | grep -E "^- \[.\] Sprint" | head -5)
                    if [[ -n "$depends_on" ]]; then
                        echo "  🔗 Depends on:"
                        echo "$depends_on" | sed 's/^- \[.\] /    • /'
                    fi
                    
                    # Show what it provides
                    local provides_for=$(sed -n '/^\*\*Provides For\*\*:/,/^###/p' "$plan_file" | grep -E "^- Sprint" | head -5)
                    if [[ -n "$provides_for" ]]; then
                        echo "  🎯 Enables:"
                        echo "$provides_for" | sed 's/^- /    • /'
                    fi
                    
                    # Show conflicts
                    local conflicts=$(sed -n '/^\*\*⚠️ Conflicts With\*\*:/,/^###/p' "$plan_file" | grep -E "^- Sprint" | head -3)
                    if [[ -n "$conflicts" ]]; then
                        echo -e "  ${RED}⚠️  Conflicts:${NC}"
                        echo "$conflicts" | sed 's/^- /    • /'
                    fi
                else
                    echo "  ⚠️  No SPRINT_PLAN.md found"
                fi
                echo ""
            fi
        fi
    done
}

# Main command handling
case "$1" in
    "status")
        show_current_status
        ;;
    "kanban")
        show_kanban
        ;;
    "next")
        show_next_task
        ;;
    "scan")
        scan_tasks
        ;;
    "list")
        scan_tasks
        ;;
    "sprints")
        list_sprints
        ;;
    "deps")
        show_dependencies
        ;;
    "check-deps")
        if [[ -z "$2" ]]; then
            echo -e "${RED}Error: Sprint name required${NC}"
            echo "Usage: $0 check-deps <sprint-name>"
            exit 1
        fi
        check_sprint_dependencies "$2"
        ;;
    "check-complete")
        if [[ -z "$2" ]]; then
            echo -e "${RED}Error: Sprint name required${NC}"
            echo "Usage: $0 check-complete <sprint-name>"
            exit 1
        fi
        check_sprint_complete "$2"
        ;;
    "archive-sprint")
        if [[ -z "$2" ]]; then
            echo -e "${RED}Error: Sprint name required${NC}"
            echo "Usage: $0 archive-sprint <sprint-name> [--force]"
            exit 1
        fi
        archive_sprint "$2" "$3"
        ;;
    "auto-archive")
        auto_archive
        ;;
    "validate-plan")
        echo -e "${BLUE}🔍 Validating Project Dependencies${NC}"
        echo "===================================="
        echo ""
        validation_result=$(python3 "$SCRIPT_DIR/yaml_parser.py" validate 2>/dev/null)
        if [[ $? -eq 0 ]]; then
            echo "$validation_result" | python3 -m json.tool
            
            # Check if valid
            if echo "$validation_result" | grep -q '"valid": true'; then
                echo ""
                echo -e "${GREEN}✅ All dependencies valid! No circular dependencies detected.${NC}"
            else
                echo ""
                echo -e "${RED}❌ Dependency issues detected! Review above for details.${NC}"
            fi
        else
            echo -e "${RED}Error running dependency validation${NC}"
        fi
        ;;
    "find-conflicts")
        if [[ -z "$2" ]]; then
            echo -e "${RED}Error: Task file required${NC}"
            echo "Usage: $0 find-conflicts <task-file>"
            exit 1
        fi
        echo -e "${BLUE}🔍 Checking Scope Conflicts${NC}"
        echo "============================"
        conflicts=$(python3 "$SCRIPT_DIR/yaml_parser.py" conflicts "$2" 2>/dev/null)
        if [[ -n "$conflicts" ]] && [[ "$conflicts" != "No conflicts detected" ]]; then
            echo -e "${RED}$conflicts${NC}"
        else
            echo -e "${GREEN}✅ No scope conflicts detected${NC}"
        fi
        ;;
    "graph")
        echo -e "${BLUE}🗺️ Generating Dependency Graph${NC}"
        echo "================================"
        echo ""
        
        # Use Python to generate dot file
        python3 "$SCRIPT_DIR/dependency_analyzer.py" graph > "$TASK_DIR/dependencies.dot" 2>/dev/null
        
        if [[ -f "$TASK_DIR/dependencies.dot" ]]; then
            # Generate PNG using Graphviz
            dot -Tpng "$TASK_DIR/dependencies.dot" -o "$TASK_DIR/dependencies.png" 2>/dev/null
            
            if [[ -f "$TASK_DIR/dependencies.png" ]]; then
                echo -e "${GREEN}✅ Dependency graph generated:${NC}"
                echo "  DOT file: $TASK_DIR/dependencies.dot"
                echo "  PNG file: $TASK_DIR/dependencies.png"
                echo ""
                echo "Open the PNG file to visualize task dependencies."
            else
                echo -e "${YELLOW}⚠️ Could not generate PNG. Is Graphviz installed?${NC}"
                echo "  DOT file available at: $TASK_DIR/dependencies.dot"
            fi
        else
            echo -e "${RED}❌ Failed to generate dependency graph${NC}"
            echo "Ensure dependency_analyzer.py exists and is executable."
        fi
        ;;
    "migrate-critical")
        echo -e "${BLUE}🔄 Migrating Critical Sprints${NC}"
        echo "=============================="
        echo ""
        python3 "$SCRIPT_DIR/migrate_tasks.py" critical
        echo ""
        echo -e "${GREEN}✅ Migration complete. Run '$0 validate-plan' to verify.${NC}"
        ;;
    "lint")
        if [[ -z "$2" ]]; then
            echo -e "${RED}Error: File or directory required${NC}"
            echo "Usage: $0 lint <file|directory>"
            exit 1
        fi
        echo -e "${BLUE}🔍 Linting Task Metadata${NC}"
        echo "========================"
        
        if [[ -f "$2" ]]; then
            # Single file
            python3 "$SCRIPT_DIR/task_linter.py" lint "$2"
        elif [[ -d "$2" ]]; then
            # Directory
            python3 "$SCRIPT_DIR/task_linter.py" lint-dir "$2"
        else
            echo -e "${RED}Error: '$2' is not a valid file or directory${NC}"
            exit 1
        fi
        ;;
    "lint-all")
        echo -e "${BLUE}🔍 Linting All Task Files${NC}"
        echo "=========================="
        python3 "$SCRIPT_DIR/task_linter.py" lint-dir "$TASK_DIR"
        if [[ $? -eq 0 ]]; then
            echo ""
            echo -e "${GREEN}✅ All task files are valid!${NC}"
        else
            echo ""
            echo -e "${RED}❌ Some task files have errors. Fix them before committing.${NC}"
            exit 1
        fi
        ;;
    "health")
        echo -e "${BLUE}📋 Task Metadata Health Report${NC}"
        echo "==============================="
        python3 "$SCRIPT_DIR/task_linter.py" report
        ;;
    "help"|"--help"|"-h")
        show_usage
        ;;
    sprint-*)
        # Delegate to sprint-status.sh for sprint-specific commands
        if [[ -x "$SCRIPT_DIR/sprint-status.sh" ]]; then
            "$SCRIPT_DIR/sprint-status.sh" "$1"
        else
            echo -e "${RED}Error: sprint-status.sh not found${NC}"
            exit 1
        fi
        ;;
    *)
        echo -e "${RED}Error: Unknown command '$1'${NC}"
        echo ""
        show_usage
        exit 1
        ;;
esac