---
status: COMPLETE
priority: HIGH
assigned_branch: feat/binary-entry-points
created: 2025-08-26
estimated_hours: 3
depends_on: TASK-002, TASK-003
completed: 2025-08-26
---

# TASK-004: Update Binary Entry Points to Use New Architecture

**Branch**: `feat/binary-entry-points`  
**NEVER WORK ON MAIN**

## Git Enforcement
```bash
# MANDATORY: Verify you're not on main before starting
if [ "$(git branch --show-current)" = "main" ]; then
    echo "❌ NEVER WORK ON MAIN BRANCH!"
    echo "Run: git checkout -b feat/binary-entry-points"
    exit 1
fi

# Create feature branch from domain-implementations
git checkout feat/domain-implementations  # Start from TASK-003 branch
git checkout -b feat/binary-entry-points
git branch --show-current  # Should show: feat/binary-entry-points
```

## Problem Statement
With the generic `Relay<T>` engine and domain-specific `RelayLogic` implementations complete, we need to update the three binary entry points to use the new architecture instead of their current duplicated main.rs implementations.

**Current State**: 3 separate main.rs files with 290+ lines of duplicated code  
**Target State**: 3 simple main.rs files that instantiate `Relay<DomainLogic>` and call `start()`

**Critical Requirement**: **100% backward compatibility** - existing scripts, deployment processes, and client connections must work identically.

## Acceptance Criteria
- [ ] **market_data_relay/src/main.rs** updated to use `Relay<MarketDataLogic>`
- [ ] **signal_relay/src/main.rs** updated to use `Relay<SignalLogic>`
- [ ] **execution_relay/src/main.rs** updated to use `Relay<ExecutionLogic>`
- [ ] **Cargo.toml files** updated with correct dependencies
- [ ] **Binary size reduction** - each main.rs should be <50 lines vs current 290+ lines
- [ ] **Functional equivalence** - all binaries work identically to current implementations
- [ ] **Performance preservation** - no measurable overhead from new architecture

## Technical Approach

### Binary Implementation Pattern
Each main.rs follows the same simple pattern:

```rust
//! [Domain] Relay - Generic + Trait Architecture
//! 
//! Uses Relay<DomainLogic> pattern to eliminate code duplication
//! while preserving exact functional behavior.

use alphapulse_relays::{Relay, DomainLogic};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging (same as before)
    tracing_subscriber::fmt::init();
    
    // Create domain-specific logic implementation  
    let domain_logic = DomainLogic::new();
    
    // Create and start generic relay engine
    let mut relay = Relay::new(domain_logic);
    relay.start().await
}
```

**Key Benefits**:
- **Massive reduction**: ~290 lines → ~15 lines per binary
- **Identical behavior**: Generic engine preserves exact functionality
- **Easy maintenance**: Bug fixes in one place benefit all relays

## Implementation Steps

### Step 1: Update Market Data Relay (1 hour)

**File**: `relays/market_data_relay/src/main.rs`

Replace current 290-line implementation with:

```rust
//! # Market Data Relay - Generic + Trait Architecture
//!
//! ## Purpose
//! High-performance bidirectional message forwarding hub for real-time market data.
//! Now using generic Relay<MarketDataLogic> to eliminate code duplication.
//!
//! ## Architecture Role
//!
//! ```mermaid
//! graph LR
//!     PP[polygon_publisher] -->|TLV Messages| Socket["/tmp/alphapulse/market_data.sock"]
//!     Socket --> Relay["Relay&lt;MarketDataLogic&gt;"]
//!     Relay -->|Broadcast| Dashboard[Dashboard Consumer]
//!     Relay -->|Broadcast| Strategy[Strategy Services]
//!
//!     subgraph "Generic Engine + Domain Logic"
//!         Relay --> Engine[Generic Relay Engine]
//!         Relay --> Logic[MarketDataLogic]
//!         Engine --> Logic
//!     end
//!
//!     classDef refactored fill:#90EE90
//!     class Relay,Engine,Logic refactored
//! ```
//!
//! ## Performance Profile
//! - **Throughput**: >1M messages/second (preserved)
//! - **Latency**: <35μs forwarding per message (preserved)  
//! - **Memory**: 64KB buffer per connection (identical)
//! - **Connections**: 1000+ concurrent supported (identical)

use alphapulse_relays::{Relay, MarketDataLogic};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    info!("🚀 Starting Market Data Relay (Generic Architecture)");
    
    let logic = MarketDataLogic::new();
    let mut relay = Relay::new(logic);
    
    relay.start().await
}
```

**Update Cargo.toml**:
```toml
[dependencies]
alphapulse-relays = { path = ".." }
tokio = { version = "1.0", features = ["full"] }
tracing = "0.1"
tracing-subscriber = "0.3"
```

### Step 2: Update Signal Relay (0.75 hours)

**File**: `relays/signal_relay/src/main.rs`

```rust
//! Production Signal Relay - Generic + Trait Architecture
//! 
//! Uses Relay<SignalLogic> for consumer tracking and Signal domain TLV validation

use alphapulse_relays::{Relay, SignalLogic};

