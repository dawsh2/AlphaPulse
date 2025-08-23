# Documentation Standards & Guidelines

## Comprehensive Documentation Strategy for AlphaPulse

This document defines our documentation standards, including AI assistance files (CLAUDE.md), style guides (STYLE.md), and distributed README strategies.

## Core Documentation Files

### 1. CLAUDE.md - AI Assistant Context

Every major directory should have a CLAUDE.md file that provides context for AI assistants like Claude.

#### Root CLAUDE.md Template
```markdown
# CLAUDE.md - AI Assistant Context

## Project Overview
AlphaPulse is a high-frequency trading system processing real-time market data through a binary protocol pipeline with <35μs latency requirements.

## Critical Context
- **Binary Protocol**: 48-byte fixed messages with 8 decimal place precision
- **Data Flow**: Exchange → Collector (Binary) → Relay → Bridge (JSON) → Dashboard
- **Performance**: Hot path <35μs, warm path 1-100ms
- **Zero-Copy**: Memory efficiency is critical

## Common Tasks & Commands

### Running Tests
```bash
# Run all tests with coverage
cargo test --workspace
pytest tests/ -v --cov=backend

# Check binary protocol precision
cargo test --package protocol --test precision_tests

# Performance benchmarks
cargo bench --workspace
```

### Linting & Formatting
```bash
# Rust
cargo fmt --all -- --check
cargo clippy --workspace -- -D warnings

# Python
ruff check backend/ --fix
black backend/ --check
mypy backend/services/ --strict
```

### Building & Running
```bash
# Start all services
./scripts/start-polygon-only.sh

# Start specific service
cargo run --release --bin exchange_collector

# Start FastAPI backend
python -m uvicorn app_fastapi:app --reload --port 8000
```

## Architecture Decisions

### Why Binary Protocol?
- Fixed 48-byte messages ensure predictable latency
- Zero-copy operations in hot path
- Precision preservation (8 decimal places)

### Why Rust for Collectors?
- Predictable performance, no GC pauses
- Memory safety without overhead
- Excellent async ecosystem

### Why Separate Relay/Bridge?
- Relay handles binary protocol efficiently
- Bridge provides JSON API for frontend
- Separation allows independent scaling

## Common Pitfalls to Avoid

1. **Never use floating point for prices** - Use fixed-point i64 with 8 decimals
2. **Always preserve nanosecond timestamps** - Don't truncate to milliseconds
3. **Symbol vs Instrument** - We migrated from Symbol to Instrument terminology
4. **Test precision on every change** - Data integrity is critical

## File Organization
```
backend/
├── services/           # Rust services (collectors, relay)
├── api/               # Python FastAPI endpoints
├── protocol/          # Binary protocol definitions
├── tests/            # Test suites
└── scripts/          # Utility scripts
```

## Key Files to Review
- `protocol/src/lib.rs` - Binary protocol definition
- `services/exchange_collector/src/main.rs` - Main collector
- `app_fastapi.py` - FastAPI backend
- `api/data_routes_fastapi.py` - Data endpoints

## Testing Requirements
Before committing any changes:
1. Run binary protocol tests to ensure no precision loss
2. Check that all exchange normalizers handle null fields
3. Verify WebSocket reconnection logic works
4. Test with production data samples if available

## Performance Considerations
- Hot path must remain allocation-free
- Use `cargo flamegraph` to profile performance
- Monitor message rates with `scripts/monitor_connections.sh`
- Check memory usage stays under 50MB per service

## Current Issues & TODOs
- [ ] Complete Symbol → Instrument migration (878 instances)
- [ ] Consolidate duplicate services in services/ and core/
- [ ] Add continuous data validation monitoring
- [ ] Implement exchange integration templates

## Contact & Resources
- Architecture Diagrams: `docs/architecture/`
- API Documentation: Run `cargo doc --open`
- Performance Benchmarks: `benches/`
```

