# Rust Migration Plan for AlphaPulse

## Current Status (Week 7 of 14)

### ✅ Completed
- **Phase 1**: Rust WebSocket collectors for Coinbase/Kraken (rust-services/collectors/)
- **Phase 2**: Redis Streams integration with XADD for multi-consumer pattern
- **Developer Dashboard**: Real-time monitoring at localhost:5174
- **Architecture Decision**: Shared memory microservices over unified engine

### 🔄 In Progress  
- Testing Rust collectors with real exchange data
- Python consumer for Redis Streams (backend/services/redis_stream_consumer.py)
- Dashboard integration with actual Redis data

### 📋 Next 5-7 Days
1. **Day 1-2**: Complete Redis Streams integration
   - Switch dashboard from test data to real Redis Streams
   - Verify multi-consumer pattern working correctly
   - Test backpressure handling at 1000+ msg/sec

2. **Day 3-4**: Shared Memory Implementation
   - Implement /dev/shm ring buffers for orderbook data
   - Add memory barriers for lock-free access
   - Create Python bindings with PyO3

3. **Day 5-7**: Performance Optimization
   - Set CPU affinity for collectors (isolcpus=2-7)
   - Enable io_uring for network I/O (Linux 5.1+)
   - Add Prometheus metrics endpoints
   - Benchmark: target <10μs orderbook update latency

## Architecture Overview

### Hybrid Rust/Python Design
```
┌─────────────────────────────────────────────────────────┐
│                   Frontend (React/TS)                    │
│                   Port 5173 (main app)                   │
│                   Port 5174 (dashboard)                  │
└─────────────────────────────────────────────────────────┘
                            │
                ┌───────────┴───────────┐
                │    API Gateway        │
                │   FastAPI (Port 8080) │
                └───────────┬───────────┘
                            │
        ┌───────────────────┼───────────────────┐
        │                   │                   │
┌───────▼────────┐ ┌────────▼────────┐ ┌───────▼────────┐
│  Rust Services │ │ Python Analytics│ │   Python API   │
│  (Tokio async) │ │  (Jupyter/ML)   │ │  (Business)    │
│                │ │                 │ │                │
│ • WebSocket    │ │ • Backtesting   │ │ • Auth/Users   │
│ • Orderbooks   │ │ • Indicators    │ │ • Strategies   │
│ • Trade Ticks  │ │ • ML Models     │ │ • UI Endpoints │
└────────┬───────┘ └────────┬────────┘ └───────┬────────┘
         │                  │                   │
         └──────────────────┼───────────────────┘
                            │
                ┌───────────▼───────────┐
                │   Data Layer          │
                │ • Redis Streams       │
                │ • Shared Memory       │
                │ • DuckDB/TimescaleDB │
                └───────────────────────┘
```

### Key Design Decisions

#### 1. Shared Memory Microservices (Not Unified Engine)
- **Why**: 10x better fault isolation, easier debugging, incremental deployment
- **How**: /dev/shm ring buffers with lock-free SPSC queues
- **Performance**: <1μs IPC latency with memory barriers

#### 2. Redis Streams for Event Bus
- **Why**: Built-in persistence, consumer groups, proven at scale
- **Pattern**: XADD for producers, XREADGROUP for consumers
- **Fallback**: Kafka only if Redis can't handle volume (unlikely)

#### 3. Keep Python for Business Logic
- **Jupyter Integration**: Essential for quant research
- **ML Models**: Scikit-learn, PyTorch ecosystem
- **Rapid Iteration**: Strategy development stays in Python

## Implementation Plan

### Phase 1: Rust Collectors ✅ COMPLETE
- WebSocket collectors for Coinbase/Kraken
- Tokio async runtime
- Redis XADD for stream writes

### Phase 2: Redis Integration (CURRENT)
```python
# backend/services/redis_stream_consumer.py
async def consume_trades(stream_patterns: List[str]):
    messages = await redis.xreadgroup(
        group_name="analytics",
        consumer_name="python-1",
        streams={stream: ">" for stream in stream_patterns},
        block=100
    )
```

