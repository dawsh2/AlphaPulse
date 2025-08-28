#!/bin/bash
# Claude Code Hook: Validate Task Completion Before Git Commits
# Ensures agents don't commit implementation without updating task status

set -e

HOOK_NAME="pre_git_commit"

echo "🔍 HOOK: $HOOK_NAME - Validating task completion status..."

# Check if there are any IN_PROGRESS tasks that might be complete
ACTIVE_TASKS_FILE=".claude/tasks/active.org"
if [[ -f "$ACTIVE_TASKS_FILE" ]]; then
    IN_PROGRESS_TASKS=$(grep -n "IN_PROGRESS" "$ACTIVE_TASKS_FILE" | wc -l || echo "0")
    
    if [[ $IN_PROGRESS_TASKS -gt 0 ]]; then
        echo ""
        echo "📋 TASK STATUS CHECK:"
        echo "   Found $IN_PROGRESS_TASKS task(s) marked IN_PROGRESS"
        echo ""
        echo "🚨 BEFORE COMMITTING - Ask yourself:"
        echo "   • Did I complete any IN_PROGRESS tasks?"
        echo "   • Should any be marked COMPLETE?"
        echo "   • Are status updates accurate in active.org?"
        echo ""
        echo "💡 REMINDER: Update task status BEFORE committing code!"
        echo "   → Edit .claude/tasks/active.org"
        echo "   → Change: IN_PROGRESS → COMPLETE"
        echo ""
        
        # Show current IN_PROGRESS tasks
        echo "Current IN_PROGRESS tasks:"
        grep -A 2 -B 1 "IN_PROGRESS" "$ACTIVE_TASKS_FILE" | head -20
        echo ""
    fi
fi

# Check for TDD violations - implementation files without corresponding tests
STAGED_FILES=$(git diff --cached --name-only --diff-filter=AM 2>/dev/null || echo "")
if [[ -n "$STAGED_FILES" ]]; then
    IMPL_WITHOUT_TESTS=""
    
    while IFS= read -r file; do
        if [[ "$file" =~ \.(rs|py)$ ]] && [[ ! "$file" =~ test ]]; then
            # This is an implementation file - check if corresponding test exists
            TEST_PATTERN=$(echo "$file" | sed 's/src\//tests\//' | sed 's/\.rs$/_test.rs/')
            if [[ ! -f "$TEST_PATTERN" ]] && [[ ! "$file" =~ _test\.rs$ ]]; then
                IMPL_WITHOUT_TESTS="$IMPL_WITHOUT_TESTS\n  - $file"
            fi
        fi
    done <<< "$STAGED_FILES"
    
    if [[ -n "$IMPL_WITHOUT_TESTS" ]]; then
        echo "⚠️  TDD COMPLIANCE WARNING:"
        echo "   Implementation files without corresponding tests:"
        echo -e "$IMPL_WITHOUT_TESTS"
        echo ""
        echo "💡 TDD REMINDER: Write tests BEFORE implementation!"
    fi
fi

# Success - hook is informational, not blocking
echo "✅ Task validation complete"
exit 0