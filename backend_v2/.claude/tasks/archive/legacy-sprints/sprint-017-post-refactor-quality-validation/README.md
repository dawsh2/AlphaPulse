# Sprint 017: Post-Refactor Quality Validation

**CRITICAL MISSION**: Systematic validation of the complete Torq system after major refactor:
- `backend_v2/` → `torq/` (root directory rename)
- `services_v2/` → `services/` (service directory cleanup)  
- `tools/rq` → `scripts/rq` (tooling consolidation)
- `libs/` restructuring: `types` (pure data) + `codec` (protocol logic) separation

## 🎯 Sprint Objectives

**PERFORMANCE REQUIREMENTS**: 
- >1M msg/s construction maintained
- >1.6M msg/s parsing maintained  
- Zero precision loss across all numeric operations
- **ARCHITECTURE**: network/ component independently extractable for future Mycelium repository

## 🚀 Quick Start (4-Phase Execution)

### Phase 1: Core Data Structures & Protocol (Days 1-3)
```bash
# Start with core validation
./VALIDATE-001_core_types_validation.md    # TLV structs, macros, precision
./VALIDATE-002_codec_validation.md         # InstrumentId, builders, parsers
./VALIDATE-003_round_trip_integrity.md     # Complete round-trip testing
```

### Phase 2: Component Integration & E2E Flow (Days 4-6)  
```bash
# Pipeline validation
./VALIDATE-004_adapter_pipeline.md         # JSON→TLV→Binary
./VALIDATE-005_relay_functionality.md      # Binary message forwarding
./VALIDATE-006_consumer_validation.md      # Binary→TLV→JSON
./VALIDATE-007_e2e_semantic_equality.md    # Full pipeline semantic equality
```

### Phase 3: System-Wide Consistency (Days 7-8)
```bash
# System validation
./VALIDATE-008_architecture_independence.md # network/ extractability validation
./VALIDATE-009_code_quality_standards.md   # Clippy, rustfmt, standards
./VALIDATE-011_performance_benchmarks.md   # Performance requirement validation
```

### Phase 4: CI/CD Integration (Days 9-10)
```bash
# Automation setup
./VALIDATE-012_cicd_integration.md         # Automated test integration
./VALIDATE-013_quality_gate_automation.md  # Quality gates automation
```

## 📋 Task Status Tracking
Check current progress:
```bash
../../scrum/task-manager.sh status
../../scrum/task-manager.sh next
```

## 🔧 Critical Validation Commands

### Pre-Validation Checklist
```bash
# Verify refactor completed
ls torq/libs/types/    # Should exist
ls torq/libs/codec/    # Should exist  
ls torq/services/      # Should exist (not services_v2)
ls scripts/rq/         # Should exist (not tools/rq)

# Verify system builds
cd torq/
cargo build --workspace --release
```

### Key Performance Validations
```bash
# Core performance requirements
cargo bench --package types    # >1M msg/s construction
cargo bench --package codec    # >1.6M parsing/sec  
cargo test precision_validation # Zero precision loss

# Architecture independence
cargo tree --package torq_network  # Only libs/ dependencies
cargo test --package torq_network --lib  # Independent compilation
```

### Success Criteria (All Must Pass)
- [ ] All 14 validation tasks completed
- [ ] Performance benchmarks meet requirements  
- [ ] Zero test failures across workspace
- [ ] Architecture independence verified (network/ extractable)
- [ ] Complete TEST_RESULTS.md documentation
- [ ] CI/CD quality gates operational

## 📂 Directory Structure (Post-Validation)
```
sprint-017-post-refactor-quality-validation/
├── README.md                              # This guide
├── SPRINT_PLAN.md                        # Complete 4-phase plan
├── VALIDATE-001_core_types_validation.md # Phase 1: Types
├── VALIDATE-002_codec_validation.md      # Phase 1: Codec  
├── VALIDATE-007_e2e_semantic_equality.md # Phase 2: E2E
├── VALIDATE-008_architecture_independence.md # Phase 3: Architecture
├── [other validation tasks...]
├── TEST_RESULTS.md                       # All validation results
└── check-status.sh                       # Quick status checker
```

## 🚨 Important Rules

- **NEVER work on main branch** - Use git worktree for each validation
- **Always update task status** (TODO → IN_PROGRESS → COMPLETE)  
- **Document all results** in TEST_RESULTS.md
- **Validate dependencies** between tasks (see SPRINT_PLAN.md)
- **Use git worktree** - Never git checkout (changes all sessions)
- **Follow validation order** - Dependencies matter!
