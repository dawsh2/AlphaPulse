# Sprint 003: Critical Data Integrity Resolution
*Sprint Duration: 1 week*
*Objective: Fix production data integrity violations - STOP LYING TO USERS*

## 🚨 CRISIS SUMMARY
Dashboard displays **completely fabricated** arbitrage data. This violates AlphaPulse's core "zero tolerance for deception" principle. Users see fake profits, fake venues, fake everything. This is a production emergency.

## Sprint Goals
1. **IMMEDIATE**: Remove all hardcoded mock data from signal output
2. **CRITICAL**: Fix protocol violations (TLV type 255 abuse)
3. **ESSENTIAL**: Re-enable profitability guards preventing losses
4. **COMPLETE**: Process all DEX events (not just swaps)

## Task Breakdown

### 🔴 PRIORITY 1: Dashboard Trust Crisis (Days 1-2)

#### INTEGRITY-001: Fix Hardcoded Signal Data
**Assignee**: TBD
**Priority**: CRITICAL - PRODUCTION EMERGENCY
**Estimate**: 4 hours
**Dependencies**: None
**Files**: `services_v2/strategies/flash_arbitrage/src/signal_output.rs`

Remove ALL hardcoded values from `send_arbitrage_analysis()` (lines 159-256):
- [ ] Remove hardcoded gas cost ($2.50)
- [ ] Remove hardcoded venues ("Uniswap V3", "SushiSwap V2")
- [ ] Remove hardcoded tokens ("WETH", "USDC")
- [ ] Use actual ArbitrageOpportunity data
- [ ] Map real pool addresses via PoolCache
- [ ] Calculate real gas costs

**Validation**:
```rust
// Test with real opportunity
cargo test --package flash_arbitrage test_real_signal_generation
```

#### INTEGRITY-002: Remove Protocol-Violating DemoDeFiArbitrageTLV
**Assignee**: TBD
**Priority**: CRITICAL
**Estimate**: 3 hours
**Dependencies**: INTEGRITY-001
**Files**:
- `protocol_v2/src/tlv/types.rs`
- `services_v2/dashboard/websocket_server/src/message_converter.rs`

Protocol violations to fix:
- [ ] Remove DemoDeFiArbitrageTLV (type 255 is ExtendedTLV marker!)
- [ ] Update dashboard to consume ArbitrageSignalTLV (type 21)
- [ ] Remove special type 255 handling in converter
- [ ] Respect Signal domain boundaries (20-39)

**Validation**:
```bash
# Verify no type 255 signals
cargo test --package protocol_v2 --test tlv_domain_validation
```

### 🟡 PRIORITY 2: Safety & Compliance (Days 3-4)

#### SAFETY-001: Re-enable Profitability Guards
**Assignee**: TBD
**Priority**: HIGH - PREVENTS FINANCIAL LOSSES
**Estimate**: 4 hours
**Dependencies**: None
**Files**: `services_v2/strategies/flash_arbitrage/src/detector.rs`

Critical guards to restore:
- [ ] Uncomment profitability checks in `check_arbitrage_opportunity_native()`
- [ ] Implement proper USD price fetching from market data
- [ ] Add configurable thresholds (min_profit_usd, max_gas_cost)
- [ ] Remove all "$1 per token" mock prices
- [ ] Add logging for rejected opportunities

**TDD Requirements**:
```rust
#[test]
fn test_unprofitable_opportunity_rejected() {
    // Test that negative profit opportunities are rejected
}

#[test]
fn test_gas_cost_threshold() {
    // Test that high gas cost opportunities are rejected
}
```

#### SAFETY-002: Complete Detector Implementation
**Assignee**: TBD
**Priority**: HIGH
**Estimate**: 6 hours
**Dependencies**: SAFETY-001
**Files**: `services_v2/strategies/flash_arbitrage/src/detector.rs`

Complete missing logic:
- [ ] Implement `evaluate_pair()` function
- [ ] Add V2/V3 mixed path detection
- [ ] Remove ALL placeholder TODOs
- [ ] Add proper error handling
- [ ] Implement cycle detection for multi-hop

**Validation**:
```bash
cargo test --package flash_arbitrage --test detector_completeness
```

