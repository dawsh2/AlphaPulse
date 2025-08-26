# Sprint 007: Generic Relay Engine Refactor
*Sprint Duration: 1 week*
*Objective: Eliminate code duplication across relay domain implementations using Generic + Trait pattern*

## 🎯 SPRINT SUMMARY
Current relay implementations have ~80% duplicated code across MarketDataRelay, SignalRelay, and ExecutionRelay. This refactor abstracts common functionality into a generic `Relay<T: RelayLogic>` engine while isolating domain-specific behavior through traits.

**Architecture Goal**: `relays/` crate becomes DRY, maintainable, and extensible with clear separation between common infrastructure (80%) and domain logic (20%).

## Sprint Goals
1. **FOUNDATION**: Create RelayLogic trait and generic Relay<T> engine
2. **RESTRUCTURE**: Reorganize relays/ crate with proper module hierarchy
3. **MIGRATE**: Convert existing relays to use new pattern
4. **VALIDATE**: Ensure zero performance regression and Protocol V2 compatibility

## Task Breakdown

### 🔵 PRIORITY 1: Foundation & Architecture (Days 1-2)

#### ARCH-001: Create RelayLogic Trait Foundation
**Priority**: HIGH - ENABLES ALL OTHER WORK
**Estimate**: 3 hours
**Dependencies**: None
**Files**: `relays/src/common/mod.rs` (new)

**Implementation Requirements**:
- [ ] Define `RelayLogic` trait with `domain()`, `socket_path()`, `should_forward()` methods
- [ ] Ensure trait is `Send + Sync + 'static` for async compatibility
- [ ] Add default implementation for `should_forward()` using domain check
- [ ] Use proper Protocol V2 types: `RelayDomain`, `MessageHeader`
- [ ] Add comprehensive trait documentation with usage examples

**Code Structure**:
```rust
pub trait RelayLogic: Send + Sync + 'static {
    fn domain(&self) -> RelayDomain;
    fn socket_path(&self) -> &'static str;
    fn should_forward(&self, header: &MessageHeader) -> bool {
        header.relay_domain == self.domain()
    }
}
```

**Validation**:
```bash
cargo check --package relays
cargo test --package relays --test trait_foundation
```

#### ARCH-002: Build Generic Relay Engine
**Priority**: HIGH 
**Estimate**: 6 hours
**Dependencies**: ARCH-001
**Files**: `relays/src/common/mod.rs`, `relays/src/common/client.rs`

**Core Engine Requirements**:
- [ ] Generic `Relay<T: RelayLogic>` struct with Arc<T> logic field
- [ ] `run()` method implementing complete async event loop
- [ ] Unix socket binding using `logic.socket_path()`
- [ ] Client connection management with subscriber list
- [ ] Message parsing with Protocol V2 32-byte header validation
- [ ] Broadcasting using `logic.should_forward()` filtering
- [ ] Graceful client disconnection handling

**Performance Requirements**:
- [ ] Must maintain current relay throughput benchmarks
- [ ] Zero-copy message forwarding where possible
- [ ] Efficient subscriber broadcast (O(n) not O(n²))

#### ARCH-003: Create Module Structure
**Priority**: HIGH
**Estimate**: 2 hours
**Dependencies**: ARCH-002
**Files**: Directory restructuring

**Directory Structure Implementation**:
```
relays/src/
├── lib.rs              # Module declarations
├── bin/                # Binary entry points
│   ├── market_data_relay.rs
│   ├── signal_relay.rs
│   └── execution_relay.rs
├── common/             # Shared engine
│   ├── mod.rs          # Relay<T> + RelayLogic
│   ├── client.rs       # Client connection logic
│   └── error.rs        # Shared error types
├── market_data.rs      # MarketDataLogic impl
├── signal.rs           # SignalLogic impl
└── execution.rs        # ExecutionLogic impl
```

**Tasks**:
- [ ] Create directory structure
- [ ] Update `Cargo.toml` binary definitions
- [ ] Create proper module declarations in `lib.rs`
- [ ] Add cross-module imports and visibility

### 🔵 PRIORITY 2: Domain Logic Implementation (Days 3-4)

#### DOMAIN-001: Implement MarketDataLogic
**Priority**: MEDIUM
**Estimate**: 2 hours
**Dependencies**: ARCH-003
**Files**: `relays/src/market_data.rs`

**Implementation**:
- [ ] Create `MarketDataLogic` struct implementing `RelayLogic`
- [ ] Return `RelayDomain::MarketData` from `domain()`
- [ ] Use correct socket path constant for market data relay
- [ ] Add domain-specific filtering if needed (TLV types 1-19)
- [ ] Include comprehensive tests

#### DOMAIN-002: Implement SignalLogic
**Priority**: MEDIUM
**Estimate**: 2 hours
**Dependencies**: ARCH-003
**Files**: `relays/src/signal.rs`

**Implementation**:
- [ ] Create `SignalLogic` struct implementing `RelayLogic`
- [ ] Return `RelayDomain::Signal` from `domain()`
- [ ] Use correct socket path for signal relay
- [ ] Respect Signal domain TLV types (20-39)
- [ ] Add signal-specific validation if required

#### DOMAIN-003: Implement ExecutionLogic
**Priority**: MEDIUM
**Estimate**: 2 hours
**Dependencies**: ARCH-003
**Files**: `relays/src/execution.rs`

**Implementation**:
- [ ] Create `ExecutionLogic` struct implementing `RelayLogic`
- [ ] Return `RelayDomain::Execution` from `domain()`
- [ ] Use correct socket path for execution relay
- [ ] Respect Execution domain TLV types (40-79)
- [ ] Add execution-specific security validations

