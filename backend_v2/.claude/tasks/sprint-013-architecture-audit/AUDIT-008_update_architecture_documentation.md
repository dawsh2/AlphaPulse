---
task_id: AUDIT-008
status: TODO
priority: LOW
estimated_hours: 2
assigned_branch: feat/architecture-documentation-update
assignee: TBD
created: 2025-08-26
completed: null
depends_on:
  - AUDIT-002  # Need codec integration documented
  - AUDIT-003  # Need plugin architecture documented
  - AUDIT-005  # Need manage.sh usage documented
blocks: []
scope:
  - "README.md"  # Main project documentation
  - "docs/ARCHITECTURE.md"  # Architecture documentation
---

# AUDIT-008: Update Architecture Documentation

## Git Worktree Setup (REQUIRED)
```bash
# Create worktree for this task
git worktree add -b feat/architecture-documentation-update ../audit-008-worktree
cd ../audit-008-worktree
```

## Status
**Status**: TODO
**Priority**: LOW
**Worktree**: `../audit-008-worktree` (Branch: `feat/architecture-documentation-update`)
**Estimated**: 2 hours

## Problem Statement
The current documentation describes the planned architecture rather than the actual implemented architecture. After completing the codec integration and adapter plugin refactoring, documentation needs to reflect the real state of the system.

## Acceptance Criteria
- [ ] Update README.md with actual (not planned) project structure
- [ ] Document codec usage patterns and integration
- [ ] Add adapter plugin architecture guide
- [ ] Include manage.sh usage instructions
- [ ] Ensure documentation matches implementation reality
- [ ] Remove references to deprecated/removed components
- [ ] Add examples of proper codec usage

## Implementation Steps
1. **Audit Current Documentation**
   - Review README.md for outdated information
   - Check docs/ARCHITECTURE.md accuracy
   - Identify planned vs actual structure differences

2. **Update README.md**
   - Correct project structure to reflect actual layout
   - Update getting started instructions
   - Add manage.sh usage examples
   - Document service dependencies and relationships

3. **Update ARCHITECTURE.md**
   - Document codec integration patterns
   - Explain adapter plugin architecture
   - Add codec usage examples
   - Update service interaction diagrams

4. **Add Usage Guides**
   - How to use alphapulse_codec properly
   - How to create new adapter plugins
   - How to use manage.sh for system control
   - Common development patterns

5. **Remove Deprecated References**
   - Remove mentions of old protocol_v2 structure
   - Update service locations and names
   - Fix any broken internal links

## Documentation Sections to Update

### README.md Updates
- Project structure reflecting libs/alphapulse_codec
- Getting started with manage.sh
- Service overview with correct paths
- Development setup instructions

### ARCHITECTURE.md Updates
- Codec integration architecture
- Adapter plugin system design
- Service communication patterns
- TLV message flow with codec usage

## Files to Create/Modify
- `README.md` - Main project documentation
- `docs/ARCHITECTURE.md` - Architecture documentation
- `docs/DEVELOPMENT.md` - Development guide (if needed)
- `docs/USAGE.md` - Usage instructions (if needed)

## Success Criteria
- Documentation accurately reflects implemented architecture
- New developers can follow documentation successfully
- All code examples in documentation are correct and tested
- No references to deprecated components remain
- Clear usage patterns for codec and plugin architecture

## Content Requirements
Include documentation for:
- ✅ How to use alphapulse_codec library
- ✅ Adapter plugin development guide
- ✅ manage.sh command reference
- ✅ Service architecture and interactions
- ✅ Development workflow and patterns