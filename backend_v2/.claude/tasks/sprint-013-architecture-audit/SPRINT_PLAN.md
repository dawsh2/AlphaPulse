# Sprint 013: Architectural State of the Union - Complete the Foundation
*Sprint Duration: 1 week*
*Objective: Complete partially finished refactorings and fix critical architectural gaps*

## Mission Statement
Complete the architectural refactorings that are partially done, fix the critical codec dependency issue, and establish the final foundation for AlphaPulse V2. This sprint addresses the gap between what was planned and what was actually implemented, ensuring all components properly use the new architecture.

## Current State Assessment

### ✅ Completed Refactorings
1. **Protocol Separation**: `protocol_v2` successfully split into `libs/types` and `libs/alphapulse_codec`
2. **Generic Relay Engine**: Clean relay architecture with `bin/` and `common/` structure
3. **Typed ID Macros**: 23 usages of `define_typed_id!` macro eliminating ID confusion bugs

### ⚠️ Partially Complete
1. **Codec Integration**: Libraries exist but services still not using `alphapulse_codec`

### ❌ Not Started
1. **Adapter Plugin Architecture**: Still monolithic, needs common trait + plugin structure
2. **Scripts Consolidation**: No unified `manage.sh` control script yet

## Critical Finding
**The most critical issue**: Services (especially relays) are NOT using the new `alphapulse_codec` library. They have `alphapulse-types` dependency but are likely using old/duplicated protocol logic instead of the new codec.

## Task Breakdown

### 🔴 CRITICAL: Codec Integration

#### AUDIT-001: Fix Relay Codec Dependencies
**Priority**: CRITICAL
**Estimate**: 4 hours
**Status**: TODO
**Files**: `relays/Cargo.toml`, relay source files

Complete the codec migration for all relay services:
- Add `alphapulse_codec` dependency to relays/Cargo.toml
- Remove duplicated protocol parsing/building logic
- Replace with calls to alphapulse_codec functions
- Verify all TLV operations use the codec

**Implementation Steps**:
- [ ] Audit current relay protocol usage
- [ ] Add alphapulse_codec to Cargo.toml
- [ ] Find and remove old protocol logic
- [ ] Update imports to use codec
- [ ] Test relay functionality
- [ ] Verify no performance regression

#### AUDIT-002: Fix Service Codec Dependencies
**Priority**: CRITICAL  
**Estimate**: 6 hours
**Status**: TODO
**Files**: All `services_v2/*/Cargo.toml`

Update all services to use alphapulse_codec:
- Audit each service for protocol usage
- Add codec dependency where needed
- Remove any inline TLV parsing
- Standardize on codec functions

**Services to Update**:
- [ ] services_v2/adapters (all exchange collectors)
- [ ] services_v2/strategies/flash_arbitrage
- [ ] services_v2/dashboard/websocket_server
- [ ] services_v2/observability/trace_collector

### 🟡 HIGH: Complete Adapter Refactoring

#### AUDIT-003: Create Adapter Plugin Architecture
**Priority**: HIGH
**Estimate**: 6 hours
**Status**: TODO
**Files**: `services_v2/adapters/`

Implement the planned plugin architecture:
- Create `common/` directory with shared adapter logic
- Define `Adapter` trait for common interface
- Move shared code to common module
- Prepare for individual adapter plugins

**Directory Structure Target**:
```
services_v2/adapters/
├── common/
│   ├── mod.rs         # Adapter trait definition
│   ├── auth.rs        # Shared auth logic
│   └── metrics.rs     # Common metrics
├── polygon_adapter/
├── uniswap_v3_adapter/
└── Cargo.toml
```

#### AUDIT-004: Migrate First Adapter to Plugin Model
**Priority**: HIGH
**Estimate**: 4 hours
**Status**: TODO
**Files**: Pick one adapter (e.g., polygon)

Migrate one adapter as proof of concept:
- Move adapter to its own subdirectory
- Implement the Adapter trait
- Remove duplicated code
- Verify functionality preserved

### 🟢 MEDIUM: Scripts Consolidation

#### AUDIT-005: Create manage.sh Control Script
**Priority**: MEDIUM
**Estimate**: 3 hours
**Status**: TODO
**Files**: `scripts/manage.sh`, `scripts/lib/`

Build unified management interface:
- Create main `manage.sh` dispatcher
- Implement `up`, `down`, `status`, `logs` commands
- Move existing scripts to `lib/` subdirectory
- Add PID tracking for process management

**Script Structure**:
```
scripts/
├── manage.sh          # Main control script
└── lib/               # Internal scripts
    ├── startup.sh
    ├── shutdown.sh
    ├── status.sh
    └── logs.sh
```

#### AUDIT-006: Consolidate Python Scripts
**Priority**: LOW
**Estimate**: 2 hours
**Status**: TODO
**Files**: All `.py` scripts in `scripts/`

Clean up Python script sprawl:
- Identify which Python scripts are still needed
- Move utility scripts to `scripts/lib/python/`
- Remove obsolete/duplicate scripts
- Update manage.sh to call remaining scripts

### 🔵 Documentation & Validation

#### AUDIT-007: Architecture Validation Tests
**Priority**: MEDIUM
**Estimate**: 3 hours
**Status**: TODO
**Files**: `tests/architecture_validation/`

Create tests to prevent regression:
- Verify all services use alphapulse_codec
- Check no duplicated protocol logic
- Validate adapter plugin interface
- Ensure typed IDs used consistently

**Test Categories**:
- [ ] Dependency validation (correct crate usage)
- [ ] No protocol duplication check
- [ ] Typed ID usage verification
- [ ] Plugin interface compliance

#### AUDIT-008: Update Architecture Documentation
**Priority**: LOW
**Estimate**: 2 hours
**Status**: TODO
**Files**: `README.md`, `docs/ARCHITECTURE.md`

Document the completed architecture:
- Update README with actual (not planned) structure
- Document codec usage patterns
- Add adapter plugin guide
- Include manage.sh usage instructions

## Success Metrics
- **Codec Adoption**: 100% of services using alphapulse_codec (0% duplication)
- **Adapter Structure**: At least 1 adapter migrated to plugin model
- **Script Usability**: Single `manage.sh up` starts entire system
- **Test Coverage**: Architecture validation tests passing
- **Documentation**: README accurately reflects actual architecture

## Risk Mitigation
- Start with relay codec migration (highest impact)
- Test each service after codec integration
- Keep old scripts working during transition
- Document breaking changes clearly

## Validation Checklist
- [ ] All services depend on alphapulse_codec
- [ ] No duplicated protocol parsing logic remains
- [ ] Relays properly use codec for TLV operations
- [ ] At least one adapter uses plugin architecture
- [ ] manage.sh provides basic up/down/status
- [ ] Architecture tests prevent regression
- [ ] Documentation matches implementation

## Dependencies

### This Sprint Depends On
- ✅ Sprint 006: Protocol optimization (types/codec split)
- ✅ Sprint 007: Generic relay refactor
- ✅ Sprint 010: Codec separation

### Unlocks Future Work
- Production deployment (needs clean architecture)
- New adapter additions (needs plugin model)
- Performance optimization (needs consistent codec usage)

## Definition of Done
- All services properly use alphapulse_codec library
- Zero protocol logic duplication in codebase
- Adapter plugin architecture implemented with one migrated adapter
- Basic manage.sh script controlling system lifecycle
- Architecture validation tests passing
- Documentation updated to reflect actual implementation