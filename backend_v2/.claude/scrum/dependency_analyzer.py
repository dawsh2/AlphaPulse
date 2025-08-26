#!/usr/bin/env python3
"""
Dependency Analyzer for AlphaPulse Self-Organizing Task System
Advanced dependency analysis, visualization, and suggestion engine
"""

import sys
import os
import subprocess
from pathlib import Path
from typing import Dict, List, Set, Tuple, Optional
from collections import defaultdict, deque
from yaml_parser import TaskParser
import json

class DependencyAnalyzer:
    """Advanced dependency analysis for task system"""
    
    def __init__(self):
        self.parser = TaskParser()
        self.task_dir = Path(__file__).parent.parent / "tasks"
        
    def build_dependency_graph(self) -> Tuple[Dict[str, List[str]], Dict[str, Dict]]:
        """
        Build complete dependency graph from all tasks
        
        Returns:
            Tuple of (adjacency list, task metadata map)
        """
        all_tasks = self.parser.get_all_tasks()
        
        graph = defaultdict(list)
        task_map = {}
        
        for task in all_tasks:
            task_id = task['metadata'].get('task_id', '')
            if not task_id:
                continue
                
            task_map[task_id] = {
                'file': task['filepath'],
                'status': task['metadata'].get('status', 'TODO'),
                'priority': task['metadata'].get('priority', 'MEDIUM'),
                'sprint': task.get('sprint', 'unknown'),
                'depends_on': task['metadata'].get('depends_on', []),
                'blocks': task['metadata'].get('blocks', []),
                'scope': task['metadata'].get('scope', [])
            }
            
            # Build adjacency list from depends_on
            for dep in task['metadata'].get('depends_on', []):
                graph[dep].append(task_id)
                
        return graph, task_map
    
    def topological_sort(self) -> Tuple[bool, List[str], List[Tuple[str, str]]]:
        """
        Perform topological sort to detect cycles and find execution order
        
        Returns:
            Tuple of (has_cycle, execution_order, cycle_edges)
        """
        graph, task_map = self.build_dependency_graph()
        
        # Calculate in-degrees
        in_degree = defaultdict(int)
        all_nodes = set(task_map.keys())
        
        for node in all_nodes:
            for neighbor in graph.get(node, []):
                in_degree[neighbor] += 1
        
        # Find nodes with no dependencies
        queue = deque([node for node in all_nodes if in_degree[node] == 0])
        execution_order = []
        
        while queue:
            node = queue.popleft()
            execution_order.append(node)
            
            for neighbor in graph.get(node, []):
                in_degree[neighbor] -= 1
                if in_degree[neighbor] == 0:
                    queue.append(neighbor)
        
        # Check for cycles
        has_cycle = len(execution_order) < len(all_nodes)
        
        # Find cycle edges if cycle exists
        cycle_edges = []
        if has_cycle:
            # Find nodes involved in cycles
            remaining = all_nodes - set(execution_order)
            for node in remaining:
                for dep in task_map[node]['depends_on']:
                    if dep in remaining:
                        cycle_edges.append((node, dep))
        
        return has_cycle, execution_order, cycle_edges
    
    def find_critical_path(self) -> List[str]:
        """
        Find the critical path through the dependency graph
        
        Returns:
            List of task IDs forming the longest dependency chain
        """
        graph, task_map = self.build_dependency_graph()
        
        # Reverse graph for finding longest path
        reverse_graph = defaultdict(list)
        for node, neighbors in graph.items():
            for neighbor in neighbors:
                reverse_graph[neighbor].append(node)
        
        # Dynamic programming to find longest path
        longest_path = {}
        path_next = {}
        
        def dfs(node):
            if node in longest_path:
                return longest_path[node]
                
            max_path = 0
            next_node = None
            
            for neighbor in graph.get(node, []):
                neighbor_path = dfs(neighbor)
                if neighbor_path >= max_path:
                    max_path = neighbor_path
                    next_node = neighbor
            
            longest_path[node] = max_path + 1
            if next_node:
                path_next[node] = next_node
            
            return longest_path[node]
        
        # Find longest path from all nodes
        max_length = 0
        start_node = None
        
        for node in task_map.keys():
            path_length = dfs(node)
            if path_length > max_length:
                max_length = path_length
                start_node = node
        
        # Reconstruct path
        critical_path = []
        current = start_node
        while current:
            critical_path.append(current)
            current = path_next.get(current)
        
        return critical_path
    
    def analyze_bottlenecks(self) -> List[Dict]:
        """
        Find bottleneck tasks that block the most other tasks
        
        Returns:
            List of bottleneck tasks sorted by impact
        """
        graph, task_map = self.build_dependency_graph()
        
        bottlenecks = []
        
        for task_id, task_info in task_map.items():
            # Count direct blocks
            direct_blocks = len(graph.get(task_id, []))
            
            # Count transitive blocks (all tasks that depend on this transitively)
            visited = set()
            queue = deque(graph.get(task_id, []))
            
            while queue:
                node = queue.popleft()
                if node not in visited:
                    visited.add(node)
                    queue.extend(graph.get(node, []))
            
            transitive_blocks = len(visited)
            
            if direct_blocks > 0 or transitive_blocks > 0:
                bottlenecks.append({
                    'task_id': task_id,
                    'status': task_info['status'],
                    'priority': task_info['priority'],
                    'direct_blocks': direct_blocks,
                    'transitive_blocks': transitive_blocks,
                    'file': Path(task_info['file']).name
                })
        
        # Sort by transitive blocks (impact)
        bottlenecks.sort(key=lambda x: x['transitive_blocks'], reverse=True)
        
        return bottlenecks[:10]  # Top 10 bottlenecks
    
    def generate_dot_graph(self) -> str:
        """
        Generate Graphviz DOT format for visualization
        
        Returns:
            DOT format string
        """
        graph, task_map = self.build_dependency_graph()
        
        dot_lines = ['digraph TaskDependencies {']
        dot_lines.append('  rankdir=LR;')
        dot_lines.append('  node [shape=box, style=rounded];')
        dot_lines.append('')
        
        # Group by sprint
        sprints = defaultdict(list)
        for task_id, info in task_map.items():
            sprints[info['sprint']].append(task_id)
        
        # Add subgraphs for each sprint
        for sprint, tasks in sorted(sprints.items()):
            if sprint != 'unknown':
                dot_lines.append(f'  subgraph "cluster_{sprint}" {{')
                dot_lines.append(f'    label="{sprint}";')
                dot_lines.append('    style=filled;')
                dot_lines.append('    fillcolor=lightgrey;')
                
                for task_id in tasks:
                    info = task_map[task_id]
                    
                    # Color based on status
                    color = 'white'
                    if info['status'] == 'COMPLETE' or info['status'] == 'DONE':
                        color = 'lightgreen'
                    elif info['status'] == 'IN_PROGRESS':
                        color = 'yellow'
                    elif info['status'] == 'BLOCKED':
                        color = 'pink'
                    
                    # Shape based on priority
                    shape = 'box'
                    if info['priority'] == 'CRITICAL':
                        shape = 'octagon'
                    elif info['priority'] == 'HIGH':
                        shape = 'diamond'
                    
                    dot_lines.append(f'    "{task_id}" [fillcolor={color}, style=filled, shape={shape}];')
                
                dot_lines.append('  }')
                dot_lines.append('')
        
        # Add edges
        dot_lines.append('  // Dependencies')
        for task_id, info in task_map.items():
            for dep in info['depends_on']:
                if dep in task_map:  # Only show edges for existing tasks
                    dot_lines.append(f'  "{dep}" -> "{task_id}";')
        
        dot_lines.append('}')
        
        return '\n'.join(dot_lines)
    
    def suggest_parallelization(self) -> List[List[str]]:
        """
        Suggest tasks that can be done in parallel
        
        Returns:
            List of task groups that can be parallelized
        """
        has_cycle, execution_order, _ = self.topological_sort()
        
        if has_cycle:
            return []
        
        graph, task_map = self.build_dependency_graph()
        
        # Calculate levels for each task
        levels = {}
        
        for task_id in execution_order:
            # Find max level of dependencies
            max_dep_level = -1
            for dep in task_map[task_id]['depends_on']:
                if dep in levels:
                    max_dep_level = max(max_dep_level, levels[dep])
            
            levels[task_id] = max_dep_level + 1
        
        # Group by level (tasks at same level can be parallel)
        parallel_groups = defaultdict(list)
        for task_id, level in levels.items():
            # Only include TODO tasks
            if task_map[task_id]['status'] == 'TODO':
                parallel_groups[level].append(task_id)
        
        # Return non-empty groups
        return [group for group in parallel_groups.values() if group]
    
    def analyze_scope_impact(self, file_pattern: str) -> List[Dict]:
        """
        Find all tasks that modify files matching a pattern
        
        Args:
            file_pattern: File path or pattern to search for
            
        Returns:
            List of tasks affecting the specified files
        """
        all_tasks = self.parser.get_all_tasks()
        affected_tasks = []
        
        for task in all_tasks:
            task_id = task['metadata'].get('task_id', '')
            scope = task['metadata'].get('scope', [])
            
            for scope_item in scope:
                if file_pattern in scope_item or scope_item in file_pattern:
                    affected_tasks.append({
                        'task_id': task_id,
                        'status': task['metadata'].get('status', 'TODO'),
                        'file': Path(task['filepath']).name,
                        'scope_match': scope_item
                    })
                    break
        
        return affected_tasks
    
    def generate_sprint_timeline(self) -> List[Dict]:
        """
        Generate a timeline showing sprint execution order
        
        Returns:
            List of sprint phases with their tasks
        """
        graph, task_map = self.build_dependency_graph()
        has_cycle, execution_order, _ = self.topological_sort()
        
        if has_cycle:
            return []
        
        # Group tasks by sprint and calculate sprint dependencies
        sprint_tasks = defaultdict(list)
        sprint_deps = defaultdict(set)
        
        for task_id in execution_order:
            if task_id in task_map:
                sprint = task_map[task_id]['sprint']
                sprint_tasks[sprint].append(task_id)
                
                # Add sprint dependencies
                for dep in task_map[task_id]['depends_on']:
                    if dep in task_map:
                        dep_sprint = task_map[dep]['sprint']
                        if dep_sprint != sprint:
                            sprint_deps[sprint].add(dep_sprint)
        
        # Calculate sprint phases using topological sort on sprints
        sprint_in_degree = defaultdict(int)
        for sprint, deps in sprint_deps.items():
            for dep in deps:
                sprint_in_degree[sprint] += 1
        
        sprint_queue = deque([s for s in sprint_tasks.keys() if sprint_in_degree[s] == 0])
        sprint_phases = []
        current_phase = 1
        
        while sprint_queue:
            phase_sprints = list(sprint_queue)
            sprint_queue.clear()
            
            phase_info = {
                'phase': current_phase,
                'sprints': []
            }
            
            for sprint in phase_sprints:
                sprint_info = {
                    'name': sprint,
                    'task_count': len(sprint_tasks[sprint]),
                    'status': self._get_sprint_status(sprint_tasks[sprint], task_map)
                }
                phase_info['sprints'].append(sprint_info)
            
            sprint_phases.append(phase_info)
            current_phase += 1
            
            # Add next phase
            for sprint in phase_sprints:
                for next_sprint in sprint_tasks.keys():
                    if sprint in sprint_deps[next_sprint]:
                        sprint_in_degree[next_sprint] -= 1
                        if sprint_in_degree[next_sprint] == 0:
                            sprint_queue.append(next_sprint)
        
        return sprint_phases
    
    def _get_sprint_status(self, task_ids: List[str], task_map: Dict) -> str:
        """Helper to determine sprint status from its tasks"""
        statuses = [task_map[tid]['status'] for tid in task_ids if tid in task_map]
        
        if all(s in ['COMPLETE', 'DONE'] for s in statuses):
            return 'COMPLETE'
        elif any(s == 'IN_PROGRESS' for s in statuses):
            return 'IN_PROGRESS'
        elif any(s in ['COMPLETE', 'DONE'] for s in statuses):
            return 'PARTIAL'
        else:
            return 'TODO'