#### Service-Specific CLAUDE.md Template
```markdown
# CLAUDE.md - Exchange Collector Service

## Service Purpose
Collects real-time market data from exchanges via WebSocket, normalizes to our format, and converts to binary protocol.

## Critical Invariants
- Must maintain <35μs processing latency
- Zero precision loss in decimal conversions
- Automatic reconnection on disconnect
- Handle 10,000+ messages/second

## Common Modifications

### Adding a New Exchange
1. Create new module in `src/exchanges/`
2. Implement `ExchangeCollector` trait
3. Add normalizer for field mapping
4. Write comprehensive tests
5. Update `main.rs` to include

### Modifying Binary Protocol
⚠️ DANGER: Changes affect entire pipeline
1. Update `protocol/src/lib.rs`
2. Run ALL precision tests
3. Update Bridge JSON conversion
4. Test end-to-end data flow

## Testing This Service
```bash
# Unit tests only
cargo test --lib

# Integration tests
cargo test --test '*'

# Benchmarks
cargo bench

# Memory profiling
cargo build --release
valgrind --tool=massif ./target/release/exchange_collector
```

## Performance Profiling
```bash
# CPU profiling
cargo build --release
perf record -g ./target/release/exchange_collector
perf report

# Flamegraph
cargo flamegraph --bin exchange_collector
```

## Common Issues

### WebSocket Disconnects
- Check `reconnect_with_backoff()` in `websocket.rs`
- Verify heartbeat interval matches exchange requirements
- Monitor with `RUST_LOG=debug`

### High Memory Usage
- Check for message buffer growth
- Verify old messages are being dropped
- Use `heaptrack` to find leaks

### Precision Loss
- Run `cargo test test_decimal_precision`
- Check normalizer decimal parsing
- Verify fixed-point conversion

## Dependencies to Know
- `tokio` - Async runtime
- `tungstenite` - WebSocket client
- `serde` - Serialization
- `rust_decimal` - Precise decimal handling
```

### 2. STYLE.md - Code Style Guide

#### Root STYLE.md
```markdown
# STYLE.md - AlphaPulse Code Style Guide

## General Principles
1. **Clarity over Cleverness** - Code is read more than written
2. **Performance with Maintainability** - Optimize hot paths, keep warm paths readable
3. **Fail Fast** - Surface errors immediately, don't hide them
4. **Document Intentions** - Why, not what

## Rust Style Guide

### Formatting
- Use `rustfmt` with default settings
- Run `cargo fmt` before committing
- Maximum line length: 100 characters

### Naming Conventions
```rust
// Modules: snake_case
mod exchange_collector;

// Types: PascalCase
struct TradeMessage;
enum MessageType;

// Functions: snake_case
fn process_message() {}

// Constants: SCREAMING_SNAKE_CASE
const MAX_BUFFER_SIZE: usize = 1000;

// Variables: snake_case
let message_count = 0;
```

### Error Handling
```rust
// Use Result for fallible operations
fn parse_price(s: &str) -> Result<Decimal, ParseError> {
    // Don't use unwrap() in production code
    s.parse().map_err(|e| ParseError::InvalidPrice(e))
}

// Custom error types for each module
#[derive(Debug, thiserror::Error)]
pub enum CollectorError {
    #[error("WebSocket disconnected")]
    Disconnected,
    
    #[error("Parse error: {0}")]
    ParseError(String),
}
```

### Performance Patterns
```rust
// Pre-allocate collections when size is known
let mut buffer = Vec::with_capacity(1000);

// Use zero-copy operations
let bytes: &[u8] = message.as_bytes();

// Avoid allocations in hot path
#[inline(always)]
fn hot_path_function() {
    // No String, Vec allocations here
}
```

### Testing
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_specific_behavior() {
        // Arrange
        let input = create_test_input();
        
        // Act
        let result = function_under_test(input);
        
        // Assert
        assert_eq!(result, expected);
    }
}
```

## Python Style Guide

### Formatting
- Use `black` with default settings
- Use `ruff` for linting
- Type hints required for public functions

### Naming Conventions
```python
# Modules: snake_case
import exchange_collector

# Classes: PascalCase
class TradeMessage:
    pass

# Functions: snake_case
def process_message():
    pass

# Constants: SCREAMING_SNAKE_CASE
MAX_BUFFER_SIZE = 1000

# Variables: snake_case
message_count = 0

# Private: prefix with underscore
_internal_state = {}
```

