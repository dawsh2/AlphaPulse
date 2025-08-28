# Sprint 005: Detailed Task Breakdown

## Day 1: Infrastructure & Critical Fix

### Task 1.1: Create Macro Infrastructure [2 hours]
**Owner**: Core Team
**Priority**: CRITICAL
**Dependencies**: None

**Subtasks**:
1. Create `libs/types/Cargo.toml` with zerocopy dependencies
2. Create directory structure:
   ```
   libs/types/src/
   ├── lib.rs
   └── macros/
       ├── mod.rs
       ├── tlv_macros.rs
       ├── zero_copy_macros.rs
       ├── validation_macros.rs
       ├── config_macros.rs
       └── error_macros.rs
   ```
3. Add to workspace `Cargo.toml`
4. Setup macro exports and re-exports

**Acceptance Criteria**:
- [ ] `cargo build -p torq-types` succeeds
- [ ] Macros accessible from other crates
- [ ] Basic macro template compiles

### Task 1.2: Fix Zero-Copy Violation [3 hours]
**Owner**: Performance Team
**Priority**: CRITICAL
**Dependencies**: Task 1.1

**Implementation**:
```rust
// In tlv_macros.rs
#[macro_export]
macro_rules! define_tlv_v2 {
    ($name:ident { $($field:ident: $type:ty),* $(,)? }) => {
        #[repr(C, packed)]
        #[derive(Debug, Clone, Copy, AsBytes, FromBytes, FromZeroes)]
        pub struct $name {
            $(pub $field: $type),*
        }

        impl $name {
            /// True zero-copy reference - no allocation, no copy
            pub fn ref_from(bytes: &[u8]) -> Result<&Self, ParseError> {
                Self::ref_from_prefix(bytes)
                    .map(|(tlv, _)| tlv)
                    .ok_or(ParseError::InvalidPayload)
            }

            /// Explicit copy when ownership needed
            pub fn read_from(bytes: &[u8]) -> Result<Self, ParseError> {
                let tlv_ref = Self::ref_from(bytes)?;
                Ok(*tlv_ref) // Explicit copy
            }

            /// Zero-copy serialization
            pub fn as_bytes(&self) -> &[u8] {
                AsBytes::as_bytes(self)
            }
        }
    };
}
```

**Acceptance Criteria**:
- [ ] No allocations in `ref_from()`
- [ ] Benchmark shows zero-copy behavior
- [ ] Old `from_bytes()` method removed

### Task 1.3: Create Benchmark Suite [1 hour]
**Owner**: Performance Team
**Priority**: HIGH
**Dependencies**: Task 1.2

**Benchmarks**:
1. Message parsing throughput (>1.6M msg/s)
2. Memory allocations (must be 0)
3. CPU cycles per message
4. Cache misses

**Files**:
- `benches/zero_copy_benchmark.rs`
- `benches/tlv_parsing_benchmark.rs`

**Acceptance Criteria**:
- [ ] Benchmarks demonstrate zero allocations
- [ ] Performance meets or exceeds current metrics
- [ ] Results documented

## Day 2: TLV Migration

### Task 2.1: Migrate Market Data TLVs [4 hours]
**Owner**: Protocol Team
**Priority**: HIGH
**Dependencies**: Day 1 completion

**Files to Update**:
- `protocol_v2/src/tlv/market_data.rs`

**TLVs to Migrate** (17 total):
1. TradeTLV
2. QuoteTLV
3. OrderBookTLV
4. PoolSwapTLV
5. PoolMintTLV
6. PoolBurnTLV
7. PoolSyncTLV
8. PoolTickTLV
9. PoolPositionTLV
10. PoolCollectTLV
11. PoolFlashTLV
12. LiquidityAddTLV
13. LiquidityRemoveTLV
14. PriceUpdateTLV
15. VolumeUpdateTLV
16. DepthUpdateTLV
17. SpreadUpdateTLV

**Pattern**:
```rust
// BEFORE: Manual implementation
#[repr(C, packed)]
pub struct TradeTLV {
    pub price: i64,
    pub quantity: i64,
}
impl TradeTLV {
    pub fn new(price: i64, quantity: i64) -> Self { ... }
    pub fn from_bytes(data: &[u8]) -> Result<Self, String> { ... }
}

// AFTER: Macro-generated
define_tlv_v2! {
    TradeTLV {
        price: i64,
        quantity: i64,
        timestamp: i64,
        venue_id: u32,
        _padding: [u8; 4],
    }
}
```

**Acceptance Criteria**:
- [ ] All 17 TLVs use new macro
- [ ] Tests still pass
- [ ] No performance regression

### Task 2.2: Update Call Sites [3 hours]
**Owner**: Integration Team
**Priority**: HIGH
**Dependencies**: Task 2.1

**Search & Replace Pattern**:
```rust
// Find all occurrences of:
TradeTLV::from_bytes(payload)?

// Replace with:
TradeTLV::ref_from(payload)?
```

**Files to Check**:
- All files in `services_v2/adapters/`
- All files in `relays/`
- All test files

**Acceptance Criteria**:
- [ ] No remaining `from_bytes()` calls
- [ ] All functions accept `&TLV` instead of `TLV`
- [ ] Integration tests pass

## Day 3: Pattern Macros

### Task 3.1: Validation Macro [2 hours]
**Owner**: Core Team
**Priority**: MEDIUM
**Dependencies**: Day 2 completion

