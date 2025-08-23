# AlphaPulse Backend v2 - High-Performance Trading Infrastructure

## System Overview

AlphaPulse is a distributed, microservice-based trading system built for ultra-low latency and parallel processing. The system processes real-time events and exchange data through a TLV (Type-Length-Value) binary message format, achieving <35μs latency for critical operations while maintaining full 20-byte Ethereum addresses for direct smart contract execution capability.

## Prerequisites

### Required Software
- **Rust** 1.75+ (for core services)
- **Python** 3.10+ (for API layer and analytics)
- **Node.js** 18+ (for dashboard)
- **Unix-like OS** (Linux/macOS) for Unix socket support

### System Requirements
- **Memory**: 8GB minimum, 16GB recommended
- **CPU**: 4+ cores for parallel service execution
- **Network**: Stable internet for blockchain/exchange connections
- **Disk**: SSD recommended for relay message buffers

## Architecture Overview

### Core Design Principles
1. **Microservice Architecture**: Independent, single-responsibility services
2. **Protocol V2 TLV**: Variable-size TLV messages with 32-byte headers for predictable parsing
3. **Full Address Architecture**: Complete 20-byte Ethereum addresses for direct execution
4. **Pool Cache Persistence**: Background disk persistence never blocks hot path
5. **Zero-Copy Operations**: Memory-mapped I/O and direct buffer passing
6. **Transport Abstraction**: Unix sockets locally, TCP for distributed
7. **No Shared State**: Services communicate only via message passing

### Service Layers

```
┌─────────────────────────────────────────────────────────────┐
│                     External Data Sources                    │
│         (Blockchain Nodes, DEX Contracts, CEX APIs)         │
└─────────────────┬───────────────────────────────────────────┘
                  │
┌─────────────────▼───────────────────────────────────────────┐
│                    Adapter Layer (Rust)                     │
│  • Polygon DEX Collector  • Kraken Collector                │
│  • Coinbase Collector     • Binance Collector               │
│  Input: WebSocket/RPC     Output: TLV Messages              │
└─────────────────┬───────────────────────────────────────────┘
                  │ Protocol V2 TLV (32-byte header + variable payload)
┌─────────────────▼───────────────────────────────────────────┐
│                     Relay Layer (Rust)                      │
│  • Market Data Relay      • Signal Relay                    │
│  • Execution Relay        • Risk Relay                      │
│  Transport: Unix Sockets  Routing: Topic-based Pub/Sub      │
└─────────────────┬───────────────────────────────────────────┘
                  │
┌─────────────────▼───────────────────────────────────────────┐
│                   Strategy Layer (Rust)                     │
│  • Flash Arbitrage Bot    • Market Making Strategy          │
│  • Statistical Arbitrage  • Liquidity Provider              │
│  Input: Market Data       Output: Trading Signals           │
└─────────────────┬───────────────────────────────────────────┘
                  │
┌─────────────────▼───────────────────────────────────────────┐
│                   Execution Layer (Rust)                    │
│  • Order Router           • Position Manager                │
│  • Risk Manager           • Settlement Engine               │
│  Input: Signals           Output: Transactions              │
└─────────────────┬───────────────────────────────────────────┘
                  │
┌─────────────────▼───────────────────────────────────────────┐
│                      API Layer (Python)                     │
│  • FastAPI REST Endpoints • WebSocket Bridge                │
│  • Metrics Aggregation    • Historical Data                 │
│  Input: Binary Protocol   Output: JSON/WebSocket            │
└─────────────────┬───────────────────────────────────────────┘
                  │
┌─────────────────▼───────────────────────────────────────────┐
│                    Dashboard (React/TypeScript)             │
│  • Real-time Visualization • Performance Metrics            │
│  • Trading Analytics       • System Monitoring              │
└─────────────────────────────────────────────────────────────┘
```

### Message Flow Example

```
1. Uniswap Swap Event on Polygon
   ↓
2. Polygon DEX Collector receives via WebSocket
   ↓
3. Converts to PoolSwapTLV with full addresses (102 bytes)
   ↓
4. Sends to Market Data Relay via Unix socket
   ↓
5. Relay broadcasts to subscribed strategies
   ↓
6. Flash Arbitrage detects opportunity
   ↓
7. Sends signal to Execution Coordinator
   ↓
8. Executes on-chain arbitrage transaction
```

