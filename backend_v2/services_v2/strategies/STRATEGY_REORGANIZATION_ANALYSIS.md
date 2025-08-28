# Strategy Directory Organizational Analysis

## Current State: Mixed Architecture Patterns

The `services_v2/strategies/` directory combines multiple organizational approaches, creating confusion about where strategy-related functionality should live and leading to code duplication.

## Current Structure Issues

### 1. Mixed Crate vs Module Pattern

```
services_v2/strategies/
├── src/                     # Main strategies crate
│   ├── flash_arbitrage/     # Strategy as module
│   ├── kraken_signals/      # Strategy as module  
│   └── bin/                 # Binaries for strategies
│       ├── flash_arbitrage_service.rs
│       └── kraken_signals_service.rs
├── mev/                     # Separate crate
│   └── src/lib.rs
└── state/                   # Separate crate
    └── src/lib.rs
```

**Problem**: Inconsistent organization - some strategies are modules, others are separate crates.

### 2. Code Duplication: MEV Implementation

**Duplicate MEV/Flashbots code:**
- `mev/src/flashbots.rs` - Standalone MEV library
- `src/flash_arbitrage/mev/flashbots.rs` - Embedded MEV implementation

**Both implement similar functionality:**
- Bundle construction
- Flashbots integration  
- MEV protection
- Private mempool submission

### 3. State Management Confusion

**The `state/` crate duplicates functionality from `libs/state/`:**
- Similar pool state tracking
- Overlapping cache management
- Duplicate validation logic
- Unclear which is canonical

### 4. Strategy Framework Inconsistency  

**No unified strategy pattern:**
- Flash arbitrage: Complex module structure with detector, executor, config
- Kraken signals: Simple module structure with strategy, indicators
- No common traits or interfaces
- Different configuration approaches

### 5. Test Organization Sprawl

**Massive test directory structure:**
```
tests/
├── flash_arbitrage/
│   ├── integration/         # 5 integration test files
│   ├── unit/               # 4 unit test files  
│   ├── property/           # 2 property test files
│   ├── mocks/
│   └── fixtures/
├── kraken_signals/
│   ├── integration/
│   ├── unit/
│   └── fixtures/
└── common/
    ├── fixtures.rs
    ├── helpers.rs
    └── validators.rs
```

**Problems:**
- Over-engineered test structure
- Duplicate test utilities
- No clear testing standards

## Analysis: What Each Component Should Be

### Flash Arbitrage Strategy
**Current**: Module in main crate + embedded MEV code  
**Should Be**: Standalone crate using shared MEV library

### Kraken Signals Strategy  
**Current**: Module in main crate  
**Should Be**: Standalone crate (simple enough to be standalone)

### MEV Functionality
**Current**: Both standalone crate AND embedded in flash_arbitrage  
**Should Be**: Single shared library used by strategies that need it

### State Management
**Current**: Separate `state/` crate + `libs/state/` exists  
**Should Be**: Use `libs/state/` or consolidate into one location

## Proposed Clean Architecture

### Option 1: Individual Strategy Crates (RECOMMENDED)

```
services_v2/strategies/
├── Cargo.toml              # Workspace definition
├── shared/                 # Strategy framework utilities
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs         # Common strategy traits
│       ├── config.rs      # Configuration framework
│       ├── testing.rs     # Test utilities
│       └── metrics.rs     # Strategy metrics
├── flash_arbitrage/        # Flash arbitrage strategy
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs        # Binary entry point
│   │   ├── lib.rs         # Strategy implementation
│   │   ├── detector.rs    # Opportunity detection
│   │   ├── executor.rs    # Trade execution
│   │   └── config.rs      # Strategy configuration
│   └── config.toml
├── kraken_signals/         # Kraken signals strategy
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs
│   │   ├── lib.rs
│   │   ├── indicators.rs
│   │   └── signals.rs
│   └── config.toml
└── mev/                    # MEV utilities (shared)
    ├── Cargo.toml
    └── src/
        ├── lib.rs
        ├── flashbots.rs
        ├── bundle.rs
        └── protection.rs
```

### Option 2: Monolithic Strategy Service

