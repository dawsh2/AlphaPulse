---
task_id: AUDIT-001
status: IN_PROGRESS
priority: CRITICAL
estimated_hours: 4
assigned_branch: fix/relay-codec-integration
assignee: TBD
created: 2025-08-26
completed: null
---

# AUDIT-001: Fix Relay Codec Dependencies

## 🔴 CRITICAL INSTRUCTIONS

### 0. 📋 MARK AS IN-PROGRESS IMMEDIATELY
**⚠️ FIRST ACTION: Change status when you start work!**
```yaml
# Edit the YAML frontmatter above:
status: TODO → status: IN_PROGRESS
```

### 1. Git Worktree Safety (NEW WORKFLOW)
```bash
# NEVER use git checkout! Use worktrees instead:
git worktree add -b fix/relay-codec-integration ../relay-codec-fix

# Work in the new directory:
cd ../relay-codec-fix

# Verify you're in the worktree:
pwd  # Should show: .../relay-codec-fix
```

## Status
**Status**: TODO (⚠️ CHANGE TO IN_PROGRESS WHEN YOU START!)
**Priority**: CRITICAL - This is blocking proper architecture
**Branch**: `fix/relay-codec-integration`
**Estimated**: 4 hours

## Critical Problem Statement
**The relays are NOT using the new `alphapulse_codec` library!** 

Despite successfully splitting protocol_v2 into libs/types and libs/alphapulse_codec, the relay services still:
- Only depend on `alphapulse-types` (not the codec)
- Likely have duplicated or old protocol parsing logic
- Are not benefiting from the centralized codec implementation

This means the architectural refactoring is incomplete and the system is inconsistent.

## Evidence of the Problem
```toml
# Current relays/Cargo.toml (WRONG)
[dependencies]
alphapulse-types = { path = "../libs/types" }
# MISSING: alphapulse_codec dependency!
```

## Acceptance Criteria
- [ ] Relays depend on BOTH `alphapulse-types` AND `alphapulse_codec`
- [ ] All TLV parsing uses `alphapulse_codec` functions
- [ ] All message building uses `alphapulse_codec` builders
- [ ] Zero duplicated protocol logic in relay code
- [ ] All relay tests still pass
- [ ] No performance regression

## Technical Approach

### Step 1: Add Codec Dependency
```toml
# relays/Cargo.toml
[dependencies]
alphapulse-types = { path = "../libs/types" }
alphapulse_codec = { path = "../libs/alphapulse_codec" }  # ADD THIS
```

### Step 2: Audit Current Protocol Usage
```bash
# Find all protocol-related code in relays
grep -r "parse\|serialize\|TLV\|MessageBuilder" relays/src/

# Look for duplicated logic
grep -r "from_bytes\|to_bytes\|parse_header" relays/src/
```

### Step 3: Remove Duplicated Logic
Identify and remove any code that:
- Manually parses TLV structures
- Builds messages without using codec
- Duplicates logic that exists in alphapulse_codec

### Step 4: Update Imports
```rust
// OLD (probably using local implementations)
use crate::protocol::{parse_message, build_message};

// NEW (use the codec)
use alphapulse_codec::{parse_message, MessageBuilder};
use alphapulse_types::{TradeTLV, SignalTLV};
```

### Step 5: Update All Usage Points
Common patterns to fix:
```rust
// OLD: Manual parsing
let trade = TradeTLV::from_bytes(&bytes)?;

// NEW: Use codec
let trade = alphapulse_codec::decode_tlv::<TradeTLV>(&bytes)?;

// OLD: Manual building
let mut buffer = Vec::new();
trade.write_to(&mut buffer)?;

// NEW: Use codec builder
let message = MessageBuilder::new()
    .add_tlv(trade)
    .build()?;
```

## Files to Modify
- `relays/Cargo.toml` - Add alphapulse_codec dependency
- `relays/src/common/relay_engine.rs` - Update to use codec
- `relays/src/bin/market_data_relay.rs` - Remove duplicated logic
- `relays/src/bin/signal_relay.rs` - Remove duplicated logic  
- `relays/src/bin/execution_relay.rs` - Remove duplicated logic
- Any other files with protocol logic

## Testing Requirements

### Unit Tests
```bash
# Run relay-specific tests
cargo test -p relays

# Verify codec is being used
cargo tree -p relays | grep alphapulse_codec
```

### Integration Tests
```bash
# Test full message flow
cargo test -p relays --test integration

# Verify no protocol errors
cargo run --bin market_data_relay &
# Send test messages and verify parsing
```

### Performance Validation
```bash
# Benchmark before changes
cargo bench -p relays > before.txt

# After changes
cargo bench -p relays > after.txt

# Compare - should be same or better
diff before.txt after.txt
```

## Common Pitfalls to Avoid
1. **Don't just add the dependency** - Actually use it!
2. **Don't leave old code** - Remove ALL duplicated logic
3. **Don't break tests** - Update tests to use codec too
4. **Don't forget performance** - Codec should be as fast or faster

## Git Workflow (Using Worktrees)
```bash
# 1. Create worktree for this task
git worktree add -b fix/relay-codec-integration ../relay-codec-fix
cd ../relay-codec-fix

# 2. Make changes
# - Update Cargo.toml
# - Remove duplicated code
# - Update imports and usage

# 3. Test thoroughly
cargo test -p relays
cargo clippy -p relays

# 4. Commit
git add -A
git commit -m "fix: integrate alphapulse_codec into relay services

- Add alphapulse_codec dependency to relays
- Remove duplicated protocol parsing logic
- Update all TLV operations to use codec
- Maintain performance and functionality"

# 5. Push
git push origin fix/relay-codec-integration

# 6. Create PR
gh pr create --title "Fix: Complete codec integration for relays" --body "Fixes AUDIT-001"
```

## Completion Checklist
- [ ] **🚨 Changed status to IN_PROGRESS when starting**
- [ ] Created worktree (not using checkout)
- [ ] Added alphapulse_codec to Cargo.toml
- [ ] Removed ALL duplicated protocol logic
- [ ] Updated all imports to use codec
- [ ] All relay tests pass
- [ ] Performance validated (no regression)
- [ ] Code reviewed and cleaned
- [ ] PR created
- [ ] **🚨 Updated task status to COMPLETE**

## Why This Matters
This is THE MOST CRITICAL fix. Without it:
- The architecture refactoring is incomplete
- We have duplicated code (maintenance nightmare)
- Bug fixes in the codec won't apply to relays
- Performance optimizations are split across multiple places
- The codebase is inconsistent and confusing

Fixing this completes the architectural foundation and ensures all services use the same, centralized protocol implementation.