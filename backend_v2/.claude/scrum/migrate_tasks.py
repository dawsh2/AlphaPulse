#!/usr/bin/env python3
"""
Task Migration Script for AlphaPulse Self-Organizing Task System
Migrates existing task files to include new dependency metadata
"""

import yaml
import sys
import os
import re
from pathlib import Path
from typing import Dict, List, Optional, Tuple
from yaml_parser import TaskParser

class TaskMigrator:
    """Migrate existing tasks to new format with dependencies and scope"""
    
    def __init__(self):
        self.parser = TaskParser()
        self.task_dir = Path(__file__).parent.parent / "tasks"
        
        # Known dependency patterns from meta sprint
        self.known_dependencies = {
            'sprint-010': [],  # Depends on sprint-013 completion
            'sprint-006': ['sprint-010'],  # Depends on codec separation
            'sprint-007': ['sprint-010', 'sprint-006'],  # Depends on codec and macros
            'sprint-011': ['sprint-010', 'sprint-006', 'sprint-007'],  # Phase 1 complete
            'sprint-009': ['sprint-010', 'sprint-006', 'sprint-007'],  # Phase 1 complete
            'sprint-014': ['sprint-010', 'sprint-006', 'sprint-007', 'sprint-011', 'sprint-009'],
            'sprint-005': ['sprint-014'],  # Mycelium after MessageSink
            'sprint-004': ['sprint-014'],  # Mycelium after MessageSink
            'sprint-012': ['sprint-005', 'sprint-004']  # Final documentation
        }
        
    def analyze_task_scope(self, filepath: str) -> List[str]:
        """
        Analyze a task file to extract likely file modifications
        
        Args:
            filepath: Path to task file
            
        Returns:
            List of file paths/patterns that task likely modifies
        """
        with open(filepath, 'r') as f:
            content = f.read()
            
        scope = []
        
        # Look for "Files to Modify" section
        files_section = re.search(r'### Files to Modify.*?\n(.*?)(?:\n###|\n##|\Z)', 
                                  content, re.DOTALL)
        if files_section:
            lines = files_section.group(1).strip().split('\n')
            for line in lines:
                # Extract file paths from markdown lists
                match = re.search(r'[`"]?([\w/._-]+\.(?:rs|toml|md|py|sh))[`"]?', line)
                if match:
                    scope.append(match.group(1))
        
        # Look for explicit file paths in backticks
        code_paths = re.findall(r'`((?:libs|services_v2|relays|network|protocol_v2|tests)/[\w/._-]+)`', 
                                content)
        scope.extend(code_paths)
        
        # Look for Cargo.toml modifications
        if 'Cargo.toml' in content:
            if 'libs/' in content:
                scope.append('libs/*/Cargo.toml')
            elif 'services_v2/' in content:
                scope.append('services_v2/*/Cargo.toml')
            else:
                scope.append('Cargo.toml')
        
        # Remove duplicates and return
        return list(set(scope))
    
    def suggest_task_dependencies(self, task_file: str, sprint_name: str) -> Tuple[List[str], List[str]]:
        """
        Suggest dependencies based on sprint and task analysis
        
        Args:
            task_file: Path to task file
            sprint_name: Name of sprint containing task
            
        Returns:
            Tuple of (depends_on list, blocks list)
        """
        depends_on = []
        blocks = []
        
        # Extract task ID
        task_id = None
        task_data = self.parser.parse_task_file(task_file)
        if 'error' not in task_data:
            task_id = task_data['metadata'].get('task_id', '')
        
        # Apply known sprint-level dependencies
        sprint_deps = self.known_dependencies.get(sprint_name, [])
        
        # For task-level dependencies within a sprint
        if sprint_name == 'sprint-010':  # Codec separation
            if 'CODEC-001' in task_file:
                pass  # No dependencies, foundational task
            elif 'CODEC-002' in task_file:
                depends_on.append('CODEC-001')
            elif 'CODEC-003' in task_file:
                depends_on.append('CODEC-002')
            elif 'CODEC-004' in task_file:
                depends_on.append('CODEC-002')
            elif 'CODEC-005' in task_file:
                depends_on.extend(['CODEC-003', 'CODEC-004'])
                
        elif sprint_name == 'sprint-007':  # Generic relay
            if 'TASK-001' in task_file:
                pass  # Design task, no deps
            elif 'TASK-002' in task_file:
                depends_on.append('S007-T001')
            elif 'TASK-003' in task_file:
                depends_on.append('S007-T002')
            elif 'TASK-004' in task_file:
                depends_on.extend(['S007-T002', 'S007-T003'])
            elif 'TASK-005' in task_file or 'TASK-006' in task_file:
                depends_on.append('S007-T004')
                
        # Determine what this task blocks
        if task_id:
            if task_id in ['CODEC-001', 'CODEC-002']:
                blocks.extend(['S006-T001', 'S007-T001'])  # Blocks macros and relay work
            elif task_id == 'S010-T005':  # Integration testing
                blocks.append('S011-T001')  # Blocks control script work
                
        return depends_on, blocks
    
    def migrate_task(self, filepath: str, interactive: bool = True) -> bool:
        """
        Migrate a single task file to new format
        
        Args:
            filepath: Path to task file
            interactive: Whether to prompt for confirmation
            
        Returns:
            True if migration successful
        """
        task = self.parser.parse_task_file(filepath)
        if 'error' in task:
            print(f"Error parsing {filepath}: {task['error']}")
            return False
            
        metadata = task['metadata']
        filename = os.path.basename(filepath)
        sprint_name = Path(filepath).parent.name
        
        # Skip if already migrated
        if metadata.get('depends_on') is not None:
            print(f"✓ {filename} already migrated")
            return True
            
        print(f"\n📋 Migrating: {filename} ({sprint_name})")
        
        # Analyze scope
        suggested_scope = self.analyze_task_scope(filepath)
        
        # Suggest dependencies
        suggested_deps, suggested_blocks = self.suggest_task_dependencies(filepath, sprint_name)
        
        # Add new fields
        metadata['depends_on'] = metadata.get('depends_on', [])
        metadata['blocks'] = metadata.get('blocks', [])
        metadata['scope'] = metadata.get('scope', [])
        
        if interactive:
            print("\n  Suggested dependencies:")
            if suggested_deps:
                print(f"    depends_on: {suggested_deps}")
            else:
                print("    depends_on: []")
                
            if suggested_blocks:
                print(f"    blocks: {suggested_blocks}")
            else:
                print("    blocks: []")
                
            print("\n  Suggested scope:")
            if suggested_scope:
                for scope_item in suggested_scope[:5]:  # Show first 5
                    print(f"    - {scope_item}")
                if len(suggested_scope) > 5:
                    print(f"    ... and {len(suggested_scope) - 5} more")
            else:
                print("    (no scope detected)")
                
            response = input("\n  Accept suggestions? (y/n/skip): ").lower()
            
            if response == 'skip':
                print("  Skipped")
                return False
            elif response == 'y':
                metadata['depends_on'] = suggested_deps
                metadata['blocks'] = suggested_blocks
                metadata['scope'] = suggested_scope
        else:
            # Non-interactive mode: apply suggestions
            metadata['depends_on'] = suggested_deps
            metadata['blocks'] = suggested_blocks
            metadata['scope'] = suggested_scope
        
        # Write back to file
        if self.parser._write_task_file(filepath, metadata, task['content']):
            print(f"  ✓ Migrated successfully")
            return True
        else:
            print(f"  ✗ Failed to write file")
            return False
    
    def migrate_sprint(self, sprint_name: str, interactive: bool = True) -> int:
        """
        Migrate all tasks in a sprint
        
        Args:
            sprint_name: Name of sprint directory
            interactive: Whether to prompt for each task
            
        Returns:
            Number of tasks migrated
        """
        sprint_dir = self.task_dir / sprint_name
        if not sprint_dir.exists():
            print(f"Sprint directory not found: {sprint_name}")
            return 0
            
        print(f"\n🚀 Migrating sprint: {sprint_name}")
        print("=" * 40)
        
        migrated = 0
        for task_file in sorted(sprint_dir.glob('*.md')):
            # Skip non-task files
            if task_file.name in ['SPRINT_PLAN.md', 'README.md', 'TEST_RESULTS.md']:
                continue
            if 'rename_me' in task_file.name or 'template' in task_file.name.lower():
                continue
                
            if self.migrate_task(str(task_file), interactive):
                migrated += 1
                
        print(f"\n✅ Migrated {migrated} tasks in {sprint_name}")
        return migrated
    
    def migrate_critical_sprints(self):
        """Migrate the critical upcoming sprints (010, 006, 007, 011, 009, 014)"""
        critical_sprints = [
            'sprint-010-codec-separation',
            'sprint-006-protocol-optimization', 
            'sprint-007-generic-relay-refactor',
            'sprint-011-control-script-management',
            'sprint-009-testing-pyramid',
            'sprint-014-messagesink-lazy-connections'
        ]
        
        total_migrated = 0
        print("\n🎯 Migrating critical sprints for execution order")
        print("=" * 50)
        
        for sprint in critical_sprints:
            migrated = self.migrate_sprint(sprint, interactive=False)
            total_migrated += migrated
            
        print(f"\n🎉 Total tasks migrated: {total_migrated}")
        return total_migrated


