#!/usr/bin/env python3
"""
Task Linter for AlphaPulse Self-Organizing Task System
Validates task metadata to ensure data integrity
"""

import sys
import os
import re
from pathlib import Path
from typing import Dict, List, Tuple, Optional
from yaml_parser import TaskParser
import json

class TaskLinter:
    """Validate task files for metadata completeness and correctness"""
    
    def __init__(self):
        self.parser = TaskParser()
        self.task_dir = Path(__file__).parent.parent / "tasks"
        
        # Validation rules
        self.required_fields = ['task_id', 'status', 'priority']
        self.valid_statuses = ['TODO', 'IN_PROGRESS', 'COMPLETE', 'DONE', 'BLOCKED']
        self.valid_priorities = ['CRITICAL', 'HIGH', 'MEDIUM', 'LOW']
        self.task_id_pattern = re.compile(r'^[A-Z]+-\d+$|^S\d{3}-T\d{3}$')
        
    def lint_task(self, filepath: str, strict: bool = False) -> Tuple[bool, List[str], List[str]]:
        """
        Lint a single task file
        
        Args:
            filepath: Path to task file
            strict: Whether to enforce strict validation rules
            
        Returns:
            Tuple of (is_valid, errors, warnings)
        """
        errors = []
        warnings = []
        
        # Check file exists
        if not os.path.exists(filepath):
            errors.append(f"File not found: {filepath}")
            return False, errors, warnings
            
        # Skip non-task files
        filename = os.path.basename(filepath)
        non_task_files = [
            'SPRINT_PLAN.md', 'README.md', 'TEST_RESULTS.md', 'STATUS.md',
            'REMAINING_ISSUES.md', 'EXECUTION_TRACKER.md', 'dependencies.md',
            'task-breakdown.md', 'COMPLETION_REPORT.md', 'GITHUB_ISSUES.md',
            'POST_REVIEW_FIXES.md', 'TODO_AUDIT.md', 'ARCHIVED.md'
        ]
        
        # Skip MVP files and other documentation
        if filename in non_task_files or filename.startswith('MVP-'):
            return True, [], []  # These files don't need task metadata
        if 'rename_me' in filename or 'template' in filename.lower():
            if strict:
                warnings.append(f"Template file found: {filename}")
            return True, [], warnings
            
        # Parse task file
        task = self.parser.parse_task_file(filepath)
        
        # Check for parsing errors
        if 'error' in task:
            errors.append(f"Parsing error: {task['error']}")
            return False, errors, warnings
            
        metadata = task['metadata']
        
        # 1. Check required fields
        for field in self.required_fields:
            if field not in metadata or metadata[field] is None or metadata[field] == '':
                errors.append(f"Missing required field: {field}")
                
        # 2. Validate task_id format
        task_id = metadata.get('task_id', '')
        if task_id:
            if not self.task_id_pattern.match(task_id):
                errors.append(f"Invalid task_id format: {task_id} (expected SXXX-TXXX or PREFIX-NNN)")
                
            # Check if task_id matches filename pattern
            if task_id not in filename:
                warnings.append(f"Task ID '{task_id}' not found in filename '{filename}'")
                
        # 3. Validate status
        status = metadata.get('status', '')
        if status and status not in self.valid_statuses:
            errors.append(f"Invalid status: {status} (must be one of {', '.join(self.valid_statuses)})")
            
        # 4. Validate priority
        priority = metadata.get('priority', '')
        if priority and priority not in self.valid_priorities:
            warnings.append(f"Invalid priority: {priority} (should be one of {', '.join(self.valid_priorities)})")
            
        # 5. Check dependencies logic
        depends_on = metadata.get('depends_on', [])
        blocks = metadata.get('blocks', [])
        scope = metadata.get('scope', [])
        
        # Ensure these fields exist (even if empty)
        if 'depends_on' not in metadata:
            errors.append("Missing 'depends_on' field (use empty list [] if no dependencies)")
        if 'blocks' not in metadata:
            warnings.append("Missing 'blocks' field (use empty list [] if nothing blocked)")
        if 'scope' not in metadata:
            warnings.append("Missing 'scope' field (use empty list [] if no file modifications)")
            
        # 6. Status-specific validation
        if status == 'IN_PROGRESS' or status == 'COMPLETE' or status == 'DONE':
            # Check if dependencies were considered
            if 'depends_on' not in metadata:
                errors.append(f"Task with status {status} must have 'depends_on' field (use [] for root tasks)")
                
            # Warn if no scope defined for completed work
            if status in ['COMPLETE', 'DONE'] and not scope:
                warnings.append(f"Completed task has no scope defined - what files were modified?")
                
        # 7. Check for circular self-dependency
        if task_id and task_id in depends_on:
            errors.append(f"Circular dependency: task depends on itself")
            
        # 8. Check for duplicate dependencies
        if len(depends_on) != len(set(depends_on)):
            warnings.append("Duplicate entries in depends_on list")
            
        # 9. Validate dependency format
        for dep in depends_on:
            if not isinstance(dep, str):
                errors.append(f"Invalid dependency format: {dep} (must be string)")
            elif not re.match(r'^[A-Z0-9-]+$', dep):
                warnings.append(f"Suspicious dependency format: {dep}")
                
        # 10. Check scope format
        for scope_item in scope:
            if not isinstance(scope_item, str):
                errors.append(f"Invalid scope format: {scope_item} (must be string)")
                
        # 11. Cross-reference validation (if strict mode)
        if strict:
            # Check if dependencies actually exist
            all_tasks = self.parser.get_all_tasks()
            existing_task_ids = {t['metadata'].get('task_id', '') for t in all_tasks}
            
            for dep in depends_on:
                if dep and dep not in existing_task_ids:
                    warnings.append(f"Dependency '{dep}' not found in project")
                    
            for blocked in blocks:
                if blocked and blocked not in existing_task_ids:
                    warnings.append(f"Blocked task '{blocked}' not found in project")
                    
            # Check for scope conflicts without dependencies
            if scope:
                conflicts = self._check_scope_conflicts_without_deps(
                    task_id, scope, depends_on, all_tasks
                )
                for conflict in conflicts:
                    warnings.append(f"Task '{conflict}' modifies same files but no dependency defined")
                    
        # Determine overall validity
        is_valid = len(errors) == 0
        
        return is_valid, errors, warnings
    
    def _check_scope_conflicts_without_deps(
        self, task_id: str, scope: List[str], depends_on: List[str], all_tasks: List[Dict]
    ) -> List[str]:
        """
        Find tasks with overlapping scope but no dependency relationship
        
        Returns:
            List of conflicting task IDs
        """
        conflicts = []
        
        for task in all_tasks:
            other_id = task['metadata'].get('task_id', '')
            if not other_id or other_id == task_id:
                continue
                
            other_scope = set(task['metadata'].get('scope', []))
            our_scope = set(scope)
            
            # Check for overlap
            if our_scope.intersection(other_scope):
                # Check if there's a dependency relationship
                other_deps = task['metadata'].get('depends_on', [])
                other_blocks = task['metadata'].get('blocks', [])
                
                has_relationship = (
                    other_id in depends_on or
                    task_id in other_deps or
                    task_id in task['metadata'].get('blocks', []) or
                    other_id in task['metadata'].get('blocks', [])
                )
                
                if not has_relationship:
                    conflicts.append(other_id)
                    
        return conflicts
    
    def lint_directory(self, directory: str, strict: bool = False) -> Tuple[int, int, int]:
        """
        Lint all task files in a directory
        
        Args:
            directory: Directory path to lint
            strict: Whether to enforce strict validation
            
        Returns:
            Tuple of (total_files, files_with_errors, files_with_warnings)
        """
        dir_path = Path(directory)
        if not dir_path.exists():
            print(f"Directory not found: {directory}")
            return 0, 0, 0
            
        total = 0
        error_count = 0
        warning_count = 0
        
        # Find all markdown files
        for md_file in dir_path.glob('**/*.md'):
            # Skip archive directory
            if 'archive' in str(md_file):
                continue
                
            total += 1
            is_valid, errors, warnings = self.lint_task(str(md_file), strict)
            
            if errors:
                error_count += 1
                print(f"\n❌ {md_file.name}:")
                for error in errors:
                    print(f"   ERROR: {error}")
                    
            if warnings:
                warning_count += 1
                if not errors:  # Only print filename once
                    print(f"\n⚠️  {md_file.name}:")
                for warning in warnings:
                    print(f"   WARN: {warning}")
                    
        return total, error_count, warning_count
    
    def generate_report(self, directory: str = None) -> Dict:
        """
        Generate comprehensive linting report for all tasks
        
        Args:
            directory: Directory to lint (default: all tasks)
            
        Returns:
            Dictionary with linting statistics
        """
        if directory is None:
            directory = str(self.task_dir)
            
        all_tasks = self.parser.get_all_tasks()
        
        report = {
            'total_tasks': len(all_tasks),
            'valid_tasks': 0,
            'tasks_with_errors': 0,
            'tasks_with_warnings': 0,
            'missing_dependencies': 0,
            'missing_scope': 0,
            'invalid_status': 0,
            'invalid_format': 0,
            'error_details': [],
            'warning_details': []
        }
        
        for task in all_tasks:
            is_valid, errors, warnings = self.lint_task(task['filepath'], strict=True)
            
            if is_valid and not errors:
                report['valid_tasks'] += 1
            else:
                report['tasks_with_errors'] += 1
                
            if warnings:
                report['tasks_with_warnings'] += 1
                
            # Categorize issues
            for error in errors:
                if 'depends_on' in error:
                    report['missing_dependencies'] += 1
                elif 'status' in error:
                    report['invalid_status'] += 1
                elif 'format' in error or 'task_id' in error:
                    report['invalid_format'] += 1
                    
                report['error_details'].append({
                    'file': Path(task['filepath']).name,
                    'error': error
                })
                
            for warning in warnings:
                if 'scope' in warning:
                    report['missing_scope'] += 1
                    
                report['warning_details'].append({
                    'file': Path(task['filepath']).name,
                    'warning': warning
                })
                
        report['health_score'] = round(
            (report['valid_tasks'] / report['total_tasks'] * 100) if report['total_tasks'] > 0 else 0,
            1
        )
        
        return report


