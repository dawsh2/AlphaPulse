#!/usr/bin/env python3
"""
Priority-based Task Dependency Extractor

Extract all tasks required to complete Priority A goals, demonstrating
the power of DAG-based task management for dynamic work planning.
"""

import json
import sys
import subprocess
from pathlib import Path
from typing import Dict, List, Set, Optional
from collections import defaultdict, deque

def parse_org_file(org_file_path: str) -> List[Dict]:
    """Parse org file using our existing Emacs parser"""
    tools_dir = Path(__file__).parent
    emacs_script = tools_dir / "org_task_manager.el"
    
    try:
        result = subprocess.run([
            "emacs", "--batch",
            "--load", str(emacs_script),
            "--eval", f'(setq torq/command-args (list "parse" "{org_file_path}"))',
            "--eval", "(torq/cli-main)"
        ], capture_output=True, text=True, timeout=10)
        
        if result.returncode != 0:
            print(f"Error parsing org file: {result.stderr}", file=sys.stderr)
            return []
        
        data = json.loads(result.stdout)
        return data.get('tasks', [])
    
    except (json.JSONDecodeError, subprocess.TimeoutExpired, FileNotFoundError) as e:
        print(f"Error: {e}", file=sys.stderr)
        return []

def extract_dependencies(task_properties: Dict) -> List[str]:
    """Extract dependency IDs from task properties"""
    depends_str = task_properties.get('DEPENDS', '')
    if not depends_str:
        return []
    return [dep.strip() for dep in depends_str.split() if dep.strip()]

def build_dependency_graph(tasks: List[Dict]) -> Dict[str, Set[str]]:
    """Build forward dependency graph: task_id -> set of required task_ids"""
    deps = defaultdict(set)
    
    for task in tasks:
        task_id = task.get('id')
        if not task_id:
            continue
            
        # Get dependencies from properties
        task_deps = extract_dependencies(task.get('properties', {}))
        deps[task_id] = set(task_deps)
    
    return dict(deps)

def extract_dependency_tree(task_id: str, dependencies: Dict[str, Set[str]], all_tasks: Dict[str, Dict]) -> Set[str]:
    """Extract all tasks needed to complete the target task (recursive DFS)"""
    if task_id not in all_tasks:
        return set()
    
    required_tasks = set()
    visited = set()
    
    def dfs(current_id: str):
        if current_id in visited or current_id not in all_tasks:
            return
        
        visited.add(current_id)
        required_tasks.add(current_id)
        
        # Recursively add all dependencies
        for dep_id in dependencies.get(current_id, set()):
            dfs(dep_id)
    
    dfs(task_id)
    return required_tasks

def get_priority_goals(tasks: List[Dict], priority: str) -> List[Dict]:
    """Get all goals with specific priority"""
    return [
        task for task in tasks
        if (task.get('is_goal') and 
            task.get('priority') == priority)
    ]

def get_ready_tasks(task_ids: Set[str], all_tasks: Dict[str, Dict], dependencies: Dict[str, Set[str]]) -> List[Dict]:
    """Get tasks that are ready to execute (all dependencies complete)"""
    ready = []
    
    for task_id in task_ids:
        if task_id not in all_tasks:
            continue
            
        task = all_tasks[task_id]
        
        # Must be actionable
        if not task.get('is_actionable'):
            continue
            
        # Must be TODO or NEXT
        if task.get('state') not in ['TODO', 'NEXT']:
            continue
        
        # All dependencies must be DONE
        task_deps = dependencies.get(task_id, set())
        deps_complete = all(
            dep_id not in all_tasks or all_tasks[dep_id].get('state') == 'DONE'
            for dep_id in task_deps
        )
        
        if deps_complete:
            ready.append(task)
    
    # Sort by priority (A > B > C) then by ID
    return sorted(ready, key=lambda t: (t.get('priority', 'Z'), t.get('id', '')))

