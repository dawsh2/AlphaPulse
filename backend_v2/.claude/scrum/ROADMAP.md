# Torq Trading System - Strategic Roadmap

**Generated**: 2025-08-27  
**Duration**: 18-24 weeks (4-6 months)  
**Status**: Active Development

---

## Executive Summary

This roadmap outlines the systematic development of Torq from current refactoring stage through to a production-ready trading system with advanced portfolio management, risk controls, and strategy infrastructure. The plan prioritizes foundation stability, progressive validation, and production-ready code standards throughout.

### Core Objectives
1. Clean, maintainable codebase with Protocol V2 architecture
2. Functioning arbitrage pipeline with real-time dashboard
3. Live trading capability with proper risk controls
4. Modular trading infrastructure (portfolio, risk, execution)
5. Advanced strategy framework with data persistence
6. Full Torq frontend integration for complete trading interface

---

## Phase I: Foundation Stabilization (Weeks 1-6)

**Goal**: Complete refactoring, establish robust test suite, clean architecture

### Week 1-2: Architecture Completion
- [x] **AUDIT-009 Completion** [XL: 16h] [CRITICAL] ✅ COMPLETED (Sprint 013)
  - Complete network layer restructuring
  - Fix remaining service integration issues
  - Validate Protocol V2 compliance
  - Dependencies: Current sprint work
  
- [ ] **Performance Validation** [M: 4h] [HIGH]
  - Verify >1M msg/s construction maintained
  - Verify >1.6M msg/s parsing maintained
  - Profile critical paths
  - Dependencies: AUDIT-009 completion

### Week 3-4: Test Infrastructure
- [ ] **Core Test Suite** [L: 8h] [CRITICAL]
  - Fix all failing tests in protocol_v2
  - Add integration tests for relay communication
  - Establish golden path e2e test
  - Target: >80% code coverage
  
- [ ] **Performance Benchmarks** [M: 6h] [HIGH]
  - Create automated performance regression tests
  - Set up CI/CD performance gates
  - Document baseline metrics
  
### Week 5-6: Code Quality & Cleanup
- [ ] **Remove Legacy Code** [L: 8h] [HIGH]
  - Delete deprecated backend/ components
  - Clean up Symbol → InstrumentId migrations
  - Remove all mock/dummy services
  
- [ ] **Documentation Update** [M: 4h] [MEDIUM]
  - Update ARCHITECTURE.md with final state
  - Document all breaking changes
  - Create migration guide for services

**Success Metrics**:
- All tests passing (100% green)
- Performance targets maintained
- Zero legacy code dependencies
- Clean dependency graph

---

## Phase II: Arbitrage Pipeline Restoration (Weeks 7-10)

**Goal**: Restore arbitrage detection and execution pipeline with dashboard visualization

### Week 7: Pipeline Infrastructure
- [ ] **Relay Consumer Recovery** [L: 8h] [CRITICAL]
  - Fix MarketDataRelay consumer connections
  - Restore SignalRelay message flow
  - Validate TLV message parsing
  - Dependencies: Phase I completion
  
- [ ] **Pool Cache Rebuild** [M: 6h] [HIGH]
  - Restore DEX pool discovery
  - Implement background RPC queuing
  - Atomic file operations for persistence

### Week 8: Arbitrage Detection
- [ ] **Flash Arbitrage Strategy** [L: 8h] [CRITICAL]
  - Restore V2/V3 path finding
  - Implement optimal sizing calculations
  - Add gas cost estimation
  - Real-time spread calculation
  
- [ ] **Signal Generation** [M: 6h] [HIGH]
  - Create ArbitrageSignalTLV messages
  - Route to SignalRelay (types 20-39)
  - Add confidence scoring

### Week 9-10: Dashboard Integration
- [ ] **WebSocket Connection** [M: 6h] [HIGH]
  - Restore dashboard WebSocket service
  - Implement TLV → JSON conversion
  - Add real-time subscription management
  
- [ ] **UI Components** [L: 8h] [MEDIUM]
  - Arbitrage opportunity display
  - P&L tracking dashboard
  - Pool liquidity visualization
  - Trade execution history

**Success Metrics**:
- Arbitrage opportunities detected in <35μs
- Dashboard showing real-time opportunities
- Historical data tracking functional
- Zero message loss in pipeline

---

## Phase III: Test Execution & Live Trading (Weeks 11-14)

**Goal**: Progressive validation from paper trading to live execution

### Week 11: Paper Trading
- [ ] **Simulated Execution** [M: 6h] [HIGH]
  - Paper trade mode implementation
  - Track theoretical P&L
  - Validate slippage models
  - Log all would-be executions
  
