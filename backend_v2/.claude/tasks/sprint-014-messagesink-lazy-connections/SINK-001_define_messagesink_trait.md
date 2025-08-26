---
task_id: SINK-001
status: TODO
priority: CRITICAL
estimated_hours: 3
assigned_branch: feat/messagesink-trait-core
assignee: TBD
created: 2025-08-26
completed: null
---

# SINK-001: Define MessageSink Trait and Core Abstractions

## 🔴 CRITICAL: Foundation for Entire Sprint

### Git Worktree Setup (REQUIRED)
```bash
# Create worktree for this task
git worktree add -b feat/messagesink-trait-core ../messagesink-001
cd ../messagesink-001
```

## Status
**Status**: TODO  
**Priority**: CRITICAL - All other tasks depend on this
**Branch**: `feat/messagesink-trait-core`
**Estimated**: 3 hours

## Problem Statement
We need a flexible abstraction for message destinations that:
- Hides connection details from business logic
- Supports both relay and direct connections
- Enables lazy connection establishment
- Can evolve from config-based to Mycelium API without changes

## Acceptance Criteria
- [ ] MessageSink trait defined with all required methods
- [ ] SinkError enum covers all failure modes
- [ ] Message type abstraction for protocol independence
- [ ] Async trait properly bounded for Send + Sync
- [ ] Support for both single and batch operations
- [ ] Connection lifecycle methods (connect/disconnect)
- [ ] Tests demonstrate trait usage patterns

## Technical Design

### Core Trait Definition
```rust
// libs/message_sink/src/lib.rs

use async_trait::async_trait;
use std::fmt::Debug;

/// A destination for messages that abstracts away connection details
#[async_trait]
pub trait MessageSink: Send + Sync + Debug {
    /// Send a single message
    async fn send(&self, message: Message) -> Result<(), SinkError>;
    
    /// Send multiple messages efficiently
    async fn send_batch(&self, messages: Vec<Message>) -> Result<(), SinkError> {
        // Default implementation: send one by one
        for message in messages {
            self.send(message).await?;
        }
        Ok(())
    }
    
    /// Check if currently connected
    fn is_connected(&self) -> bool;
    
    /// Establish connection (may be no-op if already connected)
    async fn connect(&self) -> Result<(), SinkError>;
    
    /// Close connection (may be no-op if not connected)
    async fn disconnect(&self) -> Result<(), SinkError>;
    
    /// Get sink metadata for debugging/monitoring
    fn metadata(&self) -> SinkMetadata {
        SinkMetadata::default()
    }
}
```

### Message Abstraction
```rust
/// Protocol-agnostic message wrapper
#[derive(Debug, Clone)]
pub struct Message {
    /// Raw message bytes (could be TLV, JSON, etc.)
    pub payload: Vec<u8>,
    
    /// Optional routing metadata
    pub metadata: MessageMetadata,
}

#[derive(Debug, Clone, Default)]
pub struct MessageMetadata {
    /// Target service hint (optional)
    pub target: Option<String>,
    
    /// Message priority for queueing
    pub priority: MessagePriority,
    
    /// Timestamp when created
    pub timestamp_ns: u64,
    
    /// Correlation ID for tracing
    pub correlation_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MessagePriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}
```

### Error Handling
```rust
#[derive(Debug, thiserror::Error)]
pub enum SinkError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    
    #[error("Connection lost: {0}")]
    ConnectionLost(String),
    
    #[error("Send failed: {0}")]
    SendFailed(String),
    
    #[error("Buffer full, message dropped")]
    BufferFull,
    
    #[error("Sink closed")]
    Closed,
    
    #[error("Timeout after {0} seconds")]
    Timeout(u64),
    
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Other error: {0}")]
    Other(String),
}
```

### Sink Metadata
```rust
/// Information about a sink for monitoring/debugging
#[derive(Debug, Clone, Default)]
pub struct SinkMetadata {
    /// Human-readable sink name
    pub name: String,
    
    /// Sink type (relay, direct, composite, etc.)
    pub sink_type: String,
    
    /// Connection endpoint if applicable
    pub endpoint: Option<String>,
    
    /// Current connection state
    pub state: ConnectionState,
    
    /// Messages sent successfully
    pub messages_sent: u64,
    
    /// Messages failed to send
    pub messages_failed: u64,
    
    /// Last error if any
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Failed,
}
```

## Implementation Steps

