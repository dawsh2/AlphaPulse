# Primary Agent Workflow Template - Enhanced AlphaPulse Quality Assurance

## Overview
This template defines the mandatory workflow for all AlphaPulse development tasks, integrating comprehensive quality gates, real-time progress tracking, and systematic validation to ensure every change meets the system's demanding standards.

## Core Workflow Process

### 1. Task Initiation (MANDATORY)
Every development task must begin with structured planning:

```json
{
  "task_creation": {
    "required_fields": [
      "clear_task_description",
      "quality_gates_identification", 
      "dependency_mapping",
      "success_criteria"
    ],
    "mandatory_gates": [
      "code_review",
      "compilation_check", 
      "alphapulse_compliance"
    ]
  }
}
```

**Action**: Use TodoWrite tool to create task with quality gates structure.

### 2. Pre-Development Phase (REQUIRED)
Before writing any code:

1. **Scope Definition**: Define exact changes needed
2. **Architecture Review**: Ensure changes align with Protocol V2
3. **Impact Assessment**: Identify affected systems and components  
4. **Quality Gate Setup**: Initialize all required validation gates

**Action**: Update todo status to "in_progress" with progress tracking.

### 3. Development Phase (CODE COMPLETE GATE)
During implementation:

1. **Real-time Progress**: Update completion percentage as work progresses
2. **Checkpoint Tracking**: Record major milestones
3. **Continuous Validation**: Run quick checks during development
4. **Protocol Compliance**: Ensure TLV format, magic numbers, precision preservation

**Quality Check**: Mark code_complete = true only when implementation is finished.

### 4. Code Review Phase (MANDATORY - CRITICAL GATE)
**TRIGGER**: Automatically initiated when code is marked complete.

```rust
// Pseudocode for mandatory code review
fn complete_task_implementation() -> Result<()> {
    // Mark code as complete
    update_progress(task_id, code_complete: true)?;
    
    // MANDATORY: Trigger code review
    let review_agent = Task::launch("code-review-subagent", CodeReviewTask {
        changed_files: get_changed_files()?,
        alphapulse_patterns: load_alphapulse_patterns(),
        protocol_v2_validator: ProtocolV2Validator::new()?,
        mock_detector: MockFallbackDetector::new()?,
    })?;
    
    let review_results = review_agent.execute().await?;
    
    // MANDATORY: Present findings to user
    present_review_findings(review_results)?;
    
    // MANDATORY: Wait for user response (fix/override)
    handle_user_response(review_results).await?;
}
```

**Process**:
1. Launch code-review-subagent automatically
2. Analyze all changed files with AlphaPulse-specific patterns
3. Generate executive summary with deep dive details
4. Present ALL findings to user (critical, warnings, info)
5. Await user decision: Fix issues OR Override with documentation

**User Options**:
- **Fix Issues**: Apply suggested fixes, re-run review until clean
- **Override**: Provide justification, document in override tracking
- **Hybrid**: Fix critical issues, override non-critical with justification

### 5. Compilation Verification (MANDATORY - CRITICAL GATE) 
**TRIGGER**: After code review passes or is overridden.

```rust
fn verify_compilation() -> Result<()> {
    // Launch Rusty subagent for compilation check
    let rusty_agent = Task::launch("rusty", CompilationTask {
        check_build: true,
        run_tests: true,
        check_linting: true,
        validate_types: true,
    })?;
    
    let compile_results = rusty_agent.execute().await?;
    
    if !compile_results.success {
        // MANDATORY: Fix compilation issues before proceeding
        return Err(CompilationError::MustFix(compile_results.errors));
    }
    
    // Mark compilation gate as passed
    mark_quality_gate_passed("compilation")?;
    Ok(())
}
```

**Requirements**:
- Clean build (no errors)
- All tests pass
- Linting passes
- Type checking passes

**Failure Handling**: Compilation failures block task completion. Must fix before proceeding.

### 6. AlphaPulse Compliance Check (MANDATORY - SYSTEM GATE)
**TRIGGER**: After compilation verification passes.