### Type Hints
```python
from typing import Dict, List, Optional, Union
from decimal import Decimal

def parse_price(price_str: str) -> Decimal:
    """Parse price string to Decimal with no precision loss."""
    return Decimal(price_str)

def process_trade(
    price: Decimal,
    volume: Decimal,
    timestamp_ns: int
) -> Dict[str, Union[Decimal, int]]:
    """Process trade with exact precision."""
    return {
        "price": price,
        "volume": volume,
        "timestamp_ns": timestamp_ns
    }
```

### Error Handling
```python
# Use specific exceptions
class PriceParseError(ValueError):
    """Raised when price cannot be parsed."""
    pass

def parse_price(price_str: str) -> Decimal:
    try:
        return Decimal(price_str)
    except InvalidOperation as e:
        raise PriceParseError(f"Invalid price: {price_str}") from e

# Don't catch generic exceptions
# Bad:
try:
    process()
except Exception:
    pass

# Good:
try:
    process()
except (ValueError, KeyError) as e:
    logger.error(f"Processing failed: {e}")
    raise
```

### Async Patterns
```python
async def fetch_data(session: aiohttp.ClientSession) -> Dict:
    """Fetch data with proper session management."""
    async with session.get(url) as response:
        return await response.json()

# Use asyncio.gather for concurrent operations
results = await asyncio.gather(
    fetch_data(session),
    process_data(data),
    return_exceptions=True
)
```

## TypeScript/React Style Guide

### Component Structure
```typescript
// Use functional components with TypeScript
interface DeFiArbitrageProps {
  pools: Pool[];
  onAnalyze: (poolId: string) => Promise<void>;
}

export const DeFiArbitrage: React.FC<DeFiArbitrageProps> = ({ 
  pools, 
  onAnalyze 
}) => {
  // Hooks first
  const [loading, setLoading] = useState(false);
  const [results, setResults] = useState<ArbitrageResult[]>([]);
  
  // Event handlers
  const handleAnalyze = useCallback(async (poolId: string) => {
    setLoading(true);
    try {
      await onAnalyze(poolId);
    } finally {
      setLoading(false);
    }
  }, [onAnalyze]);
  
  // Render
  return (
    <div className="defi-arbitrage">
      {/* Component JSX */}
    </div>
  );
};
```

### State Management
```typescript
// Use proper typing for state
interface AppState {
  trades: Trade[];
  connected: boolean;
  error: string | null;
}

// Action types as const
const ADD_TRADE = 'ADD_TRADE' as const;

// Reducer with exhaustive checking
function reducer(state: AppState, action: Action): AppState {
  switch (action.type) {
    case ADD_TRADE:
      return { ...state, trades: [...state.trades, action.payload] };
    default:
      const _exhaustive: never = action;
      return state;
  }
}
```

## Documentation Style

### Code Comments
```rust
// Rust: doc comments for public items
/// Processes a trade message and converts to binary protocol.
/// 
/// # Arguments
/// * `message` - Raw trade message from exchange
/// 
/// # Returns
/// * `Result<BinaryMessage, Error>` - Binary message or error
/// 
/// # Performance
/// This function is in the hot path and must complete in <35μs.
pub fn process_trade(message: &TradeMessage) -> Result<BinaryMessage, Error> {
    // Implementation comments explain "why", not "what"
    // Pre-allocate to avoid allocation in hot path
    let mut buffer = [0u8; 48];
    
    // ... implementation
}
```

```python
def process_trade(
    price: Decimal,
    volume: Decimal,
    timestamp_ns: int
) -> Dict[str, Any]:
    """
    Process trade data maintaining decimal precision.
    
    Args:
        price: Trade price as Decimal (no precision loss)
        volume: Trade volume as Decimal
        timestamp_ns: Unix timestamp in nanoseconds
        
    Returns:
        Dict containing processed trade data
        
    Raises:
        ValueError: If price or volume is negative
        
    Note:
        This function maintains 8 decimal place precision
        as required by the binary protocol.
    """
    if price < 0 or volume < 0:
        raise ValueError("Price and volume must be positive")
    
    # Convert to fixed-point for binary protocol
    # We use 8 decimal places (multiply by 1e8)
    fixed_price = int(price * Decimal('100000000'))
    
    return {
        "price": fixed_price,
        "volume": int(volume * Decimal('100000000')),
        "timestamp_ns": timestamp_ns
    }
```

