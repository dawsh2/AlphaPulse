#!/usr/bin/env python3
"""
Org-mode Task Parser for AlphaPulse Task Management System

Parses org-mode files and outputs structured JSON data for task management.
Supports dependency tracking, parallel execution groups, and goal hierarchies.
"""

import json
import sys
import re
import argparse
from datetime import datetime
from pathlib import Path
from typing import Dict, List, Optional, Set, Tuple
from dataclasses import dataclass, field, asdict
from enum import Enum

try:
    import orgparse
except ImportError:
    print("Error: orgparse library required. Install with: pip install orgparse", file=sys.stderr)
    sys.exit(1)


class TaskState(Enum):
    """Valid task states in the system"""
    TODO = "TODO"
    NEXT = "NEXT"
    IN_PROGRESS = "IN-PROGRESS"
    WAITING = "WAITING"
    DONE = "DONE"
    CANCELLED = "CANCELLED"
    
    @classmethod
    def is_complete(cls, state: str) -> bool:
        return state in [cls.DONE.value, cls.CANCELLED.value]
    
    @classmethod
    def is_actionable(cls, state: str) -> bool:
        return state in [cls.TODO.value, cls.NEXT.value]


@dataclass
class SubTask:
    """Represents a checkbox subtask"""
    text: str
    done: bool


@dataclass
class Task:
    """Represents a single task from the org file"""
    id: str
    heading: str
    state: str
    priority: Optional[str] = None
    tags: List[str] = field(default_factory=list)
    properties: Dict[str, any] = field(default_factory=dict)
    body: str = ""
    subtasks: List[SubTask] = field(default_factory=list)
    scheduled: Optional[str] = None
    deadline: Optional[str] = None
    level: int = 1
    parent_id: Optional[str] = None
    children_ids: List[str] = field(default_factory=list)
    
    def is_ready(self, all_tasks: Dict[str, 'Task']) -> bool:
        """Check if task is ready for execution"""
        if self.state not in [TaskState.TODO.value, TaskState.NEXT.value]:
            return False
        
        # Check dependencies
        depends_on = self.properties.get('depends', '').split()
        for dep_id in depends_on:
            if dep_id in all_tasks:
                if not TaskState.is_complete(all_tasks[dep_id].state):
                    return False
        
        return True
    
    def to_dict(self) -> dict:
        """Convert to dictionary for JSON serialization"""
        return {
            'id': self.id,
            'heading': self.heading,
            'state': self.state,
            'priority': self.priority,
            'tags': self.tags,
            'properties': self.properties,
            'body': self.body,
            'subtasks': [{'text': st.text, 'done': st.done} for st in self.subtasks],
            'scheduled': self.scheduled,
            'deadline': self.deadline,
            'level': self.level,
            'parent_id': self.parent_id,
            'children_ids': self.children_ids
        }


