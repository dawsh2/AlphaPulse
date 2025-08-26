#!/bin/bash
# CI/CD Hook for Sprint Archiving
# Can be called from GitHub Actions after PR merge

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TASK_MANAGER="$SCRIPT_DIR/task-manager.sh"

# This can be called with:
# 1. No arguments - checks all sprints
# 2. Sprint name - checks specific sprint  
# 3. PR title/body containing sprint info

if [[ -n "$1" ]]; then
    # If argument provided, try to extract sprint info
    SPRINT_INPUT="$1"
    
    # Try to extract sprint number from input
    if echo "$SPRINT_INPUT" | grep -qE 'sprint[_-]?[0-9]+'; then
        SPRINT_NUMBER=$(echo "$SPRINT_INPUT" | grep -oE 'sprint[_-]?0*([0-9]+)' | grep -oE '[0-9]+' | tail -1)
        SPRINT_NAME=$(printf "sprint-%03d" "$SPRINT_NUMBER")
        
        echo "🎯 Checking specific sprint: $SPRINT_NAME"
        "$TASK_MANAGER" check-complete "$SPRINT_NAME"
        
        if [[ $? -eq 0 ]]; then
            echo "📦 Archiving $SPRINT_NAME..."
            "$TASK_MANAGER" archive-sprint "$SPRINT_NAME"
        fi
    else
        echo "⚠️  Could not extract sprint number from: $SPRINT_INPUT"
        echo "Running auto-archive for all sprints..."
        "$TASK_MANAGER" auto-archive
    fi
else
    # No arguments - check all sprints
    echo "🤖 Running auto-archive for all sprints..."
    "$TASK_MANAGER" auto-archive
fi