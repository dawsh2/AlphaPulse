# CLEAN-006: Process TODO/FIXME Comments

## Task Overview
**Sprint**: 002-cleanup
**Priority**: MEDIUM
**Estimate**: 3 hours
**Status**: TODO

## Problem
Many TODO and FIXME comments exist throughout the codebase. These should be tracked properly or resolved.

## Process for Each TODO/FIXME

### Categories
1. **OBSOLETE**: No longer relevant, can be removed
2. **IMMEDIATE**: Can be fixed now in <30 minutes
3. **TASK**: Needs to be tracked as a proper task
4. **DOCUMENT**: Needs investigation/documentation

## Implementation Steps

### 1. Audit all TODOs and FIXMEs
```bash
# Count current TODOs and FIXMEs
echo "TODOs: $(grep -r "TODO" --include="*.rs" --include="*.py" | wc -l)"
echo "FIXMEs: $(grep -r "FIXME" --include="*.rs" --include="*.py" | wc -l)"

# List all with context
grep -rn "TODO\|FIXME" --include="*.rs" --include="*.py" > todo_audit.txt
```

### 2. Categorize each item
Review todo_audit.txt and categorize:

```markdown
## OBSOLETE (Remove)
- [ ] `protocol_v2/src/lib.rs:45` - TODO: Remove after migration (migration complete)

## IMMEDIATE (Fix now)
- [ ] `services_v2/adapters/src/lib.rs:23` - TODO: Add missing error type
- [ ] `relays/src/main.rs:67` - FIXME: Handle unwrap properly

## TASK (Create issue)
- [ ] `strategies/arbitrage/src/lib.rs:89` - TODO: Implement slippage calculation
- [ ] `protocol_v2/src/tlv/builder.rs:34` - TODO: Optimize allocation strategy

## INVESTIGATE
- [ ] `libs/amm/src/v3_math.rs:234` - FIXME: Verify math with Uniswap code
```

### 3. Process each category

#### For OBSOLETE
```bash
# Remove obsolete TODOs
sed -i '' '/TODO: Remove after migration/d' protocol_v2/src/lib.rs
```

#### For IMMEDIATE
```rust
// Fix simple issues directly
// Before: let result = risky_operation().unwrap(); // FIXME: Handle error
// After:
let result = risky_operation()
    .map_err(|e| AdapterError::OperationFailed(e.to_string()))?;
```

#### For TASK
Create GitHub issues:
```bash
gh issue create --title "Implement slippage calculation in arbitrage strategy" \
  --body "Found in strategies/arbitrage/src/lib.rs:89"
```

Then update the TODO:
```rust
// TODO: Implement slippage calculation
// Tracked in: https://github.com/org/repo/issues/123
```

### 4. Create tracking document
```markdown
# TODO/FIXME Tracking
Generated: 2024-01-XX

## Statistics
- Initial TODOs: 47
- Initial FIXMEs: 23
- Removed (obsolete): 12
- Fixed immediately: 8
- Created issues: 15
- Remaining (documented): 35

## GitHub Issues Created
- #123: Implement slippage calculation
- #124: Optimize TLV builder allocation
- #125: Add comprehensive error types

## Remaining Items
Items that need to stay as TODOs with context...
```

### 5. Commit the cleanup
```bash
git commit -m "chore: Process and organize TODO/FIXME comments

- Removed 12 obsolete TODOs
- Fixed 8 simple FIXMEs immediately
- Created GitHub issues for 15 complex tasks
- Documented remaining TODOs with context"
```

## Validation
- [ ] <10 undocumented TODOs remain
- [ ] All FIXMEs either fixed or tracked
- [ ] GitHub issues created for complex tasks
- [ ] Tracking document created
- [ ] Code still compiles and tests pass

## Template for Acceptable TODOs
```rust
// TODO(#123): Implement advanced feature
// This is blocked by external dependency upgrade
// Tracked in: https://github.com/org/repo/issues/123

// TODO(@username): Optimize hot path
// Current implementation is O(n²), could be O(n log n)
// Not critical for MVP but needed for scale
```

## Notes
- Don't just remove TODOs - evaluate if they're still needed
- Create issues for anything that will take >30 minutes
- Group related TODOs into single issues where appropriate
- Add context to remaining TODOs so future devs understand them
