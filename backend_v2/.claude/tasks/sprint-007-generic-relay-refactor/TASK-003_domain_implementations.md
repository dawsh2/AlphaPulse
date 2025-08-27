---
task_id: TASK-003
status: COMPLETE
priority: HIGH
assigned_branch: feat/domain-implementations
created: 2025-08-26
estimated_hours: 4
depends_on:
  - TASK-001  # Need RelayLogic trait first
  - TASK-002  # Need generic engine implementation
blocks:
  - TASK-004  # Binary entry points depend on domain implementations
completed: 2025-08-26
scope:
  - "relays/src/market_data.rs"  # Market data relay implementation
  - "relays/src/signal.rs"  # Signal relay implementation
  - "relays/src/execution.rs"  # Execution relay implementation
---

# TASK-003: Create Domain-Specific RelayLogic Implementations

**Branch**: `feat/domain-implementations`  
**NEVER WORK ON MAIN**

## Git Enforcement
```bash
# MANDATORY: Verify you're not on main before starting
if [ "$(git branch --show-current)" = "main" ]; then
    echo "❌ NEVER WORK ON MAIN BRANCH!"
    echo "Run: git worktree add -b feat/domain-implementations"
    exit 1
fi

# Create feature branch from generic-relay-engine
git checkout feat/generic-relay-engine  # Start from TASK-002 branch
git worktree add -b feat/domain-implementations
git branch --show-current  # Should show: feat/domain-implementations
```

## Problem Statement
With the RelayLogic trait defined (TASK-001) and generic Relay<T> engine implemented (TASK-002), we now need to create the three domain-specific implementations that capture the **20% unique behavior** of each relay type.

**Critical Requirement**: Each implementation must preserve the **exact current behavior** of the existing relay binaries while working through the generic engine.

## Acceptance Criteria
- [ ] **MarketDataLogic** - Implements bidirectional forwarding with broadcast pattern
- [ ] **SignalLogic** - Implements consumer-focused pattern with message tracking  
- [ ] **ExecutionLogic** - Implements consumer-focused pattern with execution-specific validation
- [ ] **Domain separation** - Correct TLV type ranges and socket paths maintained
- [ ] **Protocol V2 compatibility** - All implementations work with 32-byte headers + TLV payloads
- [ ] **Performance preserved** - No additional overhead from domain-specific logic
- [ ] **Behavioral equivalence** - Each implementation matches current relay behavior exactly

## Technical Approach

### Domain Analysis Summary
From analyzing the existing implementations:

| Domain | Socket Path | Pattern | Unique Behavior |
|--------|-------------|---------|-----------------|
| MarketData | `/tmp/alphapulse/market_data.sock` | Bidirectional forwarding | Complex broadcast with read/write tasks |
| Signals | `/tmp/alphapulse/signals.sock` | Consumer tracking | Message counting, consumer registry |
| Execution | `/tmp/alphapulse/execution.sock` | Consumer tracking | Execution-specific validation |

### Implementation Pattern
Each domain implements the RelayLogic trait with specific customizations:

```rust
// Example structure for each domain
impl RelayLogic for DomainLogic {
    fn domain(&self) -> &'static str { /* Domain name */ }
    fn socket_path(&self) -> &'static str { /* Socket path */ }  
    fn should_forward(&self, message: &[u8]) -> bool { /* Filtering logic */ }
    fn on_connection_established(&self, connection_id: u64) { /* Custom logging */ }
    fn process_message(&self, message: &[u8]) -> Vec<u8> { /* Optional processing */ }
}
```

## Implementation Steps

### Step 1: MarketDataLogic Implementation (1.5 hours)

**File**: `relays/src/domains/market_data.rs`

**Key Requirements**:
- Preserve **bidirectional forwarding pattern** (most complex)
- Forward ALL messages (no filtering) 
- Use broadcast channel pattern for multiple consumers
- Match exact logging format from existing implementation

```rust
pub struct MarketDataLogic;

impl RelayLogic for MarketDataLogic {
    fn domain(&self) -> &'static str { 
        "MarketData" 
    }
    
    fn socket_path(&self) -> &'static str { 
        "/tmp/alphapulse/market_data.sock" 
    }
    
    fn should_forward(&self, _message: &[u8]) -> bool {
        // Market data: forward everything (current behavior)
        true  
    }
    
    fn on_connection_established(&self, connection_id: u64) {
        info!("📡 Connection {} established", connection_id);
        // Match exact logging from current market_data_relay
    }
    
    fn process_message(&self, message: &[u8]) -> Vec<u8> {
        // No processing needed - direct forwarding
        message.to_vec()
    }
}
```

**Validation Steps**:
- Compare logs with existing market_data_relay output
- Verify bidirectional behavior with polygon_publisher + dashboard
- Check broadcast channel subscriber count logging

### Step 2: SignalLogic Implementation (1 hour)

**File**: `relays/src/domains/signal.rs`

**Key Requirements**:
- Consumer tracking pattern (like current signal_relay)
- Message counting and periodic logging (every 1000 messages)
- TLV type validation for Signal domain (types 20-39)
- Consumer registry cleanup on disconnect

```rust
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct SignalLogic {
    consumers: Arc<RwLock<HashMap<u64, std::time::Instant>>>,
}

impl SignalLogic {
    pub fn new() -> Self {
        Self {
            consumers: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl RelayLogic for SignalLogic {
    fn domain(&self) -> &'static str { 
        "Signals" 
    }
    
    fn socket_path(&self) -> &'static str { 
        "/tmp/alphapulse/signals.sock" 
    }
    
    fn should_forward(&self, message: &[u8]) -> bool {
        // Validate TLV type is in Signal domain range (20-39)
        if message.len() < 32 { return false; }
        
        // Parse TLV type from Protocol V2 message structure
        // (Implementation depends on Protocol V2 header format)
        true  // For now, forward all valid messages
    }
    
    fn on_connection_established(&self, connection_id: u64) {
        info!("📡 Signal consumer {} connected", connection_id);
        
        // Track consumer in registry
        tokio::spawn({
            let consumers = self.consumers.clone();
            async move {
                consumers.write().await.insert(connection_id, std::time::Instant::now());
            }
        });
    }
}
```

