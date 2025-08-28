# Adapter Clean Architecture Plan

## Problem Statement

The current adapter directory has three different implementation patterns for the same exchanges, scattered common code, and unclear ownership. This creates developer confusion and maintenance burden.

## Proposed Clean Architecture

### Directory Structure
```
services_v2/adapters/
├── Cargo.toml              # Workspace definition
├── src/                    # Adapter framework library
│   ├── lib.rs             # Common adapter traits and utilities
│   ├── circuit_breaker.rs # Circuit breaker implementation
│   ├── validation.rs      # Message validation framework
│   ├── rate_limiting.rs   # Rate limiting utilities
│   ├── parsing.rs         # Exchange data parsing utilities
│   ├── dex.rs             # DEX utilities (from dex_utils/)
│   └── config.rs          # Configuration utilities
├── exchanges/              # Exchange adapters (trading venues)
│   ├── coinbase/
│   │   ├── Cargo.toml
│   │   ├── src/main.rs
│   │   └── config.toml
│   ├── binance/
│   │   ├── Cargo.toml
│   │   ├── src/main.rs
│   │   └── config.toml
│   ├── kraken/
│   │   ├── Cargo.toml
│   │   ├── src/main.rs
│   │   └── config.toml
│   └── polygon/
│       ├── Cargo.toml
│       ├── src/main.rs
│       └── config.toml
├── data/                   # Data collectors (non-exchange data)
│   └── gas_price/
│       ├── Cargo.toml
│       ├── src/main.rs
│       └── config.toml
└── tools/                  # Development and testing utilities
    ├── validator/
    └── benchmark/
```

## Workspace Structure

### Root Workspace Cargo.toml
```toml
[workspace]
resolver = "2"
members = [
    "exchanges/coinbase",
    "exchanges/binance", 
    "exchanges/kraken",
    "exchanges/polygon",
    "data/gas_price",
    "tools/validator",
    "tools/benchmark",
]

[workspace.dependencies]
# Framework dependencies
torq-adapters = { path = "." }
torq-codec = { path = "../../libs/codec" }
torq-types = { path = "../../libs/types" }

# External dependencies
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
tracing = "0.1"
anyhow = "1.0"
```

### Framework Library (src/lib.rs)
```toml
[package]
name = "torq-adapters"
version = "0.1.0"
edition = "2021"

[dependencies]
torq-codec = { workspace = true }
torq-types = { workspace = true }
tokio = { workspace = true }
serde = { workspace = true }
tracing = { workspace = true }
```

### Individual Exchange Adapter
```toml
# exchanges/coinbase/Cargo.toml
[package]
name = "coinbase-adapter"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "coinbase_adapter"
path = "src/main.rs"

[dependencies]
torq-adapters = { workspace = true }
# coinbase-specific dependencies
tungstenite = "0.20"
serde_json = "1.0"
```

## Implementation Pattern

### Idiomatic Rust Structure
Each adapter follows standard Rust binary crate pattern:

```rust
// exchanges/coinbase/src/main.rs
use torq_adapters::{AdapterFramework, ExchangeAdapter};
use anyhow::Result;

struct CoinbaseAdapter {
    // adapter-specific state
}

impl ExchangeAdapter for CoinbaseAdapter {
    // implement standard adapter trait
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = torq_adapters::load_config("config.toml")?;
    let adapter = CoinbaseAdapter::new(config).await?;
    
    AdapterFramework::run(adapter).await
}
```

## Migration Strategy

### Phase 1: Create Clean Structure
1. Create new directory structure
2. Move `src/circuit_breaker.rs` etc. to framework library
3. Create workspace Cargo.toml

### Phase 2: Choose Canonical Implementations
**For each exchange, select ONE implementation:**
- **Coinbase**: Use `src/bin/coinbase/coinbase.rs` (most recent)
- **Binance**: Use `src/bin/binance/binance.rs`
- **Kraken**: Use `src/bin/kraken/kraken.rs`  
- **Polygon**: Use `src/bin/polygon/polygon.rs`

