# CLAUDE.md - AlphaPulse AI Assistant Context

## System Overview
AlphaPulse is a high-performance cryptocurrency trading system built on a sophisticated Protocol V2 TLV (Type-Length-Value) message architecture that processes >1M messages/second across domain-specific relays with complete precision preservation and bijective instrument identification.

**Core Mission**: Build a robust, validated, and safe trading infrastructure with complete transparency and zero tolerance for deceptive practices.

**Development Priority**: Quality over speed. Completing immediate tasks is NOT the highest priority - developing a well-organized, high-quality, robust/safe/validating system is. All work must be done with long-term reliability in mind. No shortcuts.

**Production-Ready Code**: ALWAYS write code as if it's going straight into production with real money. Never use fake/mock/dummy variables, services, or data. Every line of code must be production-quality from the start.

**Maintenance Reminder**: Regularly review `docs/MAINTENANCE.md` to ensure proper system maintenance, especially TLV type registry updates, precision validation, and performance benchmarks.

## Architecture Summary
```
Exchanges → Collectors (Rust) → Domain Relays → Consumers
         WebSocket         32-byte header +    Unix Socket/
                          Variable TLV payload  Message Bus
                          
Domain Relays:
├── MarketDataRelay (Types 1-19)   → Strategies, Portfolio, Dashboard
├── SignalRelay (Types 20-39)      → Portfolio, Dashboard, RiskManager  
└── ExecutionRelay (Types 40-79)   → Execution Engine, Dashboard
```

## Critical System Invariants - Protocol V2
1. **TLV Message Format**: MUST maintain 32-byte MessageHeader + variable TLV payload structure
2. **Full Address Architecture**: All DEX operations use complete 20-byte Ethereum addresses for direct execution
3. **Pool Cache Integrity**: Background persistence never blocks hot path, atomic file operations prevent corruption
4. **Zero Precision Loss**: Preserve native token precision (18 decimals WETH, 6 USDC) - no scaling or normalization
5. **No Deception**: Never hide failures, fake data, or simulate success - complete transparency required
6. **Domain Separation**: Respect relay domains (Market Data, Signals, Execution) and TLV type ranges
7. **Sequence Integrity**: Maintain monotonic per-source sequence numbers for gap detection
8. **Nanosecond Timestamps**: Never truncate to milliseconds
9. **Dynamic Configuration**: Use configurable values instead of hardcoded constants where adaptability is needed
10. **One Canonical Source**: Single implementation per concept - no "enhanced", "fixed", "new" duplicates
11. **Respect Project Structure**: Maintain service boundaries and established file hierarchy
12. **NO MOCKS EVER**: Never use mock data, mock services, or any form of mocked testing under any circumstances
13. **README-First Development**: Before creating new files or directories, update the containing directory's README.md to document purpose and prevent duplication
14. **Breaking Changes Welcome**: This is a greenfield codebase - make breaking changes freely to improve design, remove legacy patterns, and eliminate technical debt
15. **TLV Type Registry**: Never reuse TLV type numbers, always update expected_payload_size() when structs change
16. **Pool Discovery Workflow**: RPC calls for unknown pools must be queued/cached, never block WebSocket event processing

## Development Tools & Commands

**IMPORTANT: Consult these files during development:**
- **Before writing code**: Read [STYLE.md](STYLE.md) for conventions and patterns
- **For debugging, testing, workflows**: Reference [TOOLS.md](TOOLS.md) for procedures
- **When using tools**: Check [TOOLS.md](TOOLS.md) for proper usage and examples

Key tools to check before implementing new features:
- **rq**: `rq check <name>` to prevent code duplication
- **cargo-semver-checks**: Detect breaking changes before committing
- **rust-analyzer**: Semantic analysis and cross-references
- **cargo tree**: Dependency analysis and relationship discovery

Essential commands:
```bash
# Prevent duplication - ALWAYS run before coding
rq check TradeTLV               # Verify implementation exists
rq similar validate_pool        # Find similar functionality

# Test critical paths before committing
cargo test --package protocol_v2 --test tlv_parsing
cargo test --package protocol_v2 --test precision_validation

# Check breaking changes
cargo semver-checks check-release --baseline-rev main
```

