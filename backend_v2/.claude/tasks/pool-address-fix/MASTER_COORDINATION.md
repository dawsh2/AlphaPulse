# Critical Production Fixes - Master Coordination
*Sprint: CRITICAL-FIXES-001*
*Created: 2025-08-25*

## 🎯 Sprint Goals
Fix critical production blockers that prevent the system from functioning correctly:
1. Pool/token address extraction using placeholders
2. Precision loss in signal output destroying profits
3. Checksum validation killing performance
4. Minor fixes for code quality and stability

## 📊 Task Distribution Matrix

| Task ID | Agent Type | Branch Name | Dependencies | Priority | Status |
|---------|------------|-------------|--------------|----------|--------|
| POOL-001 | Integration Specialist | `fix/pool-cache-integration` | None | 🔴 Critical | ✅ Documented |
| POOL-002 | Event Parser | `fix/swap-event-extraction` | None | 🔴 Critical | ✅ Documented |
| POOL-003 | Integration Specialist | `fix/discovery-integration` | POOL-001 | 🔴 Critical | ✅ Documented |
| POOL-004 | RPC Integration | `fix/rpc-pool-discovery` | POOL-001 | 🔴 Critical | ✅ Documented |
| POOL-005 | Protocol Integration | `fix/tlv-pool-integration` | POOL-001,002 | 🔴 Critical | ✅ Documented |
| POOL-006 | Test Engineer | `test/pool-integration-validation` | ALL POOL | 🔴 Critical | ✅ Documented |
| PRECISION-001 | Precision Specialist | `fix/signal-precision-loss` | None | 🔴 Critical | ✅ Documented |
| PERF-001 | Performance Specialist | `fix/checksum-sampling` | None | 🔴 Critical | ✅ Documented |

## 🔄 Git Workflow for All Agents

### Initial Setup (Each Agent)
```bash
# 1. Ensure you're on main and up to date
git checkout main
git pull origin main

# 2. Create your feature branch
git checkout -b [branch-name-from-table]

# 3. Verify you're on the correct branch
git branch --show-current
```

### During Development
```bash
# Commit frequently with clear messages
git add -A
git commit -m "feat(pool): describe specific change"

# Push to remote regularly (creates remote branch on first push)
git push -u origin [branch-name]
```

### Creating Pull Request
```bash
# 1. Ensure all changes committed
git status

# 2. Push final changes
git push origin [branch-name]

# 3. Create PR via GitHub CLI (if available)
gh pr create --title "Task [TASK-ID]: Brief description" \
             --body "$(cat .claude/tasks/pool-address-fix/[TASK-ID]_complete.md)"

# Or provide instructions to create via GitHub UI
```

## 🔀 Merge Order & Dependencies

### Phase 1: Independent Tasks (Can merge in any order)
- POOL-001: Cache integration (USE EXISTING `libs/state/market/src/pool_cache.rs`)
- POOL-002: Event extraction
- PRECISION-001: Signal precision fix
- PERF-001: Checksum sampling

### Phase 2: Dependent Tasks (Merge after POOL-001)
- POOL-003: Discovery queue (needs POOL-001)
- POOL-004: RPC queries (needs POOL-001)

### Phase 3: Integration (Merge after Phase 1 & 2)
- POOL-005: TLV integration (needs POOL-001, POOL-002, ideally POOL-003/004)

### Phase 4: Validation (Merge last)
- POOL-006: Comprehensive tests (needs all POOL tasks)

## 📋 Completion Checklist

### Per-Task Requirements
- [ ] Branch created from latest main
- [ ] Task-specific acceptance criteria met
- [ ] Unit tests passing
- [ ] No merge conflicts with main
- [ ] Performance benchmarks maintained (<35μs hot path)
- [ ] PR created with template

### Epic Completion
- [ ] All 8 critical tasks merged to main
- [ ] Integration test suite passing
- [ ] Pool cache achieving >99% hit rate
- [ ] Production deployment validated
- [ ] Documentation updated

## 🚨 Coordination Points

### Daily Sync Questions
1. Any blockers discovered?
2. Any API changes affecting other tasks?
3. Any performance issues found?
4. Need to adjust task boundaries?

### Conflict Resolution
- If two branches modify same file: Coordinate in this document
- If API changes needed: Update all dependent tasks
- If blocking issue found: Mark status as ⚠️ and escalate

## 📊 Progress Tracking

### Metrics to Monitor
- Cache hit ratio: Target >99%
- Discovery queue depth: Should stay <100
- RPC call latency: <5s timeout
- Hot path performance: <35μs with cache hit
- Memory usage: <50MB for cache

### Success Validation
```bash
# Run after all merges
cargo test --package services_v2 --test pool_integration
cargo bench --package protocol_v2 -- pool_swap
./scripts/validate_pool_extraction.sh
```

## 📝 Notes & Learnings
<!-- Add discovered issues, gotchas, or important learnings here -->

---
*Use `git log --oneline --graph --all` to visualize branch relationships*