### Phase 3: Migrate Selected Implementations
1. Move chosen implementations to new structure
2. Update to use framework library
3. Standardize configuration format
4. Delete old implementations

### Phase 4: Clean Up
1. Delete `src/input/collectors/` (old pattern)
2. Delete plugin directories (coinbase_adapter/, polygon_adapter/)
3. Update build scripts and deployment
4. Update documentation

## What Gets Moved to src/

**From `dex_utils/` (consolidate into framework):**
- `dex_utils/src/abi/` → `src/dex/abi/`  
- `dex_utils/src/event_signatures.rs` → `src/dex/events.rs`
- `dex_utils/src/lib.rs` → integrated into `src/dex.rs`

**From scattered parsing code:**
- `src/input/components/parsing_utils.rs` → `src/parsing.rs`
- Exchange-specific parsers → kept in individual adapters

**From current src/:**
- `src/circuit_breaker.rs` → stays in `src/circuit_breaker.rs`
- `src/rate_limit.rs` → `src/rate_limiting.rs` 
- `src/validation.rs` → stays in `src/validation.rs`
- `src/config.rs` → stays in `src/config.rs`

## What Gets Deleted

### Duplicate Implementations
- `src/input/collectors/coinbase.rs` ❌
- `coinbase_adapter/` ❌ 
- Keep: `src/bin/coinbase/coinbase.rs` ✅

- `polygon_adapter/` ❌
- Keep: `src/bin/polygon/polygon.rs` ✅

### Dead Code
- `src/input/collectors/gemini.rs` ❌ (already disabled)
- Any other disabled/commented code

### Framework Complexity
- `src/input/components/` ❌ (over-engineered)
- `src/input/traits/` ❌ (unused abstractions)

## Benefits of Clean Architecture

### Developer Experience
- **Clear Mental Model**: One exchange = one directory = one binary
- **No Confusion**: Only one implementation per exchange
- **Standard Pattern**: All adapters follow same structure
- **Easy Onboarding**: New adapters follow clear template

### Operational Benefits
- **Independent Scaling**: Each adapter runs as separate process
- **Independent Deployment**: Update one adapter without affecting others
- **Clear Monitoring**: Process names map to business functions
- **Simple Configuration**: One config file per adapter

### Code Quality
- **Shared Framework**: Common functionality in library crate
- **No Duplication**: Circuit breaker, validation, etc. implemented once
- **Testable**: Clear boundaries for unit and integration tests
- **Maintainable**: Changes to framework benefit all adapters

## Deployment Model

```bash
# Build specific adapter
cargo build --bin coinbase_adapter

# Deploy individual adapter
./target/release/coinbase_adapter --config exchanges/coinbase/config.toml

# Build all adapters
cargo build --workspace

# Run specific adapter in development
cargo run --bin coinbase_adapter
```

## Configuration Standardization

### Standard Config Format
```toml
# exchanges/coinbase/config.toml
[exchange]
name = "coinbase"
websocket_url = "wss://ws-feed.exchange.coinbase.com"
api_url = "https://api.exchange.coinbase.com"

[instruments]
symbols = ["BTC-USD", "ETH-USD"]

[output]
relay_domain = "MarketData"
socket_path = "/tmp/torq/market_data.sock"

[performance]
circuit_breaker_threshold = 10
rate_limit_requests_per_second = 100
```

## Success Metrics

1. **Reduced Complexity**: From 3 patterns to 1 pattern
2. **Eliminated Duplication**: From 3 Coinbase implementations to 1
3. **Clear Ownership**: Each adapter has single canonical location
4. **Standard Operations**: All adapters deploy and configure the same way

This architecture eliminates the current confusion and creates a maintainable, scalable adapter system following idiomatic Rust patterns.
