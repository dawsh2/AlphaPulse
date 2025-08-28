---
task_id: RELAY-ARCH-001
status: COMPLETE
priority: HIGH
estimated_hours: 3
assigned_branch: feat/relay-logic-trait
assignee: TBD
created: 2025-08-26
completed: 2025-08-27
depends_on: []  # Foundation task - no dependencies
blocks:
  - TASK-002  # Generic relay engine needs this trait
  - TASK-003  # Domain implementations need this trait
scope:
  - "relays/src/common/relay_logic.rs"  # New trait definition
  - "relays/src/common/mod.rs"  # Export new trait
---

# ARCH-001: Create RelayLogic Trait Foundation

**Sprint**: 007 - Generic Relay Engine Refactor  
**Priority**: HIGH - ENABLES ALL OTHER WORK  
**Estimate**: 3 hours  
**Dependencies**: None  

## Objective
Create the foundational `RelayLogic` trait that defines the contract for domain-specific relay behavior, enabling the generic Relay<T> pattern.

## Technical Requirements

### Core Trait Definition
```rust
use torq_protocol_v2::{RelayDomain, MessageHeader}; // Updated import path

/// Defines domain-specific logic for relay implementations
/// This trait represents the "20%" unique behavior per relay type
pub trait RelayLogic: Send + Sync + 'static {
    /// Returns the domain this relay is responsible for
    fn domain(&self) -> RelayDomain;
    
    /// Returns the Unix socket path for this relay  
    fn socket_path(&self) -> &'static str;
    
    /// Determines if a message should be forwarded
    /// Default implementation filters by domain
    fn should_forward(&self, header: &MessageHeader) -> bool {
        header.relay_domain == self.domain()
    }
    
    /// Optional: Custom initialization logic
    fn initialize(&self) -> Result<(), RelayError> {
        Ok(())
    }
    
    /// Optional: Custom cleanup logic  
    fn cleanup(&self) -> Result<(), RelayError> {
        Ok(())
    }
}
```

### Files to Create
- `relays/src/common/mod.rs` - Main trait definition
- `relays/src/common/error.rs` - Shared error types  
- Update `relays/src/lib.rs` - Module declarations

### Implementation Checklist
- [ ] Define `RelayLogic` trait with required methods
- [ ] Ensure trait bounds: `Send + Sync + 'static`
- [ ] Add default implementation for `should_forward()`
- [ ] Use correct Protocol V2 imports (`torq_protocol_v2`)
- [ ] Create `RelayError` enum for error handling
- [ ] Add comprehensive trait documentation
- [ ] Include usage examples in doc comments
- [ ] Add trait object safety considerations

### Error Types
```rust
#[derive(Debug, thiserror::Error)]
pub enum RelayError {
    #[error("Socket binding failed: {0}")]
    SocketBind(#[from] std::io::Error),
    
    #[error("Protocol parsing error: {0}")]
    Protocol(String),
    
    #[error("Client connection error: {0}")]
    ClientConnection(String),
}
```

### Documentation Requirements
- [ ] Trait purpose and design rationale
- [ ] Usage examples for each method
- [ ] Domain separation explanation (1-19, 20-39, 40-79)
- [ ] Performance implications
- [ ] Thread safety guarantees

### Validation Commands
```bash
# Check compilation
cargo check --package relays

# Run trait-specific tests  
cargo test --package relays --test trait_foundation

# Verify documentation
cargo doc --package relays --open
```

### Acceptance Criteria
1. ✅ Trait compiles without errors
2. ✅ All trait bounds properly specified  
3. ✅ Default implementations work correctly
4. ✅ Documentation covers all methods
5. ✅ Error types properly defined
6. ✅ Module structure follows conventions
7. ✅ Compatible with Protocol V2 message types

### Next Steps
This task enables ARCH-002 (Generic Relay Engine) which will consume this trait to build the common relay infrastructure.

### Domain Validation
Ensure trait design supports all three domains:
- **MarketData**: TLV types 1-19, market data socket path
- **Signal**: TLV types 20-39, signal relay socket path  
- **Execution**: TLV types 40-79, execution relay socket path

### Performance Considerations
- Trait methods should be zero-cost abstractions
- `should_forward()` called on every message - keep efficient
- Domain check should be O(1) comparison
- No heap allocations in hot path methods