## Project Structure - Protocol V2 Architecture
```
backend_v2/
├── protocol_v2/           # Protocol V2 TLV definitions (CRITICAL)
│   ├── src/tlv/          # TLV message types and parsing
│   ├── src/identifiers/  # Bijective InstrumentId system
│   └── tests/            # Protocol validation tests
├── libs/                 # Shared libraries (CRITICAL COMPONENTS)
│   ├── adapters/         # Adapter utilities (auth, circuit breaker, metrics, rate limiting)
│   ├── amm/              # AMM math libraries (optimal sizing, v2/v3 math, pool traits)
│   ├── execution/        # Execution utilities (executor, monitoring, slippage, venue)
│   ├── mev/              # MEV protection (bundle, flashbots, protection, searcher)
│   └── state/            # State management (core, execution, market, portfolio)
├── services_v2/          # Service implementations (RESPECT BOUNDARIES)
│   ├── adapters/         # Exchange collectors and input adapters
│   ├── strategies/       # Trading strategy implementations
│   └── dashboard/        # Dashboard and monitoring services
├── infra/                # Infrastructure layer
│   ├── topology/         # Service discovery and configuration
│   └── transport/        # Transport abstraction (unix socket → message bus)
├── relays/               # Domain-specific relay implementations
├── tests/e2e/           # End-to-end integration tests
└── docs/                # Protocol documentation and maintenance guides

Legacy (backend/):
├── services/            # Legacy services being migrated
├── protocol/           # Legacy binary protocol (deprecated)
├── api/               # Python FastAPI endpoints  
└── scripts/           # Utility scripts ONLY (no core logic)
```

**IMPORTANT**: Each service has a specific responsibility. Don't scatter related code across multiple locations or create files in the wrong directory hierarchy.

## Key Technical Decisions - Protocol V2

### Why TLV Message Format?
- 32-byte header + variable TLV payload for flexibility and performance
- Enables zero-copy operations with zerocopy traits
- Preserves appropriate precision per asset type: native precision for DEX tokens, 8-decimal fixed-point for USD prices
- Measured >1M msg/s construction, >1.6M msg/s parsing performance
- Forward compatibility through unknown TLV type graceful handling

### Why Bijective InstrumentIDs?
- Self-describing IDs eliminate need for centralized registries
- Deterministic construction prevents collisions
- Reversible to extract venue, asset type, and identifying data
- O(1) cache lookups using fast_hash conversion
- Works for all asset types: stocks, tokens, pools, options

### Why Domain-Specific Relays?
- Performance isolation: market data bursts don't affect execution
- Security: execution messages have stricter validation
- Clear separation: MarketData (1-19), Signals (20-39), Execution (40-79)
- Debugging: clear message flow tracing
- Future migration: direct mapping to message bus channels

### Why Rust for Core Infrastructure?
- No garbage collection pauses affecting >1M msg/s throughput
- Predictable performance characteristics for financial operations
- Memory safety without runtime overhead
- Zero-copy serialization with zerocopy crate

### Why Shared Libraries (`libs/`)?
- **Code Reuse**: Common functionality shared across services (AMM math, MEV protection, etc.)
- **Consistent Behavior**: Same algorithms used in strategies, validation, and execution
- **Performance**: Pre-compiled libraries avoid duplicate implementations
- **Modularity**: Clean separation between business logic (services) and utility functions (libs)

## Common Pitfalls & Solutions

### ❌ DON'T: Use Floating Point for Prices
```rust
// WRONG - Precision loss!
let price: f64 = 0.12345678;
```

### ✅ DO: Use Appropriate Precision per Asset Type
```rust
// CORRECT - DEX pools: preserve native token precision
let weth_amount: i64 = 1_000_000_000_000_000_000; // 1 WETH (18 decimals)
let usdc_amount: i64 = 1_000_000;                 // 1 USDC (6 decimals)

// CORRECT - Traditional exchanges: 8-decimal fixed-point for USD prices
let btc_price: i64 = 4500000000000; // $45,000.00 (8 decimals: * 100_000_000)
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

### ❌ DON'T: Ignore TLV Bounds or Reuse Type Numbers
```rust
// WRONG - No bounds checking
let tlv_data = &payload[2..]; // Could overflow!

