#!/usr/bin/env python3
"""
Simple Priority-based Task Extraction Demo

Demonstrates the power of dependency tree extraction for priority-based work planning.
"""

from typing import Dict, List, Set
from collections import defaultdict

# Sample task data representing a realistic project
SAMPLE_TASKS = [
    # Priority A Goal: User Authentication 
    {"id": "AUTH-GOAL", "heading": "User Authentication System", "state": None, "priority": "A", "depends": [], "is_goal": True, "is_actionable": False, "effort": 0},
    {"id": "AUTH-001", "heading": "Database schema design", "state": "TODO", "priority": "A", "depends": [], "is_goal": False, "is_actionable": True, "effort": 6},
    {"id": "AUTH-002", "heading": "Authentication API endpoints", "state": "TODO", "priority": "A", "depends": [], "is_goal": False, "is_actionable": True, "effort": 8},
    {"id": "AUTH-003", "heading": "User registration implementation", "state": "TODO", "priority": "B", "depends": ["AUTH-001", "AUTH-002"], "is_goal": False, "is_actionable": True, "effort": 12},
    {"id": "AUTH-004", "heading": "Password reset flow", "state": "TODO", "priority": "B", "depends": ["AUTH-001", "AUTH-002"], "is_goal": False, "is_actionable": True, "effort": 8},
    {"id": "AUTH-005", "heading": "Frontend login forms", "state": "TODO", "priority": "C", "depends": ["AUTH-003", "AUTH-004"], "is_goal": False, "is_actionable": True, "effort": 6},
    
    # Priority A Goal: Security Hardening
    {"id": "SEC-GOAL", "heading": "Security Hardening", "state": None, "priority": "A", "depends": [], "is_goal": True, "is_actionable": False, "effort": 0},
    {"id": "SEC-001", "heading": "Input validation framework", "state": "TODO", "priority": "A", "depends": [], "is_goal": False, "is_actionable": True, "effort": 4},
    {"id": "SEC-002", "heading": "Rate limiting middleware", "state": "TODO", "priority": "A", "depends": ["SEC-001"], "is_goal": False, "is_actionable": True, "effort": 6},
    {"id": "SEC-003", "heading": "Security audit and testing", "state": "TODO", "priority": "B", "depends": ["SEC-001", "SEC-002"], "is_goal": False, "is_actionable": True, "effort": 8},
    
    # Priority B Goal: Performance Optimization
    {"id": "PERF-GOAL", "heading": "Performance Optimization", "state": None, "priority": "B", "depends": [], "is_goal": True, "is_actionable": False, "effort": 0},
    {"id": "PERF-001", "heading": "Database query optimization", "state": "TODO", "priority": "B", "depends": [], "is_goal": False, "is_actionable": True, "effort": 4},
    {"id": "PERF-002", "heading": "API response caching", "state": "TODO", "priority": "B", "depends": [], "is_goal": False, "is_actionable": True, "effort": 6},
    {"id": "PERF-003", "heading": "Frontend bundle optimization", "state": "TODO", "priority": "C", "depends": ["PERF-001", "PERF-002"], "is_goal": False, "is_actionable": True, "effort": 8},
]

def build_dependency_graph(tasks: List[Dict]) -> Dict[str, Set[str]]:
    """Build dependency graph"""
    deps = defaultdict(set)
    for task in tasks:
        deps[task['id']] = set(task['depends'])
    return dict(deps)

def extract_dependency_tree(goal_id: str, all_tasks: Dict[str, Dict]) -> Set[str]:
    """Extract all tasks needed to complete a goal (includes child tasks + dependencies)"""
    required = set()
    
    # Find all tasks that belong to this goal (by checking hierarchical relationship)
    goal_tasks = []
    for task in all_tasks.values():
        # For this demo, we'll consider tasks that would belong to the goal
        # In real org-mode, this would be based on heading hierarchy
        if goal_id == "AUTH-GOAL" and task['id'].startswith("AUTH-") and task['is_actionable']:
            goal_tasks.append(task)
        elif goal_id == "SEC-GOAL" and task['id'].startswith("SEC-") and task['is_actionable']:
            goal_tasks.append(task)
        elif goal_id == "PERF-GOAL" and task['id'].startswith("PERF-") and task['is_actionable']:
            goal_tasks.append(task)
    
    # Add all goal tasks
    for task in goal_tasks:
        required.add(task['id'])
        
        # Recursively add dependencies
        def add_dependencies(task_id: str, visited: Set[str]):
            if task_id in visited:
                return
            visited.add(task_id)
            
            if task_id in all_tasks:
                for dep_id in all_tasks[task_id]['depends']:
                    if dep_id in all_tasks:
                        required.add(dep_id)
                        add_dependencies(dep_id, visited)
        
        add_dependencies(task['id'], set())
    
    return required

