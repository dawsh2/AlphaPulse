# CLAUDE.md - AlphaPulse AI Assistant Context

## System Overview
AlphaPulse is a high-performance cryptocurrency trading system that processes real-time market data from multiple exchanges through a sophisticated binary protocol pipeline, achieving <35μs latency for critical operations.

**Core Mission**: Build a robust, validated, and safe trading infrastructure with complete transparency and zero tolerance for deceptive practices.

**Development Priority**: Quality over speed. Completing immediate tasks is NOT the highest priority - developing a well-organized, high-quality, robust/safe/validating system is. All work must be done with long-term reliability in mind. No shortcuts.

**Production-Ready Code**: ALWAYS write code as if it's going straight into production with real money. Never use fake/mock/dummy variables, services, or data. Every line of code must be production-quality from the start.

## Architecture Summary
```
Exchanges → Collectors (Rust) → Binary Protocol → Relay → Bridge → Dashboard (React)
         WebSocket            48-byte messages   Unix Socket  JSON    WebSocket
```

## Critical System Invariants
1. **Binary Protocol**: MUST maintain 48-byte fixed message size with 8 decimal precision
2. **Zero Precision Loss**: All decimal conversions must preserve full precision
3. **No Deception**: Never hide failures, fake data, or simulate success - complete transparency required
4. **Latency Requirements**: Hot path <35μs, warm path <100ms
5. **Memory Efficiency**: Zero-copy operations in performance-critical paths
6. **Nanosecond Timestamps**: Never truncate to milliseconds
7. **Dynamic Configuration**: Use configurable values instead of hardcoded constants where adaptability is needed
8. **One Canonical Source**: Single implementation per concept - no "enhanced", "fixed", "new" duplicates
9. **Respect Project Structure**: Maintain service boundaries and established file hierarchy
10. **NO MOCKS EVER**: Never use mock data, mock services, or any form of mocked testing under any circumstances

## Common Development Tasks

### Running the System
```bash
# Start all services (recommended)
./scripts/start-polygon-only.sh

# Start individual services
cargo run --release --bin exchange_collector
cargo run --release --bin relay_server
cargo run --release --bin ws_bridge
python -m uvicorn app_fastapi:app --reload --port 8000

# Monitor connections
./scripts/monitor_connections.sh
```

### Testing Commands
```bash
# CRITICAL: Always run precision tests before committing
cargo test --package protocol --test precision_tests

# Full test suite
cargo test --workspace
pytest tests/ -v --cov=backend

# Performance benchmarks
cargo bench --workspace

# Data validation tests
pytest tests/data_validation/test_binary_protocol.py
pytest tests/data_validation/test_pipeline_integrity.py
```

### Code Quality Checks
```bash
# Rust
cargo fmt --all -- --check
cargo clippy --workspace -- -D warnings

# Python  
ruff check backend/ --fix
black backend/ --check
mypy backend/services/ --strict
```

## Project Structure
```
alphapulse/
├── backend/
│   ├── services/          # Rust microservices (RESPECT BOUNDARIES)
│   │   ├── exchange_collector/  # Exchange data collection ONLY
│   │   ├── relay_server/       # Binary protocol relay
│   │   ├── ws_bridge/          # WebSocket bridge to frontend
│   │   └── defi/              # ALL DeFi-related services
│   │       ├── scanner/       # Arbitrage opportunity detection
│   │       └── arbitrage_bot/ # Arbitrage execution
│   ├── protocol/          # Binary protocol definitions (CRITICAL)
│   ├── api/              # Python FastAPI endpoints
│   ├── scripts/          # Utility scripts ONLY (no core logic)
│   └── tests/            # Comprehensive test suites
├── frontend/             # React dashboard
└── projects/            # Documentation and planning
```

**IMPORTANT**: Each service has a specific responsibility. Don't scatter related code across multiple locations or create files in the wrong directory hierarchy.

## Key Technical Decisions

### Why Binary Protocol?
- Fixed 48-byte messages ensure predictable latency
- Enables zero-copy operations in hot path
- Preserves 8 decimal place precision without floating point errors
- Efficient Unix socket transmission between services