1. **Create library structure**
```bash
# Create new library
mkdir -p libs/message_sink/src
cd libs/message_sink

# Create Cargo.toml
cat > Cargo.toml << 'EOF'
[package]
name = "alphapulse-message-sink"
version = "0.1.0"
edition = "2021"

[dependencies]
async-trait = "0.1"
thiserror = "1.0"
tokio = { version = "1", features = ["full"] }
tracing = "0.1"

[dev-dependencies]
tokio-test = "0.4"
EOF
```

2. **Implement core trait and types**
- Create lib.rs with trait definition
- Add error types
- Add message abstraction
- Add metadata structures

3. **Create test implementations**
```rust
// src/test_utils.rs

/// A sink that just collects messages for testing
pub struct CollectorSink {
    messages: Arc<Mutex<Vec<Message>>>,
    connected: AtomicBool,
}

impl CollectorSink {
    pub fn new() -> Self {
        Self {
            messages: Arc::new(Mutex::new(Vec::new())),
            connected: AtomicBool::new(false),
        }
    }
    
    pub fn received_messages(&self) -> Vec<Message> {
        self.messages.lock().unwrap().clone()
    }
}

#[async_trait]
impl MessageSink for CollectorSink {
    async fn send(&self, message: Message) -> Result<(), SinkError> {
        if !self.is_connected() {
            return Err(SinkError::ConnectionFailed("Not connected".into()));
        }
        self.messages.lock().unwrap().push(message);
        Ok(())
    }
    
    fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed)
    }
    
    async fn connect(&self) -> Result<(), SinkError> {
        self.connected.store(true, Ordering::Relaxed);
        Ok(())
    }
    
    async fn disconnect(&self) -> Result<(), SinkError> {
        self.connected.store(false, Ordering::Relaxed);
        Ok(())
    }
}
```

## Testing Requirements

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_send_requires_connection() {
        let sink = CollectorSink::new();
        let msg = Message::default();
        
        // Should fail when not connected
        assert!(sink.send(msg.clone()).await.is_err());
        
        // Should succeed after connecting
        sink.connect().await.unwrap();
        assert!(sink.send(msg).await.is_ok());
    }
    
    #[tokio::test]
    async fn test_batch_send() {
        let sink = CollectorSink::new();
        sink.connect().await.unwrap();
        
        let messages = vec![
            Message::default(),
            Message::default(),
            Message::default(),
        ];
        
        sink.send_batch(messages).await.unwrap();
        assert_eq!(sink.received_messages().len(), 3);
    }
}
```

## Files to Create/Modify
- `libs/message_sink/Cargo.toml` - New library package
- `libs/message_sink/src/lib.rs` - Core trait and types
- `libs/message_sink/src/error.rs` - Error definitions
- `libs/message_sink/src/message.rs` - Message abstraction
- `libs/message_sink/src/metadata.rs` - Metadata types
- `libs/message_sink/src/test_utils.rs` - Test helpers
- `Cargo.toml` (workspace) - Add new library to workspace

## Git Workflow
```bash
# 1. Create worktree (already done above)
cd ../messagesink-001

# 2. Create library structure
mkdir -p libs/message_sink/src

# 3. Implement trait and types
# ... code implementation ...

# 4. Run tests
cargo test -p alphapulse-message-sink

# 5. Commit
git add -A
git commit -m "feat: define MessageSink trait and core abstractions

- Create MessageSink async trait for flexible destinations
- Add Message abstraction for protocol independence  
- Define SinkError for comprehensive error handling
- Add metadata structures for monitoring
- Include test utilities for validation

Foundation for lazy connections and Mycelium integration"

# 6. Push and create PR
git push origin feat/messagesink-trait-core
gh pr create --title "feat: MessageSink trait foundation" --body "Implements SINK-001"
```

## Completion Checklist
- [ ] Worktree created
- [ ] Library structure created
- [ ] MessageSink trait defined
- [ ] Message abstraction implemented
- [ ] Error types comprehensive
- [ ] Metadata structures complete
- [ ] Test utilities created
- [ ] All tests passing
- [ ] Documentation complete
- [ ] PR created
- [ ] Status updated to COMPLETE

## Why This Matters
This trait is the foundation for the entire MessageSink architecture. It enables:
- Decoupling services from connection details
- Easy migration from config to Mycelium API
- Consistent interface for all message destinations
- Future extensibility without breaking changes