**Implementation**:
```rust
#[macro_export]
macro_rules! define_validated_tlv {
    ($name:ident {
        fields: { $($field:ident: $type:ty),* },
        validate: $validation:expr
    }) => {
        define_tlv_v2! {
            $name { $($field: $type),* }
        }

        impl $name {
            pub fn validate(&self) -> Result<(), ValidationError> {
                $validation(self)
            }

            pub fn ref_from_validated(bytes: &[u8]) -> Result<&Self, ParseError> {
                let tlv_ref = Self::ref_from(bytes)?;
                tlv_ref.validate().map_err(|_| ParseError::ValidationFailed)?;
                Ok(tlv_ref)
            }
        }
    };
}
```

**Acceptance Criteria**:
- [ ] Validation runs without allocation
- [ ] Clear error messages
- [ ] Composable with other macros

### Task 3.2: Configuration Macro [2 hours]
**Owner**: Core Team
**Priority**: MEDIUM
**Dependencies**: None

**Implementation**:
```rust
#[macro_export]
macro_rules! define_config {
    ($name:ident {
        $($field:ident: $type:ty = $default:expr),* $(,)?
    }) => {
        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub struct $name {
            $(pub $field: $type),*
        }

        impl Default for $name {
            fn default() -> Self {
                Self {
                    $($field: $default),*
                }
            }
        }

        impl $name {
            pub fn from_env() -> Result<Self, ConfigError> {
                // Load from environment with defaults
            }
        }
    };
}
```

### Task 3.3: Error Macro [2 hours]
**Owner**: Core Team
**Priority**: MEDIUM
**Dependencies**: None

**Implementation**:
```rust
#[macro_export]
macro_rules! define_error {
    ($name:ident {
        $($variant:ident $({ $($field:ident: $type:ty),* })? ),* $(,)?
    }) => {
        #[derive(Debug, thiserror::Error)]
        pub enum $name {
            $(
                #[error(stringify!($variant))]
                $variant $({ $($field: $type),* })?,
            )*
        }
    };
}
```

## Day 4: Documentation & Examples

### Task 4.1: Macro Patterns Documentation [2 hours]
**Owner**: Documentation Team
**Priority**: HIGH
**Dependencies**: Day 3 completion

**Create**: `docs/macro_patterns.md`

**Sections**:
1. When to Use Macros vs Manual Implementation
2. Zero-Copy Guarantees and Requirements
3. Performance Implications
4. Migration Guide from Old Patterns
5. Best Practices

### Task 4.2: Update Practices Documentation [1 hour]
**Owner**: Documentation Team
**Priority**: HIGH
**Dependencies**: Task 4.1

**Update**: `.claude/docs/practices.md`

**Add Sections**:
- Mandatory macro usage for TLV structures
- Zero-copy verification checklist
- Performance validation requirements

### Task 4.3: Create Example Files [2 hours]
**Owner**: Documentation Team
**Priority**: MEDIUM
**Dependencies**: Day 3 completion

**Files to Create**:
1. `examples/zero_copy_tlv.rs` - Demonstrate true zero-copy
2. `examples/validated_config.rs` - Show config patterns
3. `examples/actor_messages.rs` - Future actor patterns

## Day 5: Testing & Optimization

### Task 5.1: Performance Validation [3 hours]
**Owner**: Performance Team
**Priority**: CRITICAL
**Dependencies**: All previous tasks

**Tests**:
1. End-to-end message processing benchmark
2. Memory profiling with valgrind
3. CPU profiling with perf
4. Flame graph generation

**Acceptance Criteria**:
- [ ] >1M msg/s construction
- [ ] >1.6M msg/s parsing
- [ ] Zero allocations in hot path
- [ ] <50MB memory per service

### Task 5.2: Regression Testing [2 hours]
**Owner**: QA Team
**Priority**: HIGH
**Dependencies**: Task 5.1

**Test Suites**:
```bash
cargo test --workspace
cargo test --package protocol_v2
cargo test --package torq-types
```

### Task 5.3: Mycelium Prep [2 hours]
**Owner**: Architecture Team
**Priority**: LOW
**Dependencies**: None

**Design**: `libs/types/src/macros/actor_macros.rs`

**Prototype**:
```rust
// Future macro for Mycelium actor system
#[macro_export]
macro_rules! actor_messages {
    // Design the macro interface
}
```

## Rollback Plan

If critical issues discovered:
1. Keep old `define_tlv!` macro as `define_tlv_legacy!`
2. Feature flag for new vs old implementation
3. Gradual rollout by service
4. A/B testing with performance metrics

## Success Validation

### Performance Metrics
- [ ] Throughput: >1M msg/s maintained
- [ ] Latency: <35μs per message
- [ ] Memory: Zero allocations verified
- [ ] CPU: No regression in cycles

### Code Quality Metrics
- [ ] Boilerplate: 50% reduction achieved
- [ ] Type Safety: All validations enforced
- [ ] Test Coverage: >90% maintained
- [ ] Documentation: Complete and clear

## Communication Plan

1. **Daily Standup**: Report task completion
2. **Blockers**: Escalate immediately
3. **Performance Results**: Share benchmarks daily
4. **Code Reviews**: Required for all changes
5. **Sprint Demo**: Friday presentation of improvements

---

**Current Status**: Ready to begin Day 1 tasks
**Next Action**: Create macro infrastructure and fix zero-copy violation
