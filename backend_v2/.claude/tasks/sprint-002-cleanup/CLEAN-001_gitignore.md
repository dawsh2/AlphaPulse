# CLEAN-001: Update .gitignore

## Task Overview
**Sprint**: 002-cleanup
**Priority**: CRITICAL
**Estimate**: 1 hour
**Status**: TODO

## Problem
The current .gitignore is dangerously sparse, allowing build artifacts and temporary files to be tracked in git.

## Acceptance Criteria
- [ ] Comprehensive .gitignore file created
- [ ] All existing tracked artifacts removed from git
- [ ] No build artifacts in future commits

## Implementation Steps

### 1. Create comprehensive .gitignore
```gitignore
# Rust
/target
**/target/
Cargo.lock
**/*.rs.bk
*.pdb

# Python
__pycache__/
*.py[cod]
*~
.pytest_cache/
*.pyc

# Editors
.vscode/
.idea/
*.swp
*.swo
*~
*.bak
\#*\#
.\#*

# macOS
.DS_Store
.AppleDouble
.LSOverride

# Temporary files
*.tmp
*.temp
debug_*
profile_*
test_actual*
test_websocket*

# Logs
*.log
logs/

# Environment
.env
.env.local
```

### 2. Remove already-tracked files
```bash
# Remove tracked build artifacts
git rm -r --cached target/
git rm -r --cached backend_v2/target/
git rm -r --cached frontend/node_modules/

# Remove editor backups
git rm --cached '*~'
git rm --cached '#*#'
```

### 3. Verify and commit
```bash
# Verify no artifacts remain
git status --ignored

# Commit the cleanup
git add .gitignore
git commit -m "fix: Add comprehensive .gitignore to prevent artifact tracking"
```

## Testing
- Run `cargo build` and verify target/ is not shown in `git status`
- Create a backup file (file~) and verify it's ignored
- Check `git status --ignored` shows expected ignores

## Notes
- Use `git rm --cached` to untrack without deleting local files
- Some Cargo.lock files may be intentionally tracked for binaries
- Check with team before removing any seemingly important files
