# CLEAN-005: Clean Commented Code

## Task Overview
**Sprint**: 002-cleanup
**Priority**: MEDIUM
**Estimate**: 4 hours
**Status**: TODO

## Problem
Large blocks of commented-out code clutter the implementation, especially in OpportunityDetector and similar files. Version control should be used instead of comments.

## Areas to Clean

### High Priority Files
- [ ] `OpportunityDetector` implementation
- [ ] Strategy implementations in `services_v2/strategies/`
- [ ] Relay implementations in `relays/`

### Types of Comments to Remove
- [ ] Large blocks of commented code (>5 lines)
- [ ] Old implementations kept "just in case"
- [ ] Debugging code that's commented out
- [ ] Alternative approaches in comments

### Comments to KEEP
- [ ] Explanatory comments about WHY code works a certain way
- [ ] Documentation comments (///, //!)
- [ ] Warning comments about gotchas
- [ ] TODO/FIXME that are still valid

## Implementation Steps

### 1. Find large comment blocks
```bash
# Find files with many consecutive comment lines
find . -name "*.rs" -exec sh -c \
  'echo "$1: $(grep -c "^[[:space:]]*//" "$1")"' _ {} \; | \
  sort -t: -k2 -rn | head -20

# Find specific problem file
grep -n "^[[:space:]]*//" services_v2/strategies/*/src/detector.rs | head -50
```

### 2. Review comment blocks
For each large comment block:
1. Check if it's explanatory (KEEP) or old code (REMOVE)
2. Check git history to see when it was commented
3. Verify the active code has replaced it properly

### 3. Clean specific files
```rust
// Example: In OpportunityDetector or similar

// REMOVE blocks like this:
// fn old_detection_logic() {
//     // 50 lines of old code
//     // that's been replaced
// }

// KEEP comments like this:
// The detection threshold is 0.5% because lower values
// result in too many false positives due to gas costs
```

### 4. Use git for archaeology
```bash
# Before removing, ensure it's in git history
git log -p --follow path/to/file.rs | less

# Can always retrieve old code with:
git show COMMIT:path/to/file.rs
```

### 5. Commit the cleanup
```bash
# Commit in logical chunks
git add -p  # Interactively stage changes
git commit -m "chore: Remove commented code blocks from OpportunityDetector

- Removed old implementation kept in comments (available in git history)
- Kept explanatory comments about business logic
- Reduced file size by ~30%"
```

## Validation
- [ ] No commented code blocks >5 lines
- [ ] All remaining comments are explanatory
- [ ] Code is more readable
- [ ] `cargo build` and `cargo test` still pass
- [ ] File sizes reduced

## Guidelines
```
REMOVE:
// let old_var = calculate_old(); // Old approach
// fn entire_old_function() { ... }
// Alternative: could also do this...

KEEP:
// WARNING: Must use u128 to prevent overflow
// This threshold prevents MEV attacks
// TODO: Optimize this hot path
```

## Notes
- Use git history, not comments, for old code
- If unsure, check with team or keep the comment
- Focus on the worst offenders first
- Consider adding better documentation for complex logic
