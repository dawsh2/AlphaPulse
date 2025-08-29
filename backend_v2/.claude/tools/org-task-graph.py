#!/usr/bin/env python3
"""
Extract task dependency graph and find NEXT actions for org-edna tasks.

This tool analyzes org-mode files with org-edna TRIGGER/BLOCKER properties
to visualize dependencies and identify immediately actionable tasks.
"""

import re
import sys
from pathlib import Path
from typing import Dict, List, Set, Tuple
import argparse
from collections import defaultdict, deque

class TaskNode:
    """Represents a task in the dependency graph."""
    
    def __init__(self, task_id: str, title: str, state: str, level: int):
        self.id = task_id
        self.title = title
        self.state = state
        self.level = level
        self.blockers = []  # IDs this task is blocked by
        self.triggers = []  # IDs this task will trigger
        self.children = []  # Child task IDs
        self.parent = None  # Parent task ID
        self.priority = None
        self.effort = None
        self.assigned = None
        
    def is_actionable(self, task_map: Dict[str, 'TaskNode']) -> bool:
        """Check if this task is actionable (all blockers are done)."""
        if self.state not in ['TODO', 'NEXT']:
            return False
            
        for blocker_id in self.blockers:
            if blocker_id in task_map:
                blocker = task_map[blocker_id]
                if blocker.state not in ['DONE', 'CANCELLED']:
                    return False
        return True
        
    def __repr__(self):
        return f"Task({self.id}: {self.title} [{self.state}])"

