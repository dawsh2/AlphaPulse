# GitHub Issues to Create - High Priority TODOs

## Critical Infrastructure Issues

### Issue #1: Implement Transport Adapters for Relay Infrastructure
**Priority**: HIGH
**Labels**: infrastructure, protocol-v2, blocking
**Description**: 
Transport adapters are critical for relay infrastructure but remain unimplemented. This blocks proper message routing between services.

**Tasks**:
- [ ] Implement Unix socket transport adapter (relays/src/transport_adapter.rs:107)
- [ ] Implement TCP transport adapter (relays/src/transport_adapter.rs:125)  
- [ ] Load topology configuration for transport (relays/src/transport_adapter.rs:144)
- [ ] Add connection pooling and reconnection logic
- [ ] Add comprehensive tests for transport reliability

**Files**:
- `relays/src/transport_adapter.rs`
- `network/transport/src/lib.rs`

---

### Issue #2: TLV Payload Parsing for Venue and Custom Fields
**Priority**: HIGH
**Labels**: protocol-v2, parsing, routing
**Description**:
TLV parsing for venue extraction and custom fields is incomplete, preventing proper message routing by venue.

**Tasks**:
- [ ] Extract venue from TLV payload containing instrument ID (relays/src/topics.rs:247)
- [ ] Parse TLVs to find custom field values (relays/src/topics.rs:254)
- [ ] Add validation for extracted values
- [ ] Performance optimize for hot path (<35μs)

**Files**:
- `relays/src/topics.rs`

---

### Issue #3: Health Check Metrics Implementation
**Priority**: HIGH
**Labels**: monitoring, observability, production-readiness
**Description**:
Health check metrics for latency and memory tracking are not implemented, making production monitoring impossible.

**Tasks**:
- [ ] Implement latency tracking with avg calculation (libs/health_check/src/lib.rs:482)
- [ ] Implement p99 latency tracking (libs/health_check/src/lib.rs:483)
- [ ] Implement memory usage tracking (libs/health_check/src/lib.rs:487)
- [ ] Add metric export for Prometheus/Grafana
- [ ] Create alerting thresholds

**Files**:
- `libs/health_check/src/lib.rs`

---

## State Management Issues

### Issue #4: Execution State Management Implementation
**Priority**: MEDIUM
**Labels**: state-management, execution
**Description**:
Execution state management is stubbed but not implemented, required for order tracking.

**Tasks**:
- [ ] Implement ExecutionStateManager methods (libs/state/execution/src/lib.rs)
- [ ] Add order state tracking
- [ ] Add fill reconciliation
- [ ] Add position updates
- [ ] Add persistence layer

**Files**:
- `libs/state/execution/src/lib.rs`

---

### Issue #5: Portfolio State Management Implementation  
**Priority**: MEDIUM
**Labels**: state-management, portfolio, risk
**Description**:
Portfolio state tracking is incomplete, preventing proper risk management and P&L tracking.

**Tasks**:
- [ ] Implement PortfolioStateManager methods (libs/state/portfolio/src/lib.rs)
- [ ] Add position aggregation
- [ ] Add P&L calculations
- [ ] Add risk metrics computation
- [ ] Add state persistence

**Files**:
- `libs/state/portfolio/src/lib.rs`

---

### Issue #6: Pool Cache Snapshot Persistence
**Priority**: MEDIUM
**Labels**: persistence, pool-cache, reliability
**Description**:
Pool cache snapshot functionality is incomplete, preventing crash recovery and state persistence.

**Tasks**:
- [ ] Implement write_snapshot method (libs/state/market/src/pool_cache.rs:1163)
- [ ] Implement snapshot loading on startup (libs/state/market/src/pool_cache.rs:1183)
- [ ] Add atomic file operations for consistency
- [ ] Add compression for large snapshots
- [ ] Add snapshot rotation and cleanup

**Files**:
- `libs/state/market/src/pool_cache.rs`

---

## AMM Calculation Issues

### Issue #7: V3 AMM Slippage and Mixed Pool Calculations
**Priority**: LOW
**Labels**: amm, optimization, v3-pools
**Description**:
V3 AMM calculations for slippage and mixed pool arbitrage are incomplete.

**Tasks**:
- [ ] Calculate V3 slippage accurately (libs/amm/src/optimal_size.rs:160)
- [ ] Implement V3-V3 mixed pool arbitrage (libs/amm/src/optimal_size.rs:189)
- [ ] Implement V2-V3 mixed pool arbitrage (libs/amm/src/optimal_size.rs:192)
- [ ] Add comprehensive tests with real pool data
- [ ] Benchmark performance impact

**Files**:
- `libs/amm/src/optimal_size.rs`

---

## Test Coverage Issues

### Issue #8: Comprehensive E2E and Integration Tests
**Priority**: MEDIUM  
**Labels**: testing, e2e, quality
**Description**:
Test coverage is incomplete, particularly for E2E scenarios and Polygon DEX integration.

**Tasks**:
- [ ] Create polygon_dex_tests module (services_v2/adapters/src/input/collectors/tests/mod.rs:3)
- [ ] Add comprehensive integration tests (tests/e2e/tests/integration_test.rs:59)
- [ ] Add performance regression tests
- [ ] Add chaos testing scenarios
- [ ] Add load testing suite

**Files**:
- `services_v2/adapters/src/input/collectors/tests/mod.rs`
- `tests/e2e/tests/integration_test.rs`

---

## Cleanup Actions

### TODOs to Remove (Won't Implement)
- QUIC module (network/transport/src/network/mod.rs:13) - not currently needed
- Message queue module (network/transport/src/lib.rs:102) - using direct transport instead

### Dependencies
- Issue #1 blocks relay functionality
- Issue #2 blocks proper message routing  
- Issue #3 blocks production deployment
- Issues #4-5 required for trading functionality
- Issue #6 required for reliability

## Sprint Planning Recommendation
1. Sprint 004: Issues #1, #2, #3 (Critical Infrastructure)
2. Sprint 005: Issues #4, #5, #6 (State Management)
3. Sprint 006: Issues #7, #8 (Optimization & Testing)