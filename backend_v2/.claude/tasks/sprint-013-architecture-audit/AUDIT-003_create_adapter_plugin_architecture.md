---
task_id: AUDIT-003
status: COMPLETE
priority: HIGH
estimated_hours: 6
assigned_branch: feat/adapter-plugin-architecture
assignee: TBD
created: 2025-08-26
completed: 2025-08-26
depends_on:
  - AUDIT-002  # Need service codec dependencies working first
blocks:
  - AUDIT-004  # Plugin migration depends on architecture
scope:
  - "services_v2/adapters/common/"  # New common adapter logic
  - "services_v2/adapters/src/lib.rs"  # Export Adapter trait
  - "services_v2/adapters/Cargo.toml"  # Update dependencies
---

# AUDIT-003: Create Adapter Plugin Architecture

## Git Worktree Setup (REQUIRED)
```bash
# Create worktree for this task
git worktree add -b feat/adapter-plugin-architecture ../audit-003-worktree
cd ../audit-003-worktree
```

## Status
**Status**: ✅ COMPLETE
**Priority**: HIGH
**Worktree**: `../audit-003-worktree` (Branch: `feat/adapter-plugin-architecture`)
**Estimated**: 6 hours

## Problem Statement
Current adapter architecture is monolithic with duplicated code across exchange adapters. Need to create a plugin architecture with shared common logic and a consistent Adapter trait interface.

## Acceptance Criteria
- [ ] Create `common/` directory with shared adapter logic
- [ ] Define `Adapter` trait for common interface
- [ ] Move shared auth logic to common module
- [ ] Move shared metrics logic to common module
- [ ] Prepare directory structure for individual adapter plugins
- [ ] All existing adapters still compile and work
- [ ] Clear migration path for adapters to plugin model

## Target Directory Structure
```
services_v2/adapters/
├── common/
│   ├── mod.rs         # Adapter trait definition
│   ├── auth.rs        # Shared auth logic
│   ├── metrics.rs     # Common metrics
│   ├── circuit_breaker.rs  # Shared circuit breaker
│   └── rate_limit.rs  # Shared rate limiting
├── src/               # Current monolithic structure (temporary)
├── polygon_adapter/   # Future: Individual adapter plugins
├── uniswap_v3_adapter/
├── binance_adapter/
└── Cargo.toml
```

## Implementation Steps
1. **Create Common Module Structure**
   - Create `common/` directory
   - Set up module hierarchy with proper exports

2. **Define Adapter Trait**
   - Create common interface for all adapters
   - Include connection, authentication, and data processing methods
   - Support async operations and error handling

3. **Extract Shared Logic**
   - Move auth logic from individual adapters to `common/auth.rs`
   - Move metrics collection to `common/metrics.rs`
   - Move circuit breaker logic to `common/circuit_breaker.rs`
   - Move rate limiting to `common/rate_limit.rs`

4. **Update Exports**
   - Export Adapter trait from lib.rs
   - Make common modules available to adapters
   - Ensure backward compatibility during transition

## Adapter Trait Design
```rust
#[async_trait]
pub trait Adapter: Send + Sync + Debug {
    type Config: Clone + Send + Sync;
    type Message: Send + Sync;
    
    /// Connect to the exchange/data source
    async fn connect(&mut self, config: Self::Config) -> Result<(), AdapterError>;
    
    /// Authenticate with the service (if required)
    async fn authenticate(&mut self) -> Result<(), AdapterError>;
    
    /// Start receiving data
    async fn start_streaming(&mut self) -> Result<(), AdapterError>;
    
    /// Process incoming message
    async fn process_message(&self, raw: Vec<u8>) -> Result<Self::Message, AdapterError>;
    
    /// Get adapter name for logging/metrics
    fn name(&self) -> &'static str;
    
    /// Get health status
    fn is_healthy(&self) -> bool;
    
    /// Graceful shutdown
    async fn shutdown(&mut self) -> Result<(), AdapterError>;
}
```

## Files to Create/Modify
- `services_v2/adapters/common/mod.rs` - Module exports and trait definition
- `services_v2/adapters/common/auth.rs` - Shared authentication logic
- `services_v2/adapters/common/metrics.rs` - Common metrics collection
- `services_v2/adapters/common/circuit_breaker.rs` - Shared circuit breaker
- `services_v2/adapters/common/rate_limit.rs` - Shared rate limiting
- `services_v2/adapters/src/lib.rs` - Export common modules
- `services_v2/adapters/Cargo.toml` - Update dependencies if needed

## Success Criteria
- Common adapter logic extracted and reusable
- Adapter trait provides consistent interface
- No functionality regression in existing adapters
- Clear path for AUDIT-004 adapter migration
- Code duplication reduced across adapters