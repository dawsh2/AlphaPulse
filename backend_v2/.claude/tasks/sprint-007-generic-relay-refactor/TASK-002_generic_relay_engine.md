---
status: COMPLETE
priority: CRITICAL
assigned_branch: feat/generic-relay-engine  
created: 2025-08-26
estimated_hours: 6
depends_on: TASK-001
completed: 2025-08-26
---

# TASK-002: Implement Generic Relay<T> Engine

**Branch**: `feat/generic-relay-engine`  
**NEVER WORK ON MAIN**

## Git Enforcement
```bash
# MANDATORY: Verify you're not on main before starting
if [ "$(git branch --show-current)" = "main" ]; then
    echo "❌ NEVER WORK ON MAIN BRANCH!"  
    echo "Run: git checkout -b feat/generic-relay-engine"
    exit 1
fi

# Create feature branch from relay-logic-trait
git checkout feat/relay-logic-trait  # Start from TASK-001 branch
git checkout -b feat/generic-relay-engine
git branch --show-current  # Should show: feat/generic-relay-engine
```

## Problem Statement
After TASK-001 defines the RelayLogic trait interface, we need to extract the **80% common functionality** from the three existing relay implementations into a single `Relay<T: RelayLogic>` generic engine.

**Current State**: 290+ lines duplicated across 3 files  
**Target State**: Single generic engine that handles all common logic (Unix socket setup, connection management, async task coordination, message forwarding)

**Critical Requirement**: Preserve the **bidirectional forwarding pattern** from MarketDataRelay (the most complete implementation) while maintaining >1M msg/s performance.

## Acceptance Criteria
- [ ] **Generic Relay<T> struct** implemented with all common functionality extracted
- [ ] **Performance preserved**: >1M msg/s throughput, <35μs latency per message
- [ ] **Bidirectional forwarding**: Both read and write tasks per connection (like MarketDataRelay)
- [ ] **Connection management**: Proper cleanup, error handling, and metrics tracking
- [ ] **Memory efficient**: 64KB buffers, broadcast channels with configurable size
- [ ] **Protocol compatibility**: Works with existing Protocol V2 TLV message format
- [ ] **Zero runtime overhead**: Generic implementation compiles to same performance as direct code

## Technical Approach

### Core Generic Engine Design
Create `relays/src/generic_relay.rs`:

```rust
use crate::relay_logic::RelayLogic;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixListener;
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, error, info, warn};

pub struct Relay<T: RelayLogic> {
    logic: Arc<T>,
    listener: Option<UnixListener>,
    broadcast_tx: Option<Arc<broadcast::Sender<Vec<u8>>>>,
}

impl<T: RelayLogic> Relay<T> {
    pub fn new(logic: T) -> Self {
        Self {
            logic: Arc::new(logic),
            listener: None, 
            broadcast_tx: None,
        }
    }
    
    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Common initialization logic extracted from all 3 relays
        self.setup_unix_socket().await?;
        self.setup_broadcast_channel();
        self.run_accept_loop().await
    }
    
    async fn setup_unix_socket(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Directory creation, socket cleanup, listener binding
        // Exactly the same logic from all 3 existing implementations
    }
    
    fn setup_broadcast_channel(&mut self) {
        // Create broadcast channel for message forwarding
        // Uses same 10000 capacity as MarketDataRelay
    }
    
    async fn run_accept_loop(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Connection acceptance loop - identical across all relays
    }
    
    async fn handle_connection(&self, stream: tokio::net::UnixStream, connection_id: u64) {
        // Bidirectional connection handling - 80% common logic
        // Uses trait methods for 20% customization
    }
}
```

### Connection Handling Pattern
Extract and generify the **bidirectional pattern** from MarketDataRelay:

```rust
async fn handle_connection(&self, stream: UnixStream, connection_id: u64) {
    // Notify domain-specific logic
    self.logic.on_connection_established(connection_id);
    
    let (mut read_stream, mut write_stream) = stream.into_split();
    let message_tx = self.broadcast_tx.clone();
    let mut consumer_rx = message_tx.subscribe();
    
    // Read task: forward messages to broadcast (common pattern)
    let read_task = self.spawn_read_task(read_stream, connection_id, message_tx.clone());
    
    // Write task: send broadcast messages to connection (common pattern) 
    let write_task = self.spawn_write_task(write_stream, connection_id, consumer_rx);
    
    // Wait for either task completion (identical to MarketDataRelay)
    tokio::select! {
        _ = read_task => { /* cleanup */ }
        _ = write_task => { /* cleanup */ }
    }
}
```

