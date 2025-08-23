# Backend Cleanup - Task Checklist

## Pre-Cleanup Analysis

### File Inventory
- [ ] Count total files in backend root
  ```bash
  find backend/ -maxdepth 1 -type f | wc -l
  ```
- [ ] Generate detailed file list
  ```bash
  ls -la backend/ > backend_files_before.txt
  ```
- [ ] Categorize files by type and purpose
- [ ] Identify large files (>1MB)
- [ ] Check file modification dates

### Dependency Analysis
- [ ] Scan for imports of root-level files
  ```bash
  grep -r "from backend\." --include="*.py" | grep -v "from backend.services"
  ```
- [ ] Check Rust module references
- [ ] Identify CI/CD path dependencies
- [ ] Document configuration file usage
- [ ] Map script interdependencies

### Migration Planning
- [ ] Create file migration map
- [ ] Document new locations for each file
- [ ] Identify files to delete vs archive
- [ ] Plan import update strategy
- [ ] Schedule migration windows

## Test File Organization

### Python Test Files
- [ ] Identify all test_*.py files
  ```bash
  find backend/ -maxdepth 1 -name "test_*.py" -type f
  ```
- [ ] Create scripts/testing/ directory
- [ ] Move test files to new location
  ```bash
  for file in test_*.py; do
    mv "$file" scripts/testing/
    echo "Moved $file" >> MIGRATION.md
  done
  ```
- [ ] Update any import references
- [ ] Verify tests still run

### Rust Test Files
- [ ] Identify test_*.rs files
- [ ] Determine if integration or unit tests
- [ ] Move to appropriate test directory
- [ ] Update Cargo.toml if needed
- [ ] Run cargo test to verify

### Shell Test Scripts
- [ ] Find test_*.sh files
- [ ] Move to scripts/testing/
- [ ] Update execution permissions
- [ ] Test script functionality
- [ ] Update CI/CD references

## Debug File Cleanup

### Debug Scripts
- [ ] Locate debug_*.py files
  ```bash
  find backend/ -maxdepth 1 -name "debug_*.py" -type f
  ```
- [ ] Create scripts/debug/ directory
- [ ] Move debug files
  ```bash
  for file in debug_*.py; do
    mv "$file" scripts/debug/
    echo "Moved $file" >> MIGRATION.md
  done
  ```
- [ ] Check for active usage
- [ ] Document debug tool purpose

### Temporary Debug Files
- [ ] Identify *_debug.* patterns
- [ ] Review for important logic
- [ ] Archive or delete as appropriate
- [ ] Update .gitignore for future
- [ ] Clean git history if needed

## Service File Migration

### FastAPI Application
- [ ] Locate app_fastapi.py
- [ ] Create services/api_server/ directory
- [ ] Move to services/api_server/main.py
  ```bash
  mkdir -p services/api_server/
  mv app_fastapi.py services/api_server/main.py
  ```
- [ ] Update import statements
- [ ] Fix startup scripts
- [ ] Test API functionality

### Standalone Collectors
- [ ] Find *_collector.py files
- [ ] Create services/collectors_legacy/
- [ ] Move collector files
- [ ] Update configuration references
- [ ] Verify collector functionality

### Service Utilities
- [ ] Identify service-specific utilities
- [ ] Move to appropriate service directory
- [ ] Update relative imports
- [ ] Test service integration
- [ ] Document utility purpose

## Maintenance Script Organization

### Migration Scripts
- [ ] Locate migrate_*.py files
- [ ] Create scripts/maintenance/migration/
- [ ] Move migration scripts
  ```bash
  mkdir -p scripts/maintenance/migration/
  mv migrate_*.py scripts/maintenance/migration/
  ```
- [ ] Document migration history
- [ ] Archive completed migrations

### Cleanup Scripts
- [ ] Find cleanup.sh and similar
- [ ] Move to scripts/maintenance/
- [ ] Update cron jobs if applicable
- [ ] Test script execution
- [ ] Document maintenance schedule

### Database Scripts
- [ ] Identify DB maintenance scripts
- [ ] Move to scripts/maintenance/database/
- [ ] Update connection strings
- [ ] Test database operations
- [ ] Document backup procedures

## Log File Management

### Clean Existing Logs
- [ ] Identify all *.log files
  ```bash
  find backend/ -maxdepth 1 -name "*.log" -type f
  ```
