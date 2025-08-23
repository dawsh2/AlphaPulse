# AlphaPulse DeFi Services

## Overview

This directory contains all DeFi-related services for the AlphaPulse trading system. These services work together to detect, analyze, and execute arbitrage opportunities across decentralized exchanges.

## Service Architecture

```
services/defi/
├── capital_arbitrage/    # Simple capital-based arbitrage using wallet balances
├── scanner/             # Real-time opportunity detection across DEXs  
└── flash_loan/          # Advanced flash loan arbitrage strategies
```

**Note**: DeFi services integrate with the existing `services/relay_server/` rather than having a separate relay. This ensures unified message handling for both CEX and DeFi data.

## Services

### Capital Arbitrage (`capital_arbitrage/`)
- **Purpose**: Execute simple two-step arbitrage using existing wallet balances
- **Strategy**: Conservative, low-risk approach for proof-of-concept
- **Dependencies**: Direct DEX router interactions, no smart contracts required
- **Target Latency**: <500ms from opportunity detection to execution

### Scanner (`scanner/`)
- **Purpose**: Real-time detection of arbitrage opportunities across DEXs
- **Capabilities**: Multi-DEX monitoring, price calculation, opportunity filtering
- **Integrations**: Uniswap V2/V3, Sushiswap, Quickswap, Curve Finance
- **Output**: Broadcasts opportunities via AlphaPulse binary protocol

### Flash Loan (`flash_loan/`)
- **Purpose**: Execute advanced arbitrage strategies using Aave V3 flash loans
- **Capabilities**: Triangular arbitrage, multi-hop paths, compound strategies
- **Features**: Smart contract deployment, simulation engine, strategy optimization
- **Target**: 10+ token paths that eliminate 95% of competition

### Message Relay (Existing Infrastructure)
- **Service**: Uses existing `services/relay_server/`
- **Integration**: DeFi opportunities broadcast via existing binary protocol
- **Benefits**: Unified message handling, shared infrastructure, consistent latency

## Integration with AlphaPulse

### Message Protocol
All DeFi services extend the existing 48-byte binary protocol with new message types:
- `ArbitrageOpportunity` (type 9): DeFi arbitrage opportunities
- `StatusUpdate` (type 10): Block numbers, gas prices, system status

### Data Flow
```
DEX Events → Scanner → Main Relay → Capital/Flash Loan Bots → Execution
                           ↓
                   Dashboard (WebSocket Bridge)
```
*Uses existing relay_server infrastructure for unified CEX + DeFi message flow*

### Shared Infrastructure
- **Protocol**: Extends `alphapulse-protocol` crate
- **Monitoring**: Prometheus metrics and Grafana dashboards
- **Logging**: Structured logging with tracing subscriber
- **Database**: PostgreSQL for execution tracking and analytics

## Development Workflow

### Building All Services
```bash
cd backend/services/defi
cargo build --release --workspace
```

### Running Individual Services
```bash
# Capital arbitrage bot
cargo run --bin capital-arbitrage

# Opportunity scanner  
cargo run --bin defi-scanner

# Flash loan bot
cargo run --bin flash-loan-bot

# DeFi relay
cargo run --bin defi-relay
```

### Testing
```bash
# Run all DeFi service tests
cargo test --workspace

# Test specific service
cargo test --package defi-scanner
```

### Service Management
```bash
# Start all DeFi services
./scripts/start-defi-services.sh

# Monitor service health
./scripts/monitor_defi_health.sh

# Stop all services
./scripts/stop.sh
```

## Configuration

### Environment Variables
- `ALCHEMY_API_KEY`: Polygon RPC access (required for mainnet)
- `PRIVATE_KEY`: Wallet private key for execution (secure storage required)
- `GAS_PRICE_GWEI`: Default gas price in Gwei
- `MIN_PROFIT_USD`: Minimum profit threshold for execution

### Network Configuration
- **Testnet**: Mumbai testnet for development and testing
- **Mainnet**: Polygon mainnet for production deployment
- **RPC Endpoints**: Alchemy, Infura, or local node

## Security Considerations

### Private Key Management
- Store private keys in secure environment variables or key management systems
- Use separate wallets for different environments (testnet/mainnet)
- Implement key rotation procedures

### Smart Contract Security
- Comprehensive testing and simulation before mainnet deployment
- External security audits for flash loan contracts
- Circuit breakers and emergency stop mechanisms

### Risk Management
- Position sizing limits to prevent excessive losses
- Gas price monitoring to avoid failed transactions
- Slippage protection and deadline enforcement

## Performance Targets

### Latency Requirements
- **Opportunity Detection**: <100ms from DEX event to relay broadcast
- **Capital Arbitrage**: <500ms from opportunity to transaction submission
- **Flash Loan**: <1000ms for complex multi-hop strategies

### Throughput Targets
- **Scanner**: Process 1000+ opportunities per second
- **Execution**: Handle 50+ concurrent arbitrage transactions
- **Relay**: Broadcast to 10+ execution agents simultaneously

### Success Metrics
- **Execution Success Rate**: >90% of validated opportunities
- **Profit Target**: $500+ daily profit from arbitrage
- **Uptime**: >99.9% service availability

## Getting Started

1. **Setup Environment**: Configure API keys and wallet access
2. **Build Services**: `cargo build --release --workspace`
3. **Start Infrastructure**: Run Polygon node and database
4. **Deploy Contracts**: Deploy flash loan contracts to testnet
5. **Start Services**: Use startup scripts to launch all services
6. **Monitor Performance**: Check dashboards and logs for operation

For detailed implementation guidance, see the documentation in `projects/defi/`.