**Validation Categories**:
1. **Protocol V2 Compliance**
   - Magic number validation (0xDEADBEEF)
   - TLV format adherence
   - Precision preservation (no floating point for prices)
   - Type registry compliance

2. **Architecture Compliance**
   - Codec usage (no manual TLV parsing)
   - Service boundary respect
   - No circular dependencies

3. **Policy Compliance**
   - No mock usage (zero tolerance)
   - No fallback patterns
   - Fail-fast philosophy adherence

4. **Security Compliance**
   - No hardcoded credentials
   - Safe patterns only
   - Input validation present

**Process**:
```rust
fn validate_alphapulse_compliance() -> Result<()> {
    let validator = ProtocolV2Validator::new()?;
    let mock_detector = MockFallbackDetector::new()?;
    
    let findings = validator.validate_files(&changed_files)?;
    let mock_findings = mock_detector.scan_files(&changed_files)?;
    
    let critical_violations = findings.iter()
        .chain(mock_findings.iter())
        .filter(|f| f.severity == Critical)
        .collect();
    
    if !critical_violations.is_empty() {
        return Err(ComplianceError::CriticalViolations(critical_violations));
    }
    
    mark_quality_gate_passed("alphapulse_compliance")?;
    Ok(())
}
```

### 7. User Interaction Points (MANDATORY)
**Executive Summary Presentation**:
```
🔍 **Quality Review Summary**
📊 **Files Analyzed**: 5 files, 1,247 lines
🚨 **Critical Issues**: 2 (must fix)
⚠️  **Warnings**: 4 (should fix or override)
ℹ️  **Suggestions**: 6 (optional improvements)

**Critical Issues Requiring Action**:
1. src/adapter.rs:45 - Hardcoded password (Security)
2. src/parser.rs:123 - Float used for price (Protocol V2)

**User Decision Required**: 
[F] Fix all issues | [O] Override with justification | [D] Deep dive details
```

**Override Documentation**:
When user chooses to override:
```
📝 **Override Documentation**
Issue: Performance warning in hot path
Justification: Acceptable for MVP, optimization planned for Sprint 015
Approved by: User
Timestamp: 2025-08-27T15:30:00Z

Create tracking task for future optimization? [Y/N]
```

### 8. Task Completion (ALL GATES MUST PASS)
**Requirements for Completion**:
- Code implementation complete
- Code review passed or overridden  
- Compilation verified
- AlphaPulse compliance validated
- All quality gates satisfied

**Completion Process**:
```rust
fn complete_task() -> Result<()> {
    // Verify all gates
    validate_all_quality_gates_passed()?;
    
    // Final progress update
    update_progress(completion_percentage: 100)?;
    
    // Mark task complete
    mark_task_completed()?;
    
    // Generate completion report
    generate_completion_report()?;
    
    // Check dependent tasks
    check_dependent_tasks_unblocked()?;
    
    Ok(())
}
```

## Quality Gate Definitions

### Code Review Gate
- **Trigger**: When code_complete = true
- **Process**: Launch code-review-subagent
- **Pass Criteria**: No critical issues OR all issues overridden
- **Failure Action**: Present issues, await user decision

### Compilation Gate  
- **Trigger**: After code review passes
- **Process**: Launch Rusty subagent
- **Pass Criteria**: Clean build + tests pass
- **Failure Action**: Must fix compilation errors

### AlphaPulse Compliance Gate
- **Trigger**: After compilation passes
- **Process**: Run Protocol V2 + Mock detection
- **Pass Criteria**: No critical violations
- **Failure Action**: Must fix critical violations

## Progress Tracking Integration

### Real-time Updates
```rust
// Example progress tracking during development
update_progress(ProgressUpdate {
    task_id: "impl-polygon-v3",
    code_complete: false,
    completion_percentage: 30,
    checkpoint: "WebSocket connection implemented",
    context: {
        "files_changed": ["src/adapter.rs", "src/config.rs"],
        "lines_added": "145",
        "tests_added": "3"
    }
});
```