## Workspace Structure

**Single Workspace Architecture**: `backend_v2/` is the unified Rust workspace managing all 15+ crates for the HFT framework.

```
backend_v2/Cargo.toml         ← SINGLE WORKSPACE ROOT
├── protocol_v2/              ← Member crate  
├── services_v2/adapters/     ← Member crate (NO nested workspace)
├── libs/state/core/          ← Member crate (NO nested workspace)
├── network/transport/        ← Member crate
└── relays/                   ← Member crate
```

**Key Principles:**
- ✅ **One workspace** manages all dependencies and versions
- ✅ **No virtual manifests** - removed conflicting Cargo.toml files
- ✅ **Unified builds** - `cargo build --workspace` compiles everything
- ✅ **Clean tooling** - `cargo check`, `rq update`, IDE support all work properly

**Benefits for HFT Framework:**
- **Performance testing**: `cargo bench --workspace` tests entire system
- **Dependency consistency**: No version conflicts between trading components  
- **Development workflow**: Single command builds, tests, and analyzes all services
- **Release coordination**: Atomic updates across all trading infrastructure

### Directory Structure

```
backend_v2/
├── protocol_v2/           # Protocol V2 TLV definitions (core)
│   ├── src/
│   │   ├── tlv/         # TLV message types and parsing
│   │   │   ├── market_data.rs    # PoolSwapTLV with full addresses
│   │   │   ├── pool_cache.rs     # Pool cache persistence TLVs
│   │   │   └── types.rs          # TLV type registry
│   │   ├── identifiers/ # Bijective InstrumentId system
│   │   └── validation/  # Message validation
│   └── tests/           # Protocol validation tests
│
├── libs/                 # Shared libraries (critical components)
│   ├── adapters/        # Adapter utilities (auth, circuit breaker, metrics, rate limiting)
│   ├── amm/             # AMM math libraries (optimal sizing, v2/v3 math, pool traits)
│   ├── execution/       # Execution utilities (executor, monitoring, slippage, venue)
│   ├── mev/             # MEV protection (bundle, flashbots, protection, searcher)
│   └── state/           # State management (core, execution, market, portfolio)
│
├── infra/                # Infrastructure layer
│   ├── transport/       # Network/IPC abstractions
│   └── topology/        # Service discovery and configuration
│
├── relays/              # Message routing layer
│   ├── src/
│   │   ├── relay.rs    # Generic relay implementation
│   │   └── topics.rs   # Topic-based routing
│   └── config/
│       ├── market_data.toml
│       ├── signal.toml
│       └── execution.toml
│
├── services_v2/         # Business logic services
│   ├── adapters/       # Data source adapters
│   │   ├── src/
│   │   │   ├── input/  # Exchange/blockchain collectors
│   │   │   └── output/ # External system publishers
│   │   └── tests/
│   │
│   ├── strategies/     # Trading strategies
│   │   ├── flash_arbitrage/
│   │   └── kraken_signals/
│   │
│   └── dashboard/      # WebSocket server for UI
│       └── websocket_server/
│
└── tests/              # Integration tests
    └── e2e/           # End-to-end scenarios

```

## Microservice Communication

### Protocol V2 TLV Format
All internal communication uses Type-Length-Value encoding:
- **32-byte MessageHeader** + variable TLV payload
- **Full 20-byte addresses** for direct smart contract execution
- **Native token precision** preserved (18 decimals WETH, 6 decimals USDC)
- **Nanosecond timestamps** for accurate sequencing
- **Zero-copy deserialization** in hot paths

#### Pool Cache Persistence System
Discovered DEX pool information is persisted to disk using binary TLV format:
- **Background writer thread** ensures hot path is never blocked
- **Atomic file operations** with CRC32 checksums prevent corruption
- **Journal-based recovery** for crash resilience
- **Memory-mapped loading** for fast startup
- **Pool discovery via RPC** with local caching for performance

