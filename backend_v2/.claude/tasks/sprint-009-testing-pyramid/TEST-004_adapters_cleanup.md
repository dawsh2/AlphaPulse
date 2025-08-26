---
task_id: TEST-004
status: TODO
priority: HIGH
estimated_hours: 6
assigned_branch: refactor/adapters-cleanup
assignee: TBD
created: 2025-08-26
completed: null
depends_on:
  - TEST-001  # Need unit test framework for adapter testing
blocks: []
scope:
  - "services_v2/adapters/src/"  # Cleanup adapter implementations
  - "tests/unit/adapters/"  # Add comprehensive adapter unit tests
  - "services_v2/adapters/tests/"  # Standardize adapter integration tests
---

# TEST-004: Adapters Module Cleanup & Standardization

## 🔴 CRITICAL INSTRUCTIONS
```bash
# BEFORE STARTING - VERIFY YOU'RE NOT ON MAIN:
git branch --show-current

# If you see "main", IMMEDIATELY run:
git checkout -b refactor/adapters-cleanup

# NEVER commit directly to main!
```

## Status
**Status**: TODO
**Priority**: HIGH
**Branch**: `refactor/adapters-cleanup`
**Estimated**: 6 hours

## Problem Statement
The `services_v2/adapters/` module needs better organization and standardization. Currently adapters are inconsistently structured, making maintenance and scaling difficult.

## ⚠️ CRITICAL REQUIREMENT: COPY, DON'T REWRITE
**🚨 ABSOLUTELY NO REWRITING OF EXISTING CODE! 🚨**

The existing adapter code has been tested and is robust. We are **ONLY** reorganizing files and standardizing interfaces. All business logic must be preserved exactly as-is.

## Acceptance Criteria
- [ ] Clean adapter directory structure following plugin architecture
- [ ] Standardized `Adapter` trait implemented by all adapters
- [ ] All existing functionality preserved (no behavior changes)
- [ ] Shared utilities extracted to `common/` module
- [ ] Each adapter is self-contained with consistent internal structure
- [ ] All tests still pass after reorganization
- [ ] Integration tests verify identical behavior before/after

## Technical Approach - Copy & Reorganize Only

### Target Directory Structure
```
services_v2/adapters/
├── mod.rs              # Defines common Adapter trait + declares modules
├── common/             # Shared, reusable components
│   ├── mod.rs
│   ├── error.rs        # Unified AdapterError enum
│   ├── client.rs       # Shared HTTP/WebSocket client utilities
│   └── circuit_breaker.rs # Shared circuit breaker logic
│
├── polygon_pos/        # Self-contained adapter for Polygon PoS
│   ├── mod.rs          # Implements Adapter trait, orchestrates components
│   ├── client.rs       # Network communication (COPY existing code)
│   ├── parser.rs       # JSON parsing logic (COPY existing code)
│   └── types.rs        # Polygon-specific structs (COPY existing code)
│
├── uniswap_v3/         # Self-contained adapter for Uniswap V3
│   ├── mod.rs          # Implements Adapter trait
│   ├── client.rs       # GraphQL client (COPY existing code)
│   ├── parser.rs       # Response parsing (COPY existing code)
│   └── types.rs        # Uniswap V3 structs (COPY existing code)
│
└── kraken/             # Traditional exchange adapter
    ├── mod.rs          # Implements Adapter trait
    ├── client.rs       # WebSocket client (COPY existing code)
    ├── parser.rs       # Kraken message parsing (COPY existing code)
    └── types.rs        # Kraken API structs (COPY existing code)
```

### Files to Create/Modify

#### Step 1: Create Standardized Adapter Trait
```rust
// services_v2/adapters/mod.rs - NEW FILE
pub mod common;
pub mod polygon_pos;
pub mod uniswap_v3; 
pub mod kraken;

use anyhow::Result;
use async_trait::async_trait;

/// Standard interface for all data adapters
/// Each adapter is a self-contained "plugin" conforming to this contract
#[async_trait]
pub trait Adapter {
    /// Unique adapter identifier (e.g., "polygon_pos")
    fn name(&self) -> &'static str;
    
    /// Starts adapter's event loop - fetches data and sends to relays
    async fn start(&self) -> Result<()>;
    
    /// Health check for adapter's connection
    async fn health_check(&self) -> Result<()>;
    
    /// Graceful shutdown
    async fn stop(&self) -> Result<()>;
}
```

#### Step 2: Extract Common Utilities (COPY existing code)
```rust
// services_v2/adapters/common/error.rs - COPY from existing error handling
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AdapterError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    
    #[error("Parse error: {0}")]
    ParseError(String),
    
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    
    // COPY all existing error variants from current adapters
}
```

#### Step 3: Reorganize Polygon Adapter (COPY existing code)
```rust
// services_v2/adapters/polygon_pos/mod.rs - NEW orchestration file
use super::Adapter;
use crate::adapters::common::AdapterError;

pub struct PolygonPosAdapter {
    client: client::PolygonClient,
    parser: parser::PolygonParser,
}

#[async_trait::async_trait]
impl Adapter for PolygonPosAdapter {
    fn name(&self) -> &'static str { "polygon_pos" }
    
    async fn start(&self) -> Result<()> {
        // COPY existing startup logic exactly
    }
    
    async fn health_check(&self) -> Result<()> {
        // COPY existing health check logic
    }
    
    async fn stop(&self) -> Result<()> {
        // COPY existing shutdown logic  
    }
}
```

