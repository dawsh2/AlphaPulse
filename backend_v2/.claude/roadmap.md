# AlphaPulse Development Roadmap
*Last Updated: 2025-08-25*

## 🎯 Current Sprint: Test Suite Stabilization & Performance Optimization
**Goal**: Stabilize Protocol V2 test suite and optimize system performance

### Phase 1: Core Pipeline Fix (COMPLETED ✅)
- [x] Debug exchange WebSocket connections - Polygon data ingestion working
- [x] Debug relay message flow - TLV messages flowing between components  
- [x] Fix arbitrage strategy - receiving data and generating signals
- [x] Fix signal relay - receiving and forwarding strategy signals
- [x] Fix dashboard WebSocket connection - consuming live relay data
- [x] Validate complete flow: Exchange → Polygon → Relay → Arb → Signal → Dashboard
- [x] Dashboard displaying live arbitrage opportunities with accurate data

**Achievement**: Full end-to-end pipeline operational with real-time dashboard visualization

### Phase 2: Protocol V2 Header Fix (COMPLETED ✅)
**Critical magic byte placement issue has been resolved**

#### Protocol Header Restructure
- [x] **CRITICAL**: Move magic byte to position 0 in MessageHeader struct
- [x] Update MessageHeader layout: magic(u32) → metadata(u32) → sequence(u64) → timestamp(u64) → payload_size(u32) → checksum(u32)
- [x] Verify 32-byte total size maintained after restructure
- [x] Update all zerocopy trait implementations for new layout
- [x] Fix checksum calculation offsets for new header structure

**Achievement**: Protocol V2 header structure correctly positions magic byte at bytes 0-3 with proper 32-byte alignment

### Phase 3: Critical Production Fixes (CURRENT SPRINT)
**Production pipeline has critical issues that need immediate attention**

#### 🔴 CRITICAL: Production Blockers (Highest Priority)
- [ ] **POOL-FIX**: Pool/Token address extraction - Currently using [0u8; 20] placeholders
  - Extract real addresses from log.address
  - Integrate existing pool_cache.rs infrastructure
  - Implement async discovery for unknown pools
- [ ] **PRECISION-FIX**: Signal output precision loss - Float conversion destroying profits
  - Fix expected_profit_q calculation using integer math
  - Store amounts as cents/wei from the start
- [ ] **PERF-FIX**: Checksum validation killing performance
  - Implement sampling (every 100th message)
  - Maintain <35μs hot path target

#### 🟡 Major Fixes (This Sprint)
- [ ] **CLEANUP-FIX**: Remove unreachable pattern in relay_consumer.rs:440
- [ ] **RATE-FIX**: Add dashboard rate limiting (100 msg/sec per client)
- [ ] Fix 6 failing TLV parsing tests
- [ ] Fix TLV size assertion failures (QuoteTLV: expected 52, got 56 bytes)
- [ ] Fix zero-copy validation compilation errors
- [ ] Address performance regression in fast_timestamp (21.76ns vs 5ns target)

### Phase 4: Codebase Cleanup (NEXT)
**Major cleanup needed after protocol fix and pipeline is functional**

#### File Organization
- [ ] Remove duplicate files with 'enhanced', 'fixed', 'new', 'v2' prefixes
- [ ] Delete deprecated legacy services from backend/
- [ ] Consolidate scripts directory - remove duplicates, clear naming
- [ ] Clean up temporary test files and organize test structure
- [ ] Remove 50+ scattered files in backend root directory

#### Code Quality  
- [ ] Standardize file and module naming conventions
- [ ] Remove all commented out code and unused functions
- [ ] Complete Symbol → InstrumentId migration (878+ instances)
- [ ] Update all hardcoded values to configurable parameters
- [ ] Remove mock/dummy code and test stubs

#### Documentation
- [ ] Update all README files to reflect current state
- [ ] Remove outdated documentation
- [ ] Document actual API endpoints and data flows
- [ ] Create clear service boundary documentation

