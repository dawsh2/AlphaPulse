# AlphaPulse Development Roadmap
*Last Updated: 2025-08-25*

## 🎯 Current Sprint: Protocol V2 Header Fix
**Goal**: Resolve critical magic byte placement issue in message headers

### Phase 1: Core Pipeline Fix (COMPLETED ✅)
- [x] Debug exchange WebSocket connections - Polygon data ingestion working
- [x] Debug relay message flow - TLV messages flowing between components  
- [x] Fix arbitrage strategy - receiving data and generating signals
- [x] Fix signal relay - receiving and forwarding strategy signals
- [x] Fix dashboard WebSocket connection - consuming live relay data
- [x] Validate complete flow: Exchange → Polygon → Relay → Arb → Signal → Dashboard
- [x] Dashboard displaying live arbitrage opportunities with accurate data

**Achievement**: Full end-to-end pipeline operational with real-time dashboard visualization

### Phase 2: Protocol V2 Header Fix (URGENT)
**Critical magic byte placement issue must be resolved**

#### Protocol Header Restructure
- [ ] **CRITICAL**: Move magic byte to position 0 in MessageHeader struct
- [ ] Update MessageHeader layout: magic(u32) → padding(u32) → sequence(u64) → timestamp(u64) → payload_size(u32) → checksum(u32)
- [ ] Verify 32-byte total size maintained after restructure
- [ ] Update all zerocopy trait implementations for new layout
- [ ] Fix checksum calculation offsets for new header structure

#### Test Validation & Fixes
- [ ] Fix 6 failing TLV parsing tests (coinbase_string_decimal, multiple_tlv_corruption, binance_orderbook_overflow, etc.)
- [ ] Add missing precision_validation test target
- [ ] Validate magic byte appears at offset 0 in all test scenarios
- [ ] Update parsing logic to expect magic at position 0
- [ ] Run full Protocol V2 test suite after header changes

### Phase 3: Codebase Cleanup (NEXT)
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

### Critical Issues
1. **Magic Byte Placement - CRITICAL FINDING**: The magic byte (0xDEADBEEF) is NOT at the first position in the header as requested. Currently at bytes 16-19, needs to be moved to bytes 0-3 with proper padding for 32-byte alignment.
2. **Data Pipeline Broken**: Core functionality not working
3. **Duplicate Code**: Multiple versions of same functionality
4. **Legacy Migration**: Incomplete Protocol V1 → V2 transition
5. **Test Coverage**: Missing critical path tests

### Code Smell Inventory
- 878+ Symbol references need InstrumentId conversion
- 50+ files in wrong directories
- Multiple script versions doing same thing
- Hardcoded constants throughout codebase
- Inconsistent error handling patterns

## Velocity Tracking

### Current Sprint Metrics
- Started: 2025-08-25
- Pipeline Components Fixed: 0/6
- Cleanup Tasks Completed: 0/12
- Blockers: WebSocket connectivity issues

### Historical Velocity
- Protocol V2 Implementation: ✅ Complete
- Zero-Copy TLV: ✅ Complete  
- System Cleanup Round 1: ✅ Complete
- Production Pipeline: 🔴 Blocked

## Next Actions Queue

### Immediate (Today) - PROTOCOL CRITICAL
1. **URGENT**: Fix MessageHeader magic byte placement in protocol_v2/src/message/header.rs
2. Update checksum calculation logic for new header layout
3. Verify all zerocopy implementations work with new structure

### This Week - Protocol & Pipeline
1. Fix all 6 failing TLV parsing tests
2. Add missing precision_validation test target
3. Validate Protocol V2 changes don't break performance (>1M msg/s target)
4. Resume pipeline debugging after protocol fix
5. Run end-to-end integration test

### Next Week - Cleanup
1. Execute major codebase cleanup (Phase 3)
2. Remove all duplicate files
3. Standardize naming conventions

## Notes
- Pipeline must be functional before cleanup begins
- Breaking changes are encouraged - this is greenfield
- No backwards compatibility concerns
- Focus on getting data flowing first, optimize later