# Shared Memory Architecture Notes

## Current Status

The shared memory implementation is working for:
- ✅ Rust collectors writing to shared memory
- ✅ Synchronous readers (test programs)
- ❌ Async readers (API server with Tokio)

## SIGBUS Issue

### Problem
The API server crashes with SIGBUS (exit code 138) when trying to read from shared memory within async Tokio tasks.

### Root Cause
Memory-mapped files can cause SIGBUS when accessed from async contexts due to:
1. Page faults that can't be handled in async contexts
2. Memory maps being accessed across thread boundaries in unsafe ways
3. Tokio's work-stealing scheduler moving tasks between threads

### Verified Fixes
1. **Memory Alignment**: Fixed struct packing issues by removing `#[repr(C, packed)]` attribute
2. **Struct Sizes**: Corrected SharedOrderBookDelta to be exactly 256 bytes as expected

### Attempted Solutions
1. ❌ Using `spawn_blocking` with Arc<Mutex> - still crashes
2. ❌ Adding delays before first read - no effect
3. ⚠️ Using dedicated OS threads with channels - not yet fully implemented

## Working Architecture

Currently, the system works with:
1. Collectors write to both shared memory AND Redis
2. API server reads from Redis (avoiding shared memory in async context)
3. Dashboard connects via WebSocket to receive real-time data

## Future Solutions

### Option 1: Dedicated Thread Pool
Create a dedicated thread pool for shared memory operations:
```rust
// Use std::thread instead of tokio::spawn
std::thread::spawn(move || {
    let mut reader = SharedMemoryReader::open(...);
    loop {
        let data = reader.read_trades();
        // Send via channel to async context
        tx.send(data);
    }
});
```

### Option 2: Separate Binary
Create a separate binary that:
1. Runs synchronously (no async)
2. Reads from shared memory
3. Publishes to Redis or WebSocket

### Option 3: Use io_uring
Investigate using io_uring for truly async memory-mapped file access.

## Performance Metrics

With current Redis-based approach:
- Latency: ~500μs (vs <10μs target for shared memory)
- Throughput: 10K msgs/sec (sufficient for current needs)

## Recommendations

1. **Short term**: Continue using Redis for API server communication
2. **Medium term**: Implement dedicated thread pool solution
3. **Long term**: Research io_uring or other async-safe shared memory solutions

## Test Commands

```bash
# Test shared memory directly (works)
cargo run --bin test_shared_memory

# Test alignment (works)
cargo run --bin alignment_test

# Test memory layout (works)
cargo run --bin memory_layout_analysis

# Test async reader (crashes with SIGBUS)
cargo run --bin test_async_reader
```

## References

- [Tokio and memory-mapped files](https://github.com/tokio-rs/tokio/issues/2832)
- [SIGBUS in async contexts](https://github.com/rust-lang/rust/issues/67671)
- [Memory alignment requirements](https://doc.rust-lang.org/reference/type-layout.html)