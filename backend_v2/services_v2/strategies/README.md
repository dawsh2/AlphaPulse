# AlphaPulse Trading Strategies

High-performance trading strategy implementations for cryptocurrency markets, processing >1M messages/second with sub-millisecond decision latency.

## Structure

This crate provides a consolidated module structure for all trading strategies:

```
strategies/
├── src/
│   ├── lib.rs                    # Main crate entry with re-exports
│   ├── flash_arbitrage/           # Flash arbitrage strategy module
│   │   ├── mod.rs                # Module exports and documentation
│   │   ├── detector.rs           # Opportunity detection engine
│   │   ├── executor.rs           # Trade execution logic
│   │   ├── relay_consumer.rs     # Market data relay integration
│   │   └── ...                   # Other components
│   ├── kraken_signals/           # Kraken signals strategy module  
│   │   ├── mod.rs               # Module exports and documentation
│   │   ├── strategy.rs          # Core strategy implementation
│   │   ├── indicators.rs        # Technical indicators
│   │   └── ...                  # Other components
│   └── bin/                     # Binary executables
│       ├── flash_arbitrage_service.rs
│       └── kraken_signals_service.rs
├── configs/                      # Strategy configuration files
│   └── kraken_strategy.toml    # Kraken strategy config
└── tests/                       # Integration and unit tests
    └── flash_arbitrage/         # Flash arbitrage tests
```

## Strategies

### Flash Arbitrage
Capital-efficient arbitrage strategy capturing cross-DEX price differences using atomic flash loan execution.

**Key Features:**
- Real-time monitoring of major DEX protocols (Uniswap V2/V3, SushiSwap, QuickSwap)
- Multi-hop arbitrage paths (2-4 hops) with optimal routing
- Flash loan integration (Aave V3, Compound, Balancer)
- MEV protection via Flashbots bundles
- <5ms opportunity detection latency

**Usage:**
```rust
use alphapulse_strategies::flash_arbitrage::{OpportunityDetector, RelayConsumer, SignalOutput};

// Initialize components
let detector = OpportunityDetector::new(pool_manager, config);
let consumer = RelayConsumer::new(socket_path, pool_manager, detector, signal_output);

// Start processing
consumer.start().await?;
```

### Kraken Signals
Momentum-based signal generation analyzing Kraken market data for trading opportunities.

**Key Features:**
- Real-time processing of Kraken WebSocket feeds
- Technical indicators (RSI, MACD, Moving Averages)
- Multi-timeframe trend confirmation
- Confidence scoring (0-100 scale)
- <100ms signal generation latency

**Usage:**
```rust
use alphapulse_strategies::kraken_signals::{KrakenSignalStrategy, StrategyConfig};

// Configure strategy
let config = StrategyConfig {
    instruments: vec!["BTC-USD".to_string()],
    min_signal_confidence: 70,
    // ...
};

// Start strategy
let mut strategy = KrakenSignalStrategy::new(config);
strategy.start().await?;
```

## Feature Flags

The strategies crate supports selective compilation of individual strategies:

- `flash-arbitrage`: Include flash arbitrage strategy (default: enabled)
- `kraken-signals`: Include Kraken signals strategy (default: enabled)

Build with specific features:
```bash
# Build only flash arbitrage
cargo build --no-default-features --features flash-arbitrage

# Build only kraken signals  
cargo build --no-default-features --features kraken-signals

# Build with all features (default)
cargo build --all-features
```

## Running Services

### Flash Arbitrage Service
```bash
# Run with default configuration
cargo run --bin flash_arbitrage_service

# With custom socket paths
MARKET_DATA_SOCKET=/custom/path/market.sock cargo run --bin flash_arbitrage_service
```

### Kraken Signals Service
```bash
# Run with default configuration
cargo run --bin kraken_signals_service

# With custom config file
KRAKEN_STRATEGY_CONFIG_PATH=configs/custom.toml cargo run --bin kraken_signals_service
```

## Configuration

Configuration files are located in the `configs/` directory. Environment variables can override default paths:

- `KRAKEN_STRATEGY_CONFIG_PATH`: Path to Kraken strategy configuration (default: `configs/kraken_strategy.toml`)
- `MARKET_DATA_SOCKET`: Unix socket path for market data relay
- `SIGNAL_SOCKET`: Unix socket path for signal relay output

## Testing

Run all strategy tests:
```bash
cargo test -p alphapulse-strategies
```

Run specific strategy tests:
```bash
# Flash arbitrage tests only
cargo test -p alphapulse-strategies flash_arbitrage

# Kraken signals tests only  
cargo test -p alphapulse-strategies kraken_signals
```

Performance benchmarks:
```bash
cargo bench -p alphapulse-strategies
```

## API Documentation

Generate and view API documentation:
```bash
cargo doc -p alphapulse-strategies --open
```

## Performance Targets

- **Message Processing**: >1M messages/second throughput
- **Decision Latency**: <5ms for arbitrage detection, <100ms for signals
- **Memory Usage**: <50MB per strategy instance
- **CPU Usage**: <10% single core under normal load

## Dependencies

All strategies use workspace-level dependencies for consistency:
- Protocol: `alphapulse-types` for TLV message handling
- State: `alphapulse-state-*` for market state management
- Network: `torq-network` for transport layer
- Math: `alphapulse-amm` for AMM calculations

## Contributing

When adding new strategies:
1. Create a new module in `src/` following existing patterns
2. Add public API re-exports to `src/lib.rs`
3. Create binary entry point in `src/bin/` if needed
4. Add configuration schema to `configs/`
5. Write comprehensive tests in `tests/`
6. Update this README with strategy documentation