def main():
    """CLI interface for dependency analyzer"""
    analyzer = DependencyAnalyzer()
    
    if len(sys.argv) < 2:
        print("Usage: dependency_analyzer.py <command> [args]")
        print("Commands:")
        print("  graph              - Generate DOT graph for visualization")
        print("  critical-path      - Find critical dependency path")
        print("  bottlenecks        - Find bottleneck tasks")
        print("  parallel           - Suggest parallel task groups")
        print("  scope <pattern>    - Find tasks affecting files")
        print("  timeline           - Generate sprint execution timeline")
        print("  tsort              - Output for Unix tsort command")
        sys.exit(1)
    
    command = sys.argv[1]
    
    if command == 'graph':
        print(analyzer.generate_dot_graph())
        
    elif command == 'critical-path':
        path = analyzer.find_critical_path()
        print("Critical Dependency Path:")
        for i, task_id in enumerate(path):
            print(f"  {i+1}. {task_id}")
            
    elif command == 'bottlenecks':
        bottlenecks = analyzer.analyze_bottlenecks()
        print("Top Bottleneck Tasks:")
        for b in bottlenecks:
            print(f"  {b['task_id']}: blocks {b['transitive_blocks']} tasks [{b['status']}]")
            
    elif command == 'parallel':
        groups = analyzer.suggest_parallelization()
        print("Tasks that can be done in parallel:")
        for i, group in enumerate(groups):
            print(f"  Phase {i+1}: {', '.join(group)}")
            
    elif command == 'scope' and len(sys.argv) > 2:
        affected = analyzer.analyze_scope_impact(sys.argv[2])
        print(f"Tasks affecting '{sys.argv[2]}':")
        for task in affected:
            print(f"  {task['task_id']}: {task['scope_match']} [{task['status']}]")
            
    elif command == 'timeline':
        timeline = analyzer.generate_sprint_timeline()
        print("Sprint Execution Timeline:")
        for phase in timeline:
            print(f"\nPhase {phase['phase']}:")
            for sprint in phase['sprints']:
                status_emoji = {'COMPLETE': '✅', 'IN_PROGRESS': '🔄', 
                               'PARTIAL': '⚠️', 'TODO': '⏳'}.get(sprint['status'], '❓')
                print(f"  {status_emoji} {sprint['name']} ({sprint['task_count']} tasks)")
                
    elif command == 'tsort':
        # Output in format for Unix tsort
        graph, task_map = analyzer.build_dependency_graph()
        for task_id, info in task_map.items():
            for dep in info['depends_on']:
                print(f"{dep} {task_id}")
                
    else:
        print(f"Unknown command: {command}")
        sys.exit(1)


if __name__ == '__main__':
    main()