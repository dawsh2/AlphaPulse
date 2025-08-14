# Lessons Learned from Redis/SharedMemory Experiments

## Overview
This document captures key insights from our experiments with various approaches to building a high-performance, event-driven market data pipeline.

## Approaches Tested

### 1. Redis Streams with XREAD BLOCK
**Implementation:** Redis XADD from collectors → XREAD BLOCK in API server

**Pros:**
- Truly event-driven (XREAD BLOCK 0 blocks until data arrives - zero polling)
- Cross-process communication works seamlessly
- Production-ready and battle-tested
- Good debugging/monitoring tools available
- ~100μs latency (acceptable for dashboards)

**Cons:**
- External dependency (Redis server required)
- Network overhead adds latency
- Not suitable for <10μs requirements

### 2. POSIX Unnamed Semaphores
**Implementation:** sem_init with pshared=1 for cross-process signaling

**Result:** Failed on macOS
- macOS doesn't support unnamed POSIX semaphores with pshared=1
- Returns ENOSYS (function not implemented)
- Works on Linux but not portable

### 3. Named Semaphores
**Implementation:** sem_open/sem_post/sem_wait for cross-process signaling

**Pros:**
- Works on macOS
- True event-driven signaling

**Cons:**
- Complex to manage lifecycle (cleanup on crash)
- Still need separate mechanism for data transfer
- Permission issues with /dev/shm on some systems

### 4. eventfd (Linux-specific)
**Implementation:** eventfd for event notification + shared memory for data

**Result:** Not portable
- Linux-specific system call
- No macOS equivalent
- Would require platform-specific code

### 5. TokioTransport with Notify
**Implementation:** In-memory ring buffer with Tokio Notify for signaling

**Pros:**
- Very fast within single process
- Clean async/await interface
- Lock-free implementation

**Cons:**
- Only works within single process
- Breaks microservices architecture
- Can't share across process boundaries

## Key Insights

### Performance vs Architecture Trade-offs
1. **Single Process = Fast but Monolithic**
   - Shared memory within process: ~1μs latency
   - Loses microservices benefits (fault isolation, independent scaling)

2. **Multiple Processes = Slower but Resilient**
   - Redis Streams: ~100μs latency
   - Unix sockets: ~10-50μs latency
   - Maintains clean service boundaries

### Event-Driven is Essential
- Polling wastes CPU and adds latency
- True blocking (XREAD BLOCK, semaphores, epoll) is required
- Event notification must wake consumers immediately

### Cross-Platform Considerations
- macOS has significant limitations for IPC
- Linux-specific optimizations often don't port
- Need to design for lowest common denominator

## Decision: Tokio + Unix Domain Sockets

After extensive experimentation, we're moving forward with Unix domain sockets because:

1. **Performance:** 5-20μs latency (good enough for HFT)
2. **Portability:** Works on both macOS and Linux
3. **Simplicity:** No external dependencies
4. **Architecture:** Maintains microservices separation
5. **Proven:** Used successfully in many high-performance systems

## Architecture Evolution

### Phase 1 (Completed): Redis Streams
- Built working system with Redis XREAD BLOCK
- Achieved event-driven, cross-process communication
- Good for learning and prototyping

### Phase 2 (Current): Unix Domain Sockets
- Binary protocol for efficiency
- Tokio for async I/O
- Lock-free data structures where appropriate
- Target: <50μs end-to-end latency

### Phase 3 (Future): Potential Optimizations
- Shared memory with proper signaling for critical path
- Kernel bypass networking for exchange connections
- DPDK/SPDK for extreme performance

## Technical Debt Avoided

By experimenting early, we avoided:
- Building complex systems on incompatible foundations (POSIX semaphores)
- Over-engineering with platform-specific code (eventfd)
- Premature optimization that breaks architecture (single process)

## Best Practices Learned

1. **Test platform compatibility early**
2. **Measure actual latencies, not theoretical**
3. **Keep architecture clean even if it costs some performance**
4. **Event-driven > polling, always**
5. **Binary protocols for performance-critical paths**
6. **Profile before optimizing**

## References

- [Redis Streams Documentation](https://redis.io/docs/data-types/streams/)
- [Tokio Tutorial](https://tokio.rs/tokio/tutorial)
- [Unix Domain Sockets Performance](https://www.percona.com/blog/2020/04/13/need-to-connect-to-a-local-mysql-server-use-unix-domain-socket/)
- [High-Performance Trading Systems](https://www.youtube.com/watch?v=NH1Tta7purM)

---

*Document created: December 2024*
*Last updated: December 2024*