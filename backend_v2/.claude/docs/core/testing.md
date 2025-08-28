# Testing & Debugging Guide

## Core Testing Philosophy: Tests Are Specifications, Not Obstacles

**CRITICAL UNDERSTANDING**: Tests are not barriers to overcome - they are executable specifications that define what the system must do correctly. When a test fails, it's revealing a real problem that needs to be solved, not circumvented.

## Should Tests Be Written Before Code?

### The TDD Question for AI Agents

**Traditional TDD:** Write test → Watch it fail → Write code → Make it pass

**For AI Agents in Torq:** The answer is nuanced:

#### ✅ YES - Write Tests First When:
1. **Implementing well-defined specifications** (e.g., TLV protocol messages)
2. **Adding new features with clear requirements** (e.g., "must preserve 18 decimal precision")
3. **Fixing bugs** (write a failing test that reproduces the bug first)

#### ⚠️ MAYBE - Explore First, Then Test When:
1. **Integrating with new external systems** (explore the API, then codify expectations)
2. **Prototyping architectural changes** (experiment, then lock in behavior with tests)

#### ❌ BUT NEVER:
1. **Write tests that expect wrong behavior** just to make them pass
2. **Skip tests because "we'll add them later"** (you won't)
3. **Write meaningless tests** that don't actually validate correctness

### The Right Approach for Torq

```rust
// 1. FIRST: Define what correct behavior looks like
#[test]
fn test_weth_swap_preserves_precision() {
    // This test defines our REQUIREMENT
    let input_weth = 1_234_567_890_123_456_789_i64; // Exact wei amount
    let swap_result = process_swap(input_weth);
    assert_eq!(swap_result.amount, input_weth); // MUST preserve every wei
}

// 2. THEN: Implement to meet that requirement
pub fn process_swap(amount: i64) -> SwapResult {
    // Implementation that preserves precision
    SwapResult { amount } // No conversion, no loss
}
```

### When You Inherit Existing Code

If tests already exist, they are the **contract** you must honor:
1. **Run existing tests first** to understand requirements
2. **Never modify tests** to match your implementation
3. **If a test seems wrong**, investigate WHY it expects that behavior
4. **Add new tests** for edge cases you discover

## The Cardinal Sins of Testing (What NEVER To Do)

### 🚫 NEVER Modify Tests to Pass
```rust
// ❌ WRONG - Changing test to match broken implementation
#[test]
fn test_price_precision() {
    let price = calculate_price(100, 200);
    // assert_eq!(price, 50.0);  // Original expectation
    assert_eq!(price, 47.3);     // Changed to match bug - NEVER DO THIS
}

// ✅ CORRECT - Fix the implementation
fn calculate_price(amount: u64, quantity: u64) -> f64 {
    // Fix the calculation logic, not the test
    amount as f64 / quantity as f64  // Proper division
}
```

### 🚫 NEVER Use Mock/Dummy Data in Production Systems
```python
# ❌ WRONG - Using fake data to make tests pass
def get_exchange_price(symbol):
    # return fetch_real_price(symbol)  # Commented out because "too hard"
    return 45000.0  # Dummy value - DESTROYS SYSTEM INTEGRITY

# ✅ CORRECT - Handle real data complexity
def get_exchange_price(symbol):
    try:
        return fetch_real_price(symbol)
    except APIError as e:
        logger.error(f"Exchange API error for {symbol}: {e}")
        raise  # Let the system handle the real error
```

### 🚫 NEVER Comment Out "Troublesome" Code
```rust
// ❌ WRONG - Hiding problems instead of solving them
pub fn process_market_data(data: &[u8]) -> Result<MarketEvent> {
    let header = parse_header(data)?;
    // let validated_data = validate_tlv_structure(&data)?;  // Commented out - causes issues
    // Ok(MarketEvent::from_validated_data(validated_data))
    Ok(MarketEvent::default())  // Fake success - DANGEROUS
}

// ✅ CORRECT - Handle validation properly
pub fn process_market_data(data: &[u8]) -> Result<MarketEvent> {
    let header = parse_header(data)?;
    let validated_data = validate_tlv_structure(&data)
        .map_err(|e| {
            error!("TLV validation failed: {}", e);
            e
        })?;
    Ok(MarketEvent::from_validated_data(validated_data))
}
```

## Testing Philosophy

**Reference Documentation**: Before writing tests, consult `.claude/practices.md` for Torq-specific requirements (zero-copy, precision, TLV compliance), `.claude/principles.md` for engineering patterns that should be validated in tests, and `.claude/style.md` for test code organization and naming conventions.

### Real Data Only - NO MOCKS
- **NEVER** use mock data, mock services, or mocked responses
- **ALWAYS** use real exchange connections for testing
- **ALWAYS** test with actual market data and live price feeds
- **NO** simulation modes that fake exchange responses
- **NO** stubbed WebSocket connections or API responses

### Protocol V2 Integrity First
Every change MUST pass Protocol V2 validation:
```bash
# TLV parsing and structure validation
cargo test --package protocol_v2 --test tlv_parsing
cargo test --package protocol_v2 --test precision_validation

# Performance regression detection
cargo run --bin test_protocol --release
# Must maintain: >1M msg/s construction, >1.6M msg/s parsing

# Bijective ID validation
cargo test --package protocol_v2 --test instrument_id_bijection
```

### Performance Regression Prevention
Check performance impact:
```bash
cargo bench --baseline master
python scripts/check_performance_regression.py
```

### Exchange-Specific TLV Conversion
Each exchange requires proper precision handling:
- **Traditional Exchanges (Kraken, Coinbase)**: Array/string formats → 8-decimal fixed-point for USD prices (`* 100_000_000`)
- **DEX Protocols (Polygon, Ethereum)**: Wei values → preserve native token precision (18 decimals WETH, 6 USDC, etc.)
- **All exchanges**: Must use proper InstrumentId construction and TLVMessageBuilder

## Testing Commands
```bash
# CRITICAL: Always run Protocol V2 tests before committing
cargo test --package protocol_v2 --test tlv_parsing
cargo test --package protocol_v2 --test precision_validation

# Protocol V2 performance validation
cargo run --bin test_protocol --release

# Full test suite
cargo test --workspace
pytest tests/ -v --cov=backend

# Performance benchmarks (target: >1M msg/s construction, >1.6M msg/s parsing)
cargo bench --workspace

# TLV message validation tests  
cargo test --package protocol_v2

# Pool cache persistence tests
cargo test --package services_v2 pool_cache_manager
```

## Debugging Tips

### WebSocket Issues
```bash
# Enable debug logging
RUST_LOG=exchange_collector=debug,tungstenite=trace cargo run

# Monitor WebSocket health
websocat -v wss://stream.exchange.com
```

### TLV Message Debugging
```rust
// Inspect TLV messages with Protocol V2
use torq_protocol_v2::{parse_header, parse_tlv_extensions, TLVType};

// Parse message header (32 bytes)
let header = parse_header(&message_bytes)?;
println!("Domain: {}, Source: {}, Sequence: {}", 
         header.relay_domain, header.source, header.sequence);

// Parse TLV payload
let tlv_payload = &message_bytes[32..32 + header.payload_size as usize];
let tlvs = parse_tlv_extensions(tlv_payload)?;

// Debug specific TLV types
for tlv in tlvs {
    match TLVType::try_from(tlv.header.tlv_type) {
        Ok(TLVType::Trade) => println!("Found TradeTLV"),
        Ok(TLVType::SignalIdentity) => println!("Found SignalIdentityTLV"),
        _ => println!("Unknown TLV type: {}", tlv.header.tlv_type),
    }
}
```

### System Understanding with rq
```bash
# Discover system architecture and relationships
rq docs "architecture role"     # Component relationships and data flow
rq docs "integration points"    # How services connect and communicate
rq docs "message flow"          # End-to-end message processing

# Performance analysis and optimization
rq docs "performance profile"   # Measured metrics and bottlenecks
rq docs "hot path"              # Critical latency requirements
rq docs "zero-copy"             # Memory optimization techniques

# Error handling and recovery
rq docs "error handling"        # Comprehensive error strategies
rq docs "recovery protocol"     # Gap detection and repair
rq docs "validation"            # Input validation and integrity checks
```

### Data Flow Tracing - Protocol V2
```bash
# Trace messages through relay domains by sequence number
tail -f logs/market_data_relay.log logs/signal_relay.log logs/execution_relay.log | grep "sequence"

# Debug TLV parsing issues
RUST_LOG=torq_protocol_v2::tlv=debug cargo run

# Monitor relay consumer connections
tail -f logs/relay_consumer_registry.log
```

## Performance Monitoring - Protocol V2

### Achieved Performance (Measured)
- **Message Construction**: >1M msg/s (1,097,624 msg/s measured)
- **Message Parsing**: >1.6M msg/s (1,643,779 msg/s measured)
- **InstrumentId Operations**: >19M ops/s (19,796,915 ops/s measured)
- **Memory Usage**: <50MB per service
- **Relay Throughput**: Tested with >1M msg/s sustained load

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

## Emergency Procedures

### Service Crash Recovery
```bash
# Check service status
systemctl status torq-*

# Restart individual service
systemctl restart torq-collector

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

## Zero-Allocation Testing Infrastructure

### Automated Allocation Detection
Torq uses a custom global allocator wrapper to detect allocation regressions in hot paths:

```bash
# Run zero-allocation tests to ensure hot path performance
cargo test --package protocol_v2 --test zero_allocation_tests -- --nocapture
```

#### Key Features:
- **Global allocator tracking**: Counts allocations and bytes allocated during tests
- **Thread-local buffer verification**: Ensures buffers are reused without allocation
- **Performance targets**: Validates <100ns message construction, >1M msg/s throughput
- **Regression detection**: Automatically fails if allocations occur in hot paths

#### Usage Pattern:
```rust
// Use the assert_zero_allocations! macro to verify no allocations
assert_zero_allocations!("hot_path_operation", {
    with_hot_path_buffer(|buffer| {
        // Your hot path code here
        let size = build_message_into_buffer(buffer)?;
        Ok((size, size))
    })
});
```

#### What Gets Tested:
1. **Hot path buffer reuse**: Zero allocations after initial warmup
2. **Signal buffer operations**: Low-latency coordination messages
3. **Message construction**: TLV building with zero-copy builder
4. **Throughput targets**: >1M messages/second sustained

#### CI Integration:
```yaml
# In your CI pipeline, add:
- name: Verify Zero Allocations
  run: |
    cargo test --package protocol_v2 --test zero_allocation_tests
    # Fail build if any allocations detected in hot path
```

#### Performance Benchmarks:
```bash
# Run performance benchmarks (use --ignored flag)
cargo test --package protocol_v2 --test zero_allocation_tests -- --ignored --nocapture

# Expected output:
# Hot Path Throughput Benchmark:
#   Messages processed: 1000000
#   Throughput: >1,000,000 messages/second
#   Latency: <100 ns/message
```

## Code Quality Checks
```bash
# Rust
cargo fmt --all -- --check
cargo clippy --workspace -- -D warnings

# Python  
ruff check backend/ --fix
black backend/ --check
mypy backend/services/ --strict
```