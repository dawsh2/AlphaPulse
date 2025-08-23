# DeFi Scanner Service

## Overview

The DeFi Scanner service monitors decentralized exchanges (DEXs) on Polygon for arbitrage opportunities. It discovers pools, tracks reserve changes, calculates cross-DEX price differences, and broadcasts profitable opportunities via the AlphaPulse relay system.

## Architecture

```
DEX Events → Pool Monitor → Opportunity Detector → Relay Broadcast
     ↓             ↓              ↓                    ↓
Pool Discovery  Reserve Updates  Price Calculation  Execution Bots
```

## Components

### OpportunityDetector (`opportunity_detector.rs`)
- **Purpose**: Detects arbitrage opportunities across monitored pools
- **Process**: Scans all pools every 100ms, calculates price differences, filters by profitability
- **Output**: Broadcasts `ArbitrageOpportunity` messages via relay

### PoolMonitor (`pool_monitor.rs`) 
- **Purpose**: Discovers and monitors DEX pools
- **Process**: Fetches new pools, updates reserves, maintains pool state
- **Storage**: In-memory `DashMap` for fast access

### PriceCalculator (`price_calculator.rs`)
- **Purpose**: Calculates accurate prices across different DEX types
- **Supports**: Uniswap V2/V3 math, Sushiswap, custom fee calculations
- **Features**: Slippage estimation, gas cost calculations

### Exchange Protocols (`exchanges/`)
- **UniswapV2**: Constant product formula, 0.3% fees
- **UniswapV3**: Concentrated liquidity (simplified), variable fees
- **Sushiswap**: Uniswap V2 fork with identical math

## Configuration

### Environment Variables
```bash
ALCHEMY_RPC_URL=wss://polygon-mainnet.g.alchemy.com/v2/YOUR_KEY
MIN_PROFIT_USD=10
GAS_PRICE_GWEI=30
RUST_LOG=defi_scanner=info
```

### Config Parameters
- `min_profit_usd`: Minimum profitable arbitrage amount
- `min_profit_percentage`: Minimum profit percentage (1% default)
- `max_gas_cost_usd`: Maximum acceptable gas cost
- `confidence_threshold`: Minimum confidence score (0.8 default)

## Running the Scanner

### Development
```bash
cd backend/services/defi/scanner
cargo run --bin defi-scanner
```

### Production
```bash
cd backend/services/defi
cargo build --release
./target/release/defi-scanner
```

### With Custom Config
```bash
MIN_PROFIT_USD=20 GAS_PRICE_GWEI=50 cargo run --bin defi-scanner
```

## Monitoring

### Logs
- **Info**: Opportunities found, service status
- **Debug**: Pool updates, quote calculations
- **Warn**: Failed opportunities, network issues
- **Error**: Service failures, critical errors

### Metrics (TODO)
- Opportunities detected per minute
- Execution success rate
- Average profit per opportunity
- Pool update frequency

## Development

### Adding New DEX
1. Implement `DexProtocol` trait in `exchanges/`
2. Add exchange config to `config.rs`
3. Update pool discovery in `pool_monitor.rs`
4. Test with mock pools before mainnet

### Testing
```bash
# Unit tests
cargo test

# Integration tests with mock RPC
cargo test --features mock_rpc

# Benchmark performance
cargo bench
```

## Performance Targets

- **Opportunity Detection**: <100ms from pool update to broadcast
- **Pool Scanning**: Complete scan of 1000+ pools in <50ms
- **Memory Usage**: <100MB for 10,000 monitored pools
- **Accuracy**: >95% of broadcasted opportunities remain profitable

## Integration

### With Capital Arbitrage Bot
Scanner broadcasts opportunities → Capital bot receives via relay → Executes trades

### With Flash Loan Bot
Scanner detects complex opportunities → Flash bot simulates execution → Deploys flash loan strategy

### With Dashboard
Scanner metrics → WebSocket bridge → Frontend monitoring dashboard

## Roadmap

### Phase 1 (Current)
- ✅ Basic pool monitoring and opportunity detection
- ✅ Uniswap V2/V3 and Sushiswap support
- ✅ Binary protocol integration

### Phase 2
- [ ] Real RPC integration for pool discovery
- [ ] MEV-aware opportunity filtering
- [ ] Advanced slippage calculations

### Phase 3  
- [ ] Multi-hop arbitrage detection
- [ ] Flash loan opportunity sizing
- [ ] ML-based profitability prediction