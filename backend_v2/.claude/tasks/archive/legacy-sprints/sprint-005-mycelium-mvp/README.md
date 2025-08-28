# Sprint 005: Mycelium MVP - Zero-Copy Fix & Macro System Overhaul

**Sprint Duration**: 5 days (January 27-31, 2025)
**Priority**: CRITICAL - Performance & Technical Debt
**Impact**: System-wide performance improvement, 50% boilerplate reduction

## Executive Summary

This sprint addresses a critical zero-copy violation discovered in our TLV macro system that causes unnecessary data copies in the hot path, directly impacting our >1M msg/s performance target. We'll fix this issue and implement a comprehensive macro system to eliminate boilerplate across the codebase.

## Critical Issue Discovered

The current `define_tlv!` macro's `from_bytes()` method **copies data instead of providing true zero-copy**:
```rust
// CURRENT (BAD): Copies the entire struct!
Ok(*tlv_ref.into_ref())  // Dereferences and copies
```

This defeats the entire purpose of zero-copy for a system processing >1M msg/s.

## Sprint Objectives

1. **Fix Zero-Copy Violation** [CRITICAL]
   - Remove copying `from_bytes()` method
   - Implement true zero-copy `ref_from()` pattern
   - Migrate all TLV usage sites

2. **Implement Macro System** [HIGH]
   - Create enhanced TLV macros with proper zero-copy
   - Add validation, config, and error macros
   - Prepare actor system macros for future Mycelium runtime

3. **Eliminate Boilerplate** [MEDIUM]
   - Replace ~44 manual TLV impl blocks
   - Standardize validation patterns (~539 occurrences)
   - Consolidate config patterns (~181 occurrences)

## Success Metrics

- ✅ Zero allocations in message parsing hot path
- ✅ 50% reduction in boilerplate code
- ✅ All TLV structures use standardized macros
- ✅ >1M msg/s throughput maintained
- ✅ Type safety improved with validation

## Daily Breakdown

### Day 1 (Monday): Infrastructure & Critical Fix
- Create `libs/types/src/macros/` infrastructure
- Fix zero-copy violation in core macro
- Create `define_tlv_v2!` with proper patterns
- Benchmark to verify zero allocations

### Day 2 (Tuesday): TLV Migration
- Migrate `protocol_v2/src/tlv/market_data.rs` (17 structs)
- Migrate `protocol_v2/src/tlv/pool_state.rs`
- Update all `from_bytes()` call sites to `ref_from()`
- Run performance validation

### Day 3 (Wednesday): Pattern Macros
- Implement `define_validated_tlv!` macro
- Create `define_config!` for configurations
- Add `define_error!` for error types
- Apply to existing patterns

### Day 4 (Thursday): Documentation & Examples
- Create `docs/macro_patterns.md`
- Update `.claude/docs/practices.md`
- Write example files demonstrating patterns
- Document migration guide

### Day 5 (Friday): Testing & Optimization
- Performance benchmarks (must maintain >1M msg/s)
- Memory profiling (verify zero-copy)
- Fix any regressions
- Prepare for Mycelium actor macros

## Technical Approach

### Correct Zero-Copy Pattern
```rust
// OLD (INCORRECT): Copies data
let trade: TradeTLV = TradeTLV::from_bytes(payload)?;
process_trade(&trade);

// NEW (CORRECT): True zero-copy reference
let trade_ref: &TradeTLV = TradeTLV::ref_from(payload)
    .ok_or(ParseError::InvalidPayload)?;
process_trade(trade_ref);
```

### New Macro API
```rust
define_tlv_v2! {
    TradeTLV {
        price: i64,
        quantity: i64,
        timestamp: i64,
    }
}
// Automatically generates:
// - ref_from(bytes) -> Result<&Self>     // Zero-copy reference
// - read_from(bytes) -> Result<Self>     // Explicit copy when needed
// - as_bytes(&self) -> &[u8]             // Zero-copy serialization
// - validate(&self) -> Result<()>        // Built-in validation
```

## Risk Mitigation

1. **Keep old macros during transition** - Deprecate, don't delete
2. **Comprehensive benchmarks** - Before/after measurements
3. **Gradual migration** - Feature flags if needed
4. **Extensive testing** - Zero-copy guarantees validation

## Dependencies

- No external dependencies
- Must coordinate with any in-flight TLV changes
- Performance benchmarks required before merge

## Files to Modify

### New Files (Phase 1)
- `libs/types/src/macros/mod.rs`
- `libs/types/src/macros/tlv_macros.rs`
- `libs/types/src/macros/zero_copy_macros.rs`
- `libs/types/src/macros/validation_macros.rs`
- `libs/types/src/macros/config_macros.rs`
- `libs/types/src/macros/error_macros.rs`

### Files to Update (Phase 2)
- `protocol_v2/src/tlv/market_data.rs` (17 structs)
- `protocol_v2/src/tlv/pool_state.rs`
- `protocol_v2/src/tlv/arbitrage_signal.rs`
- `protocol_v2/src/tlv/demo_defi.rs`
- `protocol_v2/src/tlv/system.rs`
- All files with `from_bytes()` calls (~44 locations)

## Future Preparation

This sprint lays groundwork for the Mycelium actor runtime:
```rust
#[actor_messages]
impl MarketProcessor {
    async fn handle_swap(&mut self, event: Arc<PoolSwapEvent>) { }
    async fn handle_quote(&mut self, quote: Arc<QuoteUpdate>) { }
}
```

## Sprint Completion Criteria

1. Zero-copy verified through benchmarks
2. All TLV structures migrated to new macros
3. Performance maintained at >1M msg/s
4. Documentation complete
5. No memory allocations in hot path

---

**Next Steps**: Begin Day 1 tasks immediately, focusing on the critical zero-copy fix first.
