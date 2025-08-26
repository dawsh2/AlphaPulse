#!/bin/bash
# Quick status check for this sprint

SPRINT_DIR=$(dirname "$0")
echo "📊 Sprint Status Check"
echo "====================="
echo ""

# Count task statuses
TODO_COUNT=$(grep -l "Status: TODO\|status: TODO" "$SPRINT_DIR"/TASK-*.md 2>/dev/null | wc -l)
IN_PROGRESS_COUNT=$(grep -l "Status: IN_PROGRESS\|status: IN_PROGRESS" "$SPRINT_DIR"/TASK-*.md 2>/dev/null | wc -l)
COMPLETE_COUNT=$(grep -l "Status: COMPLETE\|status: COMPLETE" "$SPRINT_DIR"/TASK-*.md 2>/dev/null | wc -l)

echo "📋 Task Summary:"
echo "  TODO:        $TODO_COUNT"
echo "  IN_PROGRESS: $IN_PROGRESS_COUNT"
echo "  COMPLETE:    $COMPLETE_COUNT"
echo ""

# Check for test results
if [ -f "$SPRINT_DIR/TEST_RESULTS.md" ]; then
    if grep -q "All tests passing" "$SPRINT_DIR/TEST_RESULTS.md"; then
        echo "✅ Tests: PASSING"
    else
        echo "❌ Tests: FAILING or INCOMPLETE"
    fi
else
    echo "⚠️  Tests: NOT RUN (no TEST_RESULTS.md)"
fi

echo ""
# Check git branch
CURRENT_BRANCH=$(git branch --show-current)
if [ "$CURRENT_BRANCH" = "main" ]; then
    echo "🔴 WARNING: You are on main branch!"
else
    echo "✅ Branch: $CURRENT_BRANCH"
fi
