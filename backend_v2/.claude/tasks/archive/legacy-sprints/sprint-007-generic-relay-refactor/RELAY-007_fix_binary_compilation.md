---
task_id: RELAY-007
status: COMPLETE
priority: LOW
estimated_hours: 2
assigned_branch: fix/relay-binary-compilation
assignee: TBD
created: 2025-01-08
completed: 2025-08-27
depends_on:
  - TASK-002  # Need generic relay engine working first
blocks: []
scope:
  - "relays/src/bin/*.rs"  # Fix binary entry points
  - "relays/tests/*.rs"  # Fix test code compilation errors
---

# RELAY-007: Fix Binary Entry Point Compilation Issues

## Task Overview
**Sprint**: 007-generic-relay-refactor
**Priority**: LOW (library works, only binaries affected)
**Estimate**: 2 hours
**Status**: TODO
**Goal**: Fix remaining compilation errors in relay binary entry points

## Problem
While the relay library compiles successfully, the binary entry points have compilation errors in test code and error formatting that prevent them from building. This doesn't block the library usage but prevents running the full relay servers.

## Specific Issues Found

### 1. Test Code Field Name Errors
```rust
error[E0560]: struct `torq_types::MessageHeader` has no field named `message_type`
error[E0560]: struct `torq_types::MessageHeader` has no field named `source_type`
error[E0560]: struct `torq_types::MessageHeader` has no field named `timestamp_ns`
error[E0560]: struct `torq_types::MessageHeader` has no field named `instrument_id`
```

**Location**: Test code in binary files
**Fix**: Update to use correct MessageHeader field names:
- `message_type` → (doesn't exist, use relay_domain)
- `source_type` → `source`
- `timestamp_ns` → `timestamp`
- `instrument_id` → (not in header, in TLV payload)

### 2. Error Formatting Issues
```rust
error[E0277]: the size for values of type `dyn std::error::Error` cannot be known at compilation time
--> relays/src/bin/market_data_relay.rs:57:52
   |
57 |             error!("❌ Market Data Relay failed: {}", e);
   |                                                --   ^ doesn't have a size known at compile-time
```

**Location**: All binary files (market_data_relay.rs, signal_relay.rs, execution_relay.rs)
**Fix**: Box the error or use `.to_string()` for error display

### 3. Missing Imports in Binaries
```rust
error[E0432]: unresolved import `error`
```

**Location**: Binary files missing tracing macro imports
**Fix**: Add proper imports for tracing macros

## Files to Fix

1. `relays/src/bin/market_data_relay.rs`
2. `relays/src/bin/signal_relay.rs`
3. `relays/src/bin/execution_relay.rs`
4. `relays/src/bin/relay.rs`
5. `relays/src/bin/relay_dev.rs`

## Implementation Steps

### Step 1: Fix Test Code Field Names
Update any test code that creates MessageHeader structs to use the correct field names from Protocol V2.

### Step 2: Fix Error Display
Replace error formatting with proper boxing or string conversion:
```rust
// Before
error!("❌ Relay failed: {}", e);

// After
error!("❌ Relay failed: {}", e.to_string());
// Or
error!("❌ Relay failed: {:?}", e);
```

### Step 3: Add Missing Imports
Ensure all binaries have proper imports:
```rust
use tracing::{info, error, warn, debug};
```

### Step 4: Validate Compilation
```bash
# Test each binary
cargo build --bin market_data_relay --release
cargo build --bin signal_relay --release
cargo build --bin execution_relay --release
cargo build --bin relay --release
cargo build --bin relay_dev --release
```

## Success Criteria
- [ ] All 5 binary entry points compile without errors
- [ ] Binaries can be run and start listening on Unix sockets
- [ ] Basic smoke test: send test message through each relay type
- [ ] No regression in library compilation

## Testing

### Unit Test
The library tests should continue to pass:
```bash
cargo test --lib --package torq-relays
```

### Integration Test
Once binaries compile, test actual relay operation:
```bash
# Start relay
./target/release/market_data_relay

# In another terminal, send test message
echo "test" | nc -U /tmp/torq/market_data.sock
```

## Notes
- This is a non-critical follow-up task since the library itself works
- Binary issues are mostly in test/example code, not core functionality
- Can be done in parallel with other work since it doesn't block anything
- Good first task for someone new to the codebase

## Definition of Done
- All binary entry points compile successfully
- Binaries can start and listen on configured Unix sockets
- No compilation warnings in production code (test warnings acceptable)
- Documentation updated if any API changes required