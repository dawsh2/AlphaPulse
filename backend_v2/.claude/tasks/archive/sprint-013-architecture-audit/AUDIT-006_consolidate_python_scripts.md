---
task_id: AUDIT-006
status: MOVED_TO_BACKLOG
priority: LOW
estimated_hours: 2
assigned_branch: feat/consolidate-python-scripts
assignee: TBD
created: 2025-08-26
completed: null
moved_to_backlog: 2025-08-27
depends_on:
  - AUDIT-005  # Should consolidate after manage.sh is created
blocks: []
scope:
  - "scripts/lib/python/"  # New Python utility directory
  - "scripts/*.py"  # Existing Python scripts to reorganize
---

# AUDIT-006: Consolidate Python Scripts

## Git Worktree Setup (REQUIRED)
```bash
# Create worktree for this task
git worktree add -b feat/consolidate-python-scripts ../audit-006-worktree
cd ../audit-006-worktree
```

## Status
**Status**: TODO
**Priority**: LOW
**Worktree**: `../audit-006-worktree` (Branch: `feat/consolidate-python-scripts`)
**Estimated**: 2 hours

## Problem Statement
Python scripts are scattered throughout the scripts/ directory without organization. Many scripts may be obsolete, duplicated, or better integrated into the unified manage.sh system.

## Acceptance Criteria
- [ ] Audit all Python scripts in scripts/ directory
- [ ] Identify which scripts are still needed vs obsolete
- [ ] Move utility scripts to `scripts/lib/python/`
- [ ] Remove obsolete/duplicate scripts
- [ ] Update manage.sh to call remaining scripts appropriately
- [ ] Document script purposes and usage
- [ ] Ensure no functionality is lost

## Target Structure
```
scripts/
├── manage.sh          # Main control (calls Python when needed)
└── lib/
    ├── startup.sh     # Shell scripts
    ├── shutdown.sh
    └── python/        # Python utilities
        ├── __init__.py
        ├── data_validation.py  # Keep if needed
        ├── metrics_collection.py  # Keep if needed
        └── utilities.py  # Consolidated utilities
```

## Implementation Steps
1. **Audit Existing Scripts**
   - List all `.py` files in scripts/
   - Analyze each script's purpose and usage
   - Check git history for recent usage
   - Identify dependencies and callers

2. **Categorize Scripts**
   - **Keep**: Still needed, actively used
   - **Consolidate**: Merge similar functionality
   - **Obsolete**: Remove completely
   - **Integrate**: Move logic into manage.sh

3. **Reorganize Structure**
   - Create `scripts/lib/python/` directory
   - Move kept scripts to proper location
   - Create unified utility modules where appropriate

4. **Update References**
   - Update manage.sh to call relocated scripts
   - Fix any hardcoded paths in other scripts
   - Update documentation references

5. **Clean Up**
   - Remove obsolete scripts
   - Add README.md explaining script organization
   - Ensure executable permissions are correct

## Files to Create/Modify
- `scripts/lib/python/` - New Python utility directory
- `scripts/lib/python/__init__.py` - Python module initialization
- `scripts/lib/python/README.md` - Documentation for Python utilities
- Move/consolidate existing `.py` files as appropriate
- Update `scripts/manage.sh` to reference relocated scripts
- Remove obsolete Python scripts

## Script Audit Checklist
For each Python script in scripts/:
- [ ] What does this script do?
- [ ] When was it last modified/used?
- [ ] Are there any callers or dependencies?
- [ ] Can it be consolidated with similar scripts?
- [ ] Is the functionality needed in manage.sh instead?
- [ ] Should it be kept, moved, or removed?

## Success Criteria
- All Python scripts are properly organized
- No obsolete or duplicate scripts remain
- manage.sh correctly calls any needed Python utilities
- Script organization is documented and clear
- No functionality is lost in the consolidation