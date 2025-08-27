---
task_id: TASK-001
status: COMPLETE
priority: CRITICAL 
assigned_branch: feat/relay-logic-trait
created: 2025-08-26
estimated_hours: 4
completed: 2025-08-26
depends_on:
  - CODEC-002  # Need protocol refactoring complete
blocks:
  - TASK-002  # Generic engine depends on trait design
scope:
  - "relays/src/relay_logic.rs"  # Core trait definition
  - "relays/src/lib.rs"  # Module exports
---

# TASK-001: Design RelayLogic Trait and Core Module Structure

**🚨 CRITICAL**: Update status to COMPLETE when finished!

**Branch**: `feat/relay-logic-trait`  
**NEVER WORK ON MAIN**

## Git Enforcement
```bash
# MANDATORY: Verify you're not on main before starting
if [ "$(git branch --show-current)" = "main" ]; then
    echo "❌ NEVER WORK ON MAIN BRANCH!"
    echo "Run: git worktree add -b feat/relay-logic-trait"
    exit 1
fi

# Create and switch to feature branch
git worktree add -b feat/relay-logic-trait
git branch --show-current  # Should show: feat/relay-logic-trait
```

## Problem Statement
Currently, the AlphaPulse relay system has **massive code duplication** across three domain relays:
- `market_data_relay/src/main.rs` (290 lines)
- `signal_relay/src/main.rs` (103 lines) 
- `execution_relay/src/main.rs` (103 lines)

**Analysis shows ~80% code duplication:**
- **Common (80%)**: Unix socket setup, connection handling, async task spawning, buffer management
- **Unique (20%)**: Socket path, logging messages, domain-specific message validation/routing

**Target Architecture**: Generic `Relay<T: RelayLogic>` engine where T implements the 20% unique behavior.

## Acceptance Criteria
- [ ] **RelayLogic trait defined** with exactly the methods needed for domain customization
- [ ] **Module structure redesigned** in relays/src/ to support generic + trait pattern
- [ ] **Trait interface validates** that socket path, domain validation, and message filtering can be customized
- [ ] **Zero performance overhead** - trait methods are zero-cost abstractions
- [ ] **Backward compatibility** - existing binaries can be updated without protocol changes

## Technical Approach

### 1. RelayLogic Trait Design
Create `relays/src/relay_logic.rs`:

```rust
/// Domain-specific behavior for relay engines (20% unique logic)
pub trait RelayLogic: Send + Sync + 'static {
    /// Relay domain identifier for logging and metrics
    fn domain(&self) -> &'static str;
    
    /// Unix socket path for this relay type
    fn socket_path(&self) -> &'static str;
    
    /// Domain-specific message validation and routing
    /// Returns true if message should be forwarded to other connections
    fn should_forward(&self, message: &[u8]) -> bool;
    
    /// Optional: Custom connection handling logic  
    fn on_connection_established(&self, connection_id: u64) {
        // Default implementation - can be overridden
    }
    
    /// Optional: Custom message processing before forwarding
    fn process_message(&self, message: &[u8]) -> Vec<u8> {
        // Default: no processing, just return original
        message.to_vec()
    }
}
```

### 2. Module Structure Reorganization
```
relays/src/
├── lib.rs              # Public API and re-exports  
├── relay_logic.rs      # RelayLogic trait definition
├── generic_relay.rs    # Relay<T> generic engine (80% common logic)
├── domains/            # Domain-specific implementations  
│   ├── mod.rs
│   ├── market_data.rs  # MarketDataLogic implementation
│   ├── signal.rs       # SignalLogic implementation  
│   └── execution.rs    # ExecutionLogic implementation
└── [existing files]    # Keep existing modules for compatibility
```

### 3. Domain Implementation Pattern
Each domain implements RelayLogic:

```rust
// domains/market_data.rs
pub struct MarketDataLogic;

impl RelayLogic for MarketDataLogic {
    fn domain(&self) -> &'static str { "MarketData" }
    
    fn socket_path(&self) -> &'static str { 
        "/tmp/alphapulse/market_data.sock" 
    }
    
    fn should_forward(&self, message: &[u8]) -> bool {
        // Market data: forward everything (current behavior)
        true
    }
    
    fn on_connection_established(&self, connection_id: u64) {
        info!("📡 MarketData connection {} established", connection_id);
    }
}
```

## Implementation Steps

### Step 1: Create RelayLogic Trait (1 hour)
1. **Create** `relays/src/relay_logic.rs` with trait definition
2. **Define method signatures** based on analysis of existing relay differences
3. **Add documentation** explaining the 80/20 split and usage patterns

### Step 2: Analyze Existing Implementations (1.5 hours)
1. **Extract common patterns** from all 3 relay main.rs files
2. **Identify exact differences** that need trait customization
3. **Document current behavior** to ensure 100% compatibility 

### Step 3: Design Module Structure (1 hour)  
1. **Plan directory layout** for new modular structure
2. **Create module files** with proper pub/private boundaries
3. **Update lib.rs** to expose new public API

### Step 4: Validate Design (0.5 hours)
1. **Review trait interface** - can it handle all current use cases?
2. **Check performance implications** - are all methods zero-cost?
3. **Verify backward compatibility** - can existing binaries migrate easily?

## Files to Create/Modify
- **CREATE**: `relays/src/relay_logic.rs`
- **CREATE**: `relays/src/domains/mod.rs`
- **CREATE**: `relays/src/domains/market_data.rs`
- **CREATE**: `relays/src/domains/signal.rs` 
- **CREATE**: `relays/src/domains/execution.rs`
- **MODIFY**: `relays/src/lib.rs` (add new module exports)

## Testing Strategy
- **Unit tests** for trait methods and domain implementations
- **Integration tests** to ensure trait design supports all existing functionality
- **Performance tests** to validate zero-cost abstraction assumption

## Success Metrics
- All existing relay behaviors can be expressed through RelayLogic trait
- Trait methods compile to identical assembly as direct implementation
- Module structure is logical and supports easy extension for new domains

## Next Task Dependencies
This task **BLOCKS**:
- TASK-002 (Generic Relay Engine) - needs RelayLogic trait interface
- TASK-003 (Domain Implementations) - needs trait definition and module structure

## Documentation Required
- **Architecture diagram** showing new module relationships
- **Migration guide** for updating existing binaries to use new pattern
- **Performance comparison** validating zero-cost abstraction claims

---
**Estimated Completion**: 4 hours  
**Complexity**: High - foundational architecture change  
**Risk Level**: Medium - breaking changes but well-contained