class OrgTaskGraph:
    """Build and analyze task dependency graph from org files."""
    
    def __init__(self):
        self.tasks = {}  # ID -> TaskNode
        self.root_tasks = []  # Top-level task IDs
        
    def parse_org_file(self, filepath: Path) -> None:
        """Parse org file and build task graph."""
        with open(filepath, 'r') as f:
            content = f.read()
            
        lines = content.split('\n')
        current_task = None
        task_stack = []  # Stack to track parent tasks
        in_properties = False
        
        for i, line in enumerate(lines):
            # Check for properties drawer
            if line.strip() == ':PROPERTIES:':
                in_properties = True
                continue
            elif line.strip() == ':END:':
                in_properties = False
                continue
                
            # Parse properties
            if in_properties and current_task:
                prop_match = re.match(r'^\s*:([A-Z\-_]+):\s*(.*)$', line)
                if prop_match:
                    prop_name = prop_match.group(1)
                    prop_value = prop_match.group(2).strip()
                    
                    if prop_name == 'ID':
                        current_task.id = prop_value
                    elif prop_name == 'BLOCKER':
                        self._parse_blocker(current_task, prop_value)
                    elif prop_name == 'TRIGGER':
                        self._parse_trigger(current_task, prop_value)
                    elif prop_name == 'DEPENDS':  # Legacy support
                        deps = prop_value.split()
                        current_task.blockers.extend(deps)
                    elif prop_name == 'EFFORT':
                        current_task.effort = prop_value
                    elif prop_name == 'ASSIGNED':
                        current_task.assigned = prop_value
                        
            # Parse headings
            heading_match = re.match(r'^(\*+)\s+(\w+)?\s*(.*?)(?:\s+:([\w:]+):)?$', line)
            if heading_match:
                level = len(heading_match.group(1))
                state = heading_match.group(2) if heading_match.group(2) in ['TODO', 'NEXT', 'IN-PROGRESS', 'DONE', 'CANCELLED'] else None
                
                if not state:
                    # Not a task, might be a goal
                    continue
                    
                title = heading_match.group(3) if state else f"{heading_match.group(2)} {heading_match.group(3)}"
                tags = heading_match.group(4) if heading_match.group(4) else ""
                
                # Extract priority
                priority_match = re.match(r'\[#([A-C])\]\s*(.*)', title)
                if priority_match:
                    priority = priority_match.group(1)
                    title = priority_match.group(2)
                else:
                    priority = None
                
                # Create task node
                task = TaskNode(f"task_{i}", title, state, level)
                task.priority = priority
                
                # Update parent-child relationships
                while task_stack and task_stack[-1].level >= level:
                    task_stack.pop()
                    
                if task_stack:
                    parent = task_stack[-1]
                    task.parent = parent.id
                    parent.children.append(task.id)
                else:
                    self.root_tasks.append(task.id)
                    
                task_stack.append(task)
                current_task = task
                
                # Store task temporarily with line number as ID
                if current_task.id == f"task_{i}":
                    self.tasks[f"task_{i}"] = current_task
                    
        # Second pass: Update references with actual IDs
        id_map = {}
        for temp_id, task in list(self.tasks.items()):
            if task.id != temp_id:
                id_map[temp_id] = task.id
                self.tasks[task.id] = task
                del self.tasks[temp_id]
                
        # Update references
        for task in self.tasks.values():
            if task.parent in id_map:
                task.parent = id_map[task.parent]
            task.children = [id_map.get(c, c) for c in task.children]
            task.blockers = [id_map.get(b, b) for b in task.blockers]
            task.triggers = [id_map.get(t, t) for t in task.triggers]
            
        self.root_tasks = [id_map.get(r, r) for r in self.root_tasks]
                
    def _parse_blocker(self, task: TaskNode, blocker_str: str) -> None:
        """Parse BLOCKER property and extract dependencies."""
        # Extract IDs from ids() expressions
        ids_matches = re.findall(r'ids\(([^)]+)\)', blocker_str)
        for ids_match in ids_matches:
            task_ids = ids_match.split()
            task.blockers.extend(task_ids)
            
    def _parse_trigger(self, task: TaskNode, trigger_str: str) -> None:
        """Parse TRIGGER property and extract triggered tasks."""
        # Extract IDs from ids() expressions
        ids_matches = re.findall(r'ids\(([^)]+)\)', trigger_str)
        for ids_match in ids_matches:
            task_ids = ids_match.split()
            task.triggers.extend(task_ids)
            
    def find_next_actions(self, project_id: str = None) -> List[TaskNode]:
        """Find all actionable NEXT tasks, optionally filtered by project."""
        next_actions = []
        
        if project_id:
            # Get all tasks in project tree
            project_tasks = self._get_project_tree(project_id)
        else:
            project_tasks = set(self.tasks.keys())
            
        for task_id in project_tasks:
            if task_id in self.tasks:
                task = self.tasks[task_id]
                if task.is_actionable(self.tasks):
                    next_actions.append(task)
                    
        # Sort by priority and state
        priority_order = {'A': 0, 'B': 1, 'C': 2, None: 3}
        state_order = {'NEXT': 0, 'TODO': 1}
        
        next_actions.sort(key=lambda t: (
            priority_order.get(t.priority, 3),
            state_order.get(t.state, 2),
            t.title
        ))
        
        return next_actions
        
    def _get_project_tree(self, project_id: str) -> Set[str]:
        """Get all tasks in a project tree including dependencies."""
        if project_id not in self.tasks:
            return set()
            
        visited = set()
        queue = deque([project_id])
        
        while queue:
            task_id = queue.popleft()
            if task_id in visited or task_id not in self.tasks:
                continue
                
            visited.add(task_id)
            task = self.tasks[task_id]
            
            # Add children
            queue.extend(task.children)
            
            # Add dependencies (blockers)
            queue.extend(task.blockers)
            
            # Add triggered tasks
            queue.extend(task.triggers)
            
        return visited
        
    def extract_task_graph(self, task_id: str) -> Dict:
        """Extract dependency graph for a specific task."""
        if task_id not in self.tasks:
            return {"error": f"Task {task_id} not found"}
            
        graph = {
            "root": task_id,
            "nodes": {},
            "edges": []
        }
        
        # BFS to find all related tasks
        visited = set()
        queue = deque([task_id])
        
        while queue:
            current_id = queue.popleft()
            if current_id in visited or current_id not in self.tasks:
                continue
                
            visited.add(current_id)
            task = self.tasks[current_id]
            
            # Add node
            graph["nodes"][current_id] = {
                "title": task.title,
                "state": task.state,
                "priority": task.priority,
                "effort": task.effort,
                "actionable": task.is_actionable(self.tasks)
            }
            
            # Add edges and queue dependencies
            for blocker_id in task.blockers:
                if blocker_id in self.tasks:
                    graph["edges"].append({
                        "from": blocker_id,
                        "to": current_id,
                        "type": "blocks"
                    })
                    queue.append(blocker_id)
                    
            for trigger_id in task.triggers:
                if trigger_id in self.tasks:
                    graph["edges"].append({
                        "from": current_id,
                        "to": trigger_id,
                        "type": "triggers"
                    })
                    queue.append(trigger_id)
                    
            # Add children
            for child_id in task.children:
                if child_id in self.tasks:
                    graph["edges"].append({
                        "from": current_id,
                        "to": child_id,
                        "type": "parent"
                    })
                    queue.append(child_id)
                    
        return graph
        
    def generate_graphviz(self, task_id: str = None) -> str:
        """Generate Graphviz DOT representation of task graph."""
        dot = ["digraph TaskGraph {"]
        dot.append('  rankdir=TB;')
        dot.append('  node [shape=box, style=rounded];')
        
        if task_id:
            graph = self.extract_task_graph(task_id)
            nodes = graph["nodes"]
            edges = graph["edges"]
            
            # Add nodes
            for node_id, info in nodes.items():
                color = "green" if info["state"] == "DONE" else \
                       "orange" if info["state"] == "NEXT" else \
                       "yellow" if info["state"] == "IN-PROGRESS" else \
                       "lightblue" if info["actionable"] else "white"
                       
                shape = "octagon" if info["priority"] == "A" else \
                       "hexagon" if info["priority"] == "B" else "box"
                       
                label = f"{node_id}\\n{info['title'][:30]}...\\n[{info['state']}]"
                if info["effort"]:
                    label += f"\\n{info['effort']}"
                    
                dot.append(f'  "{node_id}" [label="{label}", fillcolor={color}, style=filled, shape={shape}];')
                
            # Add edges
            for edge in edges:
                style = "solid" if edge["type"] == "blocks" else \
                       "dashed" if edge["type"] == "triggers" else "dotted"
                color = "red" if edge["type"] == "blocks" else \
                       "green" if edge["type"] == "triggers" else "blue"
                       
                dot.append(f'  "{edge["from"]}" -> "{edge["to"]}" [style={style}, color={color}];')
        else:
            # Show all tasks
            for task_id, task in self.tasks.items():
                color = "green" if task.state == "DONE" else \
                       "orange" if task.state == "NEXT" else \
                       "yellow" if task.state == "IN-PROGRESS" else \
                       "lightblue" if task.is_actionable(self.tasks) else "white"
                       
                dot.append(f'  "{task_id}" [label="{task.title[:30]}...", fillcolor={color}, style=filled];')
                
                for blocker in task.blockers:
                    if blocker in self.tasks:
                        dot.append(f'  "{blocker}" -> "{task_id}" [color=red];')
                        
        dot.append("}")
        return "\n".join(dot)