#### TLV Message Examples
```rust
// Pool swap with full addresses for execution
pub struct PoolSwapTLV {
    pub venue: VenueId,
    pub pool_address: [u8; 20],      // Full pool contract address
    pub token_in_addr: [u8; 20],     // Full input token address
    pub token_out_addr: [u8; 20],    // Full output token address
    pub amount_in: i64,              // Native precision (no scaling)
    pub amount_out: i64,             // Native precision (no scaling)
    pub amount_in_decimals: u8,      // Token decimals metadata
    pub amount_out_decimals: u8,     // Token decimals metadata
    // ... V3 state fields, timestamps
}

// Pool cache persistence record
pub struct PoolInfoTLV {
    pub pool_address: [u8; 20],      // Full pool contract address
    pub token0_address: [u8; 20],    // Full token0 address
    pub token1_address: [u8; 20],    // Full token1 address
    pub token0_decimals: u8,         // Native decimals
    pub token1_decimals: u8,         // Native decimals
    pub pool_type: PoolType,         // UniswapV2, UniswapV3, etc.
    pub fee_tier: u32,               // Fee in basis points
    // ... venue, discovery timestamps
}
```

### Transport Mechanisms

| Transport | Use Case | Latency | Throughput |
|-----------|----------|---------|------------|
| Unix Socket | Local IPC | <1μs | >10M msg/s |
| Shared Memory | Same NUMA node | <100ns | >100M msg/s |
| TCP | Cross-machine | <1ms | >1M msg/s |
| WebSocket | External APIs | <10ms | >100K msg/s |

### Service Discovery
Services connect via well-known Unix socket paths:
- `/tmp/alphapulse/market_data.sock` - Market data relay
- `/tmp/alphapulse/signals.sock` - Trading signals relay
- `/tmp/alphapulse/execution.sock` - Execution relay

## Actor-Node Deployment Architecture

### Overview

AlphaPulse uses a two-layer deployment model: **logical actors** define service contracts and data flow, while **physical nodes** specify hardware placement and transport optimization.

## Actor Specification (Logical Layer)

Defines service contracts independent of deployment topology:

```yaml
# actors.yaml
actors:
  polygon_collector:
    type: producer
    outputs: [market_data]
    source_id: 1
    
  arbitrage_strategy:
    type: transformer
    inputs: [market_data]
    outputs: [signals]
    source_id: 20
    
  execution_coordinator:
    type: consumer
    inputs: [signals]
    source_id: 40
```

**Purpose**: Development contracts, testing, service discovery

## Node Graph (Physical Layer)

Maps actors to hardware with transport-specific optimizations:

```yaml
# nodes.yaml
nodes:
  trading_primary:
    hostname: "trade-01"
    numa_topology: [0, 1]
    
    # Intra-node: shared memory
    local_channels:
      market_data:
        type: SPMC
        buffer_size: "1GB"
        numa_node: 0
        huge_pages: true
        
    # Actor placement
    actors:
      polygon_collector: {numa: 0, cpu: [0,1]}
      arbitrage_strategy: {numa: 0, cpu: [2,3]}
      execution_coordinator: {numa: 1, cpu: [8,9]}
      
  analytics_cluster:
    hostname: "analytics-01"
    actors:
      risk_monitor: {cpu: [0,1]}
      
# Inter-node: network transport
inter_node:
  market_data_feed:
    source: trading_primary.market_data
    targets: [analytics_cluster]
    transport: tcp
    compression: lz4
```

**Purpose**: Hardware optimization, NUMA placement, transport selection

## Transport Resolution

The deployment engine resolves transport based on actor placement:

```rust
fn resolve_transport(source_actor: &Actor, target_actor: &Actor) -> Transport {
    match (source_actor.node_id, target_actor.node_id) {
        (a, b) if a == b => Transport::SharedMemory,  // Same node
        (a, b) => Transport::Network(tcp_config),     // Different nodes
    }
}
```

## Migration Strategy

**Phase 1**: Single-node deployment, all shared memory
**Phase 2**: Multi-node with explicit inter-node channels  
**Phase 3**: Dynamic actor migration and load balancing

## Benefits

- **Logical**: Clean service contracts, testable in isolation
- **Physical**: NUMA-aware, transport-optimized, hardware-specific
- **Deployment**: Infrastructure-as-code, reproducible environments
- **Performance**: Microsecond IPC where possible, efficient network where required

## Interactive Documentation

