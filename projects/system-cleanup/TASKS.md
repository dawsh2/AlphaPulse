# System Cleanup - Master Task Checklist

## Phase 0: Message Protocol Migration ⚠️ PREREQUISITE
*Timeline: Week 0 - MUST COMPLETE BEFORE ANY OTHER PHASES*

### Pre-Migration Validation
- [ ] Review `projects/registry/MESSAGE_PROTOCOL.md` for full specification
- [ ] Backup current state to `pre-protocol-migration-backup` branch
- [ ] Identify all message handling code across services
- [ ] Document current 48-byte message dependencies
- [ ] Plan bijective InstrumentId migration strategy

### Protocol Implementation
- [ ] Add zerocopy and num_enum dependencies to Cargo.toml
- [ ] Implement MessageHeader with CRC32 checksums (32 bytes)
- [ ] Implement bijective InstrumentId system (12 bytes)
- [ ] Create TradeMessage (64 bytes), QuoteMessage (80 bytes), ArbitrageMessage (96 bytes)
- [ ] Implement dynamic schema registration system
- [ ] Set up domain-specific relays (MarketDataRelay, SignalRelay, ExecutionRelay)

### Migration Execution
- [ ] Replace unsafe transmutes with zerocopy parsing
- [ ] Convert symbol hashes to bijective InstrumentIds
- [ ] Update all collectors to produce new message format
  ```bash
  # Test new message generation
  cargo test --package protocol --test bijective_id_tests
  cargo test --package protocol --test zerocopy_safety_tests
  ```
- [ ] Update relay servers for variable message sizes
- [ ] Modify bridge to handle new message types
- [ ] Run comprehensive test suite
  ```bash
  cargo test --workspace
  python -m pytest tests/
  ```

### Post-Migration Validation
- [ ] Verify all InstrumentIds are reversible and debuggable
- [ ] Confirm CRC32 checksums validate at every hop
- [ ] Test all message types (Trade, Quote, OrderBook, etc.)
- [ ] Validate domain relay routing works correctly
- [ ] Check frontend compatibility with new message format
- [ ] Commit protocol migration changes

## Phase 1: Foundation & Safety Setup ⚡
*Timeline: Week 1*

### Dependency Management Setup
- [ ] Create centralized Cargo workspace configuration
- [ ] Add message protocol dependencies:
  ```toml
  [workspace.dependencies]
  zerocopy = "0.7"
  num_enum = "0.7"
  crc32fast = "1.3"
  dashmap = "5.5"
  ```
- [ ] Set up Poetry for Python dependency management
- [ ] Audit current dependencies for conflicts
- [ ] Document dependency governance rules

### Safety Infrastructure
- [ ] Create institutional deprecation area
  ```bash
  mkdir -p _deprecated/{readme,phase1,phase2,permanent}
  mkdir -p _deprecated/old_protocol  # For 48-byte message code
  ```
- [ ] Archive old unsafe transmute code before removal
- [ ] Document deprecation lifecycle rules
- [ ] Set up migration tracking in `MIGRATION.md`
- [ ] Create rollback procedures documentation
- [ ] Test rollback on development environment

### Workspace Configuration
- [ ] Update root `Cargo.toml` with workspace members
- [ ] Configure workspace-level dependencies
- [ ] Create Python workspace with Poetry
- [ ] Validate all services compile with new config

### CI/CD Preparation
- [ ] Audit CI/CD pipelines for hardcoded paths
- [ ] Create migration validation workflow
- [ ] Set up quality gate checks
- [ ] Test pipeline with new structure (dry run)

## Phase 2: Backend Internal Cleanup 🧹
*Timeline: Week 2*

### Root Directory Cleanup
- [ ] Count files in backend root (target: <10)
- [ ] Move debug files to `scripts/debug/`
  ```bash
  for file in test_*.py debug_*.py; do
    mv "$file" scripts/debug/
  done
  ```
- [ ] Move test files to `scripts/testing/`
- [ ] Archive log files to `archive/temp/`
- [ ] Move maintenance scripts to appropriate locations

### Service File Organization
- [ ] Move `app_fastapi.py` to `services/api_server/main.py`
- [ ] Relocate standalone collectors to `services/collectors_legacy/`
- [ ] Organize configuration files into `config/` directory
- [ ] Clean up temporary and backup files

### Git Hygiene
- [ ] Update `.gitignore` with comprehensive rules
- [ ] Remove tracked files that should be ignored
- [ ] Clean up Git history if needed (with team approval)
- [ ] Verify clean `git status` after organization

### Documentation Updates
- [ ] Update file location references in README
- [ ] Fix broken links in documentation
- [ ] Update developer setup instructions
- [ ] Create file location migration map

## Phase 3: Service Consolidation 🏗️
*Timeline: Week 3*

### Core vs Services Consolidation
- [ ] Analyze overlap between `core/` and `services/`
- [ ] Create deprecation backup of `core/` directory
- [ ] Move unique files from `core/` to `services/`
- [ ] Update import paths for moved files
- [ ] Remove empty `core/` directory

### DeFi System Organization
- [ ] Create DeFi structure under `trading/defi/`
  ```bash
  mkdir -p trading/defi/{core,protocols,strategies,execution,agents,analytics,contracts}
  ```
- [ ] Move `contracts/` to `trading/defi/contracts/`
- [ ] Migrate `arbitrage_bot` to `trading/defi/agents/arbitrage_agent`
- [ ] Migrate `capital_arb_bot` to `trading/defi/agents/capital_agent`
- [ ] Update contract deployment scripts

