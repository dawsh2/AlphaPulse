# TODO/FIXME Audit Report - Sprint 002

## Summary
- **Total TODOs/FIXMEs Found**: 74
- **Target**: <10 (per Sprint Plan)
- **Status**: Needs further cleanup

## TODOs by Category

### High Priority - Core Infrastructure
1. **relays/src/topics.rs:247**: Extract venue from TLV payload containing instrument ID
2. **relays/src/topics.rs:254**: Parse TLVs to find custom field
3. **relays/src/transport_adapter.rs:107**: Create actual Unix socket transport from infra/transport
4. **relays/src/transport_adapter.rs:125**: Create actual TCP transport from infra/transport
5. **relays/src/transport_adapter.rs:144**: Load topology configuration and create transport

### Medium Priority - Performance & Monitoring
1. **libs/health_check/src/lib.rs:482-483**: Implement latency tracking (avg and p99)
2. **libs/health_check/src/lib.rs:487**: Implement memory tracking
3. **network/transport/src/lib.rs:102**: Implement message queue module when needed
4. **network/transport/src/lib.rs:109**: Implement monitoring module when needed

### Low Priority - Future Enhancements
1. **network/transport/src/network/mod.rs:13**: Implement QUIC module when needed
2. **libs/amm/src/optimal_size.rs:160**: Calculate V3 slippage
3. **libs/amm/src/optimal_size.rs:189,192**: Implement V3 mixed pool arbitrage

### State Management - Needs Implementation
1. **libs/state/execution/src/lib.rs:93,106,126,131,136**: Execution state implementation
2. **libs/state/portfolio/src/lib.rs:84,106,136,141,146**: Portfolio state implementation
3. **libs/state/market/src/pool_cache.rs:1163,1183**: Write snapshot functionality

### Test Coverage
1. **services_v2/adapters/src/input/collectors/tests/mod.rs:3**: Create polygon_dex_tests module
2. **tests/e2e/tests/integration_test.rs:59**: Add comprehensive integration tests

## Recommendations

### Immediate Actions (Convert to GitHub Issues)
1. Transport adapter implementation for Unix sockets and TCP
2. TLV payload parsing for venue and custom fields
3. Health check metrics implementation

### Technical Debt to Track
1. V3 AMM calculation improvements
2. State management implementation for execution and portfolio
3. Pool cache snapshot functionality

### Can Be Removed (Obsolete or Won't Do)
1. QUIC module (not currently needed)
2. Message queue module (using direct transport)

## Next Steps
1. Create GitHub issues for high-priority TODOs
2. Remove obsolete TODOs that won't be implemented
3. Group related TODOs into epic-level issues
4. Consider creating a technical debt backlog