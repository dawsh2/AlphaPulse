# CLEAN-002: Remove Backup and Temporary Files

## Task Overview
**Sprint**: 002-cleanup
**Priority**: CRITICAL
**Estimate**: 2 hours
**Status**: TODO
**Dependencies**: CLEAN-001

## Problem
Editor backup files and temporary Cargo configurations are littering the repository.

## Files to Remove

### Editor Backups
- [ ] `README.org~`
- [ ] `#README.md#`
- [ ] `#README.org#`
- [ ] Any other `*~` files
- [ ] Any other `#*#` files

### Temporary Configs
- [ ] `Cargo_precision.toml`
- [ ] `Cargo_temp.toml`

### Debug/Test Files (if one-offs)
- [ ] Files starting with `debug_` that are one-time use
- [ ] Files starting with `test_` that aren't proper tests

## Implementation Steps

### 1. Identify all backup files
```bash
# Find all backup files
find . -name "*~" -o -name "#*#" | grep -v .git

# Find temporary Cargo files
find . -name "Cargo_*.toml" | grep -v Cargo.toml
```

### 2. Review before deletion
```bash
# Check what these files contain (in case something important)
ls -la README.org~ Cargo_precision.toml Cargo_temp.toml
```

### 3. Remove files
```bash
# Remove editor backups
git rm README.org~
git rm '#README.md#'
git rm '#README.org#'

# Remove temp configs
git rm Cargo_precision.toml
git rm Cargo_temp.toml

# Find and remove all backup files
find . -name "*~" -exec git rm {} \;
find . -name "#*#" -exec git rm {} \;
```

### 4. Commit the cleanup
```bash
git commit -m "chore: Remove editor backup files and temporary configs

- Removed Emacs/Vim backup files (*~, #*#)
- Removed temporary Cargo configuration files
- These are now prevented by updated .gitignore"
```

## Validation
- [ ] No files matching `*~` in repository
- [ ] No files matching `#*#` in repository
- [ ] No `Cargo_*.toml` files except `Cargo.toml`
- [ ] Git status is clean

## Notes
- Ensure CLEAN-001 is complete first so these don't return
- Double-check files before deletion in case of important content
- Use `git rm` to remove from both git and filesystem
