#!/usr/bin/env python3
"""
Org-mode Task Writer for Torq Task Management System

Modifies org-mode files by adding, updating, or deleting tasks while preserving structure.
Maintains file integrity and validates changes.
"""

import json
import sys
import re
import argparse
import tempfile
import shutil
from datetime import datetime
from pathlib import Path
from typing import Dict, List, Optional, Any
import subprocess
import fcntl
from contextlib import contextmanager


@contextmanager
def file_lock(filepath: Path):
    """Context manager for file locking to prevent concurrent writes"""
    lock_file = Path(f"{filepath}.lock")
    lock_fd = None
    try:
        lock_fd = open(lock_file, 'w')
        fcntl.flock(lock_fd.fileno(), fcntl.LOCK_EX)
        yield
    finally:
        if lock_fd:
            fcntl.flock(lock_fd.fileno(), fcntl.LOCK_UN)
            lock_fd.close()
            try:
                lock_file.unlink()
            except FileNotFoundError:
                pass


class OrgTaskWriter:
    """Writer for org-mode task files"""
    
    def __init__(self, filepath: Path, backup: bool = True, validate: bool = True):
        self.filepath = filepath
        self.backup = backup
        self.validate = validate
        self.parser_path = Path(__file__).parent / "org_task_parser.py"
    
    def add_task(self, task_data: Dict[str, Any]) -> bool:
        """Add a new task to the org file"""
        with file_lock(self.filepath):
            # Create task text
            task_text = self._format_task(task_data)
            
            # Read existing content
            if self.filepath.exists():
                content = self.filepath.read_text()
            else:
                content = self._get_file_header()
            
            # Find insertion point
            insertion_point = self._find_insertion_point(content, task_data)
            
            # Insert task
            lines = content.split('\n')
            if insertion_point == -1:
                # Append to end
                lines.append('')
                lines.append(task_text)
            else:
                # Insert at specific line
                lines.insert(insertion_point, task_text)
                lines.insert(insertion_point, '')
            
            # Write back
            new_content = '\n'.join(lines)
            return self._write_content(new_content)
    
    def update_task(self, task_id: str, updates: Dict[str, Any]) -> bool:
        """Update an existing task"""
        with file_lock(self.filepath):
            if not self.filepath.exists():
                print(f"Error: File {self.filepath} does not exist", file=sys.stderr)
                return False
            
            content = self.filepath.read_text()
            lines = content.split('\n')
            
            # Find task by ID
            task_start, task_end = self._find_task_lines(lines, task_id)
            if task_start == -1:
                print(f"Error: Task {task_id} not found", file=sys.stderr)
                return False
            
            # Get current task content
            current_task = self._parse_task_from_lines(lines[task_start:task_end])
            
            # Apply updates
            updated_task = self._apply_updates(current_task, updates)
            
            # Format updated task
            updated_text = self._format_task(updated_task)
            updated_lines = updated_text.split('\n')
            
            # Replace in content
            new_lines = lines[:task_start] + updated_lines + lines[task_end:]
            new_content = '\n'.join(new_lines)
            
            return self._write_content(new_content)
    
    def delete_task(self, task_id: str) -> bool:
        """Delete a task from the org file"""
        with file_lock(self.filepath):
            if not self.filepath.exists():
                print(f"Error: File {self.filepath} does not exist", file=sys.stderr)
                return False
            
            content = self.filepath.read_text()
            lines = content.split('\n')
            
            # Find task by ID
            task_start, task_end = self._find_task_lines(lines, task_id)
            if task_start == -1:
                print(f"Error: Task {task_id} not found", file=sys.stderr)
                return False
            
            # Remove task lines
            new_lines = lines[:task_start] + lines[task_end:]
            
            # Clean up extra blank lines
            cleaned_lines = self._clean_blank_lines(new_lines)
            new_content = '\n'.join(cleaned_lines)
            
            return self._write_content(new_content)
    
    def _format_task(self, task_data: Dict[str, Any]) -> str:
        """Format task data as org-mode text"""
        lines = []
        
        # Determine level (default to 1)
        level = task_data.get('level', 1)
        stars = '*' * level
        
        # Build heading line
        state = task_data.get('state', 'TODO')
        heading = task_data.get('heading', 'New Task')
        tags = task_data.get('tags', [])
        priority = task_data.get('priority', '')
        
        heading_line = f"{stars} {state}"
        if priority:
            heading_line += f" [{priority}]"
        heading_line += f" {heading}"
        if tags:
            # Right-align tags
            tag_string = ':' + ':'.join(tags) + ':'
            padding = max(1, 80 - len(heading_line) - len(tag_string))
            heading_line += ' ' * padding + tag_string
        
        lines.append(heading_line)
        
        # Add scheduled/deadline
        if task_data.get('scheduled'):
            lines.append(f"  SCHEDULED: <{self._format_org_date(task_data['scheduled'])}>")
        if task_data.get('deadline'):
            lines.append(f"  DEADLINE: <{self._format_org_date(task_data['deadline'])}>")
        
        # Add properties
        properties = task_data.get('properties', {})
        if properties or 'id' in task_data:
            lines.append('  :PROPERTIES:')
            
            # Ensure ID is first
            if 'id' in task_data:
                lines.append(f"  :ID:          {task_data['id']}")
            
            # Add other properties
            for key, value in properties.items():
                if key.upper() != 'ID':  # Skip if ID already added
                    # Format property name
                    prop_name = f":{key.upper()}:"
                    # Pad for alignment
                    padded = prop_name.ljust(14)
                    lines.append(f"  {padded}{value}")
            
            lines.append('  :END:')
        
        # Add body
        body = task_data.get('body', '')
        if body:
            lines.append('')
            for line in body.split('\n'):
                lines.append(f"  {line}" if line else '')
        
        # Add subtasks
        subtasks = task_data.get('subtasks', [])
        if subtasks:
            if not body:
                lines.append('')
            for subtask in subtasks:
                checkbox = '[X]' if subtask.get('done') else '[ ]'
                lines.append(f"  - {checkbox} {subtask.get('text', '')}")
        
        return '\n'.join(lines)
    
    def _find_task_lines(self, lines: List[str], task_id: str) -> tuple[int, int]:
        """Find start and end line numbers for a task"""
        task_start = -1
        task_end = -1
        in_task = False
        task_level = 0
        
        for i, line in enumerate(lines):
            # Check if this line contains the task ID
            if f":ID:          {task_id}" in line or f":ID: {task_id}" in line:
                # Find the task heading (search backwards)
                for j in range(i, -1, -1):
                    if lines[j].startswith('*'):
                        task_start = j
                        task_level = lines[j].count('*')
                        in_task = True
                        break
            
            # If we're in the task, find where it ends
            if in_task and i > task_start:
                # Task ends when we hit another heading of same or higher level
                if line.startswith('*'):
                    current_level = len(line) - len(line.lstrip('*'))
                    if current_level <= task_level:
                        task_end = i
                        break
        
        # If we didn't find an end, it goes to EOF
        if task_start != -1 and task_end == -1:
            task_end = len(lines)
        
        return task_start, task_end
    
    def _parse_task_from_lines(self, lines: List[str]) -> Dict[str, Any]:
        """Parse task data from lines of text"""
        if not lines:
            return {}
        
        task_data = {}
        
        # Parse heading line
        heading_line = lines[0]
        match = re.match(r'^(\*+)\s+(\w+)\s+(?:\[([A-Z])\]\s+)?(.+?)(?:\s+(:[^:]+:))?$', heading_line)
        if match:
            level = len(match.group(1))
            state = match.group(2)
            priority = match.group(3) or ''
            heading = match.group(4)
            tags = match.group(5) or ''
            
            task_data['level'] = level
            task_data['state'] = state
            task_data['priority'] = priority
            task_data['heading'] = heading
            
            if tags:
                tag_list = tags.strip(':').split(':')
                task_data['tags'] = [t for t in tag_list if t]
        
        # Parse properties
        properties = {}
        in_properties = False
        body_lines = []
        subtasks = []
        
        for line in lines[1:]:
            if ':PROPERTIES:' in line:
                in_properties = True
            elif ':END:' in line:
                in_properties = False
            elif in_properties:
                prop_match = re.match(r'\s*:([^:]+):\s*(.+)', line)
                if prop_match:
                    key = prop_match.group(1)
                    value = prop_match.group(2).strip()
                    if key == 'ID':
                        task_data['id'] = value
                    else:
                        properties[key.lower()] = value
            elif 'SCHEDULED:' in line:
                date_match = re.search(r'<([^>]+)>', line)
                if date_match:
                    task_data['scheduled'] = date_match.group(1)
            elif 'DEADLINE:' in line:
                date_match = re.search(r'<([^>]+)>', line)
                if date_match:
                    task_data['deadline'] = date_match.group(1)
            else:
                # Check for subtasks
                subtask_match = re.match(r'\s*- \[([ X])\] (.+)', line)
                if subtask_match:
                    done = subtask_match.group(1) == 'X'
                    text = subtask_match.group(2)
                    subtasks.append({'done': done, 'text': text})
                else:
                    body_lines.append(line.lstrip())
        
        if properties:
            task_data['properties'] = properties
        if body_lines:
            task_data['body'] = '\n'.join(body_lines).strip()
        if subtasks:
            task_data['subtasks'] = subtasks
        
        return task_data
    
    def _apply_updates(self, current: Dict[str, Any], updates: Dict[str, Any]) -> Dict[str, Any]:
        """Apply updates to current task data"""
        updated = current.copy()
        
        # Direct field updates
        for field in ['state', 'heading', 'priority', 'scheduled', 'deadline', 'body']:
            if field in updates:
                updated[field] = updates[field]
        
        # Handle tags
        if 'tags' in updates:
            if updates['tags'] is None:
                updated.pop('tags', None)
            else:
                updated['tags'] = updates['tags']
        
        # Handle properties
        if 'properties' in updates:
            if 'properties' not in updated:
                updated['properties'] = {}
            updated['properties'].update(updates['properties'])
        
        # Handle subtasks
        if 'subtasks' in updates:
            updated['subtasks'] = updates['subtasks']
        
        return updated
    
    def _find_insertion_point(self, content: str, task_data: Dict[str, Any]) -> int:
        """Find the best insertion point for a new task"""
        lines = content.split('\n')
        
        # If task has a goal, try to insert under that goal
        goal = task_data.get('properties', {}).get('goal')
        if goal:
            for i, line in enumerate(lines):
                if f":ID:          {goal}" in line or f":ID: {goal}" in line:
                    # Find the end of this goal section
                    goal_level = 0
                    for j in range(i, -1, -1):
                        if lines[j].startswith('*'):
                            goal_level = lines[j].count('*')
                            break
                    
                    # Find where to insert (before next same-level heading or EOF)
                    for k in range(i + 1, len(lines)):
                        if lines[k].startswith('*'):
                            current_level = lines[k].count('*')
                            if current_level <= goal_level:
                                return k
                    
                    # Insert at end of file
                    return len(lines)
        
        # Default: append to end
        return -1
    
    def _format_org_date(self, date_string: str) -> str:
        """Format date string for org-mode"""
        # Try to parse ISO format and convert to org format
        try:
            dt = datetime.fromisoformat(date_string)
            return dt.strftime('%Y-%m-%d %a %H:%M')
        except:
            # Return as-is if we can't parse it
            return date_string
    
    def _clean_blank_lines(self, lines: List[str]) -> List[str]:
        """Remove excessive blank lines"""
        cleaned = []
        prev_blank = False
        
        for line in lines:
            is_blank = not line.strip()
            if is_blank and prev_blank:
                continue  # Skip consecutive blank lines
            cleaned.append(line)
            prev_blank = is_blank
        
        return cleaned
    
    def _get_file_header(self) -> str:
        """Get default file header for new org files"""
        return """#+TITLE: Torq Tasks
#+TODO: TODO NEXT IN-PROGRESS WAITING | DONE CANCELLED
#+STARTUP: overview
#+STARTUP: hidestars
#+STARTUP: logdone

"""
    
    def _write_content(self, content: str) -> bool:
        """Write content to file with backup and validation"""
        try:
            # Create backup if requested
            if self.backup and self.filepath.exists():
                backup_path = self.filepath.with_suffix('.org.bak')
                shutil.copy2(self.filepath, backup_path)
            
            # Write to temp file first
            with tempfile.NamedTemporaryFile(mode='w', dir=self.filepath.parent, 
                                           delete=False, suffix='.tmp') as tmp:
                tmp.write(content)
                temp_path = Path(tmp.name)
            
            # Validate if requested
            if self.validate and self.parser_path.exists():
                result = subprocess.run(
                    [sys.executable, str(self.parser_path), str(temp_path), '--validate'],
                    capture_output=True,
                    text=True
                )
                
                if result.returncode != 0:
                    print(f"Validation failed: {result.stderr}", file=sys.stderr)
                    temp_path.unlink()
                    return False
            
            # Move temp file to target
            shutil.move(str(temp_path), str(self.filepath))
            return True
            
        except Exception as e:
            print(f"Error writing file: {e}", file=sys.stderr)
            return False


