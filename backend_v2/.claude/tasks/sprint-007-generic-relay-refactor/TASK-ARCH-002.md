---
task_id: ARCH-002
status: COMPLETE
priority: HIGH
estimated_hours: 6
assigned_branch: feat/generic-relay-engine
assignee: TBD
created: 2025-08-26
completed: 2025-08-27
depends_on:
  - RELAY-ARCH-001  # Need RelayLogic trait first
blocks:
  - TASK-003  # Domain implementations need generic engine
  - TASK-004  # Binary entry points need generic engine
scope:
  - "relays/src/common/engine.rs"  # Generic Relay<T> implementation
  - "relays/src/common/client.rs"  # RelayClient management
  - "relays/src/common/mod.rs"  # Export generic engine
---

# ARCH-002: Build Generic Relay Engine

**Sprint**: 007 - Generic Relay Engine Refactor  
**Priority**: HIGH  
**Estimate**: 6 hours  
**Dependencies**: ARCH-001 (RelayLogic trait)  

## Objective
Create the generic `Relay<T: RelayLogic>` engine that implements the 80% common relay functionality, abstracting Unix socket management, client connections, and message broadcasting.

## Technical Requirements

### Core Engine Structure
```rust
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::RwLock;
use tokio::net::{UnixListener, UnixStream};

pub struct Relay<T: RelayLogic> {
    logic: Arc<T>,
    clients: Arc<RwLock<HashMap<u64, RelayClient>>>,
    next_client_id: Arc<std::sync::atomic::AtomicU64>,
}

pub struct RelayClient {
    id: u64,
    stream: Arc<tokio::sync::Mutex<UnixStream>>,
    connected_at: std::time::Instant,
}
```

### Core Implementation Requirements

#### 1. Relay Construction
```rust
impl<T: RelayLogic> Relay<T> {
    pub fn new(logic: T) -> Self {
        Self {
            logic: Arc::new(logic),
            clients: Arc::new(RwLock::new(HashMap::new())),
            next_client_id: Arc::new(std::sync::atomic::AtomicU64::new(1)),
        }
    }
}
```

#### 2. Main Event Loop
```rust
pub async fn run(&self) -> Result<(), RelayError> {
    // 1. Initialize logic
    self.logic.initialize()?;
    
    // 2. Bind Unix socket
    let listener = self.bind_socket().await?;
    
    // 3. Spawn client acceptor task
    let accept_handle = self.spawn_client_acceptor(listener);
    
    // 4. Spawn message receiver task  
    let receiver_handle = self.spawn_message_receiver();
    
    // 5. Wait for shutdown signal
    tokio::select! {
        result = accept_handle => {
            error!("Client acceptor died: {:?}", result);
            result??;
        }
        result = receiver_handle => {
            error!("Message receiver died: {:?}", result);  
            result??;
        }
        _ = tokio::signal::ctrl_c() => {
            info!("Received shutdown signal");
        }
    }
    
    // 6. Cleanup
    self.logic.cleanup()?;
    Ok(())
}
```

### Files to Create/Modify
- `relays/src/common/mod.rs` - Add Relay<T> struct
- `relays/src/common/client.rs` - Client connection management
- `relays/src/common/message.rs` - Message handling logic
- Update `relays/Cargo.toml` - Add required dependencies

### Implementation Checklist

#### Socket Management
- [ ] Unix socket binding using `logic.socket_path()`
- [ ] Proper socket cleanup on shutdown
- [ ] Handle socket already in use errors
- [ ] Set appropriate socket permissions
- [ ] Remove existing socket file if needed

#### Client Connection Management  
- [ ] Accept new client connections asynchronously
- [ ] Assign unique client IDs
- [ ] Store client connections in thread-safe map
- [ ] Handle client disconnections gracefully
- [ ] Clean up disconnected clients from map
- [ ] Track connection timestamps

#### Message Processing
- [ ] Read messages from upstream source
- [ ] Parse Protocol V2 32-byte headers
- [ ] Validate message format and checksums
- [ ] Use `logic.should_forward()` for filtering
- [ ] Broadcast to all connected clients
- [ ] Handle partial writes and connection errors

