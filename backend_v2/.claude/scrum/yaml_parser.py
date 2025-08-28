#!/usr/bin/env python3
"""
YAML Task Parser for Torq Self-Organizing Task System
Provides robust YAML parsing and manipulation for task metadata
"""

import yaml
import sys
import os
import json
from pathlib import Path
from typing import Dict, List, Optional, Any
from datetime import datetime

class TaskParser:
    """Parse and manipulate task YAML frontmatter"""
    
    def __init__(self):
        self.task_dir = Path(__file__).parent.parent / "tasks"
        
    def parse_task_file(self, filepath: str) -> Dict[str, Any]:
        """
        Parse a task markdown file and extract YAML frontmatter
        
        Args:
            filepath: Path to the task markdown file
            
        Returns:
            Dictionary containing task metadata and content
        """
        with open(filepath, 'r') as f:
            content = f.read()
            
        # Split frontmatter and content
        if not content.startswith('---'):
            return {'error': 'No YAML frontmatter found'}
            
        parts = content.split('---', 2)
        if len(parts) < 3:
            return {'error': 'Invalid YAML frontmatter format'}
            
        try:
            metadata = yaml.safe_load(parts[1])
            if metadata is None:
                metadata = {}
                
            # Ensure critical fields exist with defaults
            metadata.setdefault('depends_on', [])
            metadata.setdefault('blocks', [])
            metadata.setdefault('scope', [])
            metadata.setdefault('status', 'TODO')
            metadata.setdefault('priority', 'MEDIUM')
            
            return {
                'metadata': metadata,
                'content': parts[2],
                'filepath': filepath,
                'filename': os.path.basename(filepath)
            }
        except yaml.YAMLError as e:
            return {'error': f'YAML parsing error: {e}'}
    
    def update_task_status(self, filepath: str, new_status: str) -> bool:
        """
        Update the status field in a task file
        
        Args:
            filepath: Path to the task file
            new_status: New status value (TODO, IN_PROGRESS, COMPLETE, BLOCKED)
            
        Returns:
            True if successful, False otherwise
        """
        task = self.parse_task_file(filepath)
        if 'error' in task:
            return False
            
        task['metadata']['status'] = new_status
        
        if new_status == 'COMPLETE':
            task['metadata']['completed'] = datetime.now().strftime('%Y-%m-%d')
            
        return self._write_task_file(filepath, task['metadata'], task['content'])
    
    def add_dependency(self, filepath: str, depends_on: str) -> bool:
        """Add a dependency to a task"""
        task = self.parse_task_file(filepath)
        if 'error' in task:
            return False
            
        if depends_on not in task['metadata']['depends_on']:
            task['metadata']['depends_on'].append(depends_on)
            return self._write_task_file(filepath, task['metadata'], task['content'])
        return True
    
    def add_scope(self, filepath: str, scope_path: str) -> bool:
        """Add a file/directory to task scope"""
        task = self.parse_task_file(filepath)
        if 'error' in task:
            return False
            
        if scope_path not in task['metadata']['scope']:
            task['metadata']['scope'].append(scope_path)
            return self._write_task_file(filepath, task['metadata'], task['content'])
        return True
    
    def get_all_tasks(self) -> List[Dict[str, Any]]:
        """
        Scan all task files and return their metadata
        
        Returns:
            List of task dictionaries with metadata
        """
        tasks = []
        
        for sprint_dir in self.task_dir.glob('sprint-*'):
            if sprint_dir.is_dir() and 'archive' not in sprint_dir.name:
                for task_file in sprint_dir.glob('*.md'):
                    # Skip non-task files
                    if task_file.name in ['SPRINT_PLAN.md', 'README.md', 'TEST_RESULTS.md']:
                        continue
                    if 'rename_me' in task_file.name or 'template' in task_file.name.lower():
                        continue
                        
                    task = self.parse_task_file(str(task_file))
                    if 'error' not in task:
                        task['sprint'] = sprint_dir.name
                        tasks.append(task)
                        
        return tasks
    
    def find_ready_tasks(self) -> List[Dict[str, Any]]:
        """
        Find all tasks that are ready to start (dependencies satisfied)
        
        Returns:
            List of tasks with TODO status and satisfied dependencies
        """
        all_tasks = self.get_all_tasks()
        
        # Build a map of task_id to status
        task_status = {}
        task_map = {}
        for task in all_tasks:
            task_id = task['metadata'].get('task_id', '')
            if task_id:
                task_status[task_id] = task['metadata']['status']
                task_map[task_id] = task
        
        # Find ready tasks
        ready_tasks = []
        for task in all_tasks:
            if task['metadata']['status'] != 'TODO':
                continue
                
            # Check if all dependencies are complete
            dependencies_satisfied = True
            for dep_id in task['metadata'].get('depends_on', []):
                dep_status = task_status.get(dep_id, 'UNKNOWN')
                if dep_status not in ['COMPLETE', 'DONE']:
                    dependencies_satisfied = False
                    break
                    
            if dependencies_satisfied:
                ready_tasks.append(task)
        
        # Sort by priority
        priority_order = {'CRITICAL': 0, 'HIGH': 1, 'MEDIUM': 2, 'LOW': 3}
        ready_tasks.sort(key=lambda t: priority_order.get(t['metadata'].get('priority', 'MEDIUM'), 99))
        
        return ready_tasks
    
    def check_scope_conflicts(self, task_file: str) -> List[Dict[str, Any]]:
        """
        Check if a task's scope conflicts with other in-progress tasks
        
        Args:
            task_file: Path to the task file to check
            
        Returns:
            List of conflicting tasks with their overlapping scope
        """
        target_task = self.parse_task_file(task_file)
        if 'error' in target_task or not target_task['metadata'].get('scope'):
            return []
            
        conflicts = []
        all_tasks = self.get_all_tasks()
        
        for task in all_tasks:
            # Only check in-progress tasks
            if task['metadata']['status'] != 'IN_PROGRESS':
                continue
            if task['filepath'] == task_file:
                continue
                
            # Check for scope overlap
            task_scope = set(task['metadata'].get('scope', []))
            target_scope = set(target_task['metadata']['scope'])
            
            overlap = task_scope.intersection(target_scope)
            if overlap:
                conflicts.append({
                    'task_id': task['metadata'].get('task_id', 'UNKNOWN'),
                    'filepath': task['filepath'],
                    'overlapping_scope': list(overlap),
                    'status': task['metadata']['status']
                })
                
        return conflicts
    
    def validate_dependencies(self) -> Dict[str, Any]:
        """
        Validate all task dependencies for cycles and missing tasks
        
        Returns:
            Dictionary with validation results
        """
        all_tasks = self.get_all_tasks()
        
        # Build dependency graph
        task_ids = set()
        dependencies = {}
        for task in all_tasks:
            task_id = task['metadata'].get('task_id', '')
            if task_id:
                task_ids.add(task_id)
                dependencies[task_id] = task['metadata'].get('depends_on', [])
        
        # Check for missing dependencies
        missing = {}
        for task_id, deps in dependencies.items():
            missing_deps = [d for d in deps if d not in task_ids]
            if missing_deps:
                missing[task_id] = missing_deps
        
        # Check for cycles using DFS
        def has_cycle(graph):
            visited = set()
            rec_stack = set()
            
            def visit(node):
                if node in rec_stack:
                    return True  # Cycle detected
                if node in visited:
                    return False
                    
                visited.add(node)
                rec_stack.add(node)
                
                for neighbor in graph.get(node, []):
                    if visit(neighbor):
                        return True
                        
                rec_stack.remove(node)
                return False
            
            for node in graph:
                if node not in visited:
                    if visit(node):
                        return True
            return False
        
        has_cycles = has_cycle(dependencies)
        
        return {
            'valid': not has_cycles and not missing,
            'has_cycles': has_cycles,
            'missing_dependencies': missing,
            'total_tasks': len(all_tasks),
            'tasks_with_dependencies': len([t for t in all_tasks if t['metadata'].get('depends_on')])
        }
    
    def _write_task_file(self, filepath: str, metadata: Dict, content: str) -> bool:
        """Write updated metadata and content back to task file"""
        try:
            yaml_content = yaml.dump(metadata, default_flow_style=False, sort_keys=False)
            full_content = f"---\n{yaml_content}---\n{content}"
            
            with open(filepath, 'w') as f:
                f.write(full_content)
            return True
        except Exception as e:
            print(f"Error writing file: {e}", file=sys.stderr)
            return False


