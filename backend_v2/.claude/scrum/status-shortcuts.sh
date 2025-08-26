#!/bin/bash
# Status Check Shortcuts for Sprint Management
# Source this file to add aliases to your shell

# Quick status checking
alias sprint-status='cd $PWD/.claude && scrum/task-manager.sh status'
alias sprint-kanban='cd $PWD/.claude && scrum/task-manager.sh kanban'
alias sprint-next='cd $PWD/.claude && scrum/task-manager.sh next'

# Status reminder commands
alias mark-done='echo "🚨 REMINDER: Update task status in markdown file YAML:"
echo "   status: TODO → status: IN_PROGRESS → status: COMPLETE"
echo "   Then run: sprint-status to verify"'

alias status-help='echo "📋 Sprint Status Commands:"
echo "   sprint-status   - Show all sprint progress"  
echo "   sprint-kanban   - Show visual kanban board"
echo "   sprint-next     - Get next priority task"
echo "   mark-done       - Show status update reminder"
echo ""
echo "📝 Task Status Flow:"
echo "   1. Pick task from sprint-next"
echo "   2. Change status: TODO → IN_PROGRESS"  
echo "   3. Do the work"
echo "   4. Change status: IN_PROGRESS → COMPLETE"
echo "   5. Run sprint-status to verify"'

# Sprint-specific shortcuts (customize these)
sprint-007() {
    cd $PWD/.claude && scrum/task-manager.sh sprint-007-generic-relay-refactor
}

echo "✅ Sprint status shortcuts loaded!"
echo "   Run 'status-help' for available commands"
echo "   Run 'mark-done' for status update reminder"