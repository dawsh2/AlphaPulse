#!/bin/bash

# Quick status check for Sprint 016: Enhanced Development Workflow and Tooling Infrastructure

echo "🚀 Sprint 016: Enhanced Development Workflow and Tooling Infrastructure"
echo "================================================================"
echo ""

SPRINT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TASK_MANAGER="$SPRINT_DIR/../../scrum/task-manager.sh"

if [[ -f "$TASK_MANAGER" ]]; then
    echo "📊 Current Sprint Status:"
    "$TASK_MANAGER" status sprint-016-workflow-tooling
    echo ""
    
    echo "📋 Task Summary:"
    "$TASK_MANAGER" summary sprint-016-workflow-tooling
    echo ""
    
    echo "⚡ Next Available Tasks:"
    "$TASK_MANAGER" next sprint-016-workflow-tooling
else
    echo "⚠️  Task manager not found at: $TASK_MANAGER"
    echo "Using basic status check..."
    echo ""
    
    # Basic status check if task manager not available
    for task_file in "$SPRINT_DIR"/TOOL-*.md; do
        if [[ -f "$task_file" ]]; then
            task_id=$(basename "$task_file" .md)
            status=$(grep -E "^status:" "$task_file" | head -1 | sed 's/status: *//' | sed 's/ .*//')
            priority=$(grep -E "^priority:" "$task_file" | head -1 | sed 's/priority: *//')
            hours=$(grep -E "^estimated_hours:" "$task_file" | head -1 | sed 's/estimated_hours: *//')
            
            case $status in
                "TODO") status_icon="⏳" ;;
                "IN_PROGRESS") status_icon="🔄" ;;
                "COMPLETE") status_icon="✅" ;;
                *) status_icon="❓" ;;
            esac
            
            printf "%-8s %s %-12s [%s] %sh\n" "$task_id" "$status_icon" "$status" "$priority" "$hours"
        fi
    done
fi

echo ""
echo "🔧 Key Tools to be Implemented:"
echo "  • cargo-deny    - Dependency security & license policies"
echo "  • cargo-udeps   - Unused dependency detection"
echo "  • cargo-sort    - Consistent Cargo.toml formatting"
echo "  • Pattern enforcement for Torq-specific violations"
echo "  • Enhanced documentation organization"
echo ""
echo "💡 To start working:"
echo "  1. Pick a TODO task with no dependencies"
echo "  2. Change status to IN_PROGRESS in task file"
echo "  3. Create worktree as specified in task"
echo "  4. Begin implementation"
echo ""
echo "📖 View full sprint plan: cat SPRINT_PLAN.md"