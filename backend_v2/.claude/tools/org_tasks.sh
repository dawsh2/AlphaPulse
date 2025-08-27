#!/bin/bash
# AlphaPulse Org-mode Task Management CLI
# A wrapper around Emacs batch mode for org task management

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ORG_MANAGER_EL="$SCRIPT_DIR/org_task_manager.el"
TASKS_DIR="$SCRIPT_DIR/../tasks"
ACTIVE_FILE="$TASKS_DIR/active.org"

# Ensure Emacs is available
if ! command -v emacs &> /dev/null; then
    echo "Error: Emacs is required but not installed" >&2
    exit 1
fi

# Create tasks directory if it doesn't exist
mkdir -p "$TASKS_DIR"

# Initialize active.org if it doesn't exist
if [ ! -f "$ACTIVE_FILE" ]; then
    cat > "$ACTIVE_FILE" <<'EOF'
#+TITLE: AlphaPulse Active Tasks
#+TODO: TODO NEXT IN-PROGRESS WAITING | DONE CANCELLED
#+STARTUP: overview
#+STARTUP: hidestars
#+STARTUP: logdone

EOF
    echo "Created $ACTIVE_FILE"
fi

# Function to run Emacs in batch mode
run_emacs() {
    local command=$1
    shift
    local args="\"$command\" \"$ACTIVE_FILE\""
    while [[ $# -gt 0 ]]; do
        args="$args \"$1\""
        shift
    done
    emacs --batch \
          --load "$ORG_MANAGER_EL" \
          --eval "(setq alphapulse/command-args (list $args))" \
          --eval "(alphapulse/cli-main)"
}

# Main command handler
case "$1" in
    parse|list)
        run_emacs parse | python3 -m json.tool
        ;;
        
    ready|next)
        echo "Getting ready tasks..."
        run_emacs ready | python3 -m json.tool
        ;;
        
    update)
        if [ $# -lt 3 ]; then
            echo "Usage: $0 update <task-id> <new-state>"
            echo "States: TODO, NEXT, IN-PROGRESS, WAITING, DONE, CANCELLED"
            exit 1
        fi
        echo "Updating task $2 to $3..."
        run_emacs update "$2" "$3"
        ;;
        
    add)
        if [ $# -lt 2 ]; then
            echo "Usage: $0 add <heading> [state] [priority] [tags] [properties-json] [body] [parent-id]"
            exit 1
        fi
        echo "Adding new task: $2"
        shift
        run_emacs add "$@"
        ;;
        
    help|--help|-h)
        cat <<EOF
AlphaPulse Org-mode Task Management CLI

Usage: $0 <command> [arguments]

Commands:
    parse, list     Parse and list all tasks as JSON
    ready, next     Get tasks ready for execution
    update          Update task state
    add             Add a new task
    help            Show this help message

Examples:
    $0 list
    $0 ready
    $0 update TASK-001 DONE
    $0 add "Implement new feature" TODO A "feature:critical"

Files:
    Active tasks: $ACTIVE_FILE
    Elisp code:   $ORG_MANAGER_EL
EOF
        ;;
        
    *)
        echo "Unknown command: $1"
        echo "Run '$0 help' for usage information"
        exit 1
        ;;
esac