#### Error Handling
- [ ] Graceful degradation on client errors
- [ ] Retry logic for transient failures
- [ ] Proper logging at appropriate levels
- [ ] Resource cleanup on errors
- [ ] Error propagation to main event loop

### Performance Requirements
- [ ] Zero-copy message forwarding where possible
- [ ] Efficient client broadcast (single message parse)
- [ ] O(1) client lookup and management
- [ ] Minimal allocations in hot path
- [ ] Async operations don't block message flow

### Protocol V2 Integration
```rust
// Message structure validation
use alphapulse_protocol_v2::{MessageHeader, parse_header};

async fn process_message(&self, raw_message: &[u8]) -> Result<(), RelayError> {
    // 1. Validate minimum message size (32 bytes header)
    if raw_message.len() < 32 {
        return Err(RelayError::Protocol("Message too short".into()));
    }
    
    // 2. Parse and validate header
    let header = parse_header(&raw_message[0..32])?;
    
    // 3. Check if we should forward this message
    if !self.logic.should_forward(&header) {
        return Ok(()); // Silently drop
    }
    
    // 4. Broadcast to all clients
    self.broadcast_message(raw_message).await?;
    
    Ok(())
}
```

### Client Broadcasting Logic
```rust
async fn broadcast_message(&self, message: &[u8]) -> Result<(), RelayError> {
    let clients = self.clients.read().await;
    let mut failed_clients = Vec::new();
    
    // Broadcast to all clients concurrently
    let broadcast_futures = clients.iter().map(|(id, client)| {
        self.send_to_client(*id, client, message)
    });
    
    let results = futures::future::join_all(broadcast_futures).await;
    
    // Collect failed client IDs
    for (i, result) in results.into_iter().enumerate() {
        if let Err(e) = result {
            let client_id = clients.keys().nth(i).unwrap();
            warn!("Failed to send to client {}: {}", client_id, e);
            failed_clients.push(*client_id);
        }
    }
    
    // Remove failed clients
    if !failed_clients.is_empty() {
        drop(clients); // Release read lock
        let mut clients = self.clients.write().await;
        for client_id in failed_clients {
            clients.remove(&client_id);
            info!("Removed disconnected client {}", client_id);
        }
    }
    
    Ok(())
}
```

### Required Dependencies
Add to `relays/Cargo.toml`:
```toml
[dependencies]
tokio = { version = "1.0", features = ["full"] }
futures = "0.3"  
tracing = "0.1"
thiserror = "1.0"
alphapulse-protocol-v2 = { path = "../protocol_v2" }
```

### Testing Requirements
- [ ] Unit tests for Relay<T> construction
- [ ] Client connection/disconnection tests  
- [ ] Message filtering tests per domain
- [ ] Broadcasting performance tests
- [ ] Error handling and recovery tests
- [ ] Concurrent client stress tests

### Validation Commands
```bash
# Check compilation
cargo check --package relays

# Run engine tests
cargo test --package relays --test relay_engine

# Performance benchmarks  
cargo bench --package relays relay_throughput
```

### Acceptance Criteria
1. ✅ Generic Relay<T> compiles and runs
2. ✅ Unix socket binding works correctly
3. ✅ Client connections handled properly
4. ✅ Message filtering respects logic.should_forward()
5. ✅ Broadcasting works with multiple clients
6. ✅ Error handling and cleanup implemented
7. ✅ Performance meets >1M msg/s requirements
8. ✅ Protocol V2 header parsing integrated

### Architecture Validation
The engine must work with all three domain types:
- `Relay<MarketDataLogic>`
- `Relay<SignalLogic>`  
- `Relay<ExecutionLogic>`

### Next Steps
This task enables ARCH-003 (Module Structure) and all domain logic implementations (DOMAIN-001 through DOMAIN-003).

### Critical Performance Notes
- **Hot Path**: Message processing and broadcasting
- **Cold Path**: Client connection management
- **Memory**: Minimize allocations in message forwarding
- **Concurrency**: All client operations must be non-blocking