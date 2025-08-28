---
task_id: S015-DOCS-001
status: TODO  ← CHANGE TO "IN_PROGRESS" WHEN STARTING, THEN "COMPLETE" WHEN FINISHED!
priority: HIGH
estimated_hours: 4
assigned_branch: docs/readme-architectural-restructure
assignee: TBD
created: 2025-08-27
completed: null
# Dependencies: Should run in parallel with VALIDATE-001
depends_on: []
# Blocks: Supports all validation tasks by providing clear architectural context
blocks: []
# Scope: All README.md files across the project
scope: ["**/README.md", "docs/", "torq/*/README.md"]
---

# DOCS-001: README.md Architectural Restructure

**🚨 CRITICAL**: Update status to COMPLETE when finished!

## 🔴 CRITICAL INSTRUCTIONS

### 0. 📋 MARK AS IN-PROGRESS IMMEDIATELY
**⚠️ FIRST ACTION: Change status when you start work!**
```yaml
# Edit the YAML frontmatter above:
status: TODO → status: IN_PROGRESS

# This makes the kanban board show you're working on it!
```

### 1. Git Worktree Setup (REQUIRED)
```bash
# NEVER use git checkout - it changes all sessions!
# ALWAYS use git worktree for isolated development:
git worktree add -b docs/readme-architectural-restructure ../docs-001-worktree
cd ../docs-001-worktree

# Verify you're in the correct worktree:
git branch --show-current  # Should show: docs/readme-architectural-restructure
pwd  # Should show: ../docs-001-worktree

# NEVER work directly in main repository!
```

## Problem Statement

**Documentation Strategy Shift**: Current README.md files contain extensive technical details that should live in rustdoc. This creates:

- **Maintenance Burden**: Technical details duplicated between README.md and code comments
- **Documentation Drift**: README.md files become stale while rustdoc stays current with code  
- **Poor Developer Experience**: Developers must check multiple sources for technical information
- **Inconsistent Information Architecture**: No clear distinction between architectural overview and technical reference

**Goal**: Transform all README.md files to serve as **architectural overviews only** (<200 lines), with all technical details moved to rustdoc inline documentation.

## Documentation Philosophy Change

### ❌ OLD APPROACH: README.md as Technical Reference
```markdown
# Component X

## API Reference
### Function: process_data(input: Data) -> Result<Output, Error>
- **Parameters**: input - The data to process (must be validated)
- **Returns**: Processed output or validation error  
- **Example**: 
  ```rust
  let result = process_data(validated_input)?;
  ```
- **Error Handling**: Returns ValidationError for invalid input
- **Performance**: O(n) complexity for input size n
```

### ✅ NEW APPROACH: README.md as Architectural Overview
```markdown
# Component X

High-level component responsible for data processing within the message pipeline.

## Architecture
- **Purpose**: Transform and validate input data
- **Dependencies**: types, codec libraries
- **Integration**: Used by adapters and relays  

## Technical Details
See [generated rustdoc](https://docs.rs/component-x) for complete API reference, examples, and implementation details.
```

## Acceptance Criteria

### **README.md Compliance Standards**
- [ ] **Length Limit**: All README.md files <200 lines (enforced in CI/CD)
- [ ] **Architectural Focus**: Only high-level purpose, structure, key concepts
- [ ] **No Technical Details**: No API signatures, parameter lists, or implementation specifics
- [ ] **Rustdoc References**: Clear pointers to `cargo doc --open` for technical details
- [ ] **Consistent Structure**: Standardized sections across all README.md files

### **Content Migration Requirements**
- [ ] **Technical Details Moved**: All API docs, examples, error handling moved to rustdoc
- [ ] **Cross-References Updated**: Links between README.md files updated for new structure
- [ ] **Architectural Clarity**: Purpose and boundaries of each component clear from README.md alone
- [ ] **Navigation Aids**: Clear entry points for developers to find detailed information

### **Quality Standards**
- [ ] **Information Architecture**: Logical hierarchy from high-level (README.md) to technical (rustdoc)
- [ ] **No Duplication**: Technical information exists only in rustdoc, not README.md
- [ ] **Completeness**: All existing technical details preserved in appropriate rustdoc locations
- [ ] **Accessibility**: Developers can understand component purpose without reading code

## Implementation Strategy

### 1. **README.md Structure Standardization**
Create consistent template for all README.md files:

```markdown
# [Component Name]

[One-sentence description of purpose]

## Overview
[2-3 paragraphs describing what this component does at high level]

## Architecture
- **Purpose**: [Why this component exists]
- **Key Concepts**: [2-3 main ideas developers need to understand]
- **Dependencies**: [Major dependencies, not exhaustive list]
- **Integration Points**: [How other components use this]

## Directory Structure
```
component/
├── src/           # [Brief description]
├── tests/         # [Brief description]  
└── examples/      # [Brief description]
```

## Technical Reference
See [generated documentation](`cargo doc --open`) for complete API reference, usage examples, and implementation details.

## Quick Start
[Optional: 2-3 line code snippet showing most common usage, if helpful]

## Related Components
- [`other-component`](../other-component/) - [One line describing relationship]
```

### 2. **Content Audit and Migration Process**
For each README.md file:

