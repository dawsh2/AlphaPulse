# CLAUDE.md - AlphaPulse AI Assistant Context

## System Overview
AlphaPulse is a high-performance cryptocurrency trading system built on Protocol V2 TLV (Type-Length-Value) message architecture processing >1M messages/second across domain-specific relays with complete precision preservation and bijective instrument identification.

**Core Mission**: Build robust, validated, safe trading infrastructure with complete transparency and zero tolerance for deceptive practices.

**Development Priority**: Quality over speed. Well-organized, robust/safe/validating system with long-term reliability. No shortcuts.

**Production-Ready Code**: ALWAYS write production-quality code from the start. Never use fake/mock/dummy variables, services, or data.

**See Also**: `.agents/` directory for detailed documentation on development, testing, and tools.

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

## Critical System Invariants
1. **TLV Message Format**: 32-byte MessageHeader + variable TLV payload structure
2. **Full Address Architecture**: All DEX operations use complete 20-byte Ethereum addresses
3. **Zero Precision Loss**: Preserve native token precision (18 decimals WETH, 6 USDC)
4. **No Deception**: Never hide failures, fake data, or simulate success
5. **Domain Separation**: Respect relay domains and TLV type ranges
6. **Sequence Integrity**: Maintain monotonic per-source sequence numbers
7. **Nanosecond Timestamps**: Never truncate to milliseconds
8. **Dynamic Configuration**: Use configurable values instead of hardcoded constants
9. **One Canonical Source**: Single implementation per concept
10. **NO MOCKS EVER**: Never use mock data, mock services, or mocked testing
11. **Breaking Changes Welcome**: This is a greenfield codebase - break freely to improve
12. **TLV Type Registry**: Never reuse type numbers, update expected_payload_size()

## Project Structure
```
backend_v2/
├── protocol_v2/          # Protocol V2 TLV definitions (CRITICAL)
│   ├── src/tlv/         # TLV message types and parsing
│   └── src/identifiers/ # Bijective InstrumentId system
├── libs/                # Shared libraries
│   ├── adapters/        # Adapter utilities
│   ├── amm/            # AMM math libraries
│   ├── execution/      # Execution utilities
│   ├── mev/            # MEV protection
│   └── state/          # State management
├── services_v2/         # Service implementations
│   ├── adapters/       # Exchange collectors
│   ├── strategies/     # Trading strategies
│   └── dashboard/      # Dashboard services
├── infra/              # Infrastructure layer
├── relays/             # Domain-specific relays
├── tests/e2e/          # End-to-end tests
├── docs/               # Protocol documentation
└── .agents/            # AI assistant documentation
    ├── development.md  # Development workflows
    ├── testing.md      # Testing & debugging
    ├── rq_tool.md      # rq tool usage
    └── common_pitfalls.md # Common mistakes
```

## Key Technical Decisions

### Why TLV Message Format?
- 32-byte header + variable TLV payload for flexibility and performance
- Enables zero-copy operations with zerocopy traits
- Measured >1M msg/s construction, >1.6M msg/s parsing performance
- Forward compatibility through unknown TLV type graceful handling

### Why Bijective InstrumentIDs?
- Self-describing IDs eliminate need for centralized registries
- Deterministic construction prevents collisions
- Reversible to extract venue, asset type, and identifying data
- O(1) cache lookups using fast_hash conversion

### Why Domain-Specific Relays?
- Performance isolation: market data bursts don't affect execution
- Security: execution messages have stricter validation
- Clear separation: MarketData (1-19), Signals (20-39), Execution (40-79)
- Debugging: clear message flow tracing

## Common Development Tasks

### Rust Ecosystem Tools
Before building new analysis features, check existing tools:
- **rust-analyzer**: Complete semantic analysis, cross-references
- **cargo tree**: Dependency graphs and analysis
- **cargo-semver-checks**: Breaking change detection
- **cargo audit**: Security vulnerability analysis

### Codebase Navigation with rq
```bash
# CRITICAL: Always check before implementing
rq check TradeTLV               # Verify implementation exists
rq similar validate_pool        # Find similar implementations

# Strategic System Understanding
rq docs "zero-copy serialization"    # Implementation patterns
rq docs "performance profile"        # Measured metrics
rq docs "architecture role"          # Component relationships
rq docs "integration points"         # How components connect

# Find existing implementations
rq find Pool --type struct       # All Pool structures
rq find parse --type function    # All parsing functions
rq examples TradeTLV             # Show real usage
```

See `.agents/rq_tool.md` for complete documentation.