### Phase 3: Shared Memory Layer (NEXT)
```rust
// rust-services/shared-memory/src/lib.rs
pub struct OrderbookRingBuffer {
    data: Arc<MmapMut>,  // /dev/shm/orderbook
    write_pos: AtomicUsize,
    read_pos: AtomicUsize,
}

impl OrderbookRingBuffer {
    pub fn write(&self, orderbook: &OrderbookUpdate) {
        let pos = self.write_pos.fetch_add(1, Ordering::Release);
        // Zero-copy write to shared memory
        unsafe { 
            let ptr = self.data.as_ptr().add(pos * ENTRY_SIZE);
            std::ptr::write(ptr as *mut OrderbookUpdate, *orderbook);
        }
    }
}
```

### Phase 4: Performance Optimizations
```rust
// CPU Affinity
let core_ids = vec![2, 3, 4, 5];  // Isolated cores
affinity::set_thread_affinity(core_ids)?;

// io_uring for network I/O
let ring = IoUring::builder()
    .setup_sqpoll(1000)  // Kernel polling thread
    .build(256)?;

// Memory barriers for lock-free access
write_pos.store(new_pos, Ordering::Release);
fence(Ordering::AcqRel);
```

## Performance Targets

| Metric | Current (Python) | Target (Rust) | Method |
|--------|-----------------|---------------|---------|
| WebSocket → Storage | 50-100ms | <5ms | Tokio + Redis pipeline |
| Orderbook Update | 100ms+ | <10μs | Shared memory |
| Messages/sec | 500 | 10,000+ | Lock-free queues |
| Memory Usage | 2-4GB | <500MB | Zero-copy, no DataFrame |
| CPU Usage | 60-80% | <20% | Efficient async |

## Monitoring & Observability

### Prometheus Metrics
```rust
// rust-services/collectors/src/metrics.rs
lazy_static! {
    static ref TRADES_COUNTER: IntCounter = 
        register_int_counter!("trades_total", "Total trades processed").unwrap();
    static ref ORDERBOOK_HISTOGRAM: Histogram = 
        register_histogram!("orderbook_latency_us", "Orderbook update latency").unwrap();
}
```

### Developer Dashboard (localhost:5174)
- Real-time WebSocket firehose view
- Orderbook depth visualizer
- Trade flow stream
- System metrics (CPU, memory, network)
- Service health status

## Migration Rollout Strategy

### Week 8-9: Shared Memory Integration
- Implement ring buffers in /dev/shm
- Python bindings with PyO3
- Test with high-frequency data

### Week 10-11: Performance Tuning
- CPU affinity and NUMA optimization
- io_uring implementation
- Benchmark against targets

### Week 12-13: Production Hardening
- Blue-green deployment setup
- Monitoring and alerting
- Load testing at 10x expected volume

### Week 14: Go Live
- Shadow mode testing
- Gradual traffic migration
- Rollback procedures ready

## Risk Mitigation

1. **Complexity Risk**: Start simple (Redis), optimize later (shared memory)
2. **Integration Risk**: Keep Python/Rust boundary minimal and well-defined
3. **Performance Risk**: Benchmark early and often, have fallback to Python
4. **Operational Risk**: Blue-green deployment, feature flags, gradual rollout

## Success Criteria

- [ ] Handle 10,000+ messages/second sustained
- [ ] <10μs orderbook update latency
- [ ] <500MB memory footprint for collectors
- [ ] Zero message loss under load
- [ ] Clean separation of concerns (Rust=speed, Python=logic)

## Appendix: Technical Details

### A. Shared Memory Layout
```c
// /dev/shm/alphapulse/orderbook
struct OrderbookEntry {
    uint64_t timestamp_ns;
    uint32_t symbol_id;
    uint32_t exchange_id;
    float bids[50][2];  // price, size
    float asks[50][2];
    uint32_t sequence;
    uint8_t padding[8];  // Cache line alignment
} __attribute__((packed));
```

