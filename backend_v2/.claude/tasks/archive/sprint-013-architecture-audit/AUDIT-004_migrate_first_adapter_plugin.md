---
task_id: AUDIT-004
status: COMPLETE
priority: HIGH
estimated_hours: 4
assigned_branch: feat/polygon-adapter-plugin
assignee: TBD
created: 2025-08-26
completed: 2025-08-26
depends_on:
  - AUDIT-003  # Need adapter plugin architecture first
blocks: []
scope:
  - "services_v2/adapters/polygon_adapter/"  # New plugin directory
  - "services_v2/adapters/src/bin/polygon/"  # Existing polygon adapter
---

# AUDIT-004: Migrate First Adapter to Plugin Model

## Git Worktree Setup (REQUIRED)
```bash
# Create worktree for this task
git worktree add -b feat/polygon-adapter-plugin ../audit-004-worktree
cd ../audit-004-worktree
```

## Status
**Status**: ✅ COMPLETE
**Priority**: HIGH
**Worktree**: `../audit-004-worktree` (Branch: `feat/polygon-adapter-plugin`)
**Estimated**: 4 hours

## Problem Statement
Migrate the Polygon adapter as a proof-of-concept for the plugin architecture created in AUDIT-003. This validates the Adapter trait design and demonstrates the plugin model works in practice.

## Acceptance Criteria
- [ ] Move polygon adapter to its own `polygon_adapter/` subdirectory
- [ ] Implement the Adapter trait for polygon adapter
- [ ] Remove duplicated code using common modules
- [ ] Preserve all existing functionality
- [ ] Adapter works with existing binary entry points
- [ ] Performance is maintained or improved
- [ ] Clear template for migrating other adapters

## Target Structure
```
services_v2/adapters/
├── polygon_adapter/
│   ├── Cargo.toml     # Plugin-specific dependencies
│   ├── src/
│   │   ├── lib.rs     # PolygonAdapter implementation
│   │   ├── config.rs  # Configuration specific to Polygon
│   │   └── types.rs   # Polygon-specific message types
│   └── tests/         # Adapter-specific tests
└── src/bin/polygon/   # Binary entry point (delegates to plugin)
```

## Implementation Steps
1. **Create Plugin Directory Structure**
   - Create `polygon_adapter/` subdirectory
   - Set up Cargo.toml for the plugin
   - Create module structure

2. **Implement Adapter Trait**
   - Create `PolygonAdapter` struct implementing the Adapter trait
   - Move polygon-specific logic from monolithic structure
   - Use common modules for auth, metrics, etc.

3. **Update Binary Entry Point**
   - Modify `src/bin/polygon/polygon.rs` to use plugin
   - Keep same command-line interface and behavior
   - Ensure seamless integration

4. **Remove Duplicated Code**
   - Use common auth logic instead of polygon-specific
   - Use common metrics instead of duplicated metrics
   - Use common rate limiting instead of inline logic

5. **Test Migration**
   - Verify adapter connects to Polygon successfully
   - Confirm message processing works correctly
   - Check performance benchmarks
   - Validate no functionality regression

## Files to Create/Modify
- `services_v2/adapters/polygon_adapter/Cargo.toml` - Plugin manifest
- `services_v2/adapters/polygon_adapter/src/lib.rs` - PolygonAdapter implementation
- `services_v2/adapters/polygon_adapter/src/config.rs` - Polygon configuration
- `services_v2/adapters/src/bin/polygon/polygon.rs` - Updated binary entry point
- Move existing polygon logic from monolithic structure

## Success Criteria
- Polygon adapter works identically to before migration
- Code duplication reduced through common module usage
- Plugin can be developed and tested independently
- Clear pattern established for migrating other adapters (Binance, Coinbase, etc.)
- No performance regression in message processing

## Migration Template
This task establishes the template for migrating other adapters:
1. Create `{exchange}_adapter/` directory
2. Implement Adapter trait
3. Use common modules
4. Update binary entry point
5. Test and validate

**Next adapters to migrate**: Binance, Coinbase, Kraken, Gemini