### Why Rust for Collectors?
- No garbage collection pauses
- Predictable performance characteristics
- Memory safety without runtime overhead
- Excellent async/await ecosystem for WebSocket handling

### Why Service Separation?
- Independent scaling of components
- Language-appropriate implementations (Rust for performance, Python for flexibility)
- Fault isolation and recovery
- Clear responsibility boundaries

## Common Pitfalls & Solutions

### ❌ DON'T: Use Floating Point for Prices
```rust
// WRONG - Precision loss!
let price: f64 = 0.12345678;
```

### ✅ DO: Use Fixed-Point Integer
```rust
// CORRECT - Maintains precision
let price: i64 = 12345678; // Represents 0.12345678
```

### ❌ DON'T: Truncate Timestamps
```python
# WRONG - Loses precision
timestamp_ms = timestamp_ns // 1_000_000
```

### ✅ DO: Preserve Nanoseconds
```python
# CORRECT - Full precision
timestamp_ns = int(time.time() * 1_000_000_000)
```

### ❌ DON'T: Ignore Null Fields
```rust
// WRONG - Will panic on null
let price = data["price"].as_f64().unwrap();
```

### ✅ DO: Handle Nulls Gracefully
```rust
// CORRECT - Graceful handling
let price = match data.get("price") {
    Some(v) if !v.is_null() => v.as_str().and_then(|s| s.parse().ok()),
    _ => None,
};
```

### ❌ DON'T: Use Hardcoded Values
```rust
// WRONG - Hardcoded thresholds
if spread_percentage > 0.5 { // Hardcoded 0.5%
    execute_arbitrage();
}
const MIN_PROFIT: f64 = 100.0; // Hardcoded $100
```

### ✅ DO: Use Dynamic Configuration
```rust
// CORRECT - Configurable values
#[derive(Debug, Clone)]
pub struct ArbitrageConfig {
    pub min_spread_percentage: Decimal,
    pub min_profit_usd: Decimal,
    pub max_gas_cost_usd: Decimal,
}

if spread_percentage > config.min_spread_percentage {
    execute_arbitrage();
}
```

### ❌ DON'T: Hide Failures or Fake Success
```rust
// WRONG - Deceptive behavior
match exchange.get_price() {
    Ok(price) => price,
    Err(_) => Decimal::from(0), // Hiding failure with fake data!
}

// WRONG - Simulating success
if !connected {
    return Ok(SimulatedResult { fake: true }); // Deceptive!
}
```

### ✅ DO: Be Transparent About Failures
```rust
// CORRECT - Propagate failures honestly
let price = exchange.get_price()
    .map_err(|e| {
        error!("Failed to fetch price: {}", e);
        e
    })?; // Propagate error, don't hide it

// CORRECT - Real execution only
if !connected {
    return Err(CollectorError::NotConnected);
}
```

## Current Migration Status

### Symbol → Instrument Migration
- **Status**: In Progress
- **Scope**: 878+ instances across 102 files
- **Impact**: Breaking change requiring coordinated update
- **Command**: `python scripts/migrate_symbol_to_instrument.py --dry-run`

### Backend Cleanup
- **Status**: Planning
- **Issue**: 50+ files scattered in backend root
- **Goal**: Organize into logical directories
- **Risk**: Import path updates required

## Testing Philosophy

### Real Data Only - NO MOCKS
- **NEVER** use mock data, mock services, or mocked responses
- **ALWAYS** use real exchange connections for testing
- **ALWAYS** test with actual market data and live price feeds
- **NO** simulation modes that fake exchange responses
- **NO** stubbed WebSocket connections or API responses

### Data Integrity First
Every change MUST pass data integrity tests:
```bash
cargo test test_binary_protocol_precision
cargo test test_decimal_conversion
pytest tests/data_validation/
```

### Performance Regression Prevention
Check performance impact:
```bash
cargo bench --baseline master
python scripts/check_performance_regression.py
```