### B. Redis Stream Schema
```
Stream: trades:coinbase:BTC-USD
Fields:
  - timestamp: Unix epoch microseconds
  - price: Decimal string
  - volume: Decimal string  
  - side: "buy" | "sell"
  - trade_id: Unique identifier
```

### C. Repository Pattern (Python/Rust Bridge)
```python
# backend/core/repositories/market_data.py
class MarketDataRepository(ABC):
    @abstractmethod
    async def get_trades(self, symbol: str, start: datetime, end: datetime) -> pd.DataFrame:
        pass

class RustMarketDataRepository(MarketDataRepository):
    def __init__(self):
        self.rust_client = PyRustClient()  # PyO3 binding
    
    async def get_trades(self, symbol: str, start: datetime, end: datetime) -> pd.DataFrame:
        # Calls Rust service via shared memory or Redis
        data = await self.rust_client.query_trades(symbol, start, end)
        return pd.DataFrame(data)
```

---

## Quick Reference

### Start Services
```bash
# Rust collectors
cd rust-services/collectors
cargo run --release

# Python backend
cd backend
uvicorn app_fastapi:app --reload --port 8080

# Developer dashboard
cd frontend
npm run dashboard

# Redis Streams consumer
python services/redis_stream_consumer.py
```

### Monitor Performance
```bash
# Watch Redis Streams
redis-cli XINFO STREAM trades:coinbase:BTC-USD

# Prometheus metrics
curl localhost:9090/metrics | grep trades_total

# System resources
htop -p $(pgrep -f "rust-collector")
```

### Debug Issues
```bash
# Check service logs
journalctl -f -u alphapulse-collector

# Trace system calls
strace -p $(pgrep -f "rust-collector") -e trace=network

# Profile CPU usage
perf record -F 99 -p $(pgrep -f "rust-collector")
perf report
```

## Raw Capture Layer (Optional Enhancement)

### Why Consider Raw Capture?

**Update**: DuckDB is highly optimized for Parquet and handles sequential backtesting excellently! Raw capture is only needed for specific edge cases.

**DuckDB + Parquet handles backtesting well**:
```sql
-- DuckDB efficiently reads time-ordered data
SELECT * FROM read_parquet('trades/*.parquet')
WHERE timestamp BETWEEN '2024-01-01 09:30:00' AND '2024-01-01 16:00:00'
ORDER BY timestamp;
-- Fast due to: predicate pushdown, row group statistics, streaming
```

**When you actually need raw capture**:
- **Debugging**: "Why did our system glitch at 10:32:15.483?"
- **Exchange disputes**: Prove exact message sequence
- **Network analysis**: Detect disconnections, malformed messages
- **Regulatory**: Immutable audit trail requirement

### Solution: Dual Storage Strategy

```
                 ┌──────────────────────────┐
Exchange ───────→│   Rust Collector         │
                 └───────────┬──────────────┘
                             │
                ┌────────────┼────────────┐
                ↓            ↓            ↓
         ┌──────────┐ ┌──────────┐ ┌──────────┐
         │Raw Binary│ │  Redis   │ │TimescaleDB│
         │   Log    │ │ Streams  │ │ (Buffer)  │
         │(.wslog)  │ │(realtime)│ │ (7 days)  │
         └────┬─────┘ └──────────┘ └─────┬────┘
              │                           │
              ↓ Replay                    ↓ Batch
         ┌──────────┐              ┌──────────┐
         │Backtesting│              │ Parquet  │
         │  Engine  │              │(Analytics)│
         └──────────┘              └──────────┘
```

### Raw Capture Format (.wslog files)

Binary append-only format optimized for sequential replay:

```rust
// rust-services/collectors/src/raw_capture.rs
pub struct WsLogEntry {
    timestamp_ns: u64,      // 8 bytes - nanosecond precision
    exchange_id: u16,       // 2 bytes - exchange identifier  
    message_len: u32,       // 4 bytes - message length
    raw_message: [u8],      // Variable - exact WebSocket frame
}

impl RawCapture {
    pub async fn write(&mut self, exchange: Exchange, msg: &[u8]) {
        let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        
        // Write fixed header (14 bytes)
        self.file.write_all(&ts.as_nanos().to_le_bytes())?;
        self.file.write_all(&(exchange as u16).to_le_bytes())?;
        self.file.write_all(&(msg.len() as u32).to_le_bytes())?;
        
        // Write raw WebSocket message
        self.file.write_all(msg)?;
        
        // Flush every 1MB for durability
        if self.buffer_size > 1_048_576 {
            self.file.flush()?;
            self.buffer_size = 0;
        }
    }
}
```

### File Rotation Strategy

```
/data/raw/
├── coinbase/
│   ├── 20250113_00.wslog.zst  (compressed)
│   ├── 20250113_01.wslog.zst  (compressed)
│   └── 20250113_02.wslog      (current, uncompressed)
├── kraken/
│   └── 20250113_02.wslog      (current)
└── index.json                  (metadata)
```

### Compression & Storage

| Stage | Format | Size (1 day) | Use Case |
|-------|--------|--------------|----------|
| Live | .wslog | ~10GB | Real-time capture |
| 1 hour old | .wslog.zst | ~1GB | Recent replay |
| 1 day old | .wslog.zst | ~1GB | Backtesting |
| 1 week old | S3 + .wslog.zst | ~1GB | Archive |

### Backtesting Replay

```rust
// Fast sequential replay for backtesting
pub struct WsLogReader {
    files: Vec<File>,
    heap: BinaryHeap<(u64, WsLogEntry)>,  // Min-heap by timestamp
}

impl WsLogReader {
    pub async fn replay<F>(&mut self, mut callback: F) 
    where F: FnMut(Exchange, &[u8])
    {
        // Read all files in parallel, merge by timestamp
        while let Some((ts, entry)) = self.heap.pop() {
            callback(entry.exchange(), entry.raw_message());
            
            // Simulate original timing
            if let Some(next_ts) = self.heap.peek() {
                let delay = next_ts.0 - ts;
                if delay > 0 {
                    sleep(Duration::from_nanos(delay)).await;
                }
            }
        }
    }
}
```

### Benefits Over Parquet-Only

1. **Perfect Fidelity**: Exact bytes, exact order, exact timing
2. **Fast Sequential**: 100MB/s read speed for replay
3. **Network Debugging**: Preserves malformed messages, disconnects
4. **Simple Format**: 14-byte header + raw bytes
5. **Regulatory**: Immutable audit trail of market data

### When to Use What

| Storage | Use For | Don't Use For |
|---------|---------|---------------|
| **DuckDB + Parquet** | Backtesting, analytics, research | Raw message debugging |
| **Raw Logs** | Debugging, compliance, disputes | Analytics queries |
| **TimescaleDB** | Recent queries, monitoring | Long-term storage |
| **Redis Streams** | Real-time feeds | Historical data |

### Implementation Priority

**Primary Focus**: DuckDB + Parquet (already working!)
```python
# This is your main backtesting path - already optimal!
df = duckdb.sql("""
    SELECT * FROM read_parquet('trades/*.parquet') 
    WHERE timestamp BETWEEN ? AND ?
    ORDER BY timestamp
""").df()
```

**Optional Addition**: Raw capture (only if needed)
- Implement only if you have regulatory requirements
- Or if you need to debug production issues
- Most trading systems don't need this level of capture

---

*Document Version: 2.2 (Clarified DuckDB + Parquet is optimal for backtesting)*
*Last Updated: Week 7 of Migration*
*Next Review: End of Week 8*