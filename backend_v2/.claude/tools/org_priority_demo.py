#!/usr/bin/env python3
"""
Org-mode Priority-based Dependency Tree Extraction Demo

This demonstrates the power of DAG-based task management:
- Extract only tasks needed for Priority A goals
- Build dependency trees dynamically
- Generate parallel execution plans based on priority
"""

import json
import sys
from typing import Dict, List, Set, Optional
from dataclasses import dataclass
from collections import defaultdict, deque

@dataclass
class Task:
    id: str
    heading: str
    state: str
    priority: Optional[str]
    depends: List[str]
    blocks: List[str]
    tags: List[str]
    level: int
    is_goal: bool
    is_actionable: bool
    
    def __post_init__(self):
        self.depends = self.depends or []
        self.blocks = self.blocks or []
        self.tags = self.tags or []

class TaskGraph:
    """DAG representation of tasks with priority-based extraction"""
    
    def __init__(self, tasks: List[Task]):
        self.tasks = {t.id: t for t in tasks}
        self.dependencies = self._build_dependency_graph()
        self.reverse_dependencies = self._build_reverse_dependencies()
    
    def _build_dependency_graph(self) -> Dict[str, Set[str]]:
        """Build forward dependency graph"""
        deps = defaultdict(set)
        for task in self.tasks.values():
            for dep_id in task.depends:
                if dep_id in self.tasks:
                    deps[task.id].add(dep_id)
            # Also handle BLOCKS relationships
            for blocked_id in task.blocks:
                if blocked_id in self.tasks:
                    deps[blocked_id].add(task.id)
        return deps
    
    def _build_reverse_dependencies(self) -> Dict[str, Set[str]]:
        """Build reverse dependency graph (what depends on this task)"""
        reverse_deps = defaultdict(set)
        for task_id, deps in self.dependencies.items():
            for dep_id in deps:
                reverse_deps[dep_id].add(task_id)
        return reverse_deps
    
    def extract_dependency_tree(self, target_id: str) -> Dict[str, Task]:
        """Extract all tasks needed to complete the target task"""
        if target_id not in self.tasks:
            return {}
        
        visited = set()
        required_tasks = {}
        
        def dfs(task_id: str):
            if task_id in visited or task_id not in self.tasks:
                return
            
            visited.add(task_id)
            task = self.tasks[task_id]
            required_tasks[task_id] = task
            
            # Recursively add all dependencies
            for dep_id in self.dependencies.get(task_id, set()):
                dfs(dep_id)
        
        dfs(target_id)
        return required_tasks
    
    def get_priority_goals(self, priority: str) -> List[Task]:
        """Get all goals with specific priority"""
        return [
            task for task in self.tasks.values()
            if task.is_goal and task.priority == priority
        ]
    
    def extract_priority_work_plan(self, priority: str) -> Dict[str, any]:
        """Extract complete work plan for all goals of given priority"""
        priority_goals = self.get_priority_goals(priority)
        
        all_required_tasks = {}
        goal_trees = {}
        
        for goal in priority_goals:
            tree = self.extract_dependency_tree(goal.id)
            goal_trees[goal.id] = {
                'goal': goal,
                'required_tasks': tree,
                'task_count': len(tree)
            }
            all_required_tasks.update(tree)
        
        # Find parallel execution opportunities
        parallel_sets = self.get_parallel_execution_sets(list(all_required_tasks.values()))
        
        return {
            'priority': priority,
            'goals': goal_trees,
            'total_required_tasks': len(all_required_tasks),
            'all_required_tasks': list(all_required_tasks.values()),
            'parallel_execution_sets': parallel_sets,
            'estimated_effort': self.calculate_total_effort(list(all_required_tasks.values()))
        }
    
    def get_parallel_execution_sets(self, tasks: List[Task]) -> List[List[str]]:
        """Group tasks that can be executed in parallel"""
        # Tasks can run in parallel if they have no dependencies between them
        ready_tasks = [t for t in tasks if t.is_actionable and t.state in ['TODO', 'NEXT']]
        
        # For now, simple grouping - tasks with no dependencies can start immediately
        parallel_sets = []
        current_set = []
        
        for task in ready_tasks:
            # Check if this task has dependencies on other ready tasks
            has_deps_in_ready = any(
                dep_id in [rt.id for rt in ready_tasks] 
                for dep_id in task.depends
            )
            
            if not has_deps_in_ready:
                current_set.append(task.id)
        
        if current_set:
            parallel_sets.append(current_set)
        
        return parallel_sets
    
    def calculate_total_effort(self, tasks: List[Task]) -> str:
        """Calculate total effort for task list"""
        # This would parse effort strings like "4h", "2d" from task properties
        # For demo, return placeholder
        return f"{len(tasks)} tasks (effort calculation TBD)"
    
    def get_ready_tasks(self, task_ids: List[str]) -> List[Task]:
        """Get tasks that are ready to execute from given set"""
        ready = []
        for task_id in task_ids:
            if task_id not in self.tasks:
                continue
                
            task = self.tasks[task_id]
            if not task.is_actionable or task.state not in ['TODO', 'NEXT']:
                continue
                
            # Check if all dependencies are complete
            deps_complete = all(
                dep_id not in self.tasks or self.tasks[dep_id].state == 'DONE'
                for dep_id in task.depends
            )
            
            if deps_complete:
                ready.append(task)
        
        return sorted(ready, key=lambda t: (t.priority or 'Z', t.id))

