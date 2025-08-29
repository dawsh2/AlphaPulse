#!/usr/bin/env python3
"""
Fix org-edna syntax errors in active.org.

The migration script created invalid syntax like:
  :BLOCKER: ids(BUILD-001-TESTS) todo?(DONE)

This should be just:
  :BLOCKER: ids(BUILD-001-TESTS)

The todo?(DONE) is a condition that org-edna checks automatically for blockers.
For triggers, todo!(NEXT) is correct as it's an action.
"""

import re
import sys
from pathlib import Path

def fix_edna_syntax(filepath):
    """Fix org-edna syntax errors."""
    with open(filepath, 'r') as f:
        content = f.read()
    
    original = content
    fixes = 0
    
    # Fix BLOCKER syntax - remove todo?(DONE) as it's implicit
    # BLOCKER checks if the referenced task is DONE automatically
    pattern = r':BLOCKER:\s+(.+?)\s+todo\?\(DONE\)'
    while re.search(pattern, content):
        content = re.sub(pattern, r':BLOCKER:     \1', content)
        fixes += 1
    
    # Fix children BLOCKER - the syntax should be just "children"
    # org-edna automatically checks if children are done
    pattern = r':BLOCKER:\s+children todo\?\(DONE\)'
    while re.search(pattern, content):
        content = re.sub(pattern, r':BLOCKER:     children', content)
        fixes += 1
    
    # TRIGGER syntax is correct - todo!(NEXT) is an action
    # But fix children TRIGGER syntax
    pattern = r':TRIGGER:\s+children todo!\(NEXT\)'
    while re.search(pattern, content):
        content = re.sub(pattern, r':TRIGGER:     children todo!(NEXT)', content)
        fixes += 1
    
    if content != original:
        # Backup original
        backup_path = filepath.with_suffix('.org.bak')
        with open(backup_path, 'w') as f:
            f.write(original)
        print(f"✅ Backed up original to {backup_path}")
        
        # Write fixed content
        with open(filepath, 'w') as f:
            f.write(content)
        print(f"✅ Fixed {fixes} syntax errors in {filepath}")
        
        # Show some examples of fixes
        print("\nExample fixes:")
        print("  Before: :BLOCKER: ids(BUILD-001-TESTS) todo?(DONE)")
        print("  After:  :BLOCKER: ids(BUILD-001-TESTS)")
        print()
        print("  Before: :BLOCKER: children todo?(DONE)")  
        print("  After:  :BLOCKER: children")
        
        return True
    else:
        print(f"❌ No syntax errors found to fix")
        return False

if __name__ == '__main__':
    filepath = Path('/Users/daws/alphapulse/backend_v2/.claude/tasks/active.org')
    
    if not filepath.exists():
        print(f"Error: {filepath} not found")
        sys.exit(1)
    
    if fix_edna_syntax(filepath):
        print("\n🎉 Syntax fixed! Now test with emacs...")
    else:
        print("\n⚠️  No changes needed")