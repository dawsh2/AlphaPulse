# Sprint 006: Protocol Optimization & Macro Abstractions
*Sprint Duration: 1 week*
*Objective: Enhance type safety and reduce boilerplate through strategic macro usage*

## Mission Statement
Implement high-value macros to eliminate entire classes of bugs (typed IDs), reduce boilerplate (TLV definitions), and standardize patterns (configuration loading, test generation). Focus on patterns with high repetition and stable, clear structure.

## Core Principles
**Use Macros When**:
- High degree of repetition exists
- Pattern is stable and clear
- Significant code reduction achieved
- Type safety enhancement possible

**Avoid Macros When**:
- Simple function would suffice
- Pattern is still evolving
- Debugging complexity outweighs benefits

## Task Breakdown

### 🔴 Type Safety Enhancements

#### MACRO-001: Typed ID Wrappers
**Priority**: CRITICAL
**Estimate**: 4 hours
**Status**: TODO
**Files**: `libs/types/src/identifiers.rs`

Eliminate ID type confusion bugs:
```rust
// Before: Easy to mix up
fn process_signal(pool_id: u64, signal_id: u64) { }
process_signal(signal_id, pool_id); // Compiles but WRONG!

// After: Compiler enforces correctness
fn process_signal(pool_id: PoolId, signal_id: SignalId) { }
process_signal(signal_id, pool_id); // COMPILE ERROR!
```

**Implementation**:
- [ ] Create define_typed_id! macro
- [ ] Generate PoolId, SignalId, StrategyId, etc.
- [ ] Add Display, From<u64>, Hash traits
- [ ] Update all services to use typed IDs
- [ ] Fix compilation errors (revealing bugs!)

#### MACRO-002: TLV Definition Automation
**Priority**: HIGH
**Estimate**: 6 hours
**Status**: TODO
**Files**: `protocol_v2/src/tlv/macros.rs`

Reduce TLV boilerplate by 80%:
```rust
// 50+ lines reduced to 5
define_tlv!(
    TradeTLV,
    TLVType::Trade,
    instrument_id: u64,
    price: i64,
    quantity: i64,
    timestamp_ns: u64
);
```

**Features**:
- [ ] Auto-generate zerocopy traits
- [ ] Create as_bytes/from_bytes methods
- [ ] Add size validation
- [ ] Generate expected_payload_size()
- [ ] Include field documentation

### 🟡 Configuration & Testing

#### MACRO-003: Centralized Config Loading
**Priority**: HIGH
**Estimate**: 4 hours
**Status**: TODO
**Files**: `libs/config/src/macros.rs`

Standardize configuration across all services:
```rust
// Every service gets consistent config loading
let config = load_config!(DashboardConfig, "config/dashboard.toml");
```

**Implementation**:
- [ ] TOML file loading with path resolution
- [ ] Environment variable overrides
- [ ] Validation and defaults
- [ ] Error handling standardization
- [ ] Hot reload support (future)

#### MACRO-004: Test Generation Suite
**Priority**: MEDIUM
**Estimate**: 6 hours
**Status**: TODO
**Files**: `tests/common/macros.rs`

Generate comprehensive test suites:
```rust
test_tlv_roundtrip!(test_trade, TradeTLV, create_test_trade());
test_tlv_performance!(bench_trade, TradeTLV, 1_000_000);
test_tlv_size!(size_trade, TradeTLV, 64);
```

**Test Types**:
- [ ] Roundtrip serialization tests
- [ ] Performance benchmarks
- [ ] Size validation tests
- [ ] Property-based tests
- [ ] Cross-version compatibility

### 🟢 Domain-Specific Abstractions

#### MACRO-005: Event Handler Registration
**Priority**: MEDIUM
**Estimate**: 4 hours
**Status**: TODO
**Files**: `services_v2/adapters/src/macros.rs`

Simplify event handler patterns:
```rust
register_handlers! {
    adapter,
    Swap => handle_swap,
    Mint => handle_mint,
    Burn => handle_burn,
    Sync => handle_sync,
}
```

**Benefits**:
- [ ] Automatic dispatch table generation
- [ ] Type-safe event routing
- [ ] Metrics collection per handler
- [ ] Error handling standardization

#### MACRO-006: Metric Collection Points
**Priority**: LOW
**Estimate**: 3 hours
**Status**: TODO
**Files**: `libs/metrics/src/macros.rs`

Standardize metric collection:
```rust
#[instrument_metrics]
async fn process_arbitrage(signal: ArbitrageSignal) -> Result<()> {
    // Automatically tracks: latency, success/failure, throughput
}
```