### 🔵 PRIORITY 3: Binary Migration (Day 5)

#### BINARY-001: Convert Market Data Relay Binary
**Priority**: MEDIUM
**Estimate**: 1 hour
**Dependencies**: DOMAIN-001
**Files**: `relays/src/bin/market_data_relay.rs`

**Migration Pattern**:
```rust
use relays::common::Relay;
use relays::market_data::MarketDataLogic;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let logic = MarketDataLogic;
    let relay = Relay::new(logic);
    relay.run().await
}
```

#### BINARY-002: Convert Signal Relay Binary  
**Priority**: MEDIUM
**Estimate**: 1 hour
**Dependencies**: DOMAIN-002
**Files**: `relays/src/bin/signal_relay.rs`

#### BINARY-003: Convert Execution Relay Binary
**Priority**: MEDIUM
**Estimate**: 1 hour  
**Dependencies**: DOMAIN-003
**Files**: `relays/src/bin/execution_relay.rs`

### 🔵 PRIORITY 4: Testing & Validation (Days 6-7)

#### TEST-001: Comprehensive Unit Testing
**Priority**: HIGH - CRITICAL FOR PRODUCTION
**Estimate**: 4 hours
**Dependencies**: All BINARY tasks
**Files**: `relays/tests/`

**Test Coverage**:
- [ ] RelayLogic trait implementation tests
- [ ] Generic Relay<T> engine functionality
- [ ] Message filtering accuracy per domain
- [ ] Client connection/disconnection handling
- [ ] Error handling and recovery
- [ ] Concurrent client broadcasting

#### TEST-002: Integration Testing
**Priority**: HIGH
**Estimate**: 3 hours
**Dependencies**: TEST-001
**Files**: `relays/tests/integration/`

**Integration Scenarios**:
- [ ] End-to-end message flow through each relay
- [ ] Multiple client subscription handling
- [ ] Cross-domain message filtering validation
- [ ] Protocol V2 TLV message compatibility
- [ ] Unix socket communication verification

#### TEST-003: Performance Benchmarking
**Priority**: CRITICAL - MUST MAINTAIN >1M MSG/S
**Estimate**: 2 hours
**Dependencies**: TEST-002
**Files**: `relays/benches/`

**Performance Requirements**:
- [ ] Message throughput benchmarks (before/after)
- [ ] Latency measurements for message forwarding
- [ ] Memory usage comparison
- [ ] CPU utilization under load
- [ ] MUST maintain current performance baselines

**Validation Commands**:
```bash
# Performance regression check
cargo bench --package relays > performance_baseline.txt
# Must show no significant degradation
```

### 🔵 PRIORITY 5: Legacy Cleanup (Day 7)

#### CLEANUP-001: Remove Old Relay Implementations
**Priority**: LOW - AFTER VALIDATION
**Estimate**: 2 hours
**Dependencies**: All TEST tasks passing
**Files**: Various legacy relay files

**Cleanup Tasks**:
- [ ] Identify and remove duplicated relay implementations
- [ ] Update import statements across codebase
- [ ] Remove obsolete configuration constants
- [ ] Clean up unused dependencies in Cargo.toml
- [ ] Update documentation references

## Definition of Done
- [ ] All three domain relays use generic Relay<T> engine
- [ ] Zero code duplication across relay implementations  
- [ ] All existing functionality preserved
- [ ] Performance maintained or improved
- [ ] Comprehensive test coverage (>90%)
- [ ] Documentation updated
- [ ] Legacy code removed

## Performance Requirements
**CRITICAL**: Must maintain AlphaPulse performance targets:
- [ ] >1M msg/s message construction maintained
- [ ] <35μs relay message forwarding latency
- [ ] Memory usage not increased significantly
- [ ] No degradation in concurrent client handling

## Risk Mitigation
- **Performance Risk**: Benchmark at every step, rollback if degradation
- **Protocol Risk**: Validate TLV message compatibility continuously
- **Integration Risk**: Test with existing services before deployment
- **Concurrency Risk**: Stress test with multiple concurrent clients

## Success Metrics
- **80%** code duplication eliminated from relays
- **100%** existing relay functionality preserved
- **0%** performance regression
- **3x** easier to add new relay domains
- **90%+** test coverage on generic engine

## Deployment Strategy
1. **Phase 1**: Deploy alongside existing relays (shadow mode)
2. **Phase 2**: Route subset of traffic to new relays
3. **Phase 3**: Full cutover after validation
4. **Phase 4**: Remove legacy implementations

## Emergency Rollback Plan
- Keep existing relay binaries available during transition
- Implement feature flag for instant rollback
- Monitor performance metrics continuously
- Have rollback scripts ready for immediate deployment

---

## 🔍 RENAME_ME.MD ISSUE ANALYSIS

### Root Cause Investigation

The `rename_me.md` issue stems from template file management in the scrum system:

**Probable Causes**:
1. **Template Cleanup**: Scrum agent creates placeholder `TASK-001_rename_me.md` then removes it during structured file generation
2. **File Naming Convention**: Template system expects specific naming but removes generic placeholders
3. **Directory Structure**: Agent may be creating and cleaning up temporary files during sprint setup

**Investigation Steps**:
- [ ] Check `.claude/scrum/` directory for template management scripts
- [ ] Review scrum agent file creation patterns
- [ ] Examine other sprint directories for similar placeholder patterns
- [ ] Validate file naming conventions in scrum documentation

**Recommended Fix**:
- Modify scrum agent to directly create final task files instead of placeholders
- Add proper error handling for file creation/deletion operations
- Implement atomic file operations to prevent intermediate state issues

This refactoring sprint will eliminate significant technical debt while maintaining AlphaPulse's performance standards and Protocol V2 compatibility.