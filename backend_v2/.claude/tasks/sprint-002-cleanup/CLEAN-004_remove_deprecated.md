# CLEAN-004: Remove Deprecated Implementations

## Task Overview
**Sprint**: 002-cleanup
**Priority**: HIGH
**Estimate**: 2 hours
**Status**: TODO

## Problem
Legacy implementations and deprecated code are still present, as indicated by documentation and comments.

## Deprecated Code to Remove

### Legacy Python Relay
- [ ] Check for `simple_signal_relay.py`
- [ ] Remove if exists (replaced by Rust implementation)

### Old Zero-Copy Builder
- [ ] Check for `protocol_v2/src/tlv/zero_copy_builder.rs`
- [ ] Remove if exists (comment says "DELETED - flawed implementation")

### Other Deprecated Files
- [ ] Search for files with "deprecated" in name
- [ ] Search for files with "old" or "legacy" prefixes
- [ ] Review files marked obsolete in comments

## Implementation Steps

### 1. Find deprecated implementations
```bash
# Search for deprecated files
find . -name "*deprecated*" -o -name "*legacy*" -o -name "*old*" | grep -v .git

# Check for specific deprecated files
ls -la simple_signal_relay.py 2>/dev/null
ls -la protocol_v2/src/tlv/zero_copy_builder.rs 2>/dev/null

# Search for DEPRECATED comments
grep -r "DEPRECATED" --include="*.rs" --include="*.py" | grep -v .git
```

### 2. Review before removal
```bash
# Check git history to confirm deprecation
git log --oneline -n 10 -- simple_signal_relay.py
git log --oneline -n 10 -- protocol_v2/src/tlv/zero_copy_builder.rs

# Check if anything still references these
grep -r "simple_signal_relay" --include="*.rs" --include="*.py"
grep -r "zero_copy_builder" --include="*.rs"
```

### 3. Remove deprecated files
```bash
# Remove legacy Python relay if exists
[ -f simple_signal_relay.py ] && git rm simple_signal_relay.py

# Remove old zero-copy builder if exists
[ -f protocol_v2/src/tlv/zero_copy_builder.rs ] && git rm protocol_v2/src/tlv/zero_copy_builder.rs

# Remove any other confirmed deprecated files
# git rm <deprecated-file>
```

### 4. Update references
```bash
# Remove commented imports
# In protocol_v2/src/tlv/mod.rs, remove:
# // pub mod zero_copy_builder; // DELETED - flawed implementation

# Update any documentation that mentions deprecated code
```

### 5. Commit the cleanup
```bash
git commit -m "chore: Remove deprecated implementations

- Removed legacy Python signal relay (replaced by Rust)
- Removed flawed zero_copy_builder implementation
- Cleaned up references to deprecated code"
```

## Validation
- [ ] No files with "deprecated" in name
- [ ] No `simple_signal_relay.py` in repository
- [ ] No `zero_copy_builder.rs` in repository
- [ ] No commented references to removed code
- [ ] `cargo build` still succeeds

## Notes
- Double-check with git history before removing
- Ensure nothing in production references deprecated code
- Update documentation if it mentions removed implementations
