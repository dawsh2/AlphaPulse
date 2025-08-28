# Torq Development Backlog

This file tracks tasks that have been deferred from completed sprints and other low-priority items that should be addressed in future maintenance sprints.

## 📋 Current Backlog Items

### From Sprint 013 (Architecture Audit) - Completed 2025-08-27

#### AUDIT-006: Consolidate Python Scripts
- **Original Sprint**: Sprint 013 (Architecture Audit)
- **Priority**: LOW
- **Estimated Hours**: 2
- **Branch**: `feat/consolidate-python-scripts`
- **Description**: Reorganize scattered Python scripts into a consolidated structure under `scripts/lib/python/`
- **Scope**: 
  - Create `scripts/lib/python/` directory
  - Reorganize existing `scripts/*.py` files
- **Dependencies**: AUDIT-005 (manage.sh script - COMPLETED)
- **Rationale for Deferral**: Non-critical maintenance task that doesn't impact core architecture functionality

#### AUDIT-008: Update Architecture Documentation  
- **Original Sprint**: Sprint 013 (Architecture Audit)
- **Priority**: LOW
- **Estimated Hours**: 2  
- **Branch**: `feat/architecture-documentation-update`
- **Description**: Update main project and architecture documentation to reflect completed codec integration and plugin architecture
- **Scope**:
  - `README.md` - Main project documentation
  - `docs/ARCHITECTURE.md` - Architecture documentation
- **Dependencies**: 
  - AUDIT-002 (codec integration - COMPLETED)
  - AUDIT-003 (plugin architecture - COMPLETED) 
  - AUDIT-005 (manage.sh usage - COMPLETED)
- **Rationale for Deferral**: Architecture is now stable, documentation updates can be batched with other documentation efforts

## 📊 Backlog Management

### Prioritization Guidelines
- **HIGH**: Security issues, critical bugs, performance regressions
- **MEDIUM**: Feature enhancements, significant improvements
- **LOW**: Documentation updates, code organization, maintenance

### Review Process
- Review backlog monthly during sprint planning
- Re-prioritize based on current system needs
- Archive items that are no longer relevant
- Consider batching similar tasks (e.g., all documentation updates)

### Task Activation Process
When moving a backlog item to active development:
1. Update priority based on current context
2. Refresh estimated hours if needed
3. Verify dependencies are still accurate
4. Create new sprint or add to existing sprint
5. Update task status from BACKLOG to TODO

## 📈 Backlog Statistics
- **Total Items**: 2
- **From Completed Sprints**: 2
- **Priority Breakdown**: 
  - HIGH: 0
  - MEDIUM: 0  
  - LOW: 2

## 🗓️ Next Review Date
**Target**: During next sprint planning session

## 📝 Notes
- Backlog items from Sprint 013 represent good architecture foundations being in place - these deferrals indicate successful prioritization of critical work
- Both items can be addressed in a dedicated maintenance/documentation sprint when resources allow