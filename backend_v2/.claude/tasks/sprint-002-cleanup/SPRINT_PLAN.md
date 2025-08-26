# Sprint 002: Code Hygiene & Repository Cleanup
*Sprint Duration: 1 week*
*Objective: Remove code litter and establish clean repository hygiene*

## Sprint Goals
1. Update .gitignore to prevent build artifacts from being tracked
2. Remove deprecated implementations and temporary files
3. Organize development scripts into proper directories
4. Clean up commented code and resolve TODOs

## Task Breakdown

### 🔴 CRITICAL Tasks

#### CLEAN-001: Update .gitignore
**Assignee**: TBD
**Priority**: CRITICAL
**Estimate**: 1 hour
**Dependencies**: None

Update the dangerously sparse .gitignore file with essential entries:
```
# Rust
/target
**/target
Cargo.lock

# macOS
.DS_Store

# Editors
*.swp
*~
*.bak
*#
#*#

# Python
__pycache__/
*.py[cod]
.pytest_cache/

# Temporary files
*.tmp
*.temp
debug_*
test_*
```

#### CLEAN-002: Remove Backup and Temporary Files
**Assignee**: TBD
**Priority**: CRITICAL
**Estimate**: 2 hours
**Dependencies**: CLEAN-001

Delete files:
- `README.org~`
- `#README.md#`
- `#README.org#`
- `Cargo_precision.toml`
- `Cargo_temp.toml`
- Any editor backup files (*~, #*#)

### 🟡 IMPORTANT Tasks

#### CLEAN-003: Organize Development Scripts
**Assignee**: TBD
**Priority**: HIGH
**Estimate**: 3 hours
**Dependencies**: None

Move development/debug scripts to organized structure:
```
scripts/
├── dev/
│   ├── debug_signal_reader.py
│   ├── profile_tlv_bottlenecks.rs
│   ├── serialization_bench.py
│   └── simple_profile.sh
└── test/
    ├── test_rust_signal_relay.py
    ├── test_signal_consumer.py
    ├── test_signal_generation.py
    └── test_signal_sender.py
```

Remove compiled test binaries:
- `test_signal_relay_rust` (binary)

#### CLEAN-004: Remove Deprecated Implementations
**Assignee**: TBD
**Priority**: HIGH
**Estimate**: 2 hours
**Dependencies**: None

Remove legacy code:
- Legacy Python relay implementation if exists (`simple_signal_relay.py`)
- Old `zero_copy_builder.rs` if exists (already commented out in mod.rs)
- Any other files marked as deprecated in comments

### 🟢 MAINTENANCE Tasks

#### CLEAN-005: Clean Commented Code
**Assignee**: TBD
**Priority**: MEDIUM
**Estimate**: 4 hours
**Dependencies**: None

Review and remove large blocks of commented code:
- Focus on `OpportunityDetector` and similar files
- Use git history instead of comments for old code
- Keep only essential explanatory comments

#### CLEAN-006: Process TODO/FIXME Comments
**Assignee**: TBD
**Priority**: MEDIUM
**Estimate**: 3 hours
**Dependencies**: None

Audit all TODO and FIXME comments:
1. Create issues for valid TODOs
2. Remove obsolete TODOs
3. Convert FIXMEs to proper tasks or fix immediately

## Definition of Done
- [ ] .gitignore updated and committed
- [ ] All backup/temp files removed
- [ ] Scripts organized in proper directories
- [ ] No deprecated implementations remain
- [ ] Commented code blocks removed
- [ ] TODO/FIXME audit complete
- [ ] `cargo build` runs without warnings
- [ ] Repository size reduced by at least 10%

## Success Metrics
- **Zero** editor backup files in repository
- **Zero** build artifacts tracked in git
- **<10** TODO/FIXME comments remaining
- **Clean** root directory (no loose scripts)

## Notes
- Use `git rm --cached` for files already tracked that need to be gitignored
- Verify nothing critical is deleted - check with git history first
- Keep development scripts that are still useful, just organize them
- This is hygiene work - no functional changes to production code
