#!/usr/bin/env python3
"""
Migrate org-mode tasks from simple :DEPENDS: to org-edna TRIGGER/BLOCKER properties.

This script converts the existing dependency system to the more powerful org-edna format,
enabling automatic state transitions and bidirectional dependencies.
"""

import re
import sys
from pathlib import Path
from typing import Dict, List, Tuple
import argparse

class OrgEdnaMigrator:
    """Convert org-mode dependencies to org-edna format."""
    
    def __init__(self, dry_run: bool = True):
        self.dry_run = dry_run
        self.task_map = {}  # ID -> (heading, level, line_num)
        self.conversions = []
        
    def parse_org_file(self, content: str) -> List[Dict]:
        """Parse org file and extract task information."""
        tasks = []
        lines = content.split('\n')
        current_task = None
        
        for i, line in enumerate(lines):
            # Match org headings
            heading_match = re.match(r'^(\*+)\s+(\w+)\s+(.*?)(?:\s+:([\w:]+):)?$', line)
            if heading_match:
                if current_task:
                    tasks.append(current_task)
                    
                level = len(heading_match.group(1))
                state = heading_match.group(2) if heading_match.group(2) in ['TODO', 'NEXT', 'IN-PROGRESS', 'DONE', 'CANCELLED'] else None
                title = heading_match.group(3) if state else f"{heading_match.group(2)} {heading_match.group(3)}"
                tags = heading_match.group(4) if heading_match.group(4) else ""
                
                current_task = {
                    'line_num': i,
                    'level': level,
                    'state': state,
                    'title': title,
                    'tags': tags,
                    'properties': {},
                    'full_heading': line
                }
            # Match properties
            elif current_task and line.strip().startswith(':'):
                prop_match = re.match(r'^\s*:([A-Z\-_]+):\s*(.*)$', line)
                if prop_match:
                    prop_name = prop_match.group(1)
                    prop_value = prop_match.group(2)
                    current_task['properties'][prop_name] = {
                        'value': prop_value,
                        'line_num': i
                    }
        
        if current_task:
            tasks.append(current_task)
            
        return tasks
    
    def build_task_map(self, tasks: List[Dict]) -> None:
        """Build a map of task IDs to task info for quick lookup."""
        for task in tasks:
            if 'ID' in task['properties']:
                task_id = task['properties']['ID']['value']
                self.task_map[task_id] = task
    
    def convert_simple_depends(self, task: Dict) -> Tuple[str, str]:
        """Convert simple :DEPENDS: to BLOCKER property."""
        if 'DEPENDS' not in task['properties']:
            return None, None
            
        depends_value = task['properties']['DEPENDS']['value']
        dep_ids = depends_value.split()
        
        # Build BLOCKER property
        if len(dep_ids) == 1:
            blocker = f"ids({dep_ids[0]}) todo?(DONE)"
        else:
            id_list = ' '.join(dep_ids)
            blocker = f"ids({id_list}) todo?(DONE)"
        
        # For TDD pattern, also create TRIGGER on test tasks
        trigger = None
        task_id = task['properties'].get('ID', {}).get('value', '')
        
        # Check if this is an implementation task depending on a test task
        if task_id and '-TESTS' not in task_id:
            for dep_id in dep_ids:
                if '-TESTS' in dep_id:
                    # This is TDD pattern - add TRIGGER to test task
                    if dep_id in self.task_map:
                        test_task = self.task_map[dep_id]
                        if 'TRIGGER' not in test_task['properties']:
                            trigger = (dep_id, f"ids({task_id}) todo!(NEXT)")
                    
        return blocker, trigger
    
    def detect_patterns(self, task: Dict, tasks: List[Dict]) -> Dict[str, str]:
        """Detect common patterns and suggest edna properties."""
        suggestions = {}
        task_id = task['properties'].get('ID', {}).get('value', '')
        
        # TDD Pattern: Test tasks should trigger implementation
        if task_id and task_id.endswith('-TESTS'):
            impl_id = task_id.replace('-TESTS', '')
            if impl_id in self.task_map:
                suggestions['TRIGGER'] = f"ids({impl_id}) todo!(NEXT)"
        
        # Goal pattern: Parent tasks with children
        if task['level'] == 1 and task['state'] == 'TODO':
            # Check if has children
            has_children = False
            for other in tasks:
                if other['level'] == task['level'] + 1:
                    has_children = True
                    break
            
            if has_children:
                suggestions['BLOCKER'] = "children todo?(DONE)"
                suggestions['TRIGGER'] = "children todo!(NEXT)"
        
        # Sequential pattern: Tasks at same level
        task_idx = tasks.index(task)
        if task_idx > 0 and tasks[task_idx - 1]['level'] == task['level']:
            # Has previous sibling
            if 'BLOCKER' not in suggestions:
                prev_task = tasks[task_idx - 1]
                if 'ID' in prev_task['properties']:
                    prev_id = prev_task['properties']['ID']['value']
                    suggestions['BLOCKER'] = f"ids({prev_id}) todo?(DONE)"
        
        return suggestions
    
    def migrate_file(self, filepath: Path) -> str:
        """Migrate a single org file to org-edna format."""
        with open(filepath, 'r') as f:
            content = f.read()
            original_content = content
        
        lines = content.split('\n')
        tasks = self.parse_org_file(content)
        self.build_task_map(tasks)
        
        # Track changes
        changes = []
        triggers_to_add = {}  # task_id -> trigger property
        
        # First pass: collect conversions
        for task in tasks:
            if 'DEPENDS' in task['properties']:
                blocker, trigger_info = self.convert_simple_depends(task)
                
                if blocker:
                    changes.append({
                        'task': task,
                        'add_blocker': blocker,
                        'remove_depends': True
                    })
                    
                if trigger_info:
                    test_id, trigger = trigger_info
                    triggers_to_add[test_id] = trigger
            
            # Add pattern-based suggestions
            suggestions = self.detect_patterns(task, tasks)
            for prop_name, prop_value in suggestions.items():
                if prop_name not in task['properties']:
                    changes.append({
                        'task': task,
                        f'add_{prop_name.lower()}': prop_value
                    })
        
        # Add triggers to test tasks
        for task_id, trigger in triggers_to_add.items():
            if task_id in self.task_map:
                task = self.task_map[task_id]
                changes.append({
                    'task': task,
                    'add_trigger': trigger
                })
        
        # Apply changes (from bottom to top to preserve line numbers)
        changes.sort(key=lambda c: c['task']['line_num'], reverse=True)
        
        for change in changes:
            task = change['task']
            props_end_line = task['line_num'] + 1
            
            # Find end of properties drawer
            for i in range(task['line_num'] + 1, len(lines)):
                if lines[i].strip() == ':END:':
                    props_end_line = i
                    break
            
            # Remove DEPENDS if needed
            if change.get('remove_depends') and 'DEPENDS' in task['properties']:
                dep_line = task['properties']['DEPENDS']['line_num']
                if not self.dry_run:
                    lines[dep_line] = ''  # Remove the line
                print(f"  - Remove :DEPENDS: from {task['title']}")
            
            # Add BLOCKER
            if 'add_blocker' in change:
                blocker_line = f"   :BLOCKER:     {change['add_blocker']}"
                if not self.dry_run:
                    lines.insert(props_end_line, blocker_line)
                print(f"  + Add :BLOCKER: to {task['title']}")
                print(f"    {change['add_blocker']}")
            
            # Add TRIGGER  
            if 'add_trigger' in change:
                trigger_line = f"   :TRIGGER:     {change['add_trigger']}"
                if not self.dry_run:
                    lines.insert(props_end_line, trigger_line)
                print(f"  + Add :TRIGGER: to {task['title']}")
                print(f"    {change['add_trigger']}")
        
        # Clean up empty lines
        lines = [line for line in lines if line != '']
        new_content = '\n'.join(lines)
        
        if not self.dry_run and new_content != original_content:
            with open(filepath, 'w') as f:
                f.write(new_content)
            print(f"\n✅ Migrated {filepath}")
        elif self.dry_run:
            print(f"\n🔍 Dry run - no changes written to {filepath}")
        
        return new_content
    
    def add_edna_header(self, filepath: Path) -> None:
        """Add org-edna configuration to file header."""
        with open(filepath, 'r') as f:
            content = f.read()
        
        if '#+PROPERTY: TRIGGER' in content:
            print("  ℹ️  Edna headers already present")
            return
            
        lines = content.split('\n')
        
        # Find where to insert (after #+TODO line)
        insert_idx = 0
        for i, line in enumerate(lines):
            if line.startswith('#+TODO:'):
                insert_idx = i + 1
                break
        
        edna_config = [
            "#+PROPERTY: ORDERED true",
            "#+PROPERTY: TRIGGER_ALL true",
            "#+PROPERTY: BLOCKER_ALL true",
            ""
        ]
        
        for config_line in reversed(edna_config):
            lines.insert(insert_idx, config_line)
        
        if not self.dry_run:
            with open(filepath, 'w') as f:
                f.write('\n'.join(lines))
            print("  + Added org-edna headers")

def main():
    parser = argparse.ArgumentParser(description='Migrate org tasks to org-edna format')
    parser.add_argument('file', help='Path to org file')
    parser.add_argument('--dry-run', action='store_true', default=True,
                       help='Show what would be changed without modifying files')
    parser.add_argument('--apply', action='store_true',
                       help='Actually apply the changes')
    
    args = parser.parse_args()
    
    if args.apply:
        args.dry_run = False
        
    filepath = Path(args.file)
    if not filepath.exists():
        print(f"Error: {filepath} not found")
        sys.exit(1)
    
    print(f"🔄 Migrating {filepath} to org-edna format")
    print("=" * 50)
    
    migrator = OrgEdnaMigrator(dry_run=args.dry_run)
    
    # Add edna headers
    migrator.add_edna_header(filepath)
    
    # Migrate dependencies
    migrator.migrate_file(filepath)
    
    if args.dry_run:
        print("\n⚠️  This was a dry run. Use --apply to make changes.")
    else:
        print("\n✨ Migration complete!")

if __name__ == '__main__':
    main()