- [ ] Archive important logs
  ```bash
  mkdir -p archive/logs/$(date +%Y%m%d)
  mv *.log archive/logs/$(date +%Y%m%d)/
  ```
- [ ] Remove from git tracking
  ```bash
  git rm --cached *.log
  ```
- [ ] Add to .gitignore
- [ ] Clean git history if large

### Log Directory Setup
- [ ] Create logs/ directory
- [ ] Configure log rotation
- [ ] Update logging configuration
- [ ] Set appropriate permissions
- [ ] Document log retention policy

## Configuration File Organization

### Environment Configs
- [ ] Identify .env files
- [ ] Create config/environments/
- [ ] Move environment configs
- [ ] Update application references
- [ ] Secure sensitive configs

### Service Configs
- [ ] Find service-specific configs
- [ ] Move to config/services/
- [ ] Update config loaders
- [ ] Validate configuration loading
- [ ] Document config structure

## Git Hygiene

### Update .gitignore
- [ ] Add comprehensive ignore rules
  ```bash
  cat >> .gitignore << 'EOF'
  # Logs
  *.log
  logs/
  
  # Test artifacts
  .coverage
  htmlcov/
  .pytest_cache/
  
  # Build artifacts
  target/
  __pycache__/
  *.pyc
  
  # IDE
  .vscode/
  .idea/
  *.swp
  
  # Temporary
  *.tmp
  *.bak
  *~
  
  # Local environment
  .env.local
  EOF
  ```
- [ ] Remove already-tracked files
- [ ] Commit .gitignore updates
- [ ] Verify clean git status
- [ ] Document ignore patterns

### Repository Cleanup
- [ ] Remove large files from history
- [ ] Clean up outdated branches
- [ ] Archive old tags
- [ ] Update README paths
- [ ] Fix broken links

## Documentation Updates

### Path References
- [ ] Update README.md file paths
- [ ] Fix documentation links
- [ ] Update API documentation
- [ ] Correct setup instructions
- [ ] Update deployment guides

### Migration Documentation
- [ ] Create FILE_MIGRATION_MAP.md
- [ ] Document old → new paths
- [ ] Note deleted files
- [ ] List archived content
- [ ] Add to team wiki

## Import Updates

### Python Imports
- [ ] Scan for broken imports
  ```bash
  python -c "import sys; sys.path.append('backend'); import services"
  ```
- [ ] Update import paths
- [ ] Use automated fixer
  ```bash
  find . -name "*.py" -exec sed -i.bak 's/from backend\./from services\./g' {} \;
  ```
- [ ] Verify with pylint
- [ ] Run unit tests

### Rust Imports
- [ ] Check module paths
- [ ] Update use statements
- [ ] Fix workspace paths
- [ ] Run cargo check
- [ ] Execute cargo test

## Validation

### Compilation Checks
- [ ] Rust services compile
  ```bash
  cd backend && cargo check --workspace
  ```
- [ ] Python imports resolve
- [ ] TypeScript builds
- [ ] No missing dependencies
- [ ] All tests pass

### Functional Testing
- [ ] API endpoints respond
- [ ] Services start correctly
- [ ] Data flow works
- [ ] No performance regression
- [ ] Monitoring still works

### CI/CD Validation
- [ ] Pipeline runs successfully
- [ ] All jobs pass
- [ ] Deployment works
- [ ] No path errors
- [ ] Artifacts generate correctly

## Final Cleanup

### Deprecation Area Review
- [ ] Move old files to _deprecated/
- [ ] Document deprecation reason
- [ ] Set review date
- [ ] Update MIGRATION.md
- [ ] Notify team

### Verification
- [ ] Backend root has <10 files
- [ ] All files categorized correctly
- [ ] Documentation updated
- [ ] Team handbook updated
- [ ] Success metrics met

## Post-Cleanup Tasks

### Team Communication
- [ ] Send migration summary
- [ ] Update team wiki
- [ ] Conduct knowledge transfer
- [ ] Document lessons learned
- [ ] Schedule follow-up review

### Monitoring
- [ ] Check for broken builds
- [ ] Monitor error logs
- [ ] Track performance metrics
- [ ] Watch for import errors
- [ ] Gather team feedback

### Future Improvements
- [ ] Identify remaining issues
- [ ] Plan phase 2 improvements
- [ ] Document technical debt
- [ ] Update roadmap
- [ ] Celebrate success! 🎉