### Exchange-Specific Validation
Each exchange has unique formats:
- Kraken: Array format `[price, volume, time]`
- Coinbase: String decimals with variable precision
- Polygon DEX: Wei values requiring 18-decimal conversion

## Performance Monitoring

### Key Metrics
- **Message Processing**: Target <35μs per message
- **Throughput**: Minimum 10,000 messages/second
- **Memory Usage**: <50MB per service
- **WebSocket Latency**: <5ms to exchange

### Profiling Tools
```bash
# CPU profiling
cargo build --release
perf record -g ./target/release/exchange_collector
perf report

# Memory profiling
valgrind --tool=massif ./target/release/exchange_collector
ms_print massif.out.*

# Flamegraph
cargo flamegraph --bin exchange_collector
```

## Debugging Tips

### WebSocket Issues
```bash
# Enable debug logging
RUST_LOG=exchange_collector=debug,tungstenite=trace cargo run

# Monitor WebSocket health
websocat -v wss://stream.exchange.com
```

### Binary Protocol Issues
```python
# Inspect binary messages
from protocol import TradeMessage
msg_bytes = b'...'  # 48 bytes
trade = TradeMessage.from_bytes(msg_bytes)
print(f"Price: {trade.price / 1e8}")
print(f"Volume: {trade.volume / 1e8}")
print(f"Timestamp: {trade.timestamp_ns}")
```

### Data Flow Tracing
```bash
# Trace message through pipeline
tail -f logs/collector.log logs/relay.log logs/bridge.log | grep "msg_id"
```

## Emergency Procedures

### Service Crash Recovery
```bash
# Check service status
systemctl status alphapulse-*

# Restart individual service
systemctl restart alphapulse-collector

# Full system restart
./scripts/restart_all_services.sh
```

### Data Corruption Detection
```bash
# Run integrity checks
python scripts/validate_data_integrity.py --last-hour

# Compare exchange data with our pipeline
python scripts/compare_with_exchange.py --exchange kraken --duration 60
```

## Contributing Guidelines

### Before Making Changes
1. Read relevant CLAUDE.md files in subdirectories
2. Run existing tests to understand current behavior
3. Check for related issues or ongoing migrations
4. Update existing files instead of creating duplicates with adjective prefixes
5. Respect project structure - place files in their correct service directory

### Before Submitting PR
1. ✅ All tests passing (especially precision tests)
2. ✅ No performance regression
3. ✅ Documentation updated (including CLAUDE.md if needed)
4. ✅ Linting and formatting clean
5. ✅ Commit message follows convention
6. ✅ No duplicate files with "enhanced", "fixed", "new", "v2" prefixes
7. ✅ Files placed in correct service directories per project structure

## AI Assistant Tips

When working with this codebase:
1. **Quality First**: Never rush to complete tasks - build robust, validated solutions
2. Always prioritize data integrity over performance
3. Test decimal precision for any numeric changes
4. Consider both hot path (<35μs) and warm path impacts
5. Remember the Symbol → Instrument migration is ongoing
6. Check service-specific CLAUDE.md files for detailed context
7. **No Shortcuts**: Take time to validate, test, and ensure safety even if it delays task completion

## Quick Reference

### File Locations
- Binary Protocol: `backend/protocol/src/lib.rs`
- Main Collector: `backend/services/exchange_collector/src/main.rs`
- FastAPI Backend: `backend/app_fastapi.py`
- Dashboard: `frontend/src/dashboard/components/`
- Tests: `backend/tests/` and `tests/`

### Key Configuration Files
- Rust Workspace: `Cargo.toml`
- Python Dependencies: `pyproject.toml`
- Frontend: `package.json`
- Docker: `docker-compose.yml`

### Important Scripts
- `scripts/start-polygon-only.sh` - Start all services
- `scripts/monitor_connections.sh` - Monitor health
- `scripts/check_performance.py` - Performance validation
- `scripts/migrate_symbol_to_instrument.py` - Migration tool

## Contact for Complex Issues
For architectural decisions or breaking changes, review:
- System design docs in `docs/architecture/`
- Performance requirements in `docs/performance/`
- Migration plans in `projects/system-cleanup/`