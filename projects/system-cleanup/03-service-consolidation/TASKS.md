# Service Consolidation - Task Checklist

## Core/Services Analysis

### Directory Mapping
- [ ] List all files in core/ directory
  ```bash
  find backend/core -type f > core_files.txt
  ```
- [ ] List all files in services/ directory
  ```bash
  find backend/services -type f > services_files.txt
  ```
- [ ] Compare directory structures
  ```bash
  diff -r backend/core backend/services > duplicates.txt
  ```
- [ ] Identify unique vs duplicate files
- [ ] Document service purposes

### Duplicate Detection
- [ ] Find files with same names
  ```bash
  comm -12 <(basename -a backend/core/*) <(basename -a backend/services/*)
  ```
- [ ] Compare file contents for duplicates
- [ ] Check function/class definitions
- [ ] Identify which version is newer
- [ ] Determine canonical version

### Dependency Analysis
- [ ] Map service dependencies
- [ ] Check for circular dependencies
- [ ] Document shared code
- [ ] Identify protocol usage
- [ ] Plan consolidation order

## Service Migration

### Backup Creation
- [ ] Create safety backup
  ```bash
  cp -r backend/core _deprecated/$(date +%Y%m%d)-core-backup
  echo "Core backup created" >> MIGRATION.md
  ```
- [ ] Document backup location
- [ ] Test restoration procedure
- [ ] Verify backup completeness
- [ ] Set retention policy

### Unique File Migration
- [ ] Identify unique files in core/
- [ ] Create destination directories
- [ ] Move unique files to services/
  ```bash
  for file in $(find core -type f); do
    if [ ! -f "services/$(basename $file)" ]; then
      mv "$file" services/
      echo "Moved unique: $file" >> MIGRATION.md
    fi
  done
  ```
- [ ] Update import paths
- [ ] Test moved services

### Duplicate Resolution
- [ ] Compare duplicate implementations
- [ ] Choose better version
- [ ] Merge unique features
- [ ] Document decisions
- [ ] Archive rejected versions

### Core Directory Removal
- [ ] Verify all files moved
- [ ] Check for hidden files
- [ ] Update workspace configs
- [ ] Remove empty directories
  ```bash
  rmdir backend/core
  ```
- [ ] Update documentation

## DeFi System Setup

### Directory Structure
- [ ] Create DeFi directories
  ```bash
  mkdir -p backend/trading/defi/{core,protocols,strategies,execution,agents,analytics,contracts}
  ```
- [ ] Document structure purpose
- [ ] Set access permissions
- [ ] Create README files
- [ ] Add to git tracking

### Contract Migration
- [ ] Locate contracts/ directory
- [ ] Review contract organization
- [ ] Move to DeFi structure
  ```bash
  mv contracts/* backend/trading/defi/contracts/
  rmdir contracts/
  ```
- [ ] Update deployment scripts
- [ ] Fix hardhat config paths

### Agent Migration
- [ ] Find arbitrage_bot
- [ ] Find capital_arb_bot
- [ ] Move to agents directory
  ```bash
  mv services/arbitrage_bot trading/defi/agents/arbitrage_agent
  mv services/capital_arb_bot trading/defi/agents/capital_agent
  ```
- [ ] Update agent configurations
- [ ] Test agent functionality

### Strategy Organization
- [ ] Identify strategy files
- [ ] Create strategy categories
- [ ] Move strategy implementations
- [ ] Update strategy imports
- [ ] Document strategy purposes

## Import Path Updates

### Rust Import Updates
- [ ] Scan for core:: imports
  ```bash
  grep -r "use core::" --include="*.rs"
  ```
- [ ] Update to services:: imports
  ```bash
  find . -name "*.rs" -exec sed -i.bak 's/use core::/use services::/g' {} \;
  ```
- [ ] Update module declarations
- [ ] Fix workspace dependencies
- [ ] Run cargo check

