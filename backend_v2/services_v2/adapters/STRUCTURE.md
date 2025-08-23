# Adapter Module Structure

## Overview
The adapters module provides stateless, bidirectional gateways between AlphaPulse's internal TLV protocol and external venue APIs. Each adapter handles both market data ingestion (inbound) and order execution (outbound).

## Directory Structure

```
backend_v2/services_v2/adapters/
├── Cargo.toml
├── README.md
├── STRUCTURE.md                       # This file
├── src/
│   ├── binance/                       # Binance CEX adapter
│   │   ├── mod.rs                     # BinanceAdapter implementation
│   │   ├── config.rs                  # URLs, rate limits, circuit breaker
│   │   ├── inbound.rs                 # WebSocket → TLV
│   │   ├── outbound.rs                # TLV → REST API
│   │   ├── transformer.rs             # Binance ↔ TLV conversions
│   │   └── auth.rs                    # HMAC signing
│   │
│   ├── kraken/                        # Kraken CEX adapter
│   │   ├── mod.rs                     # KrakenAdapter implementation
│   │   ├── config.rs                  # URLs, rate limits, circuit breaker
│   │   ├── inbound.rs                 # WebSocket → TLV
│   │   ├── outbound.rs                # TLV → REST API
│   │   ├── transformer.rs             # Kraken ↔ TLV conversions
│   │   └── auth.rs                    # Kraken API signing
│   │
│   ├── polygon_dex/                   # Polygon DEX adapter
│   │   ├── mod.rs                     # PolygonDEXAdapter implementation
│   │   ├── config.rs                  # RPC URLs, contracts, retry settings
│   │   ├── inbound.rs                 # Event logs → TLV
│   │   ├── outbound.rs                # TLV → Transactions
│   │   ├── transformer.rs             # Events/Txs ↔ TLV conversions
│   │   ├── abi_decoder.rs             # ABI-based semantic validation
│   │   ├── pools.rs                   # Pool cache and discovery
│   │   ├── auth.rs                    # Web3 wallet signing
│   │   └── abi/                       # Contract ABIs
│   │       ├── mod.rs
│   │       ├── uniswap_v2.rs
│   │       ├── uniswap_v3.rs
│   │       ├── quickswap.rs
│   │       └── sushiswap.rs
│   │
│   ├── ethereum_dex/                  # Ethereum mainnet DEX adapter
│   │   ├── mod.rs                     # EthereumDEXAdapter implementation
│   │   ├── config.rs                  # RPC URLs, contracts
│   │   ├── inbound.rs                 # Event logs → TLV
│   │   ├── outbound.rs                # TLV → Transactions
│   │   ├── transformer.rs             # Events/Txs ↔ TLV conversions
│   │   ├── abi_decoder.rs             # Reuse polygon_dex decoder
│   │   ├── pools.rs                   # Pool cache for mainnet
│   │   └── auth.rs                    # Web3 wallet signing
│   │
│   ├── arbitrum_dex/                  # Arbitrum DEX adapter (future)
│   │   └── mod.rs                     # Placeholder
│   │
│   ├── alpaca/                        # Alpaca stocks adapter (future)
│   │   ├── mod.rs                     # AlpacaAdapter implementation
│   │   ├── config.rs                  # API URLs, rate limits
│   │   ├── inbound.rs                 # WebSocket/REST → TLV
│   │   ├── outbound.rs                # TLV → REST orders
│   │   ├── transformer.rs             # Alpaca ↔ TLV conversions
│   │   └── auth.rs                    # OAuth/API key auth
│   │
│   ├── databento/                     # DataBento data adapter (future)
│   │   ├── mod.rs                     # DataBentoAdapter implementation
│   │   ├── config.rs                  # API endpoints, subscriptions
│   │   ├── inbound.rs                 # DBN format → TLV
│   │   └── transformer.rs             # DataBento ↔ TLV conversions
│   │
│   ├── cex/                           # Shared CEX code
│   │   ├── mod.rs
│   │   ├── websocket.rs               # WS connection, reconnect, heartbeat
│   │   ├── rest.rs                    # REST client with retry
│   │   └── transformer.rs             # Common CEX transformations
│   │
│   ├── dex/                           # Shared DEX code
│   │   ├── mod.rs
│   │   ├── abi_decoder.rs             # ABI event decoding
│   │   ├── event_monitor.rs           # Log monitoring
│   │   ├── transaction.rs             # Transaction building
│   │   └── abi/                       # Contract ABIs (used by all DEXs)
│   │       ├── uniswap_v2.rs
│   │       ├── uniswap_v3.rs
│   │       └── events.rs
│   │
│   ├── relay.rs                       # Relay connections
│   ├── resilience.rs                  # Rate limiting, circuit breaker
│   ├── metrics.rs                     # Metrics collection
│   ├── traits.rs                      # Adapter trait
│   ├── error.rs                       # Error types
│   └── lib.rs                         # Public API
│
└── tests/
    ├── abi_validation_test.rs
    └── integration/
        ├── binance_test.rs
        ├── kraken_test.rs
        └── polygon_dex_test.rs
```

