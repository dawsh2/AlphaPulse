#!/usr/bin/env python3
"""
AI Agent Task Management Commands

Integration layer between AI agents and org-mode task management system.
Provides high-level commands for dynamic task management.
"""

import json
import subprocess
import sys
from pathlib import Path
from typing import Dict, List, Optional, Set
from dataclasses import dataclass

# Default task file location
DEFAULT_TASK_FILE = Path(__file__).parent.parent / "tasks" / "active.org"

@dataclass
class TaskCommand:
    """Result of a task management command"""
    success: bool
    message: str
    data: Optional[Dict] = None
    tasks: Optional[List[Dict]] = None

class AgentTaskManager:
    """AI Agent interface to org-mode task management"""
    
    def __init__(self, task_file: Optional[Path] = None):
        self.task_file = task_file or DEFAULT_TASK_FILE
        self.tools_dir = Path(__file__).parent
        self.org_script = self.tools_dir / "org_tasks.sh"
        self.priority_extractor = self.tools_dir / "simple_priority_demo.py"
    
    def _run_org_command(self, command: str, *args) -> TaskCommand:
        """Run org_tasks.sh command and return result"""
        try:
            cmd = [str(self.org_script), command] + list(args)
            result = subprocess.run(cmd, capture_output=True, text=True, timeout=10)
            
            if result.returncode != 0:
                return TaskCommand(False, f"Command failed: {result.stderr.strip()}")
            
            # Try to parse JSON output
            try:
                data = json.loads(result.stdout)
                return TaskCommand(True, f"{command} completed successfully", data)
            except json.JSONDecodeError:
                # Non-JSON output (like update confirmations)
                return TaskCommand(True, result.stdout.strip())
        
        except subprocess.TimeoutExpired:
            return TaskCommand(False, "Command timed out")
        except Exception as e:
            return TaskCommand(False, f"Command error: {str(e)}")
    
    def next_tasks(self, limit: int = 5) -> TaskCommand:
        """Get next tasks ready for execution"""
        result = self._run_org_command("ready")
        
        if not result.success or not result.data:
            return result
        
        ready_tasks = result.data.get('ready_tasks', [])
        limited_tasks = ready_tasks[:limit]
        
        return TaskCommand(
            True, 
            f"Found {len(ready_tasks)} ready tasks, showing {len(limited_tasks)}",
            {"ready_tasks": limited_tasks, "total_ready": len(ready_tasks)},
            limited_tasks
        )
    
    def tasks_for_goal(self, goal_id: str) -> TaskCommand:
        """Get all tasks needed to complete a specific goal"""
        # First, get all tasks
        result = self._run_org_command("parse")
        
        if not result.success or not result.data:
            return result
        
        all_tasks = {t['id']: t for t in result.data.get('tasks', [])}
        
        # Find the goal
        goal = all_tasks.get(goal_id)
        if not goal:
            return TaskCommand(False, f"Goal {goal_id} not found")
        
        if not goal.get('is_goal'):
            return TaskCommand(False, f"{goal_id} is not a goal")
        
        # Extract dependency tree (simplified logic for demo)
        required_tasks = []
        for task in all_tasks.values():
            # Tasks belong to goal if ID starts with goal prefix
            goal_prefix = goal_id.split('-')[0]  # AUTH-GOAL -> AUTH
            if (task['id'].startswith(goal_prefix) and 
                task.get('is_actionable')):
                required_tasks.append(task)
        
        # Find ready tasks from this set
        ready_tasks = []
        for task in required_tasks:
            if task.get('state') in ['TODO', 'NEXT']:
                # Check dependencies (simplified)
                depends = task.get('properties', {}).get('DEPENDS', '')
                if not depends:  # No dependencies = ready
                    ready_tasks.append(task)
        
        return TaskCommand(
            True,
            f"Goal {goal['heading']} requires {len(required_tasks)} tasks, {len(ready_tasks)} ready",
            {
                "goal": goal,
                "required_tasks": required_tasks,
                "ready_tasks": ready_tasks,
                "total_required": len(required_tasks),
                "total_ready": len(ready_tasks)
            },
            required_tasks
        )
    
    def priority_work_plan(self, priority: str) -> TaskCommand:
        """Get complete work plan for specific priority"""
        result = self._run_org_command("parse")
        
        if not result.success or not result.data:
            return result
        
        all_tasks = result.data.get('tasks', [])
        
        # Find priority goals
        priority_goals = [t for t in all_tasks if t.get('is_goal') and t.get('priority') == priority]
        
        if not priority_goals:
            return TaskCommand(False, f"No Priority {priority} goals found")
        
        # Extract all required tasks for these goals
        all_required = []
        ready_now = []
        
        for goal in priority_goals:
            goal_prefix = goal['id'].split('-')[0]
            for task in all_tasks:
                if (task['id'].startswith(goal_prefix) and 
                    task.get('is_actionable')):
                    all_required.append(task)
                    
                    # Check if ready
                    if (task.get('state') in ['TODO', 'NEXT'] and
                        not task.get('properties', {}).get('DEPENDS', '')):
                        ready_now.append(task)
        
        total_effort = sum(
            int(t.get('properties', {}).get('EFFORT', '0h').rstrip('h') or 0)
            for t in all_required
        )
        
        return TaskCommand(
            True,
            f"Priority {priority}: {len(priority_goals)} goals, {len(all_required)} tasks, {len(ready_now)} ready",
            {
                "priority": priority,
                "goals": priority_goals,
                "required_tasks": all_required,
                "ready_tasks": ready_now,
                "total_effort_hours": total_effort,
                "can_parallelize": len(ready_now)
            },
            all_required
        )
    
    def create_task(self, heading: str, priority: str = "B", 
                   dependencies: Optional[List[str]] = None) -> TaskCommand:
        """Create a new task"""
        result = self._run_org_command("add", heading, "TODO", priority)
        return result
    
    def update_task_status(self, task_id: str, new_state: str) -> TaskCommand:
        """Update task status"""
        valid_states = ["TODO", "NEXT", "IN-PROGRESS", "DONE", "CANCELLED"]
        if new_state not in valid_states:
            return TaskCommand(False, f"Invalid state. Must be one of: {', '.join(valid_states)}")
        
        result = self._run_org_command("update", task_id, new_state)
        return result
    
    def get_task_status(self) -> TaskCommand:
        """Get overall task status summary"""
        result = self._run_org_command("parse")
        
        if not result.success or not result.data:
            return result
        
        metadata = result.data.get('metadata', {})
        tasks = result.data.get('tasks', [])
        
        # Calculate priority breakdown
        by_priority = {"A": 0, "B": 0, "C": 0, "None": 0}
        actionable_tasks = []
        
        for task in tasks:
            if task.get('is_actionable'):
                actionable_tasks.append(task)
                priority = task.get('priority', 'None')
                if priority in by_priority:
                    by_priority[priority] += 1
                else:
                    by_priority['None'] += 1
        
        ready_result = self.next_tasks(100)  # Get all ready tasks
        ready_count = len(ready_result.tasks) if ready_result.tasks else 0
        
        summary = {
            "total_tasks": metadata.get('total_tasks', 0),
            "todo_count": metadata.get('todo_count', 0),
            "in_progress_count": metadata.get('in_progress_count', 0),
            "done_count": metadata.get('done_count', 0),
            "ready_count": ready_count,
            "actionable_tasks": len(actionable_tasks),
            "priority_breakdown": by_priority,
            "can_parallelize": ready_count
        }
        
        message = f"Status: {summary['todo_count']} TODO, {summary['ready_count']} ready, {summary['can_parallelize']} can start now"
        
        return TaskCommand(True, message, summary)