def main():
    """CLI interface for task migration"""
    migrator = TaskMigrator()
    
    if len(sys.argv) < 2:
        print("Usage: migrate_tasks.py <command> [args]")
        print("Commands:")
        print("  task <file>        - Migrate a single task interactively")
        print("  sprint <name>      - Migrate all tasks in a sprint")
        print("  critical           - Migrate critical sprints (010, 006, 007, 011, 009, 014)")
        print("  all                - Migrate all sprints")
        sys.exit(1)
    
    command = sys.argv[1]
    
    if command == 'task' and len(sys.argv) > 2:
        migrator.migrate_task(sys.argv[2], interactive=True)
        
    elif command == 'sprint' and len(sys.argv) > 2:
        migrator.migrate_sprint(sys.argv[2], interactive=True)
        
    elif command == 'critical':
        migrator.migrate_critical_sprints()
        
    elif command == 'all':
        total = 0
        for sprint_dir in migrator.task_dir.glob('sprint-*'):
            if sprint_dir.is_dir() and 'archive' not in sprint_dir.name:
                total += migrator.migrate_sprint(sprint_dir.name, interactive=False)
        print(f"\n🎉 Total tasks migrated across all sprints: {total}")
        
    else:
        print(f"Unknown command: {command}")
        sys.exit(1)


if __name__ == '__main__':
    main()