def main():
    """CLI interface for task parser"""
    if len(sys.argv) < 2:
        print("Usage: yaml_parser.py <command> [args]")
        print("Commands:")
        print("  parse <file>          - Parse and display task metadata")
        print("  status <file> <status> - Update task status")
        print("  ready                 - List tasks ready to start")
        print("  conflicts <file>      - Check for scope conflicts")
        print("  validate              - Validate all dependencies")
        print("  add-dep <file> <dep>  - Add dependency to task")
        print("  add-scope <file> <path> - Add scope to task")
        sys.exit(1)
    
    parser = TaskParser()
    command = sys.argv[1]
    
    if command == 'parse' and len(sys.argv) > 2:
        result = parser.parse_task_file(sys.argv[2])
        print(json.dumps(result.get('metadata', result), indent=2))
        
    elif command == 'status' and len(sys.argv) > 3:
        success = parser.update_task_status(sys.argv[2], sys.argv[3])
        print("Success" if success else "Failed")
        
    elif command == 'ready':
        tasks = parser.find_ready_tasks()
        for task in tasks:
            print(f"{task['metadata']['task_id']}: {task['filename']} [{task['metadata']['priority']}]")
            
    elif command == 'conflicts' and len(sys.argv) > 2:
        conflicts = parser.check_scope_conflicts(sys.argv[2])
        if conflicts:
            print("Scope conflicts detected:")
            for conflict in conflicts:
                print(f"  - {conflict['task_id']}: {', '.join(conflict['overlapping_scope'])}")
        else:
            print("No conflicts detected")
            
    elif command == 'validate':
        result = parser.validate_dependencies()
        print(json.dumps(result, indent=2))
        
    elif command == 'add-dep' and len(sys.argv) > 3:
        success = parser.add_dependency(sys.argv[2], sys.argv[3])
        print("Success" if success else "Failed")
        
    elif command == 'add-scope' and len(sys.argv) > 3:
        success = parser.add_scope(sys.argv[2], sys.argv[3])
        print("Success" if success else "Failed")
        
    else:
        print(f"Unknown command: {command}")
        sys.exit(1)


if __name__ == '__main__':
    main()