def main():
    """CLI interface for task linter"""
    linter = TaskLinter()
    
    if len(sys.argv) < 2:
        print("Usage: task_linter.py <command> [args]")
        print("Commands:")
        print("  lint <file>      - Lint a single task file")
        print("  lint-dir <dir>   - Lint all tasks in directory")
        print("  lint-strict <file> - Lint with strict validation")
        print("  report           - Generate full linting report")
        print("  check            - Quick health check (exit code)")
        sys.exit(1)
        
    command = sys.argv[1]
    
    if command == 'lint' and len(sys.argv) > 2:
        filepath = sys.argv[2]
        is_valid, errors, warnings = linter.lint_task(filepath)
        
        if errors:
            print(f"❌ Validation failed for {filepath}:")
            for error in errors:
                print(f"  ERROR: {error}")
        
        if warnings:
            print(f"⚠️  Warnings for {filepath}:")
            for warning in warnings:
                print(f"  WARN: {warning}")
                
        if is_valid and not warnings:
            print(f"✅ {filepath} is valid")
            
        sys.exit(0 if is_valid else 1)
        
    elif command == 'lint-dir' and len(sys.argv) > 2:
        directory = sys.argv[2]
        total, errors, warnings = linter.lint_directory(directory)
        
        print(f"\n📊 Linting Summary:")
        print(f"  Total files: {total}")
        print(f"  Files with errors: {errors}")
        print(f"  Files with warnings: {warnings}")
        
        if errors == 0:
            print(f"\n✅ All task files valid!")
        else:
            print(f"\n❌ {errors} files have errors that must be fixed")
            
        sys.exit(0 if errors == 0 else 1)
        
    elif command == 'lint-strict' and len(sys.argv) > 2:
        filepath = sys.argv[2]
        is_valid, errors, warnings = linter.lint_task(filepath, strict=True)
        
        # In strict mode, warnings are treated as errors
        if errors or warnings:
            print(f"❌ Strict validation failed for {filepath}:")
            for error in errors:
                print(f"  ERROR: {error}")
            for warning in warnings:
                print(f"  STRICT: {warning}")
        else:
            print(f"✅ {filepath} passes strict validation")
            
        sys.exit(0 if is_valid and not warnings else 1)
        
    elif command == 'report':
        report = linter.generate_report()
        
        print("\n📋 Task Metadata Health Report")
        print("=" * 40)
        print(f"Total Tasks: {report['total_tasks']}")
        print(f"Valid Tasks: {report['valid_tasks']}")
        print(f"Health Score: {report['health_score']}%")
        print(f"\nIssues Found:")
        print(f"  Tasks with errors: {report['tasks_with_errors']}")
        print(f"  Tasks with warnings: {report['tasks_with_warnings']}")
        print(f"  Missing dependencies field: {report['missing_dependencies']}")
        print(f"  Missing scope field: {report['missing_scope']}")
        print(f"  Invalid status: {report['invalid_status']}")
        print(f"  Invalid format: {report['invalid_format']}")
        
        if report['health_score'] < 80:
            print(f"\n⚠️  Health score below 80% - run migrate_tasks.py to fix")
            
        sys.exit(0 if report['health_score'] >= 95 else 1)
        
    elif command == 'check':
        # Quick check for CI - just return exit code
        report = linter.generate_report()
        sys.exit(0 if report['tasks_with_errors'] == 0 else 1)
        
    else:
        print(f"Unknown command: {command}")
        sys.exit(1)


if __name__ == '__main__':
    main()