## Design Principles

### 1. Flat Venue Structure
- Each venue gets a top-level directory (`binance/`, `kraken/`, `polygon_dex/`)
- Easy to find and grep for venue-specific code
- Maximum 3 levels of directory nesting

### 2. Shared Code Organization
- `cex/` - Code shared by all centralized exchanges
- `dex/` - Code shared by all decentralized exchanges
- Common patterns extracted, not forced together

### 3. Stateless Adapters
- No position tracking, order management, or state persistence
- Just transform and forward messages
- State management happens in portfolio/risk modules

### 4. Bidirectional Flow
- `inbound.rs` - External data → TLV → Market Data Relay
- `outbound.rs` - Execution Relay → TLV → External API
- `transformer.rs` - Handles conversions in both directions

### 5. Config with Adapter
- Each adapter owns its configuration
- No separate config tree
- Resilience settings (rate limits, circuit breakers) in config

## Adding a New Adapter

1. Create a new top-level directory: `src/new_venue/`
2. Implement the core files:
   - `mod.rs` - Adapter implementation with `Adapter` trait
   - `config.rs` - Venue-specific configuration
   - `inbound.rs` - Market data ingestion
   - `outbound.rs` - Order execution (if supported)
   - `transformer.rs` - Data transformations
   - `auth.rs` - Authentication (if needed)
3. Add tests in `tests/integration/new_venue_test.rs`

## Shared Components

### CEX Shared (`cex/`)
- WebSocket connection management
- Reconnection logic with exponential backoff
- Heartbeat/ping handling
- REST client with retry logic
- Common order type mappings

### DEX Shared (`dex/`)
- ABI event decoding
- Event log monitoring
- Gas estimation
- Transaction building
- Uniswap V2/V3 ABIs (same across all EVM chains)

### Core Utilities
- `relay.rs` - Relay connection management
- `resilience.rs` - Rate limiting and circuit breakers
- `metrics.rs` - Prometheus metrics collection
- `traits.rs` - Core `Adapter` trait definition
- `error.rs` - Common error types

## VenueId Mapping

Adapters map to Protocol V2 VenueId enum values:
- `binance/` → VenueId::Binance (100)
- `kraken/` → VenueId::Kraken (101)
- `polygon_dex/` → VenueId::Polygon (202)
- `ethereum_dex/` → VenueId::Ethereum (200)
- `alpaca/` → Future traditional markets venue
- `databento/` → Future data provider venue

## Performance Targets

- Transformation latency: <1ms
- WebSocket message processing: >10,000 msg/s
- ABI event decoding: >1,000 events/s
- Zero memory allocations in hot path
- Circuit breaker activation: <10ms

## Testing Strategy

1. **Unit Tests** - Each transformer function
2. **Integration Tests** - Full roundtrip with real venues
3. **ABI Validation** - Semantic correctness with real blockchain data
4. **Performance Benchmarks** - Transformation and decoding speed
5. **Resilience Tests** - Circuit breaker and rate limiting behavior