### Python Import Updates
- [ ] Scan for backend.core imports
  ```bash
  grep -r "from backend.core" --include="*.py"
  ```
- [ ] Update to backend.services
  ```bash
  find . -name "*.py" -exec sed -i.bak 's/from backend\.core/from backend.services/g' {} \;
  ```
- [ ] Fix relative imports
- [ ] Update __init__.py files
- [ ] Run pylint checks

### Configuration Updates
- [ ] Update service configs
- [ ] Fix path references
- [ ] Update environment variables
- [ ] Modify docker-compose
- [ ] Update CI/CD configs

## Service Validation

### Compilation Tests
- [ ] Test each Rust service
  ```bash
  for service in backend/services/*/; do
    cd "$service" && cargo check && cd -
  done
  ```
- [ ] Test Python imports
- [ ] Check TypeScript builds
- [ ] Verify no missing deps
- [ ] Document issues

### Unit Testing
- [ ] Run service unit tests
  ```bash
  cargo test --workspace
  python -m pytest tests/unit/
  ```
- [ ] Fix failing tests
- [ ] Update test imports
- [ ] Add missing tests
- [ ] Document coverage

### Integration Testing
- [ ] Test service communication
- [ ] Verify message passing
- [ ] Check data flow
- [ ] Test error handling
- [ ] Validate protocols

## Architecture Alignment

### Documentation Updates
- [ ] Update architecture diagrams
- [ ] Fix service descriptions
- [ ] Update API documentation
- [ ] Correct README files
- [ ] Update wiki/handbook

### Dependency Graph
- [ ] Generate service dependencies
  ```bash
  cargo tree --workspace > dependencies.txt
  ```
- [ ] Create visual diagram
- [ ] Identify bottlenecks
- [ ] Document interfaces
- [ ] Plan optimizations

### Protocol Validation
- [ ] Check message formats
- [ ] Verify protocol versions
- [ ] Test serialization
- [ ] Validate schemas
- [ ] Document changes

## Quality Assurance

### Code Quality
- [ ] Run linters
  ```bash
  cargo clippy --workspace
  ruff check backend/
  ```
- [ ] Fix warnings
- [ ] Check formatting
- [ ] Review error handling
- [ ] Document TODOs

### Performance Testing
- [ ] Benchmark before/after
- [ ] Check memory usage
- [ ] Test throughput
- [ ] Measure latency
- [ ] Document results

### Security Review
- [ ] Check access controls
- [ ] Review secrets handling
- [ ] Scan dependencies
- [ ] Test input validation
- [ ] Document risks

## Final Validation

### Service Health
- [ ] All services compile
- [ ] All tests pass
- [ ] No import errors
- [ ] Clean dependency tree
- [ ] Documentation complete

### System Testing
- [ ] Start all services
- [ ] Test data flow
- [ ] Check monitoring
- [ ] Verify logging
- [ ] Test shutdown

### Deployment Readiness
- [ ] Update deployment scripts
- [ ] Test in staging
- [ ] Prepare rollback plan
- [ ] Document changes
- [ ] Get team approval

## Post-Consolidation

### Cleanup
- [ ] Remove backup files (.bak)
- [ ] Clean deprecated code
- [ ] Update .gitignore
- [ ] Archive old configs
- [ ] Document removals

### Team Communication
- [ ] Send consolidation summary
- [ ] Update team documentation
- [ ] Conduct review meeting
- [ ] Gather feedback
- [ ] Plan improvements

### Monitoring
- [ ] Watch for errors
- [ ] Track performance
- [ ] Monitor builds
- [ ] Check user reports
- [ ] Document issues

## Success Verification

### Metrics
- [ ] Zero duplicate services
- [ ] All imports resolved
- [ ] 100% tests passing
- [ ] Clean architecture
- [ ] Team satisfaction

### Documentation
- [ ] Migration complete in MIGRATION.md
- [ ] Architecture diagrams updated
- [ ] README files current
- [ ] Wiki/handbook updated
- [ ] Lessons learned documented