**View all code documentation**: Use `cargo doc --open` commands below to browse interactive, always-current API documentation.

### Complete Rustdoc Navigation

#### Core Services & Strategies
```bash
cd services_v2
cargo doc --workspace --open
```
**Covers**: Adapters (Coinbase, Polygon DEX), Flash Arbitrage Strategy, Kraken Signals, Dashboard WebSocket Server

#### Core Protocol & Infrastructure
```bash
# Protocol V2 - TLV message system, InstrumentId, validation
cd protocol_v2 && cargo doc --open

# Message routing and relay system
cd relays && cargo doc --open

# Service deployment and topology management  
cd network/topology && cargo doc --open

# Transport abstraction (Unix sockets → message bus migration)
cd network/transport && cargo doc --open
```

#### State Management Libraries
```bash
cd libs/state && cargo doc --workspace --open
```
**Covers**: Core state primitives, Market state (pool cache), Execution state, Portfolio state

#### Math & Utility Libraries
```bash
# AMM math (optimal sizing, V2/V3 calculations, pool traits)
cd libs/amm && cargo doc --open

# MEV protection (flashbots, bundle creation, searcher protection)  
cd libs/mev && cargo doc --open
```

#### Integration Testing Framework
```bash
# End-to-end test scenarios and validation framework
cd tests/e2e && cargo doc --open
```

## Agent-Friendly Codebase Navigation

**Complete programmatic access to codebase architecture, types, and patterns for intelligent code exploration and duplication prevention.**

### Rustdoc JSON Format Guide

AlphaPulse uses Rust's native JSON documentation export for machine-readable API discovery. Understanding this format enables powerful codebase navigation.

#### JSON Structure Overview
```bash
# Generate JSON documentation
cargo +nightly rustdoc --lib -- --output-format json -Z unstable-options

# Top-level structure
jq '. | keys' target/doc/protocol_v2.json
# Output: ["crate_version", "external_crates", "format_version", "includes_private", "index", "paths", "root", "target"]
```

#### Core Data Structures

**1. Index**: Maps item IDs to complete definitions
```bash
jq '.index["125"]' target/doc/protocol_v2.json
# Returns complete TLVType enum definition with variants, docs, etc.
```

**2. Paths**: Maps item IDs to module hierarchy  
```bash
jq '.paths["125"]' target/doc/protocol_v2.json  
# Returns: {"crate_id": 0, "path": ["protocol_v2", "tlv", "types", "TLVType"], "kind": "enum"}
```

**3. Item Types**: Each item has an "inner" field indicating its type
- `struct` - Data structures
- `enum` - Enumerations with variants  
- `function` - Functions and methods
- `trait` - Trait definitions
- `impl` - Trait implementations
- `module` - Modules and sub-modules

#### Template Query Patterns

**Find All Items of a Type:**
```bash
# All structs with their file locations
jq '[.index | to_entries[] | select(.value.inner.struct) | {
  id: .key, 
  name: .value.name, 
  file: .value.span.filename
}]' target/doc/protocol_v2.json

# All public functions  
jq '[.index | to_entries[] | select(.value.inner.function and .value.visibility == "public") | {
  name: .value.name,
  file: .value.span.filename  
}]' target/doc/protocol_v2.json

# All traits
jq '[.index | to_entries[] | select(.value.inner.trait) | {
  name: .value.name,
  file: .value.span.filename
}]' target/doc/protocol_v2.json
```

**Find By Name Pattern:**
```bash  
# All Pool-related types
jq '[.index | to_entries[] | select(.value.name | test(".*Pool.*")) | {
  type: (.value.inner | keys[0]),
  name: .value.name,
  file: .value.span.filename
}]' target/doc/protocol_v2.json

# All parsing functions
jq '[.index | to_entries[] | select(.value.name | test(".*parse.*"; "i")) | {
  name: .value.name,
  file: .value.span.filename
}]' target/doc/protocol_v2.json
```

**Module Structure Navigation:**
```bash
# All modules with their hierarchy
jq '[.index | to_entries[] | select(.value.inner.module) | {
  name: .value.name,
  file: .value.span.filename,
  path: .paths[.key].path
}]' target/doc/protocol_v2.json

# Find items in specific module
jq '[.paths | to_entries[] | select(.value.path | contains(["tlv"])) | .key] as $tlv_ids |
    [.index | to_entries[] | select(.key | IN($tlv_ids[])) | {
      name: .value.name,
      type: (.value.inner | keys[0])
    }]' target/doc/protocol_v2.json
```