def get_ready_tasks(task_ids: Set[str], all_tasks: Dict[str, Dict], dependencies: Dict[str, Set[str]]) -> List[Dict]:
    """Get tasks ready to execute immediately"""
    ready = []
    
    for task_id in task_ids:
        task = all_tasks[task_id]
        
        if not task['is_actionable'] or task['state'] != 'TODO':
            continue
            
        # Check if all dependencies are complete (for demo, assume none are DONE yet)
        deps_complete = len(dependencies.get(task_id, set())) == 0  # No dependencies = ready
        
        if deps_complete:
            ready.append(task)
    
    return sorted(ready, key=lambda t: (t['priority'], t['id']))

def analyze_priority_work_plan(tasks: List[Dict], priority: str):
    """Analyze work plan for specific priority"""
    
    # Build lookup tables
    all_tasks = {task['id']: task for task in tasks}
    dependencies = build_dependency_graph(tasks)
    
    # Find priority goals
    priority_goals = [t for t in tasks if t['is_goal'] and t['priority'] == priority]
    
    print(f"🎯 Priority {priority} Work Plan Analysis\n")
    
    if not priority_goals:
        print(f"❌ No goals found with priority {priority}")
        return
    
    # Extract dependency trees for each goal
    all_required_ids = set()
    goal_analysis = {}
    
    for goal in priority_goals:
        required_ids = extract_dependency_tree(goal['id'], all_tasks)
        actionable_ids = {tid for tid in required_ids if all_tasks[tid]['is_actionable']}
        
        goal_analysis[goal['id']] = {
            'goal': goal,
            'required_task_ids': actionable_ids,
            'task_count': len(actionable_ids),
            'total_effort': sum(all_tasks[tid]['effort'] for tid in actionable_ids)
        }
        
        all_required_ids.update(actionable_ids)
    
    # Find immediately available parallel work
    ready_tasks = get_ready_tasks(all_required_ids, all_tasks, dependencies)
    
    # Calculate totals
    total_effort = sum(all_tasks[tid]['effort'] for tid in all_required_ids)
    
    # Display results
    print(f"📊 Priority {priority} Summary:")
    print(f"   • Goals: {len(priority_goals)}")
    print(f"   • Total Required Tasks: {len(all_required_ids)}")
    print(f"   • Ready to Start Now: {len(ready_tasks)}")
    print(f"   • Total Effort: {total_effort} hours")
    
    print(f"\n🎯 Priority {priority} Goals & Their Dependencies:")
    for goal_id, analysis in goal_analysis.items():
        goal = analysis['goal']
        print(f"\n   📋 {goal['heading']}")
        print(f"      • Required tasks: {analysis['task_count']}")
        print(f"      • Estimated effort: {analysis['total_effort']} hours")
        
        # Show the actual required tasks
        required_tasks = [all_tasks[tid] for tid in analysis['required_task_ids']]
        for task in sorted(required_tasks, key=lambda t: t['priority']):
            deps_str = f" (depends: {', '.join(task['depends'])})" if task['depends'] else ""
            print(f"        - [{task['priority']}] {task['heading']} ({task['effort']}h){deps_str}")
    
    print(f"\n⚡ Immediate Parallel Work Available:")
    if ready_tasks:
        parallel_effort = sum(t['effort'] for t in ready_tasks)
        print(f"   Can start {len(ready_tasks)} tasks in parallel ({parallel_effort} hours total):")
        for task in ready_tasks:
            print(f"   • [{task['priority']}] {task['heading']} ({task['effort']}h)")
    else:
        print("   No tasks ready - all have pending dependencies")
    
    print(f"\n💡 Strategic Insight:")
    print(f"   To achieve ALL Priority {priority} objectives:")
    print(f"   • Minimum required work: {len(all_required_ids)} tasks ({total_effort} hours)")
    print(f"   • Can parallelize {len(ready_tasks)} tasks immediately")
    print(f"   • This represents {total_effort} hours of focused {priority}-priority work")
    
    if len(ready_tasks) > 1:
        print(f"   • With {len(ready_tasks)} parallel workers, initial phase completes in ~{max(t['effort'] for t in ready_tasks)} hours")
    
    print(f"\n🔥 Recommendation:")
    if priority == 'A':
        print(f"   Focus EXCLUSIVELY on these {len(all_required_ids)} tasks to complete all critical objectives")
        print(f"   Ignore Priority B/C work until these {total_effort} hours are complete")
    else:
        print(f"   These {len(all_required_ids)} tasks represent the complete {priority}-priority scope")

def main():
    print("🎯 DAG-Based Priority Work Planning Demo\n")
    
    # Show all available tasks first
    print("📋 Available Tasks by Priority:")
    by_priority = defaultdict(list)
    for task in SAMPLE_TASKS:
        if task['is_actionable']:
            by_priority[task['priority']].append(task)
    
    for priority in ['A', 'B', 'C']:
        if priority in by_priority:
            tasks = by_priority[priority]
            total_effort = sum(t['effort'] for t in tasks)
            print(f"   Priority {priority}: {len(tasks)} tasks ({total_effort}h)")
    
    print("\n" + "="*60)
    
    # Analyze Priority A work plan
    analyze_priority_work_plan(SAMPLE_TASKS, 'A')
    
    print("\n" + "="*60)
    
    # Compare with Priority B for contrast
    analyze_priority_work_plan(SAMPLE_TASKS, 'B')

if __name__ == '__main__':
    main()