**Metrics Collected**:
- [ ] Function execution time
- [ ] Success/failure rates
- [ ] Throughput (calls/sec)
- [ ] Custom business metrics

### ⚪ Protocol V2 Optimizations

#### MACRO-007: Bijective ID Enhancements
**Priority**: LOW
**Estimate**: 4 hours
**Status**: TODO
**Files**: `protocol_v2/src/identifiers/macros.rs`

Improve InstrumentId ergonomics:
```rust
create_instrument_id! {
    venue: VenueId::Polygon,
    asset_type: AssetType::Pool,
    data: pool_address,
}
```

#### MACRO-008: Message Builder Patterns
**Priority**: LOW
**Estimate**: 3 hours
**Status**: TODO
**Files**: `protocol_v2/src/message/macros.rs`

Simplify message construction:
```rust
build_message! {
    domain: RelayDomain::MarketData,
    source: SourceType::PolygonAdapter,
    tlvs: [
        TradeTLV { ... },
        QuoteTLV { ... },
    ]
}
```

## Implementation Examples

### Typed ID Macro (MACRO-001)
```rust
#[macro_export]
macro_rules! define_typed_id {
    (
        $(#[$meta:meta])*
        $name:ident
    ) => {
        $(#[$meta])*
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, 
            Hash, serde::Serialize, serde::Deserialize
        )]
        #[repr(transparent)]
        pub struct $name(pub u64);

        impl $name {
            pub const fn new(id: u64) -> Self {
                Self(id)
            }
            
            pub const fn inner(&self) -> u64 {
                self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}({})", stringify!($name), self.0)
            }
        }

        impl From<u64> for $name {
            fn from(id: u64) -> Self {
                Self(id)
            }
        }
        
        impl From<$name> for u64 {
            fn from(id: $name) -> Self {
                id.0
            }
        }
    };
}

// Usage
define_typed_id!(
    /// Unique identifier for a liquidity pool
    PoolId
);
```

### TLV Definition Macro (MACRO-002)
```rust
#[macro_export]
macro_rules! define_tlv {
    (
        $name:ident,
        $tlv_type:expr,
        $($field:ident: $type:ty),* $(,)?
    ) => {
        #[derive(Debug, Clone, Copy, PartialEq, zerocopy::AsBytes, zerocopy::FromBytes)]
        #[repr(C, packed)]
        pub struct $name {
            $(pub $field: $type,)*
        }
        
        impl $name {
            pub const TLV_TYPE: TLVType = $tlv_type;
            
            pub fn expected_size() -> usize {
                std::mem::size_of::<Self>()
            }
            
            pub fn as_bytes(&self) -> &[u8] {
                zerocopy::AsBytes::as_bytes(self)
            }
            
            pub fn from_bytes(bytes: &[u8]) -> Result<&Self, TLVError> {
                zerocopy::LayoutVerified::<_, Self>::new(bytes)
                    .map(|lv| lv.into_ref())
                    .ok_or(TLVError::InvalidSize)
            }
        }
    };
}
```

## Migration Strategy

### Phase 1: Type Safety (Day 1-2)
1. Implement typed ID macro
2. Migrate all u64 IDs to typed versions
3. Fix revealed bugs from compile errors

### Phase 2: Protocol Enhancement (Day 3-4)
1. Implement TLV definition macro
2. Migrate existing TLV structs
3. Verify performance unchanged

### Phase 3: Testing & Config (Day 5-6)
1. Implement test generation macros
2. Standardize configuration loading
3. Generate comprehensive test suites

### Phase 4: Cleanup (Day 7)
1. Remove old boilerplate code
2. Update documentation
3. Performance validation

## Success Metrics
- **Bug Reduction**: Zero ID confusion bugs possible
- **Code Reduction**: 60-80% less boilerplate
- **Test Coverage**: 100% TLV roundtrip tests
- **Type Safety**: All IDs strongly typed
- **Maintainability**: Single source of truth for patterns

## Validation Checklist
- [ ] All macros have comprehensive tests
- [ ] No performance regression from macro usage
- [ ] Macro-generated code is debuggable
- [ ] Documentation for each macro pattern
- [ ] Migration guide for existing code

## Risk Mitigation
- Start with simple, high-value macros
- Test macro expansions with `cargo expand`
- Keep non-macro version during transition
- Benchmark before/after performance
- Document macro patterns thoroughly

## Definition of Done
- Typed IDs eliminate confusion bugs
- TLV definitions 80% smaller
- All services use standard config loading
- Comprehensive test suites generated
- No performance regression
- Documentation complete