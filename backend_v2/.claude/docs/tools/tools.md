# TOOLS.md - Torq Development Tools & Workflows

## Overview

This document contains practical development tools, commands, and workflows for working with the Torq codebase. For code style conventions, see [style.md](style.md). For AI assistant context and behavioral directives, see [../CLAUDE.md](../CLAUDE.md).

## Table of Contents

- [Rust Ecosystem Tools](#rust-ecosystem-tools)
- [Rust Observability & Analysis Tools](#rust-observability--analysis-tools)
- [Codebase Navigation with rq](#codebase-navigation-with-rq)
- [Running the System](#running-the-system)
- [Testing Commands](#testing-commands)
- [Code Quality Checks](#code-quality-checks)
- [Debugging](#debugging)
- [Performance Monitoring](#performance-monitoring)
- [Emergency Procedures](#emergency-procedures)
- [Development Workflows](#development-workflows)

## Rust Ecosystem Tools

Before building new analysis or tooling features, check if these existing tools already solve the problem:

### Code Analysis & Navigation
- **rust-analyzer**: Complete semantic analysis, cross-references, go-to-definition, call hierarchies, LSP integration
- **IDE extensions**: Most relationship discovery and code navigation is already solved in your editor

### Dependency & Compatibility Analysis  
- **cargo tree**: Dependency graphs and analysis
- **cargo-semver-checks**: API compatibility analysis, breaking change detection
- **cargo audit**: Security vulnerability analysis
- **cargo outdated**: Version compatibility and update analysis

### Code Transformation & Inspection
- **cargo expand**: Macro expansion for debugging
- **cargo clippy**: Linting and code improvement suggestions (already in use)
- **rustfmt**: Code formatting (already in use)

## Rust Observability & Analysis Tools

### Breaking Change Detection
```bash
# Install cargo-semver-checks - CRITICAL for Torq's breaking change philosophy
cargo install cargo-semver-checks

# Check if your changes would break downstream code
cargo semver-checks check-release

# Compare against specific version
cargo semver-checks check-release --baseline-version 1.0.0

# Check against git commits (useful before PRs)
cargo semver-checks check-release --baseline-rev main

# What it detects:
# - Removed public items
# - Changed function signatures  
# - Tightened trait bounds
# - Struct field visibility changes
# - Enum variant removals
```

### Public API Surface Analysis
```bash
# Install cargo-public-api
cargo install cargo-public-api

# List all public APIs (useful for understanding exposure)
cargo public-api

# Diff public APIs between versions
cargo public-api diff main

# Generate API changelog
cargo public-api changelog

# For Torq: Use before breaking changes to see full impact
cargo public-api --simplified  # Easier to read output
```

### Dependency Analysis & Security
```bash
# Visualize dependency tree
cargo tree                        # Basic tree view
cargo tree --inverse tokio       # What depends on tokio
cargo tree --duplicates          # Find duplicate dependencies
cargo tree --depth 1             # Direct dependencies only

# Find unused dependencies
cargo install cargo-udeps
cargo +nightly udeps             # Requires nightly

# Security audit
cargo install cargo-audit
cargo audit                      # Check for vulnerabilities
cargo audit fix                  # Auto-fix when possible

# Check outdated dependencies
cargo install cargo-outdated
cargo outdated --depth 1         # Show direct deps only
cargo outdated --workspace       # Check entire workspace

# Dependency graph visualization
cargo install cargo-depgraph
cargo depgraph | dot -Tpng > dependencies.png

# Detect unsafe code usage
cargo install cargo-geiger
cargo geiger                     # Show unsafe usage stats
cargo geiger --forbid-only       # Ensure no unsafe in your code
```

### Code Generation & Binary Analysis
```bash
# Macro expansion - see what code actually compiles
cargo install cargo-expand
cargo expand --lib tlv::types    # Expand specific module
cargo expand ::parse_message     # Expand specific function

# Binary size analysis
cargo install cargo-bloat
cargo bloat --release            # What's taking space
cargo bloat --release --crates   # Breakdown by crate
cargo bloat --release -n 10     # Top 10 largest functions

# LLVM codegen analysis
cargo install cargo-llvm-lines
cargo llvm-lines                # Which functions generate most code
cargo llvm-lines --lib          # Library only

# Assembly output inspection
cargo rustc --release -- --emit asm
cargo rustc --release -- --emit llvm-ir
```

### Testing & Coverage Analysis
```bash
# Code coverage with semantic understanding
cargo install cargo-tarpaulin
cargo tarpaulin --out Html       # Generate HTML report
cargo tarpaulin --branch         # Include branch coverage
cargo tarpaulin --ignore-tests   # Exclude test code from coverage

# Next-generation test runner
cargo install cargo-nextest
cargo nextest run                # Faster parallel execution
cargo nextest run --changed-since HEAD~5  # Test impact analysis

# Mutation testing - test your tests
cargo install cargo-mutants
cargo mutants                    # Mutate code to verify test quality
cargo mutants --file src/tlv/parser.rs  # Focus on specific file
```

### Performance Profiling & Benchmarking
```bash
# CPU profiling with perf (Linux)
cargo build --release
perf record -g ./target/release/exchange_collector
perf report

# Flamegraph generation
cargo install flamegraph
cargo flamegraph --bin exchange_collector

# Criterion benchmarks with history
cargo bench --bench tlv_parsing
cargo bench -- --save-baseline main  # Save for comparison
cargo bench -- --baseline main       # Compare against baseline

# Memory profiling with valgrind
valgrind --tool=massif ./target/release/exchange_collector
ms_print massif.out.*

# Cache performance
valgrind --tool=cachegrind ./target/release/exchange_collector
```

### Semantic Code Analysis
```bash
# rust-analyzer CLI tools
rust-analyzer analysis-stats .   # Workspace statistics
rust-analyzer diagnostics .      # All diagnostics
rust-analyzer ssr '($a:expr) + ($b:expr) ==>> add($a, $b)'  # Structural search & replace

# Find similar code patterns
cargo install cargo-duplicate
cargo duplicate                  # Find duplicate code blocks

# Complexity analysis
cargo install cargo-complexity
cargo complexity                 # Cyclomatic complexity metrics
```

## Codebase Navigation with rq

**rq** (Rust Query) is our simple semantic grep tool to find existing implementations before creating new ones:

### Basic Usage
```bash
# CRITICAL: Always check before implementing new functionality
rq check TradeTLV               # Verify it exists
rq check validate_pool          # Check if it doesn't exist
rq similar TradTLV              # Find similar with fuzzy matching

# Find existing implementations (basic patterns)
rq find Pool                    # All Pool-related code
rq find TLV --type struct       # All TLV structures
rq find parse --type function   # All parsing functions
rq find Pool --public           # Only public Pool APIs
```

### Advanced Regex Patterns
```bash
# Advanced pattern matching with regex
rq find "^Pool.*TLV$" --regex   # Pool TLV structs with exact pattern
rq find "Trade|Quote" --regex   # Multiple patterns (OR)
rq find "parse_.*_message" --regex  # All parse_*_message functions
rq find "handle.*[Ee]vent" --regex   # handle*Event or handle*event
rq find "^(get|set)_" --regex   # All getters and setters
rq find ".*Error$" --regex      # All error types
rq find "^[A-Z]{3,}.*" --regex  # Constants (uppercase prefixes)
rq find ".*_v[0-9]+" --regex    # Versioned functions (_v1, _v2, etc)
rq find "(impl|trait).*Pool" --regex  # Pool implementations and traits
rq find "async.*send" --regex   # Async send methods
rq find "test_.*arbitrage" --regex  # Arbitrage test functions
rq find "(?i)websocket" --regex  # Case-insensitive websocket search

# Complex regex patterns for specific needs
rq find "^(Market|Signal|Execution).*Relay$" --regex  # All relay types
rq find ".*TLV::<.*>" --regex   # Generic TLV implementations  
rq find "fn.*\(.*Config.*\)" --regex  # Functions taking Config params
rq find "Result<.*,.*Error>" --regex  # Result return types with errors
rq find "#\[derive\(.*Serialize.*\)\]" --regex  # Serializable structs
rq find "pub\((crate|super)\)" --regex  # Crate/super visibility items
```

### Relationships and Documentation
```bash
# Find usage examples and relationships
rq examples TradeTLV            # Show real usage from test files
rq callers execute_arbitrage    # What functions call this
rq calls TradeTLV               # What this calls (simplified)

# Search documentation and get stats
rq docs "liquidity"             # Search doc strings
rq stats                        # Cache statistics

# Update rustdoc cache when needed
rq update                       # Update all crates
rq update --force               # Force update
```

**IMPORTANT**: Running `rq check <name>` before implementing prevents code duplication and maintains our "One Canonical Source" principle.

## Running the System

### Quick Start
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

### Service Management
```bash
# Check service status
ps aux | grep torq

# Check relay connections
netstat -an | grep /tmp/torq

# Monitor message flow
nc -U /tmp/torq/market_data.sock | head -n 10

# Clean restart
pkill -f torq
rm -f /tmp/torq/*.sock
./scripts/start_all.sh
```

## Testing Commands

### Core Protocol Tests
```bash
# CRITICAL: Always run Protocol V2 tests before committing
cargo test --package protocol_v2 --test tlv_parsing
cargo test --package protocol_v2 --test precision_validation

# Protocol V2 performance validation
cargo run --bin test_protocol --release
```

### Full Test Suite
```bash
# Unit tests for all services
cargo test --workspace

# Integration tests with real data
cd services_v2/adapters
cargo test --test live_polygon_dex -- --nocapture

# Performance benchmarks (target: >1M msg/s construction, >1.6M msg/s parsing)
cargo bench --workspace

# TLV message validation tests  
cargo test --package protocol_v2

# Pool cache persistence tests
cargo test --package services_v2 pool_cache_manager

# Python tests
pytest tests/ -v --cov=backend
```

## Code Quality Checks

### Rust
```bash
# Formatting
cargo fmt --all -- --check

# Linting
cargo clippy --workspace -- -D warnings

# Documentation check
cargo doc --workspace --no-deps
cargo test --doc --workspace
```

### Python
```bash
# Formatting
black backend/ --check

# Linting
ruff check backend/ --fix

# Type checking
mypy backend/services/ --strict

# Test coverage
pytest tests/ --cov=backend --cov-report=html
```

## Debugging

### WebSocket Issues
```bash
# Enable debug logging
RUST_LOG=exchange_collector=debug,tungstenite=trace cargo run

# Monitor WebSocket health
websocat -v wss://stream.exchange.com

# Trace specific components
RUST_LOG=torq_adapters=trace cargo run --bin live_polygon_relay
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

### Data Flow Tracing
```bash
# Trace messages through relay domains by sequence number
tail -f logs/market_data_relay.log logs/signal_relay.log logs/execution_relay.log | grep "sequence"

# Debug TLV parsing issues
RUST_LOG=torq_protocol_v2::tlv=debug cargo run

# Monitor relay consumer connections
tail -f logs/relay_consumer_registry.log
```

## Performance Monitoring

### Achieved Performance Targets
- **Message Construction**: >1M msg/s (1,097,624 msg/s measured)
- **Message Parsing**: >1.6M msg/s (1,643,779 msg/s measured)
- **InstrumentId Operations**: >19M ops/s (19,796,915 ops/s measured)
- **Memory Usage**: <50MB per service
- **Relay Throughput**: Tested with >1M msg/s sustained load

### Monitoring Metrics
- **Message Latency**: Target <35μs for market data (hot path)
- **Pool Discovery**: RPC calls queued, never block event processing
- **Cache Performance**: Hit rate >95% for known pools
- **Throughput**: >1M messages/second per relay
- **CPU Usage**: <25% per core under normal load
- **Pool Cache**: Background writes, atomic operations, crash recovery

### System Configuration for Performance
```bash
# Increase file descriptor limits
ulimit -n 65536

# Enable huge pages for shared memory
echo 1024 > /proc/sys/vm/nr_hugepages

# Pin services to CPU cores
taskset -c 0-3 cargo run --release --bin market_data_relay
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

### Common Issues & Solutions

| Issue | Cause | Solution |
|-------|-------|----------|
| "Connection refused" on socket | Relay not running | Start relay before collectors |
| High latency spikes | GC or allocation | Use `--release` builds |
| Missing events | Rate limiting | Use alternative RPC endpoints |
| Message corruption | Version mismatch | Rebuild all services |

## Development Workflows

### Before Making Breaking Changes
```bash
# 1. Understand current API surface
cargo public-api --simplified > before.txt

# 2. Check what would break
cargo semver-checks check-release --baseline-rev main

# 3. See dependency impact
cargo tree --inverse <crate-name>

# 4. Make changes...

# 5. Verify changes are complete
cargo public-api --simplified > after.txt
diff before.txt after.txt

# 6. Ensure all references updated
rg "old_function_name" --type rust  # Should return nothing
```

### Performance Investigation
```bash
# 1. Profile current performance
cargo flamegraph --bin exchange_collector --features profiling

# 2. Check binary size impact
cargo bloat --release --crates > before_size.txt

# 3. Make optimization...

# 4. Compare binary size
cargo bloat --release --crates > after_size.txt
diff before_size.txt after_size.txt

# 5. Verify performance improvement
cargo bench -- --baseline main
```

### Security & Quality Audit
```bash
# Weekly security check
cargo audit
cargo geiger --forbid-only

# Monthly dependency review
cargo outdated --workspace
cargo tree --duplicates

# Before releases
cargo +nightly udeps
cargo tarpaulin --branch
cargo mutants --timeout 300
```

### Adding a New Service

1. Create service in appropriate layer:
   ```bash
   cd services_v2/strategies
   cargo new my_strategy --lib
   ```

2. Add to workspace:
   ```toml
   # services_v2/Cargo.toml
   members = ["strategies/my_strategy"]
   ```

3. Implement service traits:
   ```rust
   use torq_protocol::{TLVMessage, InputAdapter};
   
   impl InputAdapter for MyStrategy {
       async fn start(&mut self) -> Result<()> { ... }
       async fn stop(&mut self) -> Result<()> { ... }
   }
   ```

4. Connect to relay:
   ```rust
   let socket = UnixStream::connect("/tmp/torq/market_data.sock").await?;
   ```

### Protocol Changes

Changes to the binary protocol require coordinated updates:

1. Update protocol definitions in `protocol_v2/src/messages.rs`
2. Increment version number in `protocol_v2/Cargo.toml`
3. Rebuild and test all dependent services
4. Update documentation in `protocol_v2/docs/`

## Tool Installation Summary

### Essential Tools
```bash
# Breaking change detection
cargo install cargo-semver-checks

# Security and dependencies
cargo install cargo-audit
cargo install cargo-outdated

# Performance analysis
cargo install flamegraph
cargo install cargo-bloat

# Testing
cargo install cargo-nextest
cargo install cargo-tarpaulin

# Code quality
cargo install cargo-geiger
```

### Advanced Tools
```bash
# Requires nightly
rustup toolchain install nightly
cargo +nightly install cargo-udeps

# Optional analysis tools
cargo install cargo-public-api
cargo install cargo-expand
cargo install cargo-llvm-lines
cargo install cargo-mutants
cargo install cargo-duplicate
```

## See Also

- [STYLE.md](STYLE.md) - Code style conventions and patterns
- [CLAUDE.md](CLAUDE.md) - AI assistant behavioral directives
- [README.md](README.md) - Project overview and architecture