class OrgTaskParser:
    """Parser for org-mode task files"""
    
    def __init__(self, verbose: bool = False):
        self.verbose = verbose
        self.tasks: Dict[str, Task] = {}
        self.errors: List[str] = []
        self.warnings: List[str] = []
    
    def parse_file(self, filepath: Path) -> Dict[str, Task]:
        """Parse a single org file"""
        try:
            root = orgparse.load(filepath)
            self._process_node(root)
            return self.tasks
        except Exception as e:
            self.errors.append(f"Failed to parse {filepath}: {str(e)}")
            return {}
    
    def _process_node(self, node, parent_id: Optional[str] = None):
        """Recursively process org nodes"""
        # Skip the root node
        if node.level > 0:
            task = self._node_to_task(node, parent_id)
            if task:
                if task.id in self.tasks:
                    self.warnings.append(f"Duplicate ID found: {task.id}")
                self.tasks[task.id] = task
                parent_id = task.id
        
        # Process children
        for child in node.children:
            self._process_node(child, parent_id)
    
    def _node_to_task(self, node, parent_id: Optional[str] = None) -> Optional[Task]:
        """Convert org node to Task object"""
        # Skip nodes without TODO keywords
        if not node.todo:
            return None
        
        # Extract ID from properties
        task_id = node.get_property('ID')
        if not task_id:
            # Generate ID from heading if not provided
            task_id = self._generate_id(node.heading)
            self.warnings.append(f"No ID for task '{node.heading}', generated: {task_id}")
        
        # Extract properties
        properties = {}
        for prop in ['EFFORT', 'GOAL', 'DEPENDS', 'BLOCKS', 'PARALLEL_GROUP', 
                     'ASSIGNEE', 'COMPLEXITY', 'RISK', 'CREATED']:
            value = node.get_property(prop)
            if value:
                properties[prop.lower()] = value
        
        # Parse subtasks from body
        subtasks = self._parse_subtasks(node.body)
        
        # Clean body text (remove subtasks)
        body = self._clean_body(node.body)
        
        # Format dates
        scheduled = self._format_date(node.scheduled) if node.scheduled else None
        deadline = self._format_date(node.deadline) if node.deadline else None
        
        # Create task
        task = Task(
            id=task_id,
            heading=node.heading,
            state=node.todo,
            priority=node.priority,
            tags=node.tags,
            properties=properties,
            body=body,
            subtasks=subtasks,
            scheduled=scheduled,
            deadline=deadline,
            level=node.level,
            parent_id=parent_id
        )
        
        return task
    
    def _generate_id(self, heading: str) -> str:
        """Generate ID from heading"""
        # Simple ID generation - can be improved
        clean = re.sub(r'[^a-zA-Z0-9]+', '-', heading.upper())
        clean = clean.strip('-')[:20]
        timestamp = datetime.now().strftime('%Y%m%d%H%M%S')
        return f"{clean}-{timestamp}"
    
    def _parse_subtasks(self, body: str) -> List[SubTask]:
        """Extract checkbox subtasks from body"""
        subtasks = []
        checkbox_pattern = r'^\s*- \[([ X])\] (.+)$'
        
        for line in body.split('\n'):
            match = re.match(checkbox_pattern, line)
            if match:
                done = match.group(1) == 'X'
                text = match.group(2).strip()
                subtasks.append(SubTask(text=text, done=done))
        
        return subtasks
    
    def _clean_body(self, body: str) -> str:
        """Remove subtasks and clean body text"""
        lines = []
        checkbox_pattern = r'^\s*- \[([ X])\]'
        
        for line in body.split('\n'):
            if not re.match(checkbox_pattern, line):
                lines.append(line)
        
        return '\n'.join(lines).strip()
    
    def _format_date(self, date_obj) -> str:
        """Format org date to ISO string"""
        if hasattr(date_obj, 'start'):
            return date_obj.start.isoformat()
        return str(date_obj)
    
    def validate(self) -> Tuple[bool, List[str]]:
        """Validate parsed tasks"""
        validation_errors = []
        
        # Check for dependency cycles
        cycles = self._find_dependency_cycles()
        if cycles:
            for cycle in cycles:
                validation_errors.append(f"Dependency cycle detected: {' -> '.join(cycle)}")
        
        # Check for missing dependencies
        for task_id, task in self.tasks.items():
            depends = task.properties.get('depends', '').split()
            for dep_id in depends:
                if dep_id and dep_id not in self.tasks:
                    validation_errors.append(f"Task {task_id} depends on non-existent task {dep_id}")
            
            blocks = task.properties.get('blocks', '').split()
            for block_id in blocks:
                if block_id and block_id not in self.tasks:
                    validation_errors.append(f"Task {task_id} blocks non-existent task {block_id}")
        
        # Check effort format
        for task_id, task in self.tasks.items():
            effort = task.properties.get('effort')
            if effort and not re.match(r'^\d+[hdwm]$', effort):
                validation_errors.append(f"Task {task_id} has invalid effort format: {effort}")
        
        return len(validation_errors) == 0, validation_errors
    
    def _find_dependency_cycles(self) -> List[List[str]]:
        """Detect circular dependencies using DFS"""
        cycles = []
        visited = set()
        rec_stack = set()
        
        def dfs(task_id: str, path: List[str]) -> bool:
            visited.add(task_id)
            rec_stack.add(task_id)
            path.append(task_id)
            
            task = self.tasks.get(task_id)
            if task:
                depends = task.properties.get('depends', '').split()
                for dep_id in depends:
                    if dep_id in self.tasks:
                        if dep_id not in visited:
                            if dfs(dep_id, path.copy()):
                                return True
                        elif dep_id in rec_stack:
                            # Found cycle
                            cycle_start = path.index(dep_id)
                            cycles.append(path[cycle_start:] + [dep_id])
                            return True
            
            rec_stack.remove(task_id)
            return False
        
        for task_id in self.tasks:
            if task_id not in visited:
                dfs(task_id, [])
        
        return cycles
    
    def get_ready_tasks(self) -> List[Task]:
        """Get all tasks ready for execution"""
        ready = []
        for task in self.tasks.values():
            if task.is_ready(self.tasks):
                ready.append(task)
        
        # Sort by priority and deadline
        ready.sort(key=lambda t: (
            t.priority or 'Z',  # A-Z priority
            t.deadline or '9999-12-31',
            t.id
        ))
        
        return ready
    
    def get_parallel_groups(self) -> Dict[str, List[Task]]:
        """Group tasks by parallel execution groups"""
        groups = {}
        for task in self.tasks.values():
            group = task.properties.get('parallel_group')
            if group:
                if group not in groups:
                    groups[group] = []
                groups[group].append(task)
        
        return groups