#[tokio::main]  
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    info!("🚀 Starting Signal Relay (Generic Architecture)");
    info!("📋 Signal domain: TLV types 20-39, consumer tracking enabled");
    
    let logic = SignalLogic::new();
    let mut relay = Relay::new(logic);
    
    relay.start().await
}
```

### Step 3: Update Execution Relay (0.75 hours)

**File**: `relays/execution_relay/src/main.rs`

```rust
//! Production Execution Relay - Generic + Trait Architecture
//!
//! Uses Relay<ExecutionLogic> for execution domain validation and enhanced security

use alphapulse_relays::{Relay, ExecutionLogic};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    info!("🚀 Starting Execution Relay (Generic Architecture)");
    info!("📋 Execution domain: TLV types 40-79, enhanced validation enabled");
    
    let logic = ExecutionLogic::new();
    let mut relay = Relay::new(logic);
    
    relay.start().await
}
```

### Step 4: Update Dependencies and Build Configuration (0.5 hours)

**Update each relay's Cargo.toml**:
- Add dependency on parent relays crate
- Remove duplicated dependencies that are now handled by generic engine
- Ensure tokio features are correctly specified

**Verify build process**:
```bash
# Each relay should build successfully
cargo build --release -p alphapulse-relays --bin market_data_relay
cargo build --release -p alphapulse-relays --bin signal_relay  
cargo build --release -p alphapulse-relays --bin execution_relay
```

## Files to Modify

### MODIFY
- `relays/market_data_relay/src/main.rs` - Replace with generic implementation
- `relays/signal_relay/src/main.rs` - Replace with generic implementation  
- `relays/execution_relay/src/main.rs` - Replace with generic implementation
- `relays/market_data_relay/Cargo.toml` - Update dependencies
- `relays/signal_relay/Cargo.toml` - Update dependencies
- `relays/execution_relay/Cargo.toml` - Update dependencies

## Backward Compatibility Validation

### Command Line Interface  
All existing commands must work identically:
```bash
# Same commands as before
cargo run --release -p alphapulse-relays --bin market_data_relay
cargo run --release -p alphapulse-relays --bin signal_relay
cargo run --release -p alphapulse-relays --bin execution_relay
```

### Socket Paths
Must remain identical:
- Market Data: `/tmp/alphapulse/market_data.sock`
- Signals: `/tmp/alphapulse/signals.sock`
- Execution: `/tmp/alphapulse/execution.sock`

### Protocol Compatibility
- Same Protocol V2 TLV message handling
- Identical Unix socket behavior
- Same connection handling patterns

### Logging Compatibility
- Same log levels and formats
- Same performance metrics logging  
- Same connection establishment messages

## Testing Strategy

### Functional Equivalence Testing
```bash
# Side-by-side comparison
./test_scripts/compare_relay_behavior.sh market_data
./test_scripts/compare_relay_behavior.sh signal
./test_scripts/compare_relay_behavior.sh execution
```

### Integration Testing
```bash
# Full system integration
cargo run --release --bin polygon_publisher &
cargo run --release -p alphapulse-relays --bin market_data_relay &
cargo run --release -p alphapulse-dashboard-websocket -- --port 8080 &

# Verify data flow end-to-end
curl http://localhost:8080/health
```

### Performance Regression Testing
```bash
# Before/after performance comparison
cargo run --release --bin relay_throughput_test -- --relay market_data
cargo run --release --bin relay_latency_test -- --relay signal
```

## Success Metrics
- [ ] All three binaries compile and run successfully  
- [ ] Binary size reduced by >80% (from ~290 lines to ~15 lines per main.rs)
- [ ] Functional equivalence tests pass
- [ ] Integration tests pass with polygon_publisher + dashboard
- [ ] No performance regression measured
- [ ] Same logging output and connection behavior

## Risk Mitigation

### Deployment Risk
**Mitigation**: Parallel deployment validation
- Test new binaries in staging environment
- Run side-by-side with existing binaries
- Validate with real traffic patterns

### Dependency Risk
**Mitigation**: Careful dependency management  
- Ensure all required dependencies are properly specified
- Test build process from clean state
- Validate runtime dependencies are available

### Configuration Risk
**Mitigation**: Preserve all existing configuration
- Same socket paths and permissions
- Same logging configuration
- Same performance tuning parameters

## Performance Validation Requirements

### Zero Overhead Goal
Generic architecture should add no measurable overhead:
- **Throughput**: Maintain >1M msg/s 
- **Latency**: Maintain <35μs per message
- **Memory**: Same 64KB buffers per connection
- **CPU**: No additional processing overhead

### Validation Commands
```bash
# Performance regression detection
cargo bench --baseline original
hyperfine --warmup 3 \
  './old_market_data_relay' \
  './new_market_data_relay'
```

## Next Task Dependencies
This task **BLOCKS**:
- TASK-005 (Performance Validation) - needs updated binaries for testing
- TASK-006 (Migration Testing) - needs new binaries for deployment validation

This task **DEPENDS ON**:
- TASK-002 (Generic Engine) - needs Relay<T> implementation
- TASK-003 (Domain Implementations) - needs MarketDataLogic, SignalLogic, ExecutionLogic

## Documentation Updates Required
- **Binary usage documentation** showing new simplified implementations
- **Architecture documentation** explaining Generic + Trait pattern benefits
- **Migration guide** for deployments using these binaries

---
**Estimated Completion**: 3 hours  
**Complexity**: Medium - straightforward refactoring with careful validation  
**Risk Level**: Medium - critical deployment compatibility required