def main():
    parser = argparse.ArgumentParser(description='Extract task graph and find NEXT actions')
    parser.add_argument('file', help='Path to org file')
    parser.add_argument('--task', '-t', help='Extract graph for specific task ID')
    parser.add_argument('--next', '-n', action='store_true', help='Find NEXT actionable tasks')
    parser.add_argument('--project', '-p', help='Filter by project ID')
    parser.add_argument('--graph', '-g', action='store_true', help='Generate Graphviz output')
    
    args = parser.parse_args()
    
    filepath = Path(args.file)
    if not filepath.exists():
        print(f"Error: {filepath} not found")
        sys.exit(1)
        
    # Build task graph
    graph = OrgTaskGraph()
    graph.parse_org_file(filepath)
    
    if args.next:
        # Find NEXT actions
        print("🎯 NEXT Actionable Tasks")
        print("=" * 50)
        
        next_actions = graph.find_next_actions(args.project)
        if next_actions:
            for task in next_actions:
                priority = f"[#{task.priority}]" if task.priority else ""
                effort = f"({task.effort})" if task.effort else ""
                assigned = f"→ {task.assigned}" if task.assigned else ""
                
                print(f"\n{task.state:12} {priority:4} {task.id}")
                print(f"  📋 {task.title}")
                print(f"  ⏱️  {effort} {assigned}")
                
                if task.blockers:
                    blocker_states = []
                    for b_id in task.blockers:
                        if b_id in graph.tasks:
                            b_state = graph.tasks[b_id].state
                            blocker_states.append(f"{b_id}[{b_state}]")
                    if any('[DONE]' not in s for s in blocker_states):
                        print(f"  ⚠️  Blocked by: {', '.join(blocker_states)}")
        else:
            print("\n❌ No actionable tasks found!")
            print("\nPossible reasons:")
            print("- All tasks are blocked by incomplete dependencies")
            print("- All tasks are already DONE or IN-PROGRESS")
            print("- No tasks match the project filter")
            
    elif args.task:
        # Extract task graph
        print(f"📊 Task Graph for {args.task}")
        print("=" * 50)
        
        task_graph = graph.extract_task_graph(args.task)
        
        if "error" in task_graph:
            print(f"Error: {task_graph['error']}")
        else:
            print(f"\n📌 Root Task: {args.task}")
            
            # Show nodes
            print(f"\n📦 Tasks in Graph ({len(task_graph['nodes'])} total):")
            for node_id, info in task_graph["nodes"].items():
                status = "✅" if info["state"] == "DONE" else \
                        "🔄" if info["state"] in ["NEXT", "IN-PROGRESS"] else \
                        "⭐" if info["actionable"] else "⏳"
                priority = f"[#{info['priority']}]" if info['priority'] else "[  ]"
                print(f"  {status} {priority} {node_id}: {info['title'][:50]}")
                
            # Show dependencies
            print(f"\n🔗 Dependencies ({len(task_graph['edges'])} edges):")
            blocks = [e for e in task_graph["edges"] if e["type"] == "blocks"]
            triggers = [e for e in task_graph["edges"] if e["type"] == "triggers"]
            
            if blocks:
                print("\n  Blocking relationships:")
                for edge in blocks:
                    print(f"    {edge['from']} → blocks → {edge['to']}")
                    
            if triggers:
                print("\n  Trigger relationships:")
                for edge in triggers:
                    print(f"    {edge['from']} → triggers → {edge['to']}")
                    
    elif args.graph:
        # Generate Graphviz
        dot_output = graph.generate_graphviz(args.project)
        print(dot_output)
    else:
        # Default: show summary
        print("📈 Task Graph Summary")
        print("=" * 50)
        print(f"Total tasks: {len(graph.tasks)}")
        
        # Count by state
        state_counts = defaultdict(int)
        for task in graph.tasks.values():
            state_counts[task.state] += 1
            
        print("\nBy State:")
        for state, count in sorted(state_counts.items()):
            print(f"  {state:12}: {count:3}")
            
        # Count actionable
        actionable = [t for t in graph.tasks.values() if t.is_actionable(graph.tasks)]
        print(f"\n🎯 Actionable tasks: {len(actionable)}")
        
        # Show top actionable by priority
        if actionable:
            print("\nTop actionable tasks:")
            for task in actionable[:5]:
                priority = f"[#{task.priority}]" if task.priority else ""
                print(f"  {priority:4} {task.id}: {task.title[:40]}")

if __name__ == '__main__':
    main()