def main():
    """CLI entry point"""
    parser = argparse.ArgumentParser(description='Parse org-mode task files')
    parser.add_argument('filepath', type=Path, help='Path to org file')
    parser.add_argument('-o', '--output', type=Path, help='Output JSON file (default: stdout)')
    parser.add_argument('-v', '--verbose', action='store_true', help='Verbose output')
    parser.add_argument('--validate', action='store_true', help='Run validation checks')
    parser.add_argument('--ready', action='store_true', help='Only output ready tasks')
    parser.add_argument('--parallel-groups', action='store_true', help='Group by parallel execution')
    
    args = parser.parse_args()
    
    if not args.filepath.exists():
        print(f"Error: File {args.filepath} does not exist", file=sys.stderr)
        sys.exit(1)
    
    # Parse file
    parser_obj = OrgTaskParser(verbose=args.verbose)
    tasks = parser_obj.parse_file(args.filepath)
    
    # Validation
    if args.validate:
        valid, errors = parser_obj.validate()
        if not valid:
            print("Validation errors:", file=sys.stderr)
            for error in errors:
                print(f"  - {error}", file=sys.stderr)
            sys.exit(1)
    
    # Prepare output
    output_data = {}
    
    if args.ready:
        ready_tasks = parser_obj.get_ready_tasks()
        output_data['ready_tasks'] = [t.to_dict() for t in ready_tasks]
    elif args.parallel_groups:
        groups = parser_obj.get_parallel_groups()
        output_data['parallel_groups'] = {
            group: [t.to_dict() for t in tasks]
            for group, tasks in groups.items()
        }
    else:
        output_data['tasks'] = [t.to_dict() for t in tasks.values()]
    
    # Add metadata
    output_data['metadata'] = {
        'total_tasks': len(tasks),
        'todo_count': sum(1 for t in tasks.values() if t.state == TaskState.TODO.value),
        'next_count': sum(1 for t in tasks.values() if t.state == TaskState.NEXT.value),
        'in_progress_count': sum(1 for t in tasks.values() if t.state == TaskState.IN_PROGRESS.value),
        'waiting_count': sum(1 for t in tasks.values() if t.state == TaskState.WAITING.value),
        'done_count': sum(1 for t in tasks.values() if t.state == TaskState.DONE.value),
        'cancelled_count': sum(1 for t in tasks.values() if t.state == TaskState.CANCELLED.value),
        'parse_timestamp': datetime.now().isoformat(),
        'warnings': parser_obj.warnings,
        'errors': parser_obj.errors
    }
    
    # Output
    json_output = json.dumps(output_data, indent=2)
    
    if args.output:
        args.output.write_text(json_output)
        if args.verbose:
            print(f"Output written to {args.output}", file=sys.stderr)
    else:
        print(json_output)
    
    # Exit with error if there were parsing errors
    if parser_obj.errors:
        sys.exit(1)


if __name__ == '__main__':
    main()