## Git Commit Style

### Commit Message Format
```
<type>(<scope>): <subject>

<body>

<footer>
```

### Types
- `feat`: New feature
- `fix`: Bug fix
- `perf`: Performance improvement
- `refactor`: Code refactoring
- `test`: Adding tests
- `docs`: Documentation only
- `style`: Formatting, no code change
- `chore`: Maintenance tasks

### Examples
```
feat(collector): add Kraken exchange support

- Implement Kraken WebSocket client
- Add message normalizer for Kraken format
- Include comprehensive test suite
- Performance: <30μs message processing

Closes #123
```

```
fix(protocol): preserve precision in decimal conversion

The previous implementation lost precision beyond 6 decimal
places due to float conversion. Now using fixed-point
arithmetic throughout.

BREAKING CHANGE: Binary protocol version bumped to v2
```

## Review Checklist

Before submitting PR:
- [ ] Code follows style guide
- [ ] Tests added/updated
- [ ] Documentation updated
- [ ] No precision loss (run precision tests)
- [ ] Performance impact assessed
- [ ] Error handling comprehensive
- [ ] Commit messages follow format
```

### 3. Distributed README Strategy

```markdown
# Distributed README Strategy

## Overview
Each major directory and module should have its own README.md providing local context and documentation.

## Directory Structure with READMEs

```
alphapulse/
├── README.md                    # Project overview, getting started
├── CLAUDE.md                    # AI assistant context
├── STYLE.md                     # Code style guide
│
├── backend/
│   ├── README.md               # Backend architecture overview
│   ├── CLAUDE.md              # Backend-specific AI context
│   │
│   ├── services/
│   │   ├── README.md          # Services overview
│   │   │
│   │   ├── exchange_collector/
│   │   │   ├── README.md      # Collector service docs
│   │   │   ├── CLAUDE.md      # Collector AI context
│   │   │   └── src/
│   │   │       └── exchanges/
│   │   │           ├── README.md  # Exchange integrations
│   │   │           ├── kraken/
│   │   │           │   └── README.md  # Kraken-specific
│   │   │           └── polygon/
│   │   │               └── README.md  # Polygon DEX docs
│   │   │
│   │   ├── relay_server/
│   │   │   ├── README.md      # Relay service docs
│   │   │   └── CLAUDE.md      # Relay AI context
│   │   │
│   │   └── ws_bridge/
│   │       ├── README.md      # Bridge service docs
│   │       └── CLAUDE.md      # Bridge AI context
│   │
│   ├── protocol/
│   │   ├── README.md          # Binary protocol specification
│   │   └── CLAUDE.md          # Protocol AI context
│   │
│   ├── api/
│   │   ├── README.md          # API endpoints documentation
│   │   └── routes/
│   │       └── README.md      # Route-specific docs
│   │
│   └── tests/
│       ├── README.md          # Testing strategy
│       ├── e2e/
│       │   └── README.md      # E2E test scenarios
│       └── data_validation/
│           └── README.md      # Data validation approach
│
├── frontend/
│   ├── README.md              # Frontend architecture
│   ├── CLAUDE.md              # Frontend AI context
│   │
│   └── src/
│       ├── dashboard/
│       │   ├── README.md      # Dashboard components
│       │   ├── components/
│       │   │   └── README.md  # Component library
│       │   └── hooks/
│       │       └── README.md  # Custom hooks docs
│       └── services/
│           └── README.md      # Frontend services
│
├── projects/
│   ├── README.md              # Project documentation
│   │
│   ├── defi/
│   │   ├── README.md          # DeFi project overview
│   │   └── CLAUDE.md          # DeFi AI context
│   │
│   └── system-cleanup/
│       ├── README.md          # Cleanup project overview
│       └── CLAUDE.md          # Cleanup AI context
│
└── docs/
    ├── README.md              # Documentation index
    ├── architecture/
    │   └── README.md          # Architecture diagrams
    ├── api/
    │   └── README.md          # API reference
    └── deployment/
        └── README.md          # Deployment guides
```