### Step 3: ExecutionLogic Implementation (1 hour)

**File**: `relays/src/domains/execution.rs`

**Key Requirements**:
- Consumer tracking pattern (similar to SignalLogic)
- Execution-specific validation for TLV types 40-79
- Enhanced security/validation for execution messages
- Match logging format from current execution_relay

```rust
pub struct ExecutionLogic {
    consumers: Arc<RwLock<HashMap<u64, std::time::Instant>>>,
}

impl ExecutionLogic {
    pub fn new() -> Self {
        Self {
            consumers: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl RelayLogic for ExecutionLogic {
    fn domain(&self) -> &'static str { 
        "Execution" 
    }
    
    fn socket_path(&self) -> &'static str { 
        "/tmp/alphapulse/execution.sock" 
    }
    
    fn should_forward(&self, message: &[u8]) -> bool {
        // Validate TLV type is in Execution domain range (40-79)
        if message.len() < 32 { return false; }
        
        // Enhanced validation for execution messages
        // (More strict than signal validation)
        true  // For now, forward all valid messages
    }
    
    fn on_connection_established(&self, connection_id: u64) {
        info!("📡 Execution consumer {} connected", connection_id);
        
        // Track consumer with execution-specific logging
        tokio::spawn({
            let consumers = self.consumers.clone();
            async move {
                consumers.write().await.insert(connection_id, std::time::Instant::now());
            }
        });
    }
}
```

### Step 4: Module Integration (0.5 hours)

**File**: `relays/src/domains/mod.rs`

```rust
//! Domain-specific RelayLogic implementations
//! 
//! Each domain captures the 20% unique behavior while the generic Relay<T>
//! handles the 80% common functionality.

pub mod market_data;
pub mod signal; 
pub mod execution;

pub use market_data::MarketDataLogic;
pub use signal::SignalLogic;
pub use execution::ExecutionLogic;
```

Update `relays/src/lib.rs`:
```rust
pub mod domains;
pub use domains::{MarketDataLogic, SignalLogic, ExecutionLogic};
```

## Files to Create/Modify

### CREATE
- `relays/src/domains/mod.rs` - Module exports
- `relays/src/domains/market_data.rs` - MarketDataLogic implementation
- `relays/src/domains/signal.rs` - SignalLogic implementation  
- `relays/src/domains/execution.rs` - ExecutionLogic implementation

### MODIFY
- `relays/src/lib.rs` - Export domain implementations

## Protocol V2 Integration Requirements

### TLV Type Validation
Each domain must respect TLV type ranges:
- **MarketData**: Types 1-19 (no filtering, accept all)
- **Signals**: Types 20-39 (validate range)  
- **Execution**: Types 40-79 (validate range + security checks)

### Message Structure
All implementations must handle Protocol V2 format:
```
[32-byte MessageHeader] [Variable TLV Payload]
```

### Header Parsing
```rust
// Common pattern for TLV type extraction
fn extract_tlv_type(message: &[u8]) -> Option<u16> {
    if message.len() < 32 { return None; }
    
    // Parse TLV type from message header
    // (Implementation depends on exact Protocol V2 format)
    Some(0) // Placeholder
}
```

## Testing Strategy

### Unit Tests per Domain
```bash
# Test each domain implementation
cargo test domains::market_data::tests
cargo test domains::signal::tests  
cargo test domains::execution::tests
```

### Integration Testing
```bash
# Test with generic Relay<T> engine
cargo test --bin test_market_data_relay_generic
cargo test --bin test_signal_relay_generic
cargo test --bin test_execution_relay_generic
```

### Behavioral Equivalence Testing
Compare output with existing relays:
```bash
# Run side-by-side comparison
./test_relay_equivalence.sh market_data
./test_relay_equivalence.sh signal  
./test_relay_equivalence.sh execution
```

## Success Metrics
- [ ] All three domain implementations compile without errors
- [ ] Each domain preserves exact behavior of current relay
- [ ] TLV type filtering works correctly for Signal/Execution domains
- [ ] Consumer tracking matches existing registry behavior
- [ ] Logging output identical to current implementations
- [ ] Integration tests pass with generic Relay<T> engine

## Risk Mitigation

### Behavior Divergence Risk
**Mitigation**: Side-by-side testing with current implementations
- Run both old and new relays simultaneously
- Compare message handling and logging output  
- Validate with real polygon_publisher and dashboard connections

### Protocol Compatibility Risk  
**Mitigation**: Comprehensive Protocol V2 testing
- Test with actual TLV messages from polygon_publisher
- Validate header parsing and TLV type extraction
- Ensure domain filtering doesn't break message flow

## Next Task Dependencies
This task **BLOCKS**:
- TASK-004 (Binary Updates) - needs domain implementations for new main.rs files

This task **DEPENDS ON**:
- TASK-001 (RelayLogic Trait) - needs trait interface
- TASK-002 (Generic Engine) - needs Relay<T> implementation

## Documentation Updates Required
- **Domain behavior documentation** explaining 20% customization per domain
- **TLV type filtering** documentation with examples
- **Consumer tracking patterns** comparison between Signal and Execution domains

---
**Estimated Completion**: 4 hours  
**Complexity**: Medium - implementing well-defined interfaces  
**Risk Level**: Medium - behavior compatibility critical