### Quality Gate Status
```json
{
  "quality_gates": {
    "code_review": {
      "status": "in_progress", 
      "started_at": "2025-08-27T15:15:00Z",
      "findings_count": 0,
      "agent_id": "code-review-001"
    },
    "compilation": {
      "status": "pending",
      "dependencies": ["code_review"]
    },
    "alphapulse_compliance": {
      "status": "pending", 
      "dependencies": ["compilation"]
    }
  }
}
```

## Error Handling and Recovery

### Quality Gate Failures
1. **Code Review Failures**: Present findings, provide fix/override options
2. **Compilation Failures**: Must fix before proceeding
3. **Compliance Failures**: Must fix critical violations

### Recovery Actions
```rust
match quality_gate_result {
    QualityGateResult::Failed { findings, suggestions } => {
        present_findings_to_user(findings)?;
        let user_action = await_user_decision()?;
        
        match user_action {
            UserAction::Fix => {
                // Apply fixes and re-run gate
                apply_suggested_fixes(suggestions)?;
                retry_quality_gate()?;
            },
            UserAction::Override { reason } => {
                // Document override and mark gate as passed
                document_override(findings, reason)?;
                mark_gate_overridden()?;
            }
        }
    },
    QualityGateResult::Passed => {
        mark_gate_passed()?;
        proceed_to_next_gate()?;
    }
}
```

## Integration with Existing Systems

### TodoWrite Integration
- Enhanced structure with quality gates
- Real-time progress tracking
- Override documentation
- Completion validation

### Agent Communication
```rust
// Primary agent delegates to specialized subagents
let code_review_agent = launch_subagent("code-review-subagent", task_context)?;
let rusty_agent = launch_subagent("rusty", compilation_context)?;

// Coordinate results
let review_results = code_review_agent.await?;
if review_results.requires_user_input {
    let user_decision = handle_user_interaction(review_results)?;
    apply_user_decision(user_decision)?;
}
```

### File System Integration
```bash
# Quality gate artifacts
.claude/
├── tasks/
│   ├── task-{id}.json          # Task progress and quality gates
│   ├── overrides/              # Override documentation
│   └── reports/               # Quality gate reports
├── tools/
│   ├── mock_fallback_detector.rs  # Mock/fallback detection
│   ├── protocol_v2_validator.rs   # Protocol V2 compliance
│   └── progress_tracker.rs        # Progress tracking
└── templates/
    ├── enhanced_todowrite.md       # Quality gate structure  
    ├── code_review_patterns.md     # Review patterns
    └── primary_agent_workflow.md   # This template
```

## Performance Monitoring

### Latency Tracking
```rust
// Track quality gate performance
let start = Instant::now();
let review_results = code_review_gate.execute().await?;
let duration = start.elapsed();

if duration > Duration::from_secs(30) {
    warn!("Code review gate took {}s (>30s threshold)", duration.as_secs());
}
```

### Success Metrics
- Quality gate pass rates
- Override frequency by category
- Time to completion by gate
- User satisfaction with findings

## Compliance Requirements

### MANDATORY Elements
1. **TodoWrite with Quality Gates**: Every task must use enhanced TodoWrite
2. **Code Review Gate**: Automatic after code completion
3. **User Interaction**: Present ALL findings, require explicit decisions
4. **Compilation Verification**: Must pass before completion
5. **AlphaPulse Compliance**: Zero tolerance for critical violations
6. **Progress Tracking**: Real-time updates throughout workflow

### Override Policy
- **Critical Issues**: Can be overridden but must be documented
- **Documentation Required**: Reason, approval, future tracking
- **Tracking**: All overrides logged for audit and future improvement

## Template Usage

### For Primary Agent Implementation
1. Import this workflow template at task start
2. Initialize progress tracker with quality gates  
3. Execute each phase in sequence
4. Handle user interactions as specified
5. Ensure all gates pass before completion

### For User Experience
1. Clear progress visibility throughout task
2. Executive summaries for decision making
3. Deep dive details available on request
4. Override documentation options
5. Completion confidence through systematic validation

This enhanced workflow ensures systematic quality while maintaining user control and providing comprehensive audit trails for AlphaPulse's demanding development standards.