## Implementation Steps

### Step 1: Extract Common Socket Setup (1.5 hours)
1. **Analyze identical code** across all 3 main.rs files:
   - Unix socket directory creation (`/tmp/alphapulse`)  
   - Socket file cleanup (remove existing)
   - UnixListener::bind() with error handling
2. **Extract to generic methods** in Relay<T>::setup_unix_socket()
3. **Use trait method** for socket path: `self.logic.socket_path()`

### Step 2: Extract Connection Management (2 hours)
1. **Unify connection acceptance loops** - identical logic in all 3 relays
2. **Extract connection handling** - preserve MarketDataRelay bidirectional pattern
3. **Generic connection tracking** with atomic counters (same pattern)
4. **Error handling and logging** using trait domain() for custom messages

### Step 3: Extract Message Processing (1.5 hours) 
1. **Bidirectional forwarding pattern** - from MarketDataRelay (most complete)
2. **Buffer management** - 64KB buffers, identical across all relays
3. **Broadcast channel setup** - 10,000 capacity, same as MarketDataRelay
4. **Message filtering** - use `trait.should_forward(message)` for domain logic

### Step 4: Performance Validation (1 hour)
1. **Benchmark generic implementation** vs original MarketDataRelay
2. **Verify zero-cost abstraction** - check assembly output
3. **Memory usage comparison** - ensure no additional allocations
4. **Throughput testing** - must maintain >1M msg/s

## Files to Create/Modify

### CREATE
- `relays/src/generic_relay.rs` - Main generic engine implementation

### MODIFY  
- `relays/src/lib.rs` - Export new Relay<T> type
- `relays/Cargo.toml` - Any new dependencies needed

## Critical Performance Requirements

### Throughput Targets
- **Message Construction**: >1M msg/s (currently 1,097,624 msg/s measured)
- **Message Parsing**: >1.6M msg/s (currently 1,643,779 msg/s measured)  
- **Relay Forwarding**: <35μs latency per message

### Memory Constraints
- **Buffer Size**: 64KB per connection (same as current)
- **Broadcast Channel**: 10,000 message capacity 
- **Connection Limit**: 1000+ concurrent connections

### Zero-Cost Abstraction Validation
```bash
# Compare assembly output to ensure no overhead
cargo build --release
objdump -d target/release/market_data_relay > before.asm
objdump -d target/release/new_market_data_relay > after.asm
diff -u before.asm after.asm  # Should be minimal differences
```

## Testing Strategy

### Unit Tests
- Generic relay setup and teardown  
- Connection handling with mock RelayLogic implementations
- Message forwarding accuracy and ordering

### Integration Tests  
- Full relay lifecycle with real Unix sockets
- Multi-connection scenarios (publisher + consumers)
- Error conditions and recovery

### Performance Tests
```bash
# Benchmark suite to run after implementation
cargo run --release --bin relay_throughput_test
cargo run --release --bin latency_benchmark  
```

## Risk Mitigation

### Performance Regression Risk
**Mitigation**: Continuous benchmarking at each step
- Measure after each method extraction
- Compare assembly output for zero-cost validation
- Use `#[inline]` hints where needed

### Compatibility Risk
**Mitigation**: Preserve exact behavior patterns
- Keep identical buffer sizes and channel capacities
- Match logging formats and connection handling
- Test with existing polygon_publisher and dashboard

## Success Metrics
- [ ] Generic Relay<T> compiles without errors
- [ ] Performance benchmarks show <5% overhead vs original
- [ ] All existing relay behaviors work through generic engine
- [ ] Memory usage identical to original implementations
- [ ] Integration tests pass with real Protocol V2 messages

## Next Task Dependencies
This task **BLOCKS**:
- TASK-003 (Domain Implementations) - needs generic engine to build against
- TASK-004 (Binary Updates) - needs generic engine for new main.rs files

This task **DEPENDS ON**:
- TASK-001 (RelayLogic Trait) - needs trait interface defined

## Documentation Updates Required
- **Generic relay usage examples** in module documentation
- **Performance benchmark results** comparing generic vs direct implementation  
- **Architecture diagram** showing Relay<T> + RelayLogic relationship

---
**Estimated Completion**: 6 hours  
**Complexity**: High - core architecture extraction  
**Risk Level**: Medium-High - performance critical path