### Ready-to-Use Navigation Functions

#### Complete Navigation Setup
```bash
# Load all navigation functions
source protocol_v2/.bashrc_nav        # Single-crate detailed analysis  
source .bashrc_workspace              # Multi-crate workspace analysis
source .bashrc_query_builder          # Interactive query construction

# Verify setup
nav_help                              # Single-crate functions
workspace_help                        # Workspace functions  
query_builder_help                    # Interactive query tools
```

#### Single-Crate Analysis (`.bashrc_nav`)
```bash
# Core architecture discovery
show_all_structs      # All data structures
show_all_traits       # All trait definitions
show_all_modules      # Module organization  
show_public_api       # Public functions only

# Code duplication prevention
find_similar "parse"  # Functions with similar names
find_by_type "Pool"   # All Pool-related types
find_in_module "tlv"  # Everything in tlv module

# Cross-reference analysis  
show_dependencies     # External crate usage
show_implementations  # Trait implementations

# Development helpers
check_for_duplicates "validator"
suggest_existing_impl "parse_address" 
```

#### Multi-Crate Workspace Analysis (`.bashrc_workspace`)
```bash
# Workspace-wide pattern discovery
find_workspace_pattern "TLV"          # All TLV-related code across crates
find_workspace_pattern "Adapter"      # All adapters across services
analyze_workspace_deps                # Cross-crate dependencies

# Architecture mapping
map_service_architecture              # Complete system overview
show_protocol_interfaces              # Key integration points

# Agent onboarding helpers
search_existing "price_calculator"    # Before writing new code
show_adapter_examples                 # Learn from existing adapters
show_strategy_examples                # Strategy implementation patterns
```

#### Interactive Query Construction (`.bashrc_query_builder`)
```bash
# Flexible search with multiple filters
code_search --type struct --name "Pool.*"                    # Pool-related structs
code_search --type function --visibility public              # Public API functions
code_search --module tlv --type enum                        # TLV module enums

# Architecture analysis
architecture_map --service adapters                         # Adapter service focus
architecture_map --service protocol --show interfaces       # Protocol with APIs

# Implementation discovery
find_implementations "validate" "address"                   # Validation patterns
trace_data_flow "PoolSwap"                                  # Message flow analysis
```

### Multi-Crate Workspace Navigation

**Generate JSON for entire workspace:**
```bash
# Generate docs for all crates
cd services_v2 && cargo +nightly doc --workspace --output-format json -Z unstable-options
cd protocol_v2 && cargo +nightly doc --output-format json -Z unstable-options  
cd infra && cargo +nightly doc --workspace --output-format json -Z unstable-options
```

**Cross-crate analysis:**
```bash
# Find all adapters across services
find_workspace_pattern "Adapter|Collector"

# Show service dependencies
analyze_workspace_deps

# Find cross-service message types
find_shared_protocols
```

### Agent Onboarding Patterns

**Before writing new code:**
```bash
# 1. Check for existing implementations
search_existing "price_calculator" 
search_existing "pool_validator"

# 2. Find integration patterns
show_adapter_examples     # How other adapters work
show_strategy_examples    # How strategies are built
show_validation_patterns  # Common validation code

# 3. Understand data flow  
trace_message_types "PoolSwap"  # Where this type flows
show_tlv_consumers "Trade"      # What consumes this message
```

**Understanding system architecture:**
```bash
# Service layer overview
map_service_architecture

# Protocol boundaries  
show_protocol_interfaces

# State management patterns
analyze_state_patterns
```

### Performance & Best Practices

- **JSON Generation**: Only regenerate when source changes (check file timestamps)
- **Query Caching**: Store complex query results for repeated use  
- **Incremental Discovery**: Start with high-level structure, drill down as needed
- **Cross-Reference**: Use both `index` and `paths` for complete picture

**Result**: Agents can intelligently explore the codebase, understand architecture, find existing implementations, and avoid code duplication through systematic API discovery.

### Quick API Discovery