def analyze_priority_work_plan(org_file_path: str, priority: str) -> Dict:
    """Analyze complete work plan needed for goals of given priority"""
    
    print(f"🔍 Parsing {org_file_path}...")
    tasks = parse_org_file(org_file_path)
    
    if not tasks:
        return {"error": "Failed to parse org file or no tasks found"}
    
    # Build lookup tables
    all_tasks = {task['id']: task for task in tasks if task.get('id')}
    dependencies = build_dependency_graph(tasks)
    
    # Find priority goals
    priority_goals = get_priority_goals(tasks, priority)
    
    if not priority_goals:
        return {"error": f"No goals found with priority {priority}"}
    
    # Extract dependency trees for each priority goal
    all_required_task_ids = set()
    goal_trees = {}
    
    for goal in priority_goals:
        goal_id = goal['id']
        required_ids = extract_dependency_tree(goal_id, dependencies, all_tasks)
        
        # Filter to only actionable tasks (exclude the goal itself if it's not actionable)
        actionable_required_ids = {
            tid for tid in required_ids 
            if tid in all_tasks and (all_tasks[tid].get('is_actionable') or all_tasks[tid].get('is_goal'))
        }
        
        goal_trees[goal_id] = {
            'goal': goal,
            'all_required_task_ids': required_ids,
            'actionable_required_task_ids': actionable_required_ids,
            'task_count': len(actionable_required_ids)
        }
        
        all_required_task_ids.update(actionable_required_ids)
    
    # Find tasks ready to start immediately
    ready_tasks = get_ready_tasks(all_required_task_ids, all_tasks, dependencies)
    
    # Calculate total effort
    total_effort_hours = 0
    for task_id in all_required_task_ids:
        if task_id in all_tasks:
            effort_str = all_tasks[task_id].get('properties', {}).get('EFFORT', '0h')
            # Simple parsing for demo - extract number before 'h'
            try:
                hours = int(effort_str.rstrip('h'))
                total_effort_hours += hours
            except (ValueError, AttributeError):
                pass
    
    return {
        'priority': priority,
        'goals': goal_trees,
        'total_goals': len(priority_goals),
        'total_required_tasks': len(all_required_task_ids),
        'ready_to_start': len(ready_tasks),
        'ready_tasks': ready_tasks,
        'total_effort_hours': total_effort_hours,
        'immediate_parallel_work': [t['heading'] for t in ready_tasks[:5]]  # Show first 5
    }

def main():
    if len(sys.argv) != 3:
        print("Usage: python3 priority_extractor.py <org_file> <priority>")
        print("Example: python3 priority_extractor.py tasks.org A")
        sys.exit(1)
    
    org_file = sys.argv[1]
    priority = sys.argv[2].upper()
    
    print(f"🎯 Priority {priority} Task Extraction for {org_file}\n")
    
    analysis = analyze_priority_work_plan(org_file, priority)
    
    if "error" in analysis:
        print(f"❌ Error: {analysis['error']}")
        sys.exit(1)
    
    # Display results
    print(f"📊 Priority {priority} Work Plan Summary:")
    print(f"   • Goals: {analysis['total_goals']}")
    print(f"   • Total Required Tasks: {analysis['total_required_tasks']}")
    print(f"   • Ready to Start Immediately: {analysis['ready_to_start']}")
    print(f"   • Estimated Total Effort: {analysis['total_effort_hours']} hours")
    
    print(f"\n🎯 Priority {priority} Goals:")
    for goal_id, goal_info in analysis['goals'].items():
        goal = goal_info['goal']
        print(f"   • {goal['heading']}")
        print(f"     Required Tasks: {goal_info['task_count']}")
    
    print(f"\n⚡ Immediate Parallel Work Available:")
    if analysis['ready_tasks']:
        for task in analysis['ready_tasks']:
            priority_marker = f"[#{task.get('priority', '?')}]"
            effort = task.get('properties', {}).get('EFFORT', '?')
            print(f"   • {priority_marker} {task['heading']} ({effort})")
    else:
        print("   No tasks ready - check dependencies")
    
    print(f"\n💡 Insight:")
    print(f"   To complete ALL Priority {priority} goals, you need {analysis['total_required_tasks']} tasks ({analysis['total_effort_hours']}h)")
    print(f"   You can start {analysis['ready_to_start']} tasks immediately in parallel")
    print(f"   This represents the minimum viable work for Priority {priority} objectives")

if __name__ == '__main__':
    main()