def main():
    """CLI entry point"""
    parser = argparse.ArgumentParser(description='Modify org-mode task files')
    parser.add_argument('filepath', type=Path, help='Path to org file')
    parser.add_argument('action', choices=['add', 'update', 'delete'], 
                       help='Action to perform')
    parser.add_argument('data', help='JSON data for the action')
    parser.add_argument('--no-backup', action='store_true', 
                       help='Skip creating backup file')
    parser.add_argument('--no-validate', action='store_true',
                       help='Skip validation after write')
    
    args = parser.parse_args()
    
    # Parse JSON data
    try:
        data = json.loads(args.data)
    except json.JSONDecodeError as e:
        print(f"Error parsing JSON: {e}", file=sys.stderr)
        sys.exit(1)
    
    # Create writer
    writer = OrgTaskWriter(
        args.filepath,
        backup=not args.no_backup,
        validate=not args.no_validate
    )
    
    # Perform action
    success = False
    if args.action == 'add':
        success = writer.add_task(data)
    elif args.action == 'update':
        task_id = data.get('id')
        if not task_id:
            print("Error: 'id' field required for update", file=sys.stderr)
            sys.exit(1)
        success = writer.update_task(task_id, data)
    elif args.action == 'delete':
        task_id = data.get('id')
        if not task_id:
            print("Error: 'id' field required for delete", file=sys.stderr)
            sys.exit(1)
        success = writer.delete_task(task_id)
    
    if success:
        print(f"Successfully performed {args.action} on {args.filepath}")
        sys.exit(0)
    else:
        print(f"Failed to perform {args.action}", file=sys.stderr)
        sys.exit(1)


if __name__ == '__main__':
    main()