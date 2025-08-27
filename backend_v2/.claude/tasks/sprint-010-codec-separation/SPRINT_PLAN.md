# Sprint 010: Protocol Codec Separation

**Sprint Goal**: Refactor protocol_v2 into a clean three-part separation of concerns: types/codec/network layers.

## 🎯 Sprint Mission
Create a robust, scalable architecture that separates "What" (data structures), "Rules" (protocol logic), and "How/Where" (transport) into distinct, focused layers.

## 🏗️ Target Architecture

### The Three-Part Separation
```
libs/
├── types/              // The "What": Pure data structures
│   └── src/lib.rs      // TradeTLV, PoolType, etc.
│
└── codec/   // The "Rules": Protocol grammar & logic
    └── src/lib.rs      // TLVMessageBuilder, parse_header, etc.

network/                // The "How/Where": Transport layer
└── src/lib.rs          // MyceliumConnectionManager, sockets, etc.
```

### Responsibilities After Separation

#### 1. `libs/types` (The "What")
- Pure data structures with no behavior
- Raw structs and enums representing system concepts
- **Examples**: `TradeTLV`, `QuoteTLV`, `PoolType`, `VenueId`
- **No**: Parsing logic, network code, protocol rules

#### 2. `libs/codec` (The "Rules")  
- Protocol definition and encoding/decoding rules
- The "grammar" of the AlphaPulse system
- **Examples**: `TLVMessageBuilder`, `parse_header()`, `InstrumentId` system
- **No**: Network transport, raw data definitions

#### 3. `network/` (The "How/Where")
- Pure transport layer for moving bytes
- Socket management, connection handling, serialization
- **Examples**: `MyceliumConnectionManager`, socket pools, wire protocols
- **No**: Protocol logic, data structure definitions

## 📋 Sprint Tasks

### CODEC-001: Create libs/codec Foundation 
**Priority**: CRITICAL  
**Estimate**: 4 hours
- Set up new codec crate structure
- Move bijective InstrumentId system
- Move TLVType registry and constants

### CODEC-002: Move Core Protocol Logic
**Priority**: CRITICAL
**Estimate**: 6 hours  
- Move TLVMessageBuilder to codec
- Move parsing functions (parse_header, parse_tlv_extensions)
- Move ProtocolError enum and validation

### CODEC-003: Separate Network Transport Layer
**Priority**: HIGH
**Estimate**: 5 hours
- Create network/ crate structure
- Move socket/connection management
- Move serialization/deserialization for wire transport

### CODEC-004: Update libs/types for Pure Data
**Priority**: HIGH  
**Estimate**: 3 hours
- Ensure types crate contains only data structures
- Remove any embedded behavior or protocol logic
- Update dependencies and exports

### CODEC-005: Integration & Testing
**Priority**: CRITICAL
**Estimate**: 4 hours
- Update all imports across codebase
- Verify clean separation of concerns
- Run full test suite for regressions

### CODEC-006: Documentation & Architecture Validation
**Priority**: MEDIUM
**Estimate**: 2 hours
- Document new architecture in CLAUDE.md
- Create architecture diagrams
- Validate clean dependency graph

## 🎯 Success Criteria

### Clean Separation Achieved
- [ ] `libs/types` contains only pure data structures
- [ ] `libs/codec` contains only protocol rules/logic
- [ ] `network/` contains only transport/connection management
- [ ] No circular dependencies between layers

### Functionality Preserved  
- [ ] All existing protocol_v2 functionality works identically
- [ ] Performance maintained (>1M msg/s construction, >1.6M parsing)
- [ ] All tests pass without modification
- [ ] No breaking changes to public APIs

### Architecture Quality
- [ ] Clear, single responsibility per layer
- [ ] Easy to reason about and test each layer independently
- [ ] Scalable foundation for future protocol evolution
- [ ] Clean dependency graph: types ← codec ← network

## 🚫 Anti-Goals (What NOT to Change)

- **No behavior changes**: All functionality must work identically
- **No performance regressions**: Maintain >1M msg/s benchmarks
- **No API breaking changes**: External users unaffected
- **No new features**: Pure architectural refactoring only

## 🧪 Testing Strategy

### Layer Isolation Tests
- Test each layer independently with clear boundaries
- Mock dependencies to verify clean separation
- Validate that each layer only knows about its concerns

### Integration Tests
- Verify all three layers work together correctly
- Test full message flow: types → codec → network → codec → types
- Performance benchmarks to ensure no regressions

### Behavioral Preservation
- Before/after testing to verify identical functionality
- All existing protocol_v2 tests must pass unchanged

## ⚠️ Risk Mitigation

### Dependency Management
- Carefully manage import chains during refactoring
- Use feature flags if needed for gradual migration
- Maintain backward compatibility during transition

### Performance Monitoring
- Benchmark before/after each major move
- Profile hot paths to ensure no regressions
- Maintain the >1M msg/s performance target

### Rollback Strategy
- Commit after each successful layer separation
- Keep protocol_v2 functional until full validation
- Clear rollback points if issues discovered

## 🎁 Benefits After Completion

### Maintainability
- Changes to data structures isolated to `libs/types`
- Protocol logic changes isolated to `libs/codec`
- Network improvements isolated to `network/`

### Testability
- Test protocol logic without network complexity
- Test network layer without protocol details
- Mock boundaries clearly defined

### Scalability
- Easy to add new data types in `libs/types`
- Easy to extend protocol rules in `libs/codec`  
- Easy to swap network implementations in `network/`

### Developer Experience
- Clear mental model: What vs Rules vs Transport
- New developers can understand each layer independently
- Debugging becomes easier with clear boundaries

## 📅 Sprint Timeline

**Week 1**: Foundation & Core Logic (CODEC-001, CODEC-002)
**Week 2**: Network Separation & Types Cleanup (CODEC-003, CODEC-004)  
**Week 3**: Integration & Validation (CODEC-005, CODEC-006)

## 🔧 Development Guidelines

### Code Movement Strategy
1. **Copy before delete**: Keep original working during transition
2. **Incremental validation**: Test after each major component move
3. **Import updates**: Update all references in single commits
4. **Clean separation**: Ensure no cross-layer business logic

### Quality Gates
- Each task must pass its own tests before proceeding
- Integration tests after each pair of tasks
- Full system test after all moves complete
- Performance validation as final gate

---

**This sprint transforms protocol_v2 from a monolithic communication stack into three focused, maintainable layers that will serve as the foundation for AlphaPulse's continued evolution.**