def main():
    """CLI interface for agent task commands"""
    if len(sys.argv) < 2:
        print("Usage: python3 agent_task_commands.py <command> [args...]")
        print("Commands:")
        print("  next [limit]          - Get next ready tasks")
        print("  goal <goal-id>        - Get tasks for specific goal") 
        print("  priority <A|B|C>      - Get priority work plan")
        print("  create <heading>      - Create new task")
        print("  update <id> <state>   - Update task status")
        print("  status                - Get overall status")
        sys.exit(1)
    
    manager = AgentTaskManager()
    command = sys.argv[1].lower()
    
    if command == "next":
        limit = int(sys.argv[2]) if len(sys.argv) > 2 else 5
        result = manager.next_tasks(limit)
    
    elif command == "goal":
        if len(sys.argv) < 3:
            print("Error: goal command requires goal ID")
            sys.exit(1)
        result = manager.tasks_for_goal(sys.argv[2])
    
    elif command == "priority":
        if len(sys.argv) < 3:
            print("Error: priority command requires priority (A, B, or C)")
            sys.exit(1)
        result = manager.priority_work_plan(sys.argv[2].upper())
    
    elif command == "create":
        if len(sys.argv) < 3:
            print("Error: create command requires task heading")
            sys.exit(1)
        result = manager.create_task(sys.argv[2])
    
    elif command == "update":
        if len(sys.argv) < 4:
            print("Error: update command requires task ID and new state")
            sys.exit(1)
        result = manager.update_task_status(sys.argv[2], sys.argv[3])
    
    elif command == "status":
        result = manager.get_task_status()
    
    else:
        print(f"Unknown command: {command}")
        sys.exit(1)
    
    # Display result
    if result.success:
        print(f"✅ {result.message}")
        if result.data:
            print(json.dumps(result.data, indent=2))
    else:
        print(f"❌ {result.message}")
        sys.exit(1)

if __name__ == '__main__':
    main()