// WRONG - Reusing TLV type numbers
pub enum TLVType {
    Trade = 1,
    Quote = 1, // COLLISION!
}
```

### ✅ DO: Validate TLV Bounds and Maintain Type Registry
```rust
// CORRECT - Bounds checking
if offset + tlv_length > payload.len() {
    return Err(ParseError::TruncatedTLV);
}

// CORRECT - Unique type numbers with proper ranges
pub enum TLVType {
    Trade = 1,        // Market Data domain (1-19)
    Quote = 2,
    SignalIdentity = 20, // Signal domain (20-39)
}
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

### ❌ DON'T: Hide Failures or Break Message Structure
```rust
// WRONG - Deceptive behavior
match relay.send_tlv_message() {
    Ok(_) => {},
    Err(_) => { /* silently ignore - WRONG! */ }
}

// WRONG - Breaking TLV message structure
let broken_header = MessageHeader {
    magic: 0x12345678, // WRONG! Must be 0xDEADBEEF
    payload_size: 100,
    // ... but payload is actually 200 bytes
};
```

### ✅ DO: Be Transparent and Maintain Protocol Integrity
```rust
// CORRECT - Propagate TLV parsing failures
let message = parse_tlv_message(&bytes)
    .map_err(|e| {
        error!("TLV parsing failed: {}", e);
        e
    })?;

// CORRECT - Proper message construction
let mut builder = TLVMessageBuilder::new(relay_domain, source);
builder.add_tlv(TLVType::Trade, &trade_tlv);
let message = builder.build(); // Calculates correct sizes and checksum
```

## Current Migration Status

### Protocol V2 Migration
- **Status**: ✅ PRODUCTION READY - Complete implementation
- **Performance**: >1M msg/s construction, >1.6M msg/s parsing (measured)
- **Coverage**: All 3 relay domains implemented with comprehensive tests
- **Location**: `backend_v2/` - new Protocol V2 architecture

