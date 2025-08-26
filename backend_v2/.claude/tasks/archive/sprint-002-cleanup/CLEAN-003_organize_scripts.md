# CLEAN-003: Organize Development Scripts

## Task Overview
**Sprint**: 002-cleanup
**Priority**: HIGH
**Estimate**: 3 hours
**Status**: COMPLETE

## Problem
Development and debug scripts are scattered in the root directory, creating clutter.

## Scripts to Organize

### Debug/Profile Scripts → `scripts/dev/`
- [ ] `debug_signal_reader.py`
- [ ] `profile_tlv_bottlenecks.rs`
- [ ] `serialization_bench.py`
- [ ] `simple_profile.sh`
- [ ] Any `debug_*.rs` files in protocol_v2/src/bin/

### Test Scripts → `scripts/test/`
- [ ] `test_rust_signal_relay.py`
- [ ] `test_signal_consumer.py`
- [ ] `test_signal_generation.py`
- [ ] `test_signal_sender.py`

### Binaries to Remove
- [ ] `test_signal_relay_rust` (compiled binary)
- [ ] Any other compiled test binaries

## Implementation Steps

### 1. Create directory structure
```bash
# Create organized structure
mkdir -p scripts/dev
mkdir -p scripts/test
```

### 2. Move debug/profiling scripts
```bash
# Move debug scripts
git mv debug_signal_reader.py scripts/dev/
git mv profile_tlv_bottlenecks.rs scripts/dev/
git mv serialization_bench.py scripts/dev/
git mv simple_profile.sh scripts/dev/

# Move protocol debug scripts if they're one-offs
git mv protocol_v2/src/bin/debug_*.rs scripts/dev/ 2>/dev/null || true
```

### 3. Move test scripts
```bash
# Move test scripts
git mv test_rust_signal_relay.py scripts/test/
git mv test_signal_consumer.py scripts/test/
git mv test_signal_generation.py scripts/test/
git mv test_signal_sender.py scripts/test/
```

### 4. Remove compiled binaries
```bash
# Remove compiled test binaries
git rm test_signal_relay_rust
git rm test_signal_relay_rust.dSYM 2>/dev/null || true
```

### 5. Create README for scripts
```bash
cat > scripts/README.md << 'EOF'
# Development Scripts

## Directory Structure

### `dev/`
Development and debugging utilities:
- Profiling scripts
- Debug helpers
- Performance analysis tools

### `test/`
Testing utilities and harnesses:
- Integration test scripts
- Manual testing tools
- Test data generators

## Usage
These scripts are for development only and not part of the production system.
EOF

git add scripts/README.md
```

### 6. Commit the organization
```bash
git commit -m "refactor: Organize development scripts into proper directories

- Moved debug/profiling scripts to scripts/dev/
- Moved test utilities to scripts/test/
- Removed compiled test binaries
- Added README for script organization"
```

## Validation
- [ ] Root directory has no loose debug_* files
- [ ] Root directory has no loose test_* files
- [ ] scripts/dev/ contains debug utilities
- [ ] scripts/test/ contains test utilities
- [ ] No compiled binaries in git

## Notes
- Keep scripts that are actively useful
- Remove truly one-off scripts that were just for debugging
- Update any documentation that references old paths