- [ ] **Performance Analysis** [S: 3h] [HIGH]
  - Measure opportunity capture rate
  - Analyze missed opportunities
  - Optimize detection thresholds

### Week 12: Testnet Validation
- [ ] **Polygon Mumbai Setup** [M: 4h] [CRITICAL]
  - Deploy to testnet
  - Configure testnet pools
  - Execute test swaps
  - Validate gas estimation
  
- [ ] **Risk Controls** [L: 8h] [CRITICAL]
  - Implement circuit breakers
  - Add position limits
  - Create emergency shutdown
  - Monitor error rates

### Week 13-14: Live Trading
- [ ] **Mainnet Configuration** [M: 6h] [CRITICAL]
  - Production wallet setup
  - Gas price optimization
  - MEV protection integration
  - Dependencies: Successful testnet validation
  
- [ ] **Progressive Scaling** [L: 8h] [CRITICAL]
  - Start with $100 position limits
  - Monitor execution quality
  - Scale based on success metrics
  - 24/7 monitoring setup
  
- [ ] **Execution Monitoring** [M: 6h] [HIGH]
  - Real-time P&L tracking
  - Slippage analysis
  - Failed execution diagnostics
  - Performance reporting

**Success Metrics**:
- >95% execution success rate
- Positive P&L after gas costs
- <50ms execution latency
- Zero critical failures

---

## Phase IV: Codebase Repolishing & Trading Infrastructure (Weeks 15-19)

**Goal**: Clean architecture, establish trading module APIs

### Week 15-16: Architecture Polish
- [ ] **Service Boundaries** [L: 8h] [HIGH]
  - Clearly separate concerns
  - Define service interfaces
  - Document API contracts
  - Remove circular dependencies
  
- [ためc) **Shared Libraries Organization** [M: 6h] [MEDIUM]
  - Consolidate libs/ utilities
  - Remove code duplication
  - Establish versioning strategy

### Week 17: Portfolio Management Module
- [ ] **Portfolio State Engine** [XL: 16h] [CRITICAL]
  - Position tracking across venues
  - P&L calculation engine
  - Risk metric computation
  - Real-time NAV updates
  
- [ ] **Portfolio API Design** [L: 8h] [HIGH]
  - RESTful query interface
  - WebSocket subscriptions
  - Historical data access
  - Position reconciliation

### Week 18: Risk Management Module
- [ ] **Risk Engine Core** [XL: 16h] [CRITICAL]
  - VaR calculation
  - Position limits enforcement
  - Exposure monitoring
  - Drawdown protection
  
- [ ] **Risk API Design** [L: 8h] [HIGH]
  - Risk metric streaming
  - Alert configuration
  - Override mechanisms
  - Compliance reporting

### Week 19: Execution Management
- [ ] **Smart Order Router** [XL: 16h] [HIGH]
  - Venue selection logic
  - Order splitting algorithms
  - Best execution tracking
  - Retry mechanisms
  
- [ ] **Execution API Design** [L: 8h] [HIGH]
  - Order submission interface
  - Status tracking
  - Cancel/modify support
  - Fill notifications

**Success Metrics**:
- Clean module separation
- <10ms API response times
- 100% test coverage on APIs
- Zero breaking changes after release

---

## Phase V: Advanced Strategy Infrastructure (Weeks 20-24)

**Goal**: Strategy framework, signal persistence, Jupyter integration, Torq frontend

### Week 20-21: Signal Database
- [ ] **TimeSeries Database** [L: 8h] [HIGH]
  - InfluxDB or TimescaleDB setup
  - Signal storage schema
  - Performance indexing
  - Retention policies
  
- [ ] **Signal Cache Layer** [M: 6h] [HIGH]
  - Redis implementation
  - Hot data caching
  - TTL management
  - Cache invalidation

### Week 22: Strategy Framework
- [ ] **Base Strategy Classes** [L: 8h] [HIGH]
  - Abstract strategy interface
  - Signal generation framework
  - Backtesting support
  - Parameter optimization
  
- [ ] **Strategy Registry** [M: 6h] [MEDIUM]
  - Dynamic strategy loading
  - Configuration management
  - Performance tracking
  - A/B testing support

### Week 23: Jupyter Integration
- [ ] **Jupyter Bridge** [L: 8h] [MEDIUM]
  - Data access layer
  - Live market data streaming
  - Historical data queries
  - Trade execution interface
  
- [ ] **Research Notebooks** [M: 6h] [MEDIUM]
  - Strategy development templates
  - Performance analysis tools
  - Risk analysis notebooks
  - Data visualization libraries