### Legacy System Status
- **backend/**: Legacy services being gradually migrated to Protocol V2
- **Symbol → Instrument**: 878+ instances across 102 files still in progress
- **Backend Cleanup**: 50+ files scattered in backend root need organization

### TLV Type Registry Maintenance
- **Critical**: Review `protocol_v2/src/tlv/types.rs` for type additions
- **Rule**: Never reuse type numbers, always update expected_payload_size()
- **Validation**: Run `cargo test --package protocol_v2` before commits

## Testing Philosophy

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
use alphapulse_protocol_v2::{parse_header, parse_tlv_extensions, TLVType};

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

### Data Flow Tracing - Protocol V2
```bash
# Trace messages through relay domains by sequence number
tail -f logs/market_data_relay.log logs/signal_relay.log logs/execution_relay.log | grep "sequence"

# Debug TLV parsing issues
RUST_LOG=alphapulse_protocol_v2::tlv=debug cargo run

# Monitor relay consumer connections
tail -f logs/relay_consumer_registry.log
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
1. **Ask Clarifying Questions**: Present questions to ensure complete understanding of requirements and technical trade-offs
2. Read relevant CLAUDE.md files in subdirectories
3. Run existing tests to understand current behavior
4. Check for related issues or ongoing migrations
5. Update existing files instead of creating duplicates with adjective prefixes
6. Respect project structure - place files in their correct service directory

### Breaking Changes Philosophy
**This is a greenfield codebase - breaking changes are encouraged for system improvement:**
- **No backward compatibility concerns** - break APIs freely to improve design
- **Remove deprecated code immediately** - don't leave legacy cruft
- **Clean up after yourself** - remove old patterns when introducing new ones
- **Refactor aggressively** - improve naming, structure, and patterns without hesitation
- **Delete unused code** - don't keep "just in case" code
- **Update all references** - when changing interfaces, update ALL callers

### Breaking Change Examples (Encouraged)
```rust
// OLD: Confusing naming
pub struct ExchangeDataHandler {
    pub async fn handle_data(&self, data: String) { ... }
}

// NEW: Clear naming + breaking change
pub struct MarketDataProcessor {
    pub async fn process_market_event(&self, event: MarketEvent) { ... }
}
// DELETE the old struct entirely, update ALL references
```

### Before Submitting PR
1. ✅ All tests passing (especially precision tests)
2. ✅ No performance regression
3. ✅ Documentation updated (including CLAUDE.md if needed)
4. ✅ Linting and formatting clean
5. ✅ Commit message follows convention
6. ✅ No duplicate files with "enhanced", "fixed", "new", "v2" prefixes
7. ✅ Files placed in correct service directories per project structure
8. ✅ **Deprecated code removed** - no legacy patterns left behind
9. ✅ **All references updated** - breaking changes propagated throughout codebase

## Documentation Standards

**Write clear technical documentation, not marketing material:**
- **No hype language**: Avoid "revolutionary", "transformative", "cutting-edge", etc.
- **Be precise**: State capabilities and limitations clearly
- **Context-aware**: Write so future engineers and AI agents can understand without additional context
- **Honest limitations**: Clearly state what cannot be done, not just what can
- **Factual only**: "Processes messages in <35μs" not "Lightning-fast message processing"

## Development Process & Clarifying Questions

### Core Philosophy: Always Ask Clarifying Questions
**Optimize for clarity and user involvement in the development process.** Before beginning any task, present clarifying questions to ensure complete understanding. When ambiguity arises during implementation, pause and ask for guidance.

### When to Ask Clarifying Questions
1. **Before Starting Tasks**: Always present a list of questions before beginning work
2. **During Implementation**: When requirements become unclear or technical trade-offs emerge
3. **At Decision Points**: When multiple implementation approaches are possible
4. **For Complex Changes**: Especially involving Protocol V2 TLV messages, precision handling, or performance-critical paths

### Question Categories for AlphaPulse

#### Technical Architecture Questions
- **TLV Message Changes**: "Should this new field use native token precision or 8-decimal fixed-point?"
- **Performance Trade-offs**: "This optimization could improve throughput by 15% but adds complexity. Should we prioritize raw speed or maintainability?"
- **Protocol Compatibility**: "This change breaks backward compatibility with legacy services. Should we proceed or find an alternative approach?"

#### Business Logic Questions  
- **Trading Parameters**: "What should the default minimum profit threshold be for arbitrage opportunities?"
- **Risk Management**: "Should we implement circuit breakers for this new strategy, and at what thresholds?"
- **Precision Requirements**: "For this new exchange integration, should we preserve their native precision or normalize to our standard?"

#### Implementation Approach Questions
- **Service Boundaries**: "Should this functionality go in a shared library or be service-specific?"
- **Testing Strategy**: "Should we test against mainnet pools or create isolated test scenarios?"
- **Migration Path**: "How should we handle the transition from Symbol-based to InstrumentId-based code?"

### Presenting Technical Options

When asking clarifying questions:
1. **Present Clear Options**: "We can implement this as either A (fast, more complex) or B (slower, simpler). Which approach aligns better with system goals?"
2. **Include Trade-offs**: Explain performance, complexity, and maintenance implications
3. **Provide Context**: Reference relevant system invariants, performance targets, or architectural principles
4. **Be Specific**: "This change affects the hot path and could add 2-3μs latency" vs. "This might be slower"

### AlphaPulse-Specific Clarification Examples

#### DEX Integration Questions
- "Which DEX pools should we prioritize for testing? High-volume pairs or edge cases?"
- "Should we implement V2 and V3 math separately or create a unified interface?"
- "What's the acceptable slippage tolerance for execution validation?"

#### Protocol V2 Questions  
- "This new TLV type needs a unique number. Should we use the next available in the Market Data range (1-19)?"
- "Should this message include full 20-byte addresses or is a hash sufficient for this use case?"
- "What's the expected message frequency to determine optimal buffer sizes?"

#### Performance Optimization Questions
- "We can achieve sub-microsecond latency with unsafe code or maintain safety with ~5μs overhead. What's the priority?"
- "Should we optimize for memory usage or CPU cycles in this hot path component?"
- "This caching strategy could reduce RPC calls but uses 50MB additional memory. Is that acceptable?"

### User Involvement Guidelines

1. **Pause for Clarity**: Stop work immediately when requirements are unclear
2. **Technical Translation**: Explain complex technical concepts in accessible terms when needed
3. **Decision Documentation**: Record the reasoning behind technical decisions for future reference
4. **Iterative Refinement**: Re-engage when new questions arise during implementation

## Development Tools

### rq (Rust Query) - Semantic Code Discovery

**Architecture**: Simple semantic grep tool for Rust codebases
- **Location**: `backend_v2/tools/rq/`
- **Purpose**: Prevent code duplication by discovering existing implementations
- **Design Philosophy**: Direct rustdoc JSON parsing, no over-engineering

**Key Features**:
- Pattern search with regex support
- Type filtering (struct, enum, function, etc.)
- Usage examples from test files  
- Relationship discovery (what calls what)
- Documentation search
- Fuzzy matching for typos

**Why This Design**:
- ❌ **Avoided Over-Engineering**: No SQLite, bloom filters, plugins, TUI, LSP server
- ✅ **Simple & Fast**: Direct JSON parsing, file-based caching, <600 lines of code
- ✅ **Focused**: Solves the original problem (4+ hour Coinbase adapter) with minimal complexity
- ✅ **Maintainable**: Easy to understand, modify, and extend

**Installation & Usage**:
```bash
# Install (in rq directory)
cargo install --path .

# Basic usage
rq find TLV --type struct       # Find TLV structures
rq examples TradeTLV            # See usage examples  
rq check SomeType               # Check if exists
```

## AI Assistant Tips

When working with this codebase:
1. **Quality First**: Never rush to complete tasks - build robust, validated solutions
2. **Ask Clarifying Questions**: Always present questions before starting work and during implementation when ambiguity arises
3. Always prioritize data integrity over performance
4. Test decimal precision for any numeric changes
5. Consider both hot path (<35μs) and warm path impacts
6. Remember the Symbol → Instrument migration is ongoing
7. Check service-specific CLAUDE.md files for detailed context
8. **No Shortcuts**: Take time to validate, test, and ensure safety even if it delays task completion
9. **Clear Documentation**: Write technical docs for engineers, not marketing copy
10. **User Involvement**: Optimize for clarity by involving users in technical decisions and trade-offs

## Quick Reference

### File Locations - Protocol V2
- **Protocol V2 Core**: `backend_v2/protocol_v2/src/lib.rs`
- **TLV Definitions**: `backend_v2/protocol_v2/src/tlv/`
- **Bijective IDs**: `backend_v2/protocol_v2/src/identifiers/`
- **Shared Libraries**: `backend_v2/libs/` (adapters, amm, execution, mev, state)
- **Relay Infrastructure**: `backend_v2/infra/transport/` and `backend_v2/infra/topology/`
- **Domain Relays**: `backend_v2/relays/`
- **Service Adapters**: `backend_v2/services_v2/adapters/`
- **Strategy Implementations**: `backend_v2/services_v2/strategies/`
- **Protocol Documentation**: `backend_v2/docs/protocol.md`
- **Maintenance Guide**: `backend_v2/docs/MAINTENANCE.md`

### Key Configuration Files
- Rust Workspace: `Cargo.toml`
- Python Dependencies: `pyproject.toml`
- Frontend: `package.json`
- Docker: `docker-compose.yml`

### Important Scripts and Commands
- `backend_v2/scripts/start_system.sh` - Start Protocol V2 services
- `scripts/start-polygon-only.sh` - Start legacy services
- `scripts/monitor_connections.sh` - Monitor health
- `cargo run --bin test_protocol --release` - Protocol V2 performance validation
- `cargo test --package protocol_v2` - TLV protocol tests

## Contact for Complex Issues
For Protocol V2 architecture decisions or maintenance issues, review:
- **Primary**: `backend_v2/docs/protocol.md` - Complete Protocol V2 specification
- **Maintenance**: `backend_v2/docs/MAINTENANCE.md` - TLV system maintenance procedures
- **Performance**: Measured benchmarks in protocol documentation
- **Migration**: `projects/system-cleanup/` for legacy system migration plans

## Critical Maintenance Reminders
**📋 Regular Maintenance Checklist:**
1. **Weekly**: Review TLV type registry for additions/conflicts
2. **Before commits**: Run `cargo test --package protocol_v2`
3. **Performance**: Monitor that >1M msg/s construction, >1.6M msg/s parsing maintained
4. **Monthly**: Review `docs/MAINTENANCE.md` for system health procedures
5. **TLV Changes**: Always update `expected_payload_size()` when structs change
6. **Never**: Reuse TLV type numbers or break message header format