1. **Identify Technical Content**:
   ```bash
   # Find README.md files with technical details
   find . -name "README.md" -exec grep -l "fn \|struct \|enum \|impl \|Example:\|Parameters:\|Returns:" {} \;
   ```

2. **Extract Architectural Content**:
   - Component purpose and goals
   - High-level design decisions
   - Integration relationships  
   - Directory structure overview

3. **Move Technical Content**:
   - API signatures → rustdoc on functions/structs
   - Usage examples → rustdoc examples or `examples/` directory  
   - Parameter details → rustdoc parameter documentation
   - Error handling → rustdoc error sections

4. **Verify Preservation**:
   - All technical information moved to appropriate rustdoc location
   - No information lost in migration
   - Rustdoc examples compile and work correctly

### 3. **README.md File Priority Order**
Process in this order for maximum impact:

**Phase 1 - Core Components** (Day 1):
1. `torq/libs/types/README.md` - Core type definitions
2. `torq/libs/codec/README.md` - Protocol logic  
3. `torq/protocol_v2/README.md` - Protocol specification
4. Root `README.md` - Project overview

**Phase 2 - Service Components** (Day 1 continued):
5. `torq/services/adapters/README.md` - Adapter interfaces
6. `torq/services/strategies/README.md` - Strategy implementations
7. `torq/relays/README.md` - Relay infrastructure

**Phase 3 - Supporting Components** (Day 1 wrap-up):
8. `torq/tests/README.md` - Testing strategy
9. `docs/README.md` - Documentation guide
10. Any remaining component README.md files

## Validation Steps

### **Length and Content Validation**
```bash
# Check line count for all README.md files
find . -name "README.md" -exec wc -l {} \; | sort -n

# Identify files exceeding 200 lines
find . -name "README.md" -exec sh -c 'lines=$(wc -l < "$1"); if [ $lines -gt 200 ]; then echo "$1: $lines lines"; fi' _ {} \;

# Search for technical content that should be in rustdoc
find . -name "README.md" -exec grep -H -n "fn \|struct \|enum \|impl \|Parameters:\|Returns:\|Example:" {} \;
```

### **Cross-Reference Validation**
```bash
# Verify README.md files reference rustdoc appropriately
find . -name "README.md" -exec grep -L "cargo doc\|rustdoc\|generated documentation" {} \;

# Check for broken internal links
find . -name "README.md" -exec grep -H -n "\[.*\](\.\.\/.*)" {} \;
```

### **Architectural Clarity Check**
For each README.md file, verify:
- [ ] Purpose is clear from first paragraph
- [ ] Role in overall system is explained
- [ ] Integration points are identified
- [ ] Technical details appropriately deferred to rustdoc

## Success Metrics

### **Quantitative Metrics**
- [ ] **100% Compliance**: All README.md files <200 lines
- [ ] **Zero Technical Details**: No API signatures or implementation details in README.md files
- [ ] **Complete Migration**: All technical content successfully moved to rustdoc
- [ ] **Link Validation**: All cross-references work correctly

### **Qualitative Metrics**  
- [ ] **Developer Experience**: New developers can understand component architecture from README.md alone
- [ ] **Information Hierarchy**: Clear progression from architectural (README.md) to technical (rustdoc)
- [ ] **Maintenance Reduction**: Technical details only need updating in rustdoc, not README.md
- [ ] **Consistency**: All README.md files follow consistent structure and style

## Git Workflow
```bash
# 1. Working in correct worktree (already created above)
cd ../docs-001-worktree

# 2. Process README.md files systematically
git add README.md                    # Add individual files as completed
git commit -m "docs: restructure [component] README.md to architectural focus"

# 3. Process all priority README.md files
# ... continue with systematic commits for each component

# 4. Final validation and push
git add -A
git commit -m "docs: complete README.md architectural restructure

- All README.md files <200 lines
- Technical details moved to rustdoc  
- Consistent architectural focus
- Clear rustdoc references"

git push origin docs/readme-architectural-restructure

# 5. Create PR
gh pr create --title "docs: Restructure README.md files to architectural focus" --body "
## Summary
- Restructure all README.md files to <200 lines, architectural focus only
- Move technical details to rustdoc inline documentation  
- Establish consistent information architecture: README.md (architectural) → rustdoc (technical)
- Improve developer experience with clear separation of concerns

## Validation
- [ ] All README.md files <200 lines
- [ ] No technical API details in README.md files  
- [ ] Clear rustdoc references for technical information
- [ ] Consistent structure across all components

Closes DOCS-001"
```

## Integration with Other Tasks

This task supports all validation tasks by:
- **VALIDATE-001 to VALIDATE-014**: Providing clear architectural context for validation work
- **DOCS-002**: Establishing foundation for rustdoc-focused technical documentation  
- **DOCS-003**: Creating consistent entry points for documentation navigation
- **DOCS-004**: Enabling automated validation of documentation structure

## Notes
[Space for implementation notes, discovered issues, or architectural decisions]

## ✅ Before Marking Complete
- [ ] All README.md files restructured and <200 lines
- [ ] Technical content successfully migrated to appropriate rustdoc locations
- [ ] Cross-references updated and validated
- [ ] Consistent structure applied across all README.md files  
- [ ] **UPDATE: Change `status: TODO` to `status: COMPLETE` in YAML frontmatter above**
- [ ] Verify with: `../../../scrum/task-manager.sh sprint-017`