### Phase 4: DevOps & Operations (FUTURE)
**Standardize deployment, monitoring, and operations**

#### Deployment Infrastructure
- [ ] Create Docker compose for local development environment
- [ ] Implement Kubernetes manifests for production deployment
- [ ] Set up CI/CD pipeline with GitHub Actions
- [ ] Create infrastructure as code (Terraform/Pulumi)
- [ ] Implement blue-green deployment strategy

#### Operations & Monitoring
- [ ] Standardize service startup scripts and systemd units
- [ ] Implement centralized logging (ELK/Loki stack)
- [ ] Set up Prometheus metrics and Grafana dashboards
- [ ] Create health check endpoints for all services
- [ ] Implement distributed tracing (OpenTelemetry)

#### Developer Experience
- [ ] Create single command to run entire system locally
- [ ] Implement hot-reload for development workflow
- [ ] Standardize environment variable configuration
- [ ] Create developer setup script for new team members
- [ ] Document standard operating procedures (runbooks)

## Technical Debt Registry

### Critical Issues (Updated)
1. ~~**Magic Byte Placement**: RESOLVED ✅ - Magic byte (0xDEADBEEF) now correctly positioned at bytes 0-3~~
2. ~~**Data Pipeline Broken**: RESOLVED ✅ - Full end-to-end pipeline operational~~
3. **Test Suite Instability**: 17 failing protocol tests affecting CI/CD confidence
4. **Performance Regression**: Fast timestamp and hot path buffers not meeting targets
5. **TLV Size Mismatches**: Protocol struct sizes don't match expected values (52 vs 56 bytes)

### Code Smell Inventory
- 878+ Symbol references need InstrumentId conversion
- 50+ files in wrong directories
- Multiple script versions doing same thing
- Hardcoded constants throughout codebase
- Inconsistent error handling patterns

## Velocity Tracking

### Current Sprint Metrics
- Started: 2025-08-25
- Pipeline Components Fixed: 6/6 ✅ COMPLETE
- Protocol Header Fixed: 1/1 ✅ COMPLETE
- Test Failures: 17 ❌ (6 TLV parsing, 11 protocol core)
- Performance Targets: 2/4 ❌ (timestamp & buffer allocation regressed)

### Historical Velocity
- Protocol V2 Implementation: ✅ Complete
- Zero-Copy TLV: ✅ Complete  
- System Cleanup Round 1: ✅ Complete
- Protocol V2 Header Fix: ✅ Complete
- Production Pipeline: ✅ OPERATIONAL

## Next Actions Queue

### Immediate (Today) - TEST STABILIZATION
1. **HIGH PRIORITY**: Fix 6 failing TLV parsing tests to restore CI/CD confidence
2. Investigate TLV size mismatches (QuoteTLV 52 vs 56 bytes)
3. Fix zero-copy validation compilation errors (PoolSyncTLV::new method missing)
4. Address performance regression in fast_timestamp (21.76ns vs 5ns target)

### This Week - Performance & Quality
1. Fix hot path buffer allocation performance (1083ns vs <500ns target)
2. Resolve demo arbitrage TLV roundtrip serialization issues
3. Add comprehensive integration tests for end-to-end pipeline
4. Validate >1M msg/s throughput is maintained after recent changes
5. Set up automated performance regression detection

### Next Week - System Optimization
1. Execute major codebase cleanup (Phase 4)
2. Remove all duplicate files and scattered test artifacts
3. Optimize memory usage and reduce service startup times
4. Complete Symbol → InstrumentId migration (878+ instances remaining)

## Notes
- ✅ **MAJOR MILESTONE**: Full end-to-end pipeline operational with live dashboard
- ✅ **CRITICAL FIX**: Protocol V2 header magic byte correctly positioned at bytes 0-3
- **CURRENT FOCUS**: Test suite stabilization to restore development confidence  
- **NEXT PHASE**: System optimization and performance tuning
- Breaking changes are encouraged - this is greenfield codebase
- No backwards compatibility concerns - optimize freely