### 🟢 PRIORITY 3: Complete Event Processing (Days 5-7)

#### EVENTS-001: Process All DEX Events
**Assignee**: TBD
**Priority**: MEDIUM - INCOMPLETE STATE TRACKING
**Estimate**: 6 hours
**Dependencies**: None
**Files**:
- `services_v2/adapters/src/bin/polygon/polygon.rs`
- `services_v2/adapters/src/bin/polygon/relay_consumer.rs`

Add missing event handlers:
- [ ] Process Mint events (liquidity additions)
- [ ] Process Burn events (liquidity removals)
- [ ] Process Sync events (reserve updates)
- [ ] Create TLV messages for each event type
- [ ] Use PoolCache for token metadata

**TDD Requirements**:
```rust
#[test]
fn test_mint_event_processing() {
    // Verify Mint creates PoolMintTLV
}

#[test]
fn test_burn_event_processing() {
    // Verify Burn creates PoolBurnTLV
}

#[test]
fn test_sync_event_processing() {
    // Verify Sync updates reserves
}
```

#### EVENTS-002: Update PoolStateManager
**Assignee**: TBD
**Priority**: MEDIUM
**Estimate**: 4 hours
**Dependencies**: EVENTS-001
**Files**: `services_v2/strategies/flash_arbitrage/src/pool_state.rs`

Implement liquidity tracking:
- [ ] Implement `process_pool_mint()`
- [ ] Implement `process_pool_burn()`
- [ ] Update reserves on Sync events
- [ ] Maintain liquidity provider state
- [ ] Track total liquidity per pool

### ⚪ PRIORITY 4: Protocol Optimization (Week 2)

#### OPTIMIZE-001: Evaluate packed_struct Migration
**Assignee**: TBD
**Priority**: LOW - OPTIMIZATION
**Estimate**: 8 hours
**Dependencies**: All PRIORITY 1-3 tasks
**Files**: `protocol_v2/src/tlv/*.rs`

Performance-preserving safety improvements:
- [ ] Benchmark current manual padding performance
- [ ] Test packed_struct on non-hot-path structs
- [ ] Measure overhead (MUST be <1%)
- [ ] Migrate if performance maintained
- [ ] Document performance results

**Performance Requirements**:
```bash
# Before ANY changes
cargo run --bin test_protocol --release > baseline.txt

# After changes - MUST maintain:
# - >1M msg/s construction
# - >1.6M msg/s parsing
```

## Definition of Done
- [ ] Dashboard shows ONLY real arbitrage data
- [ ] No hardcoded values in production paths
- [ ] All TLV types respect domain boundaries
- [ ] Profitability guards prevent losses
- [ ] All DEX events processed (Swap, Mint, Burn, Sync)
- [ ] Performance maintained >1M msg/s
- [ ] All tests passing

## Emergency Deployment Plan

### Phase 1: Stop the Bleeding (Immediate)
```bash
# Fix and deploy signal_output.rs changes
cargo test --package flash_arbitrage
./scripts/deploy_emergency_fix.sh
```

### Phase 2: Protocol Compliance (Day 2)
```bash
# Remove type 255 violations
cargo test --package protocol_v2
./scripts/deploy_protocol_fix.sh
```

### Phase 3: Complete Implementation (Days 3-7)
```bash
# Full event processing
cargo test --workspace
./scripts/deploy_complete_fix.sh
```

## Success Metrics
- **ZERO** mock data in dashboard
- **ZERO** TLV domain violations
- **ZERO** unprofitable trades executed
- **100%** DEX event coverage
- **>1M msg/s** maintained performance
- **<10ms** arbitrage detection latency

## Risk Mitigation
- Deploy incrementally with monitoring
- Keep rollback scripts ready
- Add alerts for negative profit signals
- Log all data transformations
- Monitor dashboard for suspicious values

## Notes
This sprint addresses a PRODUCTION CRISIS. The dashboard is lying to users with fabricated data. This violates everything AlphaPulse stands for. Every task here is about restoring truth and integrity to the system.

**Remember**: "zero tolerance for deception" - NO FAKE DATA, NO SHORTCUTS, NO LIES.