### Week 24: Torq Frontend Integration
- [ ] **Frontend API Gateway** [L: 8h] [CRITICAL]
  - RESTful API endpoints
  - WebSocket real-time feeds
  - Authentication & authorization
  - Rate limiting & security
  
- [ ] **Frontend Data Services** [M: 6h] [HIGH]
  - Portfolio state streaming
  - Trade execution interface
  - Historical data queries
  - Risk metrics dashboard
  
- [ ] **UI/UX Integration** [L: 8h] [HIGH]
  - Connect existing React components
  - Real-time chart updates
  - Order entry forms
  - P&L visualization
  - Alert notifications

**Success Metrics**:
- <1ms signal cache latency
- >1M signals/second ingestion
- Seamless Jupyter workflow
- 5+ production strategies deployed
- Full frontend operational with <100ms latency

---

## Risk Matrix

### Technical Risks
| Risk | Impact | Mitigation |
|------|--------|------------|
| Performance degradation | HIGH | Continuous benchmarking, profiling |
| Message loss in pipeline | CRITICAL | Sequence tracking, gap detection |
| Precision loss in calculations | CRITICAL | Native precision preservation, validation tests |
| Exchange API changes | MEDIUM | Adapter abstraction, version management |

### Business Risks
| Risk | Impact | Mitigation |
|------|--------|------------|
| Execution failures | HIGH | Circuit breakers, retry logic |
| Adverse market conditions | HIGH | Position limits, risk controls |
| Regulatory compliance | MEDIUM | Trade reporting, audit logs |
| Capital loss | CRITICAL | Progressive scaling, stop losses |

---

## Dependencies & Blockers

### Critical Dependencies
1. **Protocol V2 Stability**: All features depend on message architecture
2. **Exchange Connectivity**: Reliable WebSocket connections required
3. **RPC Node Access**: DEX operations need stable RPC endpoints
4. **Capital Allocation**: Live trading requires funding

### Current Blockers
1. ~~**AUDIT-009 Completion**: Must finish before Phase II~~ ✅ RESOLVED (Sprint 013 completed 2025-08-27)
2. **Test Infrastructure**: Blocking confidence in changes
3. **Performance Validation**: Required before scaling

---

## Success Metrics Summary

### Phase I Success
- ✅ All tests passing
- ✅ Performance maintained (>1M msg/s)
- ✅ Zero legacy dependencies
- ✅ Clean architecture documented

### Phase II Success
- ✅ Arbitrage pipeline operational
- ✅ Dashboard displaying opportunities
- ✅ <35μs opportunity detection
- ✅ Historical data collection

### Phase III Success
- ✅ Live trades executing
- ✅ Positive P&L achieved
- ✅ Risk controls operational
- ✅ 24/7 monitoring active

### Phase IV Success
- ✅ Clean module APIs
- ✅ Portfolio tracking live
- ✅ Risk metrics computed
- ✅ Smart routing operational

### Phase V Success
- ✅ Signals persisted to database
- ✅ Multiple strategies running
- ✅ Jupyter workflow established
- ✅ Research to production pipeline
- ✅ Torq frontend fully integrated
- ✅ Complete trading interface operational

---

## Current Sprint Focus (Week 1)

### This Week's Priorities
1. **Complete AUDIT-009** [16h] - CRITICAL
2. **Fix failing tests** [8h] - CRITICAL  
3. **Performance validation** [4h] - HIGH
4. **Clean up legacy code** [8h] - MEDIUM

### Daily Targets
- **Monday**: AUDIT-009 network layer (4h)
- **Tuesday**: AUDIT-009 service integration (4h)
- **Wednesday**: Test suite fixes (8h)
- **Thursday**: Performance benchmarking (4h)
- **Friday**: Legacy cleanup, documentation (8h)

### Definition of Done
- [ ] All architectural changes merged
- [ ] Test suite 100% green
- [ ] Performance benchmarks passing
- [ ] Documentation updated
- [ ] Ready for Phase II

---

## Notes & Considerations

### Architectural Principles
- **No Mocks**: Real data and connections only
- **Production Ready**: Every line production-quality
- **Performance First**: Maintain sub-35μs hot paths
- **Breaking Changes Welcome**: Improve freely
- **Clean Boundaries**: Service separation maintained

### Technical Constraints
- Protocol V2 TLV message format (32-byte header)
- Domain relay separation (MarketData/Signal/Execution)
- Native precision preservation (no normalization)
- Bijective InstrumentId system
- Unix socket → message bus migration path

### Quality Gates
- Performance: >1M msg/s construction
- Testing: >80% code coverage
- Documentation: Always current
- Security: No exposed secrets
- Monitoring: Full observability

---

*This roadmap is a living document. Update weekly based on progress and learnings.*