```
services_v2/strategies/
├── Cargo.toml
├── src/
│   ├── lib.rs              # Strategy framework
│   ├── flash_arbitrage/    # Strategy modules
│   ├── kraken_signals/
│   ├── mev/               # Shared MEV utilities
│   └── bin/               # Strategy binaries
│       ├── flash_arbitrage.rs
│       └── kraken_signals.rs
└── config/
    ├── flash_arbitrage.toml
    └── kraken_signals.toml
```

## Why Individual Strategy Crates (Option 1) is Better

### 1. Independent Development & Deployment
- Each strategy can be developed independently
- Deploy only the strategies you need
- Independent versioning and releases
- Clear ownership boundaries

### 2. Dependency Management
- Flash arbitrage needs complex AMM math, MEV libraries
- Kraken signals needs simple indicators, no MEV
- No unnecessary dependencies for simple strategies

### 3. Configuration Clarity  
- Each strategy has its own config file in its directory
- No confusion about which config applies to which strategy
- Environment-specific strategy deployment

### 4. Testing Isolation
- Strategy-specific tests stay with the strategy
- No massive shared test directory
- Clear test ownership

### 5. Performance Isolation
- Memory usage per strategy is isolated
- Can optimize each strategy's performance profile independently
- Easier profiling and debugging

## Migration Strategy

### Phase 1: Consolidate MEV Code
1. Delete `src/flash_arbitrage/mev/` (duplicate)
2. Use `mev/` crate as canonical MEV library
3. Update flash_arbitrage to depend on mev crate

### Phase 2: Resolve State Management
1. Choose between `state/` crate and `libs/state/`
2. Consolidate to single implementation
3. Update strategy dependencies

### Phase 3: Extract Strategy Crates
1. Create `flash_arbitrage/` as standalone crate
2. Create `kraken_signals/` as standalone crate  
3. Create `shared/` framework crate
4. Update workspace Cargo.toml

### Phase 4: Simplify Test Structure
1. Move tests to individual strategy crates
2. Create shared test utilities in `shared/`
3. Delete over-engineered test hierarchy

### Phase 5: Clean Up
1. Delete old `src/flash_arbitrage/` and `src/kraken_signals/`
2. Update import paths
3. Update deployment scripts

## Workspace Structure

### Root Workspace Cargo.toml
```toml
[workspace]
resolver = "2"
members = [
    "shared",
    "flash_arbitrage",
    "kraken_signals", 
    "mev",
]

[workspace.dependencies]
torq-strategy-shared = { path = "shared" }
torq-mev = { path = "mev" }
torq-codec = { path = "../../libs/codec" }
torq-types = { path = "../../libs/types" }
```

### Individual Strategy Crate
```toml
# flash_arbitrage/Cargo.toml
[package]
name = "flash-arbitrage-strategy"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "flash_arbitrage"
path = "src/main.rs"

[dependencies]
torq-strategy-shared = { workspace = true }
torq-mev = { workspace = true }
# flash arbitrage specific dependencies
```

## Benefits of Clean Architecture

### Developer Experience
- **Clear Boundaries**: Each strategy is self-contained
- **Simple Mental Model**: One strategy = one directory = one crate
- **Easy Testing**: Strategy tests live with strategy code
- **Independent Development**: Teams can work on different strategies independently

### Operational Benefits  
- **Selective Deployment**: Deploy only needed strategies
- **Independent Scaling**: Scale strategies based on their needs
- **Clear Monitoring**: Process names map to business strategies
- **Resource Isolation**: Memory and CPU usage per strategy

### Code Quality
- **No Duplication**: Shared functionality in common libraries
- **Clear Dependencies**: Explicit dependency relationships
- **Better Testing**: Strategy-specific test suites
- **Maintainable**: Easy to understand and modify

## Success Metrics

1. **Eliminated Duplication**: From 2 MEV implementations to 1
2. **Clear Organization**: Each strategy in its own crate
3. **Simplified Testing**: From complex test hierarchy to strategy-specific tests
4. **Independent Deployment**: Each strategy can be deployed separately
5. **Shared Framework**: Common strategy functionality in shared library

This reorganization creates a scalable, maintainable strategy architecture that can grow with the system's needs.