### Running the System
```bash
# Start all services
./scripts/start-polygon-only.sh

# Start individual services
cargo run --release --bin exchange_collector
cargo run --release --bin relay_server
cargo run --release --bin ws_bridge
python -m uvicorn app_fastapi:app --reload --port 8000

# Monitor connections
./scripts/monitor_connections.sh
```

### Critical Testing Commands
```bash
# Protocol V2 tests (MUST pass before commit)
cargo test --package protocol_v2 --test tlv_parsing
cargo test --package protocol_v2 --test precision_validation

# Performance validation
cargo run --bin test_protocol --release
# Must maintain: >1M msg/s construction, >1.6M msg/s parsing
```

See `.agents/testing.md` for complete testing guide.

## Current Migration Status

### Protocol V2 Migration
- **Status**: ✅ PRODUCTION READY
- **Performance**: >1M msg/s construction, >1.6M msg/s parsing (measured)
- **Coverage**: All 3 relay domains implemented with comprehensive tests

### TLV Type Registry Maintenance
- **Critical**: Review `protocol_v2/src/tlv/types.rs` for type additions
- **Rule**: Never reuse type numbers, always update expected_payload_size()
- **Validation**: Run `cargo test --package protocol_v2` before commits

## Development Guidelines

### Before Making Changes
1. **Ask Clarifying Questions**: Ensure complete understanding of requirements
2. **Use rq for Discovery**: Search for existing functionality
3. Run existing tests to understand current behavior
4. Update existing files instead of creating duplicates
5. Respect project structure - place files in correct directories

### Breaking Changes Philosophy
**This is a greenfield codebase - breaking changes are encouraged:**
- Break APIs freely to improve design
- Remove deprecated code immediately
- Clean up old patterns when introducing new ones
- Refactor aggressively to improve naming and structure
- Delete unused code - don't keep "just in case" code
- Update ALL references when changing interfaces

See `.agents/development.md` for complete development workflow.

## Documentation Standards
**Write clear technical documentation for humans and AI agents:**
- No hype language ("revolutionary", "transformative", etc.)
- Be precise about capabilities and limitations
- Factual only: "Processes messages in <35μs" not "Lightning-fast"
- Include structured `//!` documentation for rq discovery

### Architecture Diagrams
- **Use Mermaid diagrams** instead of ASCII art for better visualization
- **Standard pattern**: Create `architecture_diagram()` function with `#[cfg_attr(doc, aquamarine::aquamarine)]`
- **Reference from docs**: Link to diagram function in "Architecture Role" section
- **Search commands**: Use `rq docs "mermaid"` or `rqd` to find all diagrams
- **Benefits**: Rendered SVG in rustdoc, GitHub integration, easy maintenance

## Quick Reference

### File Locations
- **Protocol V2 Core**: `protocol_v2/src/lib.rs`
- **TLV Definitions**: `protocol_v2/src/tlv/`
- **Bijective IDs**: `protocol_v2/src/identifiers/`
- **Shared Libraries**: `libs/`
- **Service Adapters**: `services_v2/adapters/`
- **Strategy Implementations**: `services_v2/strategies/`

### Key Commands
```bash
# System understanding
rq docs "architecture"           # Component relationships
rq check function_name           # Prevent duplication
rq docs "mermaid"               # Find architecture diagrams
rqd                             # Short alias for diagram search

# Testing
cargo test --package protocol_v2
cargo run --bin test_protocol --release

# Code quality
cargo fmt --all -- --check
cargo clippy --workspace -- -D warnings
```

## Critical Maintenance Reminders
1. **Weekly**: Review TLV type registry for additions/conflicts
2. **Before commits**: Run `cargo test --package protocol_v2`
3. **Performance**: Monitor >1M msg/s construction, >1.6M msg/s parsing
4. **TLV Changes**: Always update `expected_payload_size()` when structs change
5. **Never**: Reuse TLV type numbers or break message header format

## AI Assistant Tips
1. **Quality First**: Never rush - build robust, validated solutions
2. **Ask Clarifying Questions**: Present questions before starting work
3. **Use rq for Discovery**: Leverage `rq docs` and `rq check`
4. **No Shortcuts**: Take time to validate and ensure safety
5. **Check .agents/ directory**: Reference detailed guides for specific tasks

For detailed information on:
- Development workflows → `.agents/development.md`
- Testing & debugging → `.agents/testing.md`
- Code style guide → `.agents/style.md`
- Development tools → `.agents/tools.md`
- CI/CD & deployment → `.agents/cicd.md`
- rq tool usage → `.agents/rq_tool.md`
- Common mistakes → `.agents/common_pitfalls.md`