## README Templates

### Service README Template
```markdown
# [Service Name]

## Purpose
Brief description of what this service does and why it exists.

## Architecture
How this service fits into the overall system architecture.

## Configuration
```yaml
# Example configuration
service:
  port: 8080
  workers: 4
```

## Running
```bash
# Development
cargo run --bin service_name

# Production
cargo run --release --bin service_name
```

## Testing
```bash
# Unit tests
cargo test --lib

# Integration tests
cargo test --test '*'
```

## API/Interface
Description of the service's API or interface.

## Performance
- Latency: <35μs
- Throughput: 10k msg/sec
- Memory: <50MB

## Monitoring
- Metrics: `service.name.*`
- Logs: `RUST_LOG=service_name=debug`
- Health: `http://localhost:8080/health`

## Dependencies
Key dependencies and why they're used.

## Common Issues
Known issues and their solutions.

## Contributing
How to contribute to this service.
```

### Module README Template
```markdown
# [Module Name]

## Overview
What this module provides.

## Usage
```rust
use module_name::{Feature, function};

let result = function(input)?;
```

## Key Types
- `TypeA` - Description
- `TypeB` - Description

## Examples
```rust
// Example usage
```

## Testing
How to test this module.

## Performance Considerations
Any performance notes.
```

### Project README Template
```markdown
# [Project Name]

## Mission Statement
Clear statement of project goals.

## Status
- [ ] Phase 1: Planning
- [x] Phase 2: Implementation
- [ ] Phase 3: Testing
- [ ] Phase 4: Deployment

## Structure
```
project/
├── README.md           # This file
├── TASKS.md           # Task tracking
├── phase1/            # Phase 1 work
│   └── README.md
└── phase2/            # Phase 2 work
    └── README.md
```

## Key Deliverables
1. Deliverable 1
2. Deliverable 2

## Timeline
- Week 1: Task A
- Week 2: Task B

## Resources
- [Design Doc](link)
- [Architecture](link)

## Team
- Lead: @username
- Contributors: @user1, @user2
```

## Benefits of Distributed Documentation

### 1. **Local Context**
- Documentation lives next to code
- Easier to keep in sync
- Reduces context switching

### 2. **Progressive Disclosure**
- High-level overview at root
- Detailed docs deeper in tree
- Readers can drill down as needed

### 3. **Ownership**
- Each team owns their docs
- Clear responsibility
- Better maintenance

### 4. **AI-Friendly**
- CLAUDE.md provides AI context
- Local READMEs for exploration
- Clear navigation structure

### 5. **Onboarding**
- New developers start at root
- Follow README trail to learn
- Self-guided exploration

## Implementation Checklist

### Phase 1: Core Documentation
- [ ] Create root CLAUDE.md
- [ ] Create root STYLE.md
- [ ] Update root README.md

### Phase 2: Service Documentation
- [ ] Add README.md to each service
- [ ] Add CLAUDE.md to complex services
- [ ] Document service interfaces

### Phase 3: Module Documentation
- [ ] Add README.md to key modules
- [ ] Document module APIs
- [ ] Include usage examples

### Phase 4: Automation
- [ ] Script to check for missing READMEs
- [ ] Template generator for new services
- [ ] Documentation linting in CI/CD

## Maintenance Strategy

### Regular Updates
- Update READMEs with code changes
- Review quarterly for accuracy
- Update CLAUDE.md with new patterns

### Documentation Reviews
- Include docs in PR reviews
- Check examples still work
- Verify performance numbers

### Automation
```bash
# Script to find missing READMEs
find . -type d -name "src" -o -name "services" -o -name "api" | while read dir; do
  if [ ! -f "$dir/README.md" ]; then
    echo "Missing README: $dir"
  fi
done
```

This distributed documentation strategy ensures every part of the codebase is well-documented and maintainable.
```