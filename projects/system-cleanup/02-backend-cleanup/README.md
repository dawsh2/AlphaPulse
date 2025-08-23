# Backend Cleanup - Mission Statement

## Mission
Transform the chaotic backend root directory containing 50+ scattered files into a clean, organized structure where every file has a clear purpose and location, improving developer productivity and system maintainability.

## Core Objectives
1. **File Organization**: Reduce backend root files from 50+ to <10
2. **Test Separation**: Move all test files to dedicated directories
3. **Script Consolidation**: Organize operational scripts by purpose
4. **Git Hygiene**: Clean up tracked files and improve .gitignore
5. **Documentation Updates**: Fix all file references and paths

## Current State Analysis

### The Chaos (What We're Fixing)
```
backend/
├── 50+ files scattered at root level  # 😱 This is the problem
├── app_fastapi.py                    # Should be in services/
├── kraken_collector.py               # Should be in services/
├── test_*.py files                   # Should be in tests/
├── debug_*.py files                  # Should be in scripts/
├── *.log files                       # Should be gitignored
├── simple_pol_test*                  # Temporary files
├── migrate_*.py                      # Migration scripts
└── Random utility scripts            # Need organization
```

### The Goal (Clean Structure)
```
backend/
├── Cargo.toml                    # Rust workspace root ✓
├── requirements.txt              # Python dependencies ✓
├── README.md                     # Backend documentation ✓
├── services/                     # All services organized ✓
├── shared/                       # Cross-cutting concerns ✓
├── protocol/                     # Binary protocol ✓
├── config/                       # Configuration files ✓
└── scripts/                      # Operational scripts ✓
```

## Strategic Value
- **Developer Experience**: Find files in seconds, not minutes
- **Onboarding Speed**: New developers understand structure immediately
- **Build Performance**: Cleaner workspace = faster builds
- **Maintenance Reduction**: Less "where is X?" questions
- **Professional Image**: Clean codebase reflects engineering excellence

## File Migration Strategy

### Categories and Destinations

#### Test Files → `scripts/testing/`
- `test_*.py` - Python test scripts
- `test_*.rs` - Rust test files
- `test_*.sh` - Shell test scripts

#### Debug Files → `scripts/debug/`
- `debug_*.py` - Debug utilities
- `*_debug.py` - Alternative naming
- Temporary debugging scripts

#### Service Files → `services/`
- `app_fastapi.py` → `services/api_server/main.py`
- `*_collector.py` → `services/collectors_legacy/`
- Service-specific utilities

#### Maintenance → `scripts/maintenance/`
- `cleanup.sh` - Cleanup scripts
- `migrate_*.py` - Migration utilities
- Database maintenance scripts

#### Archive → `archive/temp/`
- `*.log` - Log files (then gitignore)
- `simple_pol_test*` - Temporary experiments
- Old backup files

## Deliverables
- [ ] Backend root contains <10 files
- [ ] All files logically organized by purpose
- [ ] No test files in production directories
- [ ] Clean git status with proper ignores
- [ ] Updated documentation with new paths
- [ ] Migration map for reference

## Organizational Note
**Important**: File cleanup may reveal hidden dependencies:
1. **Import Dependencies**: Files may be imported from unexpected places
2. **Script Dependencies**: CI/CD may reference specific paths
3. **Configuration Files**: May need careful relocation
4. **Historical Scripts**: May contain important but undocumented logic

Expected subdirectories for complex issues:
```
02-backend-cleanup/
├── import-analysis/           # Tracking import dependencies
├── script-dependencies/       # CI/CD and automation dependencies
├── configuration-migration/   # Config file reorganization
├── historical-scripts/       # Important legacy code
└── file-mapping/            # Detailed migration tracking
```

## Success Criteria
- **File Count**: Backend root has <10 files (from 50+)
- **Organization**: Clear purpose for every file location
- **Discoverability**: Any file found in <10 seconds
- **Git Status**: Clean with no untracked files
- **Documentation**: All paths updated and accurate

## Risk Mitigation
- **Import Breakage**: Scan for imports before moving
- **CI/CD Failure**: Check pipeline references
- **Lost Files**: Copy-first strategy with deprecation area
- **Team Confusion**: Clear migration map and communication

## Migration Approach

### Phase 1: Analysis and Preparation
1. Catalog all files in backend root
2. Identify dependencies and imports
3. Check CI/CD references
4. Create migration map

### Phase 2: Safe Migration
1. Copy files to new locations (don't move yet)
2. Update imports in copied files
3. Test that copies work correctly
4. Move originals to deprecation area
5. Validate everything still works

### Phase 3: Cleanup
1. Update .gitignore comprehensively
2. Remove files from git tracking
3. Clean up deprecation area
4. Update all documentation

## Timeline
- **Day 1**: File analysis and categorization
- **Day 2**: Migration map and dependency checking
- **Day 3-4**: File migration with validation
- **Day 5**: Import updates and testing
- **Day 6-7**: Documentation and final cleanup

## Next Steps
1. Run file analysis script
2. Create detailed migration map
3. Check import dependencies
4. Begin systematic migration
5. Validate at each step