```rust
// services_v2/adapters/polygon_pos/client.rs - COPY existing client code
// Move existing Polygon client code here EXACTLY as-is
// No changes to business logic, only file organization

// services_v2/adapters/polygon_pos/parser.rs - COPY existing parser code  
// Move existing Polygon parsing logic here EXACTLY as-is

// services_v2/adapters/polygon_pos/types.rs - COPY existing types
// Move existing Polygon struct definitions here EXACTLY as-is
```

### Implementation Steps

#### Phase 1: Setup (1 hour)
1. **Create new directory structure** (empty files first)
2. **Copy existing tests** to validate behavior before changes
3. **Run baseline tests** to establish current behavior

#### Phase 2: Extract Common Code (2 hours)  
1. **Identify shared code** across existing adapters
2. **COPY shared utilities** to `common/` module
3. **Create unified error types** by copying existing error handling
4. **Extract shared client utilities** (rate limiting, circuit breakers)

#### Phase 3: Reorganize Each Adapter (3 hours)
1. **For each existing adapter**:
   - Create new directory structure
   - **COPY** existing client code to `client.rs`
   - **COPY** existing parsing logic to `parser.rs` 
   - **COPY** existing type definitions to `types.rs`
   - Create new `mod.rs` that implements `Adapter` trait
   - **Preserve all existing functionality exactly**

2. **Update imports** to new file locations
3. **Run tests after each adapter** to verify no behavior changes

### Testing Strategy - Behavior Preservation

#### Before/After Integration Tests
```rust
// tests/adapter_behavior_preservation.rs - NEW FILE
#[tokio::test]  
async fn test_polygon_adapter_behavior_unchanged() {
    // Test the OLD adapter
    let old_adapter = create_old_polygon_adapter();
    let old_results = old_adapter.fetch_recent_trades().await;
    
    // Test the NEW reorganized adapter
    let new_adapter = PolygonPosAdapter::new();
    let new_results = new_adapter.fetch_recent_trades().await;
    
    // Results must be IDENTICAL
    assert_eq!(old_results, new_results);
}

#[tokio::test]
async fn test_all_adapters_implement_trait() {
    let adapters: Vec<Box<dyn Adapter>> = vec![
        Box::new(PolygonPosAdapter::new()),
        Box::new(UniswapV3Adapter::new()),
        Box::new(KrakenAdapter::new()),
    ];
    
    for adapter in adapters {
        assert!(adapter.health_check().await.is_ok());
        assert!(!adapter.name().is_empty());
    }
}
```

## Testing Instructions
```bash
# CRITICAL: Run tests before any changes
cargo test --package services_v2 --lib adapters

# After each phase, verify behavior unchanged
cargo test --package services_v2 --lib adapters -- --nocapture

# Integration test to verify identical behavior
cargo test test_adapter_behavior_preservation

# Performance regression check
cargo bench --baseline before_cleanup adapters
```

## Git Workflow
```bash
# 1. Start on your branch
git checkout -b refactor/adapters-cleanup

# 2. Commit after each phase for easy rollback
git add services_v2/adapters/
git commit -m "refactor: Phase 1 - Create adapter directory structure"

git commit -m "refactor: Phase 2 - Extract common utilities (copied existing code)"  

git commit -m "refactor: Phase 3 - Reorganize polygon adapter (copied existing code)"

# 3. Final verification commit
git commit -m "refactor: Complete adapters cleanup - all behavior preserved"

# 4. Push and create PR
git push origin refactor/adapters-cleanup
gh pr create --title "TEST-004: Adapters module cleanup & standardization" \
             --body "Reorganizes adapter code for better maintainability. All existing functionality preserved."
```

## Completion Checklist  
- [ ] Working on correct branch (not main)
- [ ] All existing adapter code COPIED, not rewritten
- [ ] New directory structure follows plugin architecture
- [ ] Common utilities extracted to shared module
- [ ] Every adapter implements standardized `Adapter` trait
- [ ] All original tests still pass
- [ ] Integration tests verify identical behavior
- [ ] No performance regressions
- [ ] PR created with detailed behavior preservation verification
- [ ] **🚨 CRITICAL: Updated task status to COMPLETE** ← AGENTS MUST DO THIS!

## ⚠️ IMPORTANT: Status Updates Required
**When you finish this task, you MUST:**
1. Change `status: TODO` to `status: COMPLETE` in the YAML frontmatter above
2. This is NOT optional - the task-manager.sh depends on accurate status
3. If you forget, the task will show as incomplete forever
4. Update immediately after PR is merged, not before

## Benefits of This Cleanup

### Scalability
- Adding new data source = create new directory + implement `Adapter` trait
- No other code needs to change

### Maintainability  
- API format changes isolated to single adapter's `types.rs` and `parser.rs`
- Problems are contained and easy to locate

### Testability
- Can test parsing logic separately from network logic
- Mock data testing becomes trivial
- Each component has clear responsibilities

### Decoupling
- Core business logic only knows about clean `Adapter` trait
- External API details completely hidden

## Notes
**🚨 REMEMBER: This is ONLY a reorganization task. The goal is better structure, not better code. All existing functionality must work identically after the cleanup.**