| Need Documentation For | Command |
|------------------------|---------|
| **Building new CEX adapter** | `cd services_v2 && cargo doc -p alphapulse-adapter-service --open` → See `CoinbaseCollector` |
| **Building new DEX adapter** | `cd services_v2 && cargo doc -p alphapulse-adapter-service --open` → See `PolygonDEXCollector` |
| **Creating TLV messages** | `cd protocol_v2 && cargo doc --open` → See `TLVMessageBuilder` |
| **InstrumentId creation** | `cd protocol_v2 && cargo doc --open` → See `InstrumentId` methods |
| **Validation framework** | `cd services_v2 && cargo doc -p alphapulse-adapter-service --open` → See `validation` module |
| **AMM math calculations** | `cd libs/amm && cargo doc --open` → See pool calculation functions |
| **Strategy implementation** | `cd services_v2 && cargo doc -p alphapulse-flash-arbitrage --open` |

### Benefits of Rustdoc Architecture
- **Always Current**: Documentation auto-updates with code changes
- **Interactive**: Searchable with cross-references between types  
- **Complete Examples**: Runnable code samples embedded in docs
- **Zero Maintenance**: No separate documentation to keep in sync

## Getting Started

### Quick Start

```bash
# 1. Build all services
cd backend_v2
cargo build --workspace --release

# 2. Start core infrastructure (in separate terminals)
# Terminal 1: Start Market Data Relay
cd protocol_v2
cargo run --release --bin market_data_relay -- --socket /tmp/alphapulse/market_data.sock

# Terminal 2: Start Polygon DEX Collector
cd services_v2
cargo run --release --bin live_polygon_relay

# Terminal 3: Start Flash Arbitrage Strategy
cd services_v2/strategies/flash_arbitrage
cargo run --release

# 4. Monitor system (optional)
tail -f /tmp/alphapulse/logs/*.log
```

### Running Tests

```bash
# Unit tests for all services
cargo test --workspace

# Integration tests with real data
cd services_v2/adapters
cargo test --test live_polygon_dex -- --nocapture

# Performance benchmarks
cargo bench --workspace
```

### Common Operations

#### Check Service Health
```bash
# List running services
ps aux | grep alphapulse

# Check relay connections
netstat -an | grep /tmp/alphapulse

# Monitor message flow
nc -U /tmp/alphapulse/market_data.sock | head -n 10
```

#### Debug Message Flow
```bash
# Enable debug logging
RUST_LOG=debug cargo run --bin market_data_relay

# Trace specific components
RUST_LOG=alphapulse_adapters=trace cargo run --bin live_polygon_relay
```

#### Clean Restart
```bash
# Stop all services
pkill -f alphapulse

# Clean sockets
rm -f /tmp/alphapulse/*.sock

# Restart with fresh state
./scripts/start_all.sh
```

## Performance Tuning

### System Configuration
```bash
# Increase file descriptor limits
ulimit -n 65536

# Enable huge pages for shared memory
echo 1024 > /proc/sys/vm/nr_hugepages

# Pin services to CPU cores
taskset -c 0-3 cargo run --release --bin market_data_relay
```

### Monitoring Metrics
- **Message Latency**: Target <35μs for market data (hot path)
- **Pool Discovery**: RPC calls queued, never block event processing
- **Cache Performance**: Hit rate >95% for known pools
- **Throughput**: >1M messages/second per relay
- **Memory Usage**: <50MB per service
- **CPU Usage**: <25% per core under normal load
- **Pool Cache**: Background writes, atomic operations, crash recovery

## Troubleshooting

### Common Issues

| Issue | Cause | Solution |
|-------|-------|----------|
| "Connection refused" on socket | Relay not running | Start relay before collectors |
| High latency spikes | GC or allocation | Use `--release` builds |
| Missing events | Rate limiting | Use alternative RPC endpoints |
| Message corruption | Version mismatch | Rebuild all services |

### Debug Commands

```bash
# Check protocol version compatibility
cargo tree -p alphapulse-protocol

# Verify TLV message integrity
cd protocol_v2
cargo test test_message_roundtrip

# Test relay connectivity
echo -e '\x00' | nc -U /tmp/alphapulse/market_data.sock
```

