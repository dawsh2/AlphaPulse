# Sprint 010: Protocol Codec Separation

Clean three-part separation: types (What) / codec (Rules) / network (How/Where)

## 🎯 Sprint Mission
Transform protocol_v2 from a monolithic communication stack into three focused, maintainable layers that clearly separate concerns and improve system scalability.

## 🏗️ The Three-Part Architecture

```
libs/types/              ← The "What": Pure data structures  
libs/alphapulse_codec/   ← The "Rules": Protocol grammar & logic
network/                 ← The "How/Where": Transport layer
```

### Clean Separation of Concerns

| Layer | Responsibility | Examples | What It Doesn't Do |
|-------|---------------|----------|-------------------|
| **libs/types** | Pure data structures | `TradeTLV`, `PoolType` | No parsing, no network, no protocol rules |
| **libs/alphapulse_codec** | Protocol rules & logic | `TLVMessageBuilder`, `parse_header()` | No transport, no raw data definitions |
| **network/** | Transport & connections | Socket pools, wire protocols | No protocol logic, no data structures |

## 🚀 Quick Start

### Review the Sprint Plan
```bash
cat SPRINT_PLAN.md  # See complete architecture and strategy
```

### Start Your First Task
```bash
# Day 1-2: Foundation (CODEC-001) 
cat CODEC-001_create_codec_foundation.md
git checkout -b refactor/codec-foundation

# Day 3-4: Core Logic (CODEC-002)
cat CODEC-002_move_core_protocol.md  
git checkout -b refactor/core-protocol-logic

# Day 5-6: Network Layer (CODEC-003)
cat CODEC-003_separate_network_layer.md
git checkout -b refactor/network-layer
```

### Check Sprint Status
```bash
../../scrum/task-manager.sh status
```

## 📋 Task Overview

| Task | Description | Priority | Hours | Status |
|------|-------------|----------|-------|---------|
| CODEC-001 | Create libs/alphapulse_codec foundation | CRITICAL | 4 | TODO |
| CODEC-002 | Move core protocol logic (builders, parsers) | CRITICAL | 6 | TODO |
| CODEC-003 | Separate network transport layer | HIGH | 5 | TODO |
| CODEC-004 | Update libs/types for pure data | HIGH | 3 | TODO |
| CODEC-005 | Integration & testing | CRITICAL | 4 | TODO |
| CODEC-006 | Documentation & architecture validation | MEDIUM | 2 | TODO |

## 🎯 Success Metrics

- ✅ **Clean separation**: Each layer has single responsibility
- ✅ **No regressions**: >1M msg/s performance maintained
- ✅ **Zero behavior changes**: All functionality identical
- ✅ **Clean dependencies**: types ← codec ← network
- ✅ **Independent testing**: Each layer testable in isolation
- ✅ **Developer experience**: Clear mental model for new contributors

## 🚫 What This Sprint Does NOT Change

- **No new features**: Pure architectural refactoring only
- **No performance changes**: Must maintain >1M msg/s benchmarks  
- **No API breaking changes**: External users unaffected
- **No behavior modifications**: All functionality works identically

## ⚠️ Critical Guidelines

### Code Movement Strategy
1. **COPY before delete**: Keep original working during transition
2. **Incremental validation**: Test after each component move
3. **Clean imports**: Update all references in atomic commits
4. **Layer boundaries**: No cross-layer business logic

### Quality Gates
- Each task must pass independent tests
- Integration tests after major moves
- Performance validation throughout
- Full system test before sprint completion

## 📊 Benefits After Completion

### 🔧 Maintainability
- Data changes isolated to `libs/types`
- Protocol changes isolated to `libs/alphapulse_codec`
- Network changes isolated to `network/`

### 🧪 Testability  
- Test protocol without network complexity
- Test network without protocol details
- Clear mock boundaries for unit testing

### 📈 Scalability
- Easy to add new data types
- Easy to extend protocol rules
- Easy to swap transport implementations

### 👥 Developer Experience
- Clear mental model: What vs Rules vs Transport
- New developers understand each layer independently
- Debugging easier with clear boundaries

## Directory Structure After Completion

```
libs/
├── types/                    # Pure data structures (no behavior)
│   └── src/lib.rs           # TradeTLV, QuoteTLV, PoolType, etc.
│
├── alphapulse_codec/        # Protocol rules and logic
│   ├── src/
│   │   ├── lib.rs          # Main exports
│   │   ├── instrument_id.rs # Bijective InstrumentId system
│   │   ├── tlv_types.rs    # TLVType registry  
│   │   ├── message_builder.rs # TLVMessageBuilder
│   │   ├── parser.rs       # parse_header(), parse_tlv_extensions()
│   │   └── constants.rs    # MESSAGE_MAGIC, protocol constants
│   └── tests/
│       └── codec_tests.rs  # Protocol logic tests
│
network/                     # Transport and connection management
├── src/
│   ├── lib.rs              # Main exports
│   ├── connection_manager.rs # MyceliumConnectionManager
│   ├── socket_pool.rs      # Socket management
│   └── wire_protocol.rs    # Serialization for transport
└── tests/
    └── network_tests.rs    # Transport layer tests
```

## 🎓 Architecture Principles

### Single Responsibility
- **libs/types**: "I define what data looks like"
- **libs/alphapulse_codec**: "I define how data is encoded/decoded"  
- **network/**: "I define how bytes move between systems"

### Dependency Direction
```
network/ ──depends on──> libs/alphapulse_codec ──depends on──> libs/types
```
- Network layer imports codec for message construction
- Codec layer imports types for data structures
- Types layer has no dependencies (pure data)

### Testing Strategy
- **Unit tests**: Test each layer's internal logic independently
- **Integration tests**: Test layer boundaries and contracts
- **E2E tests**: Test full stack functionality

## Important Rules

- **NEVER commit to main branch** - Use feature branches
- **Test after each major move** - Verify no behavior changes
- **Preserve performance** - Maintain >1M msg/s benchmarks
- **Clean dependencies** - No circular imports between layers
- **Update task status** - Mark COMPLETE when done

## Definition of Done

This sprint is complete when:
1. All six tasks completed successfully
2. Clean three-layer architecture implemented
3. All existing functionality preserved
4. Performance benchmarks maintained
5. Full test suite passing
6. Documentation updated with new architecture

## 🚨 Remember

**This is architectural refactoring, not feature development.**

Every line of moved code should work identically to the original. The value is in the improved structure, not changed behavior.