### Import Path Updates
- [ ] Run automated import fixer for Rust
  ```bash
  find services -name "*.rs" -exec sed -i.bak \
    's/alphapulse_protocol/protocol/g' {} \;
  ```
- [ ] Run automated import fixer for Python
  ```bash
  find services -name "*.py" -exec sed -i.bak \
    's/from backend.shared/from shared/g' {} \;
  ```
- [ ] Validate all imports resolve correctly
- [ ] Test inter-service communication

### Service Validation
- [ ] Each service compiles independently
- [ ] Unit tests pass for each service
- [ ] Integration tests pass between services
- [ ] No circular dependencies detected

## Phase 4: Quality Gates & Validation ✅
*Timeline: Week 4*

### Testing Infrastructure
- [ ] Run full test suite (unit, integration, e2e)
  ```bash
  cargo test --workspace --all-features
  poetry run pytest tests/ -v --cov=services
  ```
- [ ] Execute message protocol specific tests:
  ```bash
  # Bijective ID tests
  cargo test --package protocol --test bijective_id_tests
  # Zerocopy safety tests  
  cargo test --package protocol --test zerocopy_safety_tests
  # CRC32 checksum validation
  cargo test --package protocol --test checksum_tests
  # Dynamic schema tests
  cargo test --package protocol --test schema_registration_tests
  ```
- [ ] Execute property-based tests for data validation
- [ ] Run performance benchmarks (<35μs requirement)
- [ ] Compare results with baseline metrics

### Documentation Generation
- [ ] Generate Rust documentation: `cargo doc --workspace`
- [ ] Generate Python documentation with Sphinx
- [ ] Create architecture diagrams from code
- [ ] Build unified documentation site

### Quality Enforcement
- [ ] Configure Rust linting (clippy) rules
- [ ] Set up Python linting with Ruff
- [ ] Enforce documentation coverage >80%
- [ ] Integrate quality gates in CI/CD

### Performance Validation
- [ ] Benchmark service startup times
- [ ] Measure message throughput
- [ ] Check memory usage patterns
- [ ] Validate no regression in latency

## Phase 5: Final Validation & Rollout 🚀
*Timeline: Week 4-5*

### Staging Deployment
- [ ] Deploy to staging environment
- [ ] Run smoke tests on staging
- [ ] Execute load tests
- [ ] Monitor for 24 hours

### Production Preparation
- [ ] Create production deployment plan
- [ ] Schedule maintenance window
- [ ] Prepare rollback procedures
- [ ] Brief operations team

### Migration Completion
- [ ] Execute production deployment
- [ ] Monitor system health metrics
- [ ] Validate all services operational
- [ ] Document lessons learned

### Post-Migration Cleanup
- [ ] Review `_deprecated/` folder after 30 days
- [ ] Remove truly obsolete files after 90 days
- [ ] Archive migration documentation
- [ ] Celebrate successful completion! 🎉

## Continuous Tasks (Throughout All Phases)

### Communication
- [ ] Daily updates in #backend-migration channel
- [ ] Weekly stakeholder status reports
- [ ] Immediate escalation of blockers
- [ ] Documentation of decisions in MIGRATION.md

### Risk Management
- [ ] Monitor for service disruptions
- [ ] Track performance metrics
- [ ] Maintain rollback readiness
- [ ] Document unexpected issues

### Quality Assurance
- [ ] Continuous integration testing
- [ ] Code review for all changes
- [ ] Documentation updates in real-time
- [ ] Security scanning of changes

## Success Criteria Checklist

### Quantitative Metrics
- [ ] Backend root files: <10 (from 50+)
- [ ] Service duplication: 0 (from multiple)
- [ ] Test coverage: >80% maintained
- [ ] Build time: <10% increase
- [ ] Documentation coverage: >80%

### Qualitative Metrics
- [ ] Developer feedback positive
- [ ] Code discovery improved
- [ ] Onboarding time reduced
- [ ] Architecture clarity enhanced

## Emergency Procedures

### Rollback Triggers
- [ ] Service communication failure
- [ ] >10% performance degradation
- [ ] Data corruption detected
- [ ] Critical production issue

### Rollback Steps
1. [ ] Stop current migration phase
2. [ ] Restore from backup branch
3. [ ] Revert infrastructure changes
4. [ ] Validate system stability
5. [ ] Document root cause
6. [ ] Plan remediation

## Tools and Scripts

### Available Automation
- `migrate_symbol_to_instrument.py` - Terminology migration
- `scripts/migrate_service.sh` - Service migration helper
- `scripts/fix_imports.sh` - Import path fixer
- `scripts/validate_migration.sh` - Migration validator
- `scripts/deprecation_review.sh` - Deprecation management

### Monitoring Commands
```bash
# Check migration progress
cat MIGRATION.md | grep "completed"

# Validate service health
for service in services/*/; do
  cd "$service" && cargo check && cd -
done

# Count backend root files
find backend/ -maxdepth 1 -type f | wc -l

# Check documentation coverage
python -m docstring_coverage services/ --percentage
```

## Notes and Reminders

⚠️ **CRITICAL**: Message protocol migration MUST be completed before ANY file reorganization

🔐 **SAFETY**: Replace all unsafe transmutes with zerocopy for memory safety

🆔 **BIJECTIVE**: Ensure all InstrumentIds are reversible for debugging

✅ **CHECKSUMS**: Validate CRC32 checksums at every message hop

📝 **IMPORTANT**: Update MIGRATION.md after each significant change

🔄 **REMEMBER**: Always use copy-first strategy, validate, then remove

📊 **TRACK**: Performance metrics before and after each phase (<35μs target)

🎯 **FOCUS**: One phase at a time, with full validation between phases