## Development Workflow

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
   use alphapulse_protocol::{TLVMessage, InputAdapter};
   
   impl InputAdapter for MyStrategy {
       async fn start(&mut self) -> Result<()> { ... }
       async fn stop(&mut self) -> Result<()> { ... }
   }
   ```

4. Connect to relay:
   ```rust
   let socket = UnixStream::connect("/tmp/alphapulse/market_data.sock").await?;
   ```

### Protocol Changes

Changes to the binary protocol require coordinated updates:

1. Update protocol definitions in `protocol_v2/src/messages.rs`
2. Increment version number in `protocol_v2/Cargo.toml`
3. Rebuild and test all dependent services
4. Update documentation in `protocol_v2/docs/`

## Documentation with Rustdoc

### Interactive Code Documentation

AlphaPulse uses **rustdoc** as the primary documentation system for self-updating, interactive code documentation. This eliminates stale documentation and provides instant API reference.

### Quick Start: Viewing Documentation

```bash
# View complete documentation for specific crates
cd backend_v2/services_v2/adapters
cargo doc --package alphapulse-adapter-service --open

# View Protocol V2 documentation  
cd backend_v2/protocol_v2
cargo doc --package alphapulse_protocol_v2 --open

# Generate docs for all workspace members
cd backend_v2
cargo doc --workspace --open
```

**Key Benefits:**
- **Auto-updating**: Documentation always matches current code
- **Interactive**: Searchable with cross-references
- **API Discovery**: Browse all public types and methods
- **Examples**: Runnable code samples embedded in docs

### Essential Documentation Locations

| Component | Command | Key Content |
|-----------|---------|-------------|
| **Protocol V2** | `cargo doc -p alphapulse_protocol_v2 --open` | InstrumentId creation, TLV types, message building |
| **Adapters** | `cargo doc -p alphapulse-adapter-service --open` | Exchange collectors, validation framework |
| **Flash Arbitrage** | `cargo doc -p alphapulse-flash-arbitrage --open` | Strategy implementation, MEV protection |
| **AMM Math** | `cargo doc -p alphapulse-amm --open` | Pool calculations, optimal sizing |

### Rustdoc Best Practices for Contributors

#### 1. Module-Level Documentation
```rust
//! # Exchange Adapters
//! 
//! Stateless data transformers that convert external formats to TLV messages.
//! 
//! ## Quick Start
//! 
//! ```rust
//! use alphapulse_adapters::CoinbaseCollector;
//! let collector = CoinbaseCollector::new(/* ... */);
//! ```

pub mod adapters;
```

#### 2. Comprehensive Type Documentation
```rust
/// Coinbase WebSocket trade event adapter.
/// 
/// Converts Coinbase WebSocket trade messages to Protocol V2 TLV format.
/// Reference implementation for all CEX (centralized exchange) adapters.
/// 
/// # Example
/// 
/// ```rust
/// use alphapulse_adapters::CoinbaseCollector;
/// 
/// let collector = CoinbaseCollector::new(
///     vec!["BTC-USD".to_string()], 
///     output_channel
/// );
/// collector.start().await?;
/// ```
/// 
/// # Architecture
/// 
/// - **Input**: Coinbase WebSocket JSON messages
/// - **Output**: [`TradeTLV`] messages to market data relay
/// - **State**: Stateless transformer (no internal state)
/// 
/// # Performance
/// 
/// - **Latency**: <1ms event-to-TLV conversion
/// - **Throughput**: 10,000+ trades/second
/// - **Memory**: <64MB per collector instance
pub struct CoinbaseCollector { /* ... */ }
```

#### 3. Method Documentation with Examples
```rust
impl CoinbaseCollector {
    /// Creates a new Coinbase trade collector.
    /// 
    /// # Arguments
    /// 
    /// * `trading_pairs` - List of symbol pairs like "BTC-USD", "ETH-USD"
    /// * `output_tx` - Channel for sending TLV messages to relay
    /// 
    /// # Example
    /// 
    /// ```rust
    /// use tokio::sync::mpsc;
    /// use alphapulse_adapters::CoinbaseCollector;
    /// 
    /// let (tx, rx) = mpsc::channel(1000);
    /// let collector = CoinbaseCollector::new(
    ///     vec!["BTC-USD".to_string(), "ETH-USD".to_string()],
    ///     tx
    /// );
    /// ```
    /// 
    /// # Errors
    /// 
    /// Returns [`AdapterError::InvalidConfig`] if trading pairs are malformed.
    pub fn new(
        trading_pairs: Vec<String>, 
        output_tx: mpsc::Sender<TLVMessage>
    ) -> Result<Self, AdapterError> {
        // Implementation...
    }
}
```

### Advanced Rustdoc Features

#### Cross-References and Links
```rust
/// Processes incoming WebSocket messages.
/// 
/// Converts JSON trade data to [`TradeTLV`] format and forwards
/// to the configured relay via [`RelayOutput`].
/// 
/// See also:
/// - [`TradeTLV::from_coinbase_event`] for conversion details
/// - [`validation::complete_validation_pipeline`] for testing
/// - [Protocol V2 documentation](../protocol_v2/index.html)
pub async fn process_message(&self, json: &str) -> Result<()> {
    // ...
}
```

#### Code Examples with Tests
```rust
/// Converts Coinbase decimal strings to fixed-point integers.
/// 
/// # Example
/// 
/// ```rust
/// # use alphapulse_adapters::utils::parse_coinbase_decimal;
/// assert_eq!(parse_coinbase_decimal("45000.50", 8)?, 4500050000000);
/// assert_eq!(parse_coinbase_decimal("0.000123", 8)?, 12300);
/// ```
pub fn parse_coinbase_decimal(s: &str, decimals: u8) -> Result<i64> {
    // Implementation...
}
```

### JSON Export for AI/Tooling

#### Native Rust JSON Export (Nightly)

```bash
# Generate machine-readable JSON docs (requires nightly)
cargo +nightly rustdoc --lib -- --output-format json -Z unstable-options

# Install nightly if needed
rustup toolchain install nightly
```

#### Custom AI-Friendly Export

```bash
# Use our enhanced JSON generator
cd backend_v2/services_v2/adapters
./scripts/generate_rustdoc_json.sh

# Output: target/doc/json/api_index.json
```

The custom script provides:
- **Curated API index** with common methods
- **Type relationship mapping** 
- **Usage pattern examples**
- **Cross-package references**

### Documentation Maintenance Workflow

#### For New Features
1. **Write rustdoc first** - Document the API before implementing
2. **Include examples** - Show typical usage patterns
3. **Link related types** - Use `[Type]` syntax for cross-references
4. **Test examples** - Ensure code samples actually compile

#### For Bug Fixes
1. **Update affected docs** - Fix any outdated examples
2. **Add troubleshooting** - Document common issues and solutions
3. **Update error documentation** - Explain new error conditions

#### Quality Checklist
```bash
# Check for missing docs
cargo doc --workspace 2>&1 | grep "warning: missing documentation"

# Test all doc examples
cargo test --doc --workspace

# Verify links work
cargo doc --workspace --no-deps
```

### Troubleshooting Documentation Issues

| Issue | Solution |
|-------|----------|
| "Missing docs" warnings | Add `///` comments to public items |
| Broken cross-references | Check type names and module paths |
| Examples don't compile | Test with `cargo test --doc` |
| Large workspace build time | Use `--package` for specific crates |

### Integration with Development Workflow

#### Pre-commit Hook
```bash
#!/bin/bash
# Check documentation completeness
cargo doc --workspace --no-deps 2>&1 | grep -q "warning: missing documentation" && {
    echo "Missing documentation detected!"
    exit 1
}

# Test documentation examples
cargo test --doc --workspace || exit 1
```

#### IDE Integration
- **VSCode**: Install "rust-analyzer" for inline docs
- **IntelliJ**: Enable "Show quick documentation" hotkey
- **Vim/Neovim**: Use `:RustDoc` command

### Documentation Architecture Philosophy

**Self-Documenting Code**: Every public API should be understandable without external documentation.

**Examples Over Prose**: Show usage patterns rather than describing them.

**Auto-Updating**: Documentation that lives in code stays current automatically.

**Machine-Readable**: JSON export enables AI assistants and tooling integration.

## Contributing

See [CONTRIBUTING.md](../CONTRIBUTING.md) for development guidelines and [CLAUDE.md](../CLAUDE.md) for AI assistant context.