def demo_priority_extraction():
    """Demo the priority-based dependency extraction"""
    
    # Sample task data (would come from org file parsing)
    sample_tasks = [
        # Priority A Goal
        Task("AUTH-GOAL", "User Authentication System", "", "A", [], [], ["critical"], 1, True, False),
        Task("AUTH-001", "Database schema", "TODO", "A", [], [], ["database"], 2, False, True),
        Task("AUTH-002", "API design", "TODO", "A", [], [], ["api"], 2, False, True),
        Task("AUTH-003", "Registration flow", "TODO", "B", ["AUTH-001", "AUTH-002"], [], ["feature"], 2, False, True),
        
        # Priority B Goal  
        Task("PERF-GOAL", "Performance Optimization", "", "B", [], [], ["performance"], 1, True, False),
        Task("PERF-001", "Query optimization", "TODO", "B", [], [], ["database"], 2, False, True),
        Task("PERF-002", "Bundle size", "TODO", "B", [], [], ["frontend"], 2, False, True),
        
        # Priority A Goal (second one)
        Task("SEC-GOAL", "Security Hardening", "", "A", [], [], ["security"], 1, True, False),
        Task("SEC-001", "Input validation", "TODO", "A", [], [], ["security"], 2, False, True),
        Task("SEC-002", "Rate limiting", "TODO", "A", ["SEC-001"], [], ["security"], 2, False, True),
    ]
    
    graph = TaskGraph(sample_tasks)
    
    print("🎯 Priority-Based Task Extraction Demo\n")
    
    # Extract Priority A work plan
    priority_a_plan = graph.extract_priority_work_plan("A")
    
    print("📋 Priority A Work Plan:")
    print(f"   Goals: {len(priority_a_plan['goals'])}")
    print(f"   Total Required Tasks: {priority_a_plan['total_required_tasks']}")
    print(f"   Estimated Effort: {priority_a_plan['estimated_effort']}")
    
    print("\n🎯 Priority A Goals:")
    for goal_id, goal_info in priority_a_plan['goals'].items():
        goal = goal_info['goal']
        print(f"   • {goal.heading} ({goal_info['task_count']} tasks)")
        
        # Show dependency tree for this goal
        tree_tasks = goal_info['required_tasks']
        ready_tasks = graph.get_ready_tasks(list(tree_tasks.keys()))
        
        print(f"     Ready to start: {len(ready_tasks)} tasks")
        for task in ready_tasks[:3]:  # Show first 3
            print(f"       - {task.heading}")
    
    print(f"\n⚡ Parallel Execution Opportunities:")
    for i, parallel_set in enumerate(priority_a_plan['parallel_execution_sets']):
        print(f"   Set {i+1}: {len(parallel_set)} tasks can run in parallel")
        for task_id in parallel_set[:3]:  # Show first 3
            task = graph.tasks[task_id]
            print(f"     - {task.heading}")
    
    print(f"\n📊 Summary:")
    print(f"   Focus on Priority A = {priority_a_plan['total_required_tasks']} tasks")
    print(f"   Immediate parallel work = {len(priority_a_plan['parallel_execution_sets'][0]) if priority_a_plan['parallel_execution_sets'] else 0} tasks")
    print(f"   This represents the minimum viable work to complete all Priority A goals")

if __name__ == '__main__':
    demo_priority_extraction()