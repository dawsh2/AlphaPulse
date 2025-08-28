#!/bin/bash
# Claude Code Hook: Remind About Status Updates When Starting Tasks
# Provides clear guidance on status management workflow

set -e

HOOK_NAME="post_task_start"

echo "🚨 CRITICAL REMINDER: Task Status Management"
echo ""
echo "📋 MANDATORY WORKFLOW:"
echo "   1. STARTING WORK:"
echo "      → Edit .claude/tasks/active.org"
echo "      → Find your task"
echo "      → Change: TODO → IN_PROGRESS"
echo ""
echo "   2. WHILE WORKING:"
echo "      → Keep status as IN_PROGRESS"
echo "      → Add notes to task if needed"
echo ""
echo "   3. COMPLETING WORK:"
echo "      → All tests passing?"
echo "      → All acceptance criteria met?"
echo "      → Change: IN_PROGRESS → COMPLETE"
echo ""
echo "🎯 WHY THIS MATTERS:"
echo "   • Prevents multiple agents working on same task"
echo "   • Enables accurate progress tracking"
echo "   • Unblocks dependent tasks when completed"
echo "   • Maintains system integrity"
echo ""
echo "📖 DETAILED STANDARDS:"
echo "   → Read: @.claude/docs/TASK_EXECUTION_STANDARDS.md"
echo ""
echo "🔧 QUICK COMMANDS:"
echo "   ./org_tasks.sh update TASK-ID IN_PROGRESS"
echo "   ./org_tasks.sh update TASK-ID COMPLETE"
echo ""

# Always succeed
exit 0