#!/bin/bash
# Claude Code Hook: Enforce Task Status Updates
# Triggers after any file edit to remind about status management

set -e

HOOK_NAME="post_file_edit"
EDITED_FILE="$1"

# Only check task-related files
if [[ "$EDITED_FILE" =~ \.org$ ]] || [[ "$EDITED_FILE" =~ tasks.*\.md$ ]]; then
    echo "🔍 HOOK: $HOOK_NAME - Checking task status updates..."
    
    # Check if this looks like a task file being worked on
    if grep -q "IN_PROGRESS\|TODO\|COMPLETE" "$EDITED_FILE" 2>/dev/null; then
        echo ""
        echo "📋 TASK STATUS REMINDER:"
        echo "   • Starting work? Update status: TODO → IN_PROGRESS"
        echo "   • Finishing work? Update status: IN_PROGRESS → COMPLETE"
        echo "   • Read: @.claude/docs/TASK_EXECUTION_STANDARDS.md"
        echo ""
        
        # Check for common violations
        if grep -q "status.*TODO" "$EDITED_FILE" && grep -q "implementation" "$EDITED_FILE"; then
            echo "⚠️  WARNING: Task shows TODO but contains implementation notes"
            echo "   → Did you forget to mark it IN_PROGRESS?"
        fi
        
        if grep -q "All tests pass" "$EDITED_FILE" && grep -q "IN_PROGRESS" "$EDITED_FILE"; then
            echo "⚠️  WARNING: Tests passing but status still IN_PROGRESS"
            echo "   → Ready to mark COMPLETE?"
        fi
    fi
fi

# Always succeed - this is informational only
exit 0