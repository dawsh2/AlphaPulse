# MYC-002: Transport Layer Implementation

## Status
- **Status**: pending
- **Assignee**: TBD
- **Estimated Effort**: 2 days
- **Priority**: High (enables producer/consumer connections)

## Description
Implement the transport layer abstraction with unix socket support, connection management, and timestamp utilities. This provides the foundational networking capabilities needed for high-performance message passing between broker and services.

## Objectives
1. Implement Transport trait for unix socket connections
2. Create Connection and Listener abstractions for server/client patterns
3. Implement SystemClock for consistent nanosecond timestamps
4. Add connection pooling and lifecycle management
5. Ensure transport supports >1M msg/s throughput requirements

## Technical Approach

### Core Transport Implementation
```rust
// mycelium-transport/src/unix_socket.rs
use tokio::net::{UnixListener, UnixStream};
use std::path::Path;

pub struct UnixSocketTransport {
    stream: UnixStream,
    read_buffer: Vec<u8>,
    write_buffer: Vec<u8>,
}

#[async_trait]
impl Transport for UnixSocketTransport {
    type Error = TransportError;

    async fn send(&self, data: &[u8]) -> Result<(), Self::Error> {
        use tokio::io::AsyncWriteExt;
        
        // Write message length prefix (4 bytes) + data
        let len = data.len() as u32;
        self.stream.write_all(&len.to_le_bytes()).await?;
        self.stream.write_all(data).await?;
        self.stream.flush().await?;
        Ok(())
    }

    async fn recv(&self) -> Result<Vec<u8>, Self::Error> {
        use tokio::io::AsyncReadExt;
        
        // Read length prefix
        let mut len_bytes = [0u8; 4];
        self.stream.read_exact(&mut len_bytes).await?;
        let len = u32::from_le_bytes(len_bytes) as usize;
        
        // Validate length to prevent DoS
        if len > MAX_MESSAGE_SIZE {
            return Err(TransportError::MessageTooLarge(len));
        }
        
        // Read message data
        let mut data = vec![0u8; len];
        self.stream.read_exact(&mut data).await?;
        Ok(data)
    }

    async fn close(&self) -> Result<(), Self::Error> {
        // Unix sockets clean up automatically when dropped
        Ok(())
    }
}
```

### Connection Management
```rust
// mycelium-transport/src/connection.rs
pub struct ConnectionManager {
    connections: Arc<RwLock<HashMap<ConnectionId, Arc<dyn Transport>>>>,
    listener: Option<UnixSocketListener>,
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            listener: None,
        }
    }

    pub async fn bind(&mut self, socket_path: &str) -> Result<(), TransportError> {
        // Clean up existing socket file
        let _ = std::fs::remove_file(socket_path);
        
        let listener = UnixListener::bind(socket_path).await?;
        self.listener = Some(UnixSocketListener { inner: listener });
        
        // Set appropriate permissions for socket
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o660);
            std::fs::set_permissions(socket_path, perms)?;
        }
        
        Ok(())
    }

    pub async fn accept_connections(&self) -> Result<(), TransportError> {
        let listener = self.listener.as_ref()
            .ok_or(TransportError::NotBound)?;
            
        loop {
            match listener.accept().await {
                Ok(transport) => {
                    let conn_id = ConnectionId::new();
                    self.connections.write().await
                        .insert(conn_id, Arc::new(transport));
                }
                Err(e) => {
                    tracing::warn!("Failed to accept connection: {}", e);
                }
            }
        }
    }
}
```

### SystemClock Implementation
```rust
// mycelium-transport/src/time.rs
pub struct SystemClock;

impl SystemClock {
    pub fn now_nanos() -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time before Unix epoch")
            .as_nanos() as u64
    }

    pub fn now_micros() -> u64 {
        Self::now_nanos() / 1000
    }

    pub fn now_millis() -> u64 {
        Self::now_nanos() / 1_000_000
    }
}

// Performance-optimized clock for hot paths
pub struct MonotonicClock {
    start_time: std::time::Instant,
    epoch_nanos: u64,
}

impl MonotonicClock {
    pub fn new() -> Self {
        Self {
            start_time: std::time::Instant::now(),
            epoch_nanos: SystemClock::now_nanos(),
        }
    }

    pub fn now_nanos(&self) -> u64 {
        let elapsed = self.start_time.elapsed().as_nanos() as u64;
        self.epoch_nanos + elapsed
    }
}
```

### Error Handling
```rust
// mycelium-transport/src/error.rs
#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Message too large: {0} bytes (max: {1})")]
    MessageTooLarge(usize, usize),
    
    #[error("Connection closed")]
    ConnectionClosed,
    
    #[error("Not bound to socket")]
    NotBound,
    
    #[error("Timeout after {0}ms")]
    Timeout(u64),
    
    #[error("Buffer overflow")]
    BufferOverflow,
}
```

## Acceptance Criteria

### Core Functionality
- [ ] Unix socket transport implements Transport trait correctly
- [ ] Connection manager handles multiple concurrent connections
- [ ] Message framing prevents partial reads/writes
- [ ] SystemClock provides nanosecond timestamp precision

### Performance Requirements
- [ ] Transport supports >1M msg/s throughput (benchmark test)
- [ ] Connection setup/teardown completes in <1ms
- [ ] Message serialization adds <10μs overhead
- [ ] Memory usage scales linearly with connection count

### Reliability Features
- [ ] Graceful connection failure handling
- [ ] Socket cleanup on process termination
- [ ] Buffer overflow protection with configurable limits
- [ ] Connection pooling prevents resource exhaustion

### Integration Points
- [ ] Transport trait compatible with broker layer
- [ ] Clock utilities integrate with existing timestamp systems
- [ ] Error types propagate cleanly to service layers
- [ ] Configuration supports multiple socket paths

## Dependencies
- **Upstream**: MYC-001 (Platform Foundation) - requires trait definitions
- **Downstream**: MYC-005 (Producer Migration), MYC-006 (Consumer Migration)
- **External**: tokio for async networking, tracing for observability

## Testing Requirements

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn unix_socket_send_recv() {
        let dir = tempdir().unwrap();
        let socket_path = dir.path().join("test.sock");
        
        // Test basic send/receive functionality
        let (transport1, transport2) = create_connected_pair(&socket_path).await;
        
        let test_data = b"hello world";
        transport1.send(test_data).await.unwrap();
        
        let received = transport2.recv().await.unwrap();
        assert_eq!(received, test_data);
    }

    #[tokio::test]
    async fn connection_manager_multiple_clients() {
        let dir = tempdir().unwrap();
        let socket_path = dir.path().join("multi.sock");
        
        let mut manager = ConnectionManager::new();
        manager.bind(socket_path.to_str().unwrap()).await.unwrap();
        
        // Connect multiple clients concurrently
        let handles = (0..10).map(|_| {
            let path = socket_path.clone();
            tokio::spawn(async move {
                UnixSocketTransport::connect(path.to_str().unwrap()).await
            })
        }).collect::<Vec<_>>();
        
        for handle in handles {
            assert!(handle.await.unwrap().is_ok());
        }
    }

    #[test]
    fn system_clock_precision() {
        let t1 = SystemClock::now_nanos();
        std::thread::sleep(std::time::Duration::from_nanos(1000));
        let t2 = SystemClock::now_nanos();
        
        assert!(t2 > t1);
        assert!(t2 - t1 >= 1000); // At least 1000ns difference
    }
}
```

### Performance Tests
```rust
#[cfg(test)]
mod perf_tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Run with --ignored flag
    async fn throughput_benchmark() {
        let (sender, receiver) = create_connected_pair("bench.sock").await;
        
        let message = vec![0u8; 1024]; // 1KB message
        let num_messages = 1_000_000;
        
        let start = std::time::Instant::now();
        
        // Send messages in parallel with receiving
        let send_handle = tokio::spawn(async move {
            for _ in 0..num_messages {
                sender.send(&message).await.unwrap();
            }
        });
        
        let recv_handle = tokio::spawn(async move {
            for _ in 0..num_messages {
                receiver.recv().await.unwrap();
            }
        });
        
        tokio::join!(send_handle, recv_handle);
        
        let elapsed = start.elapsed();
        let msg_per_sec = num_messages as f64 / elapsed.as_secs_f64();
        
        println!("Throughput: {:.0} msg/s", msg_per_sec);
        assert!(msg_per_sec > 1_000_000.0); // >1M msg/s requirement
    }
}
```

### Integration Tests
```rust
#[tokio::test]
async fn integration_with_broker_pattern() {
    // Simulate broker using transport layer
    let (broker_transport, client_transport) = create_connected_pair("integration.sock").await;
    
    // Broker receives and echoes messages
    let broker_handle = tokio::spawn(async move {
        loop {
            match broker_transport.recv().await {
                Ok(data) => {
                    broker_transport.send(&data).await.unwrap();
                }
                Err(_) => break,
            }
        }
    });
    
    // Client sends message and receives echo
    let test_msg = b"integration test";
    client_transport.send(test_msg).await.unwrap();
    
    let echo = client_transport.recv().await.unwrap();
    assert_eq!(echo, test_msg);
}
```

## Rollback Plan

### If Performance Targets Not Met
1. Revert to synchronous I/O if async overhead too high
2. Use raw TCP sockets instead of unix sockets if needed
3. Implement zero-copy optimizations with unsafe code

### If Stability Issues
1. Add more conservative buffer management
2. Implement exponential backoff for connection retries
3. Add more comprehensive error recovery mechanisms

### If Integration Problems
1. Simplify Transport trait interface
2. Remove advanced features like connection pooling
3. Use concrete types instead of trait objects

## Technical Notes

### Design Decisions
- **Length-Prefixed Framing**: Prevents partial message reads/writes
- **Async by Default**: Enables high concurrency without thread overhead
- **Buffer Management**: Pre-allocated buffers reduce allocation overhead
- **Connection Pooling**: Reuses connections for better performance

### Performance Optimizations
- **Zero-Copy Where Possible**: Use `&[u8]` slices to avoid copies
- **Buffer Reuse**: Maintain per-connection read/write buffers
- **Batch Operations**: Group multiple small messages when possible
- **Monotonic Clock**: Avoid system call overhead for timestamps

### Unix Socket Advantages
- **Performance**: Higher throughput than TCP for local communication
- **Security**: Filesystem permissions control access
- **Reliability**: No network stack overhead or failures
- **Compatibility**: Works across different platforms

## Validation Steps

1. **Basic Functionality**:
   ```bash
   cargo test --package mycelium-transport
   ```

2. **Performance Validation**:
   ```bash
   cargo test --package mycelium-transport --release -- --ignored
   ```

3. **Integration Testing**:
   ```bash
   cargo test --package mycelium-transport integration
   ```

4. **Benchmark Comparison**:
   ```bash
   cargo bench --package mycelium-transport
   ```

This transport layer provides the high-performance foundation needed for the Mycelium broker while maintaining the reliability and observability requirements of the AlphaPulse system.