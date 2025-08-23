# Unified Arbitrage Service

## Overview

The unified arbitrage service combines the best of flash loan and capital-based arbitrage strategies. It defaults to **flash loans** for capital efficiency but can fallback to **capital-based** execution for testing and small opportunities.

## Key Features

- **Flash Loan First**: Primary execution mode using Aave V3
- **Capital Fallback**: Testing mode using wallet balance
- **Compound Arbitrage**: 10+ token paths for competitive advantage
- **Multi-DEX Support**: Uniswap V2/V3, SushiSwap, QuickSwap
- **Real-time Execution**: Sub-second opportunity capture
- **Risk Management**: Simulation, slippage protection, position limits

## Architecture

```
Relay Opportunities → Strategy Selection → Simulation → Execution Mode → Profit
                           ↓                 ↓            ↓              ↓
                    Simple/Compound      Pre-validate   Flash/Capital   Capture
```

## Execution Modes

### Flash Loan Mode (Default)
- **Capital Required**: $0 (borrowed from Aave)
- **Max Trade Size**: Unlimited (subject to Aave liquidity)
- **Cost**: 0.09% flash loan fee
- **Atomicity**: Single transaction (all-or-nothing)
- **Best For**: Large opportunities, compound arbitrage

### Capital Mode (Fallback)
- **Capital Required**: Your wallet balance
- **Max Trade Size**: Limited by holdings
- **Cost**: No flash loan fee
- **Atomicity**: Multiple transactions
- **Best For**: Testing, small opportunities, debugging

## Strategy Types

### 1. Simple Arbitrage
Basic 2-DEX price difference exploitation:
```
USDC → WMATIC (QuickSwap) → USDC (Uniswap V3)
```

### 2. Triangular Arbitrage
3-token cycles within single DEX or across DEXs:
```
USDC → WMATIC → USDT → USDC
```

### 3. Compound Arbitrage (Primary Edge)
Complex 10+ token paths that eliminate 95% of competition:
```
USDC → WETH → WMATIC → DAI → USDT → LINK → AAVE → CRV → COMP → UNI → SUSHI → USDC
```

## Configuration

### Environment Variables
```bash
# Execution mode (flash_loan or capital)
EXECUTION_MODE=flash_loan

# Flash loan settings
AAVE_POOL_ADDRESS=0x794a61358D6845594F94dc1DB02A252b5b4814aD
FLASH_LOAN_FEE=0.0009  # 0.09%

# Capital mode settings
PRIVATE_KEY="<your_private_key_here>"
MAX_CAPITAL_PERCENTAGE=0.5  # Use max 50% of balance

# Strategy settings
MIN_PROFIT_USD=10
MIN_PROFIT_PERCENTAGE=0.005  # 0.5%
MAX_GAS_COST_USD=5
SIMULATION_REQUIRED=true

# Compound arbitrage
COMPOUND_ENABLED=true
MAX_TOKEN_PATH_LENGTH=15
MIN_COMPOUND_PROFIT_USD=50
```

### Strategy Thresholds
```toml
[simple]
min_profit_usd = 5
max_slippage = 0.005  # 0.5%

[triangular]
min_profit_usd = 15
max_slippage = 0.01   # 1.0%

[compound]
min_profit_usd = 50
max_slippage = 0.02   # 2.0%
min_confidence = 0.8
```

## Running the Service

### Production Mode (Flash Loans)
```bash
cd backend/services/defi/arbitrage
EXECUTION_MODE=flash_loan cargo run --bin arbitrage-bot
```

### Testing Mode (Capital)
```bash
EXECUTION_MODE=capital SIMULATION_REQUIRED=true cargo run --bin arbitrage-bot
```

### Compound Arbitrage Only
```bash
cargo run --bin compound-arb
```

### Balance Checker
```bash
cargo run --bin balance-check
```

## Service Components

### Core (`src/lib.rs`)
- `ArbitrageEngine`: Main orchestration logic
- `OpportunityHandler`: Processes incoming opportunities
- `StrategySelector`: Chooses optimal strategy for each opportunity

### Execution (`src/execution/`)
- `FlashLoanExecutor`: Aave V3 flash loan execution
- `CapitalExecutor`: Wallet balance execution
- `ContractDeployer`: Smart contract management

### Strategies (`src/strategies/`)
- `SimpleStrategy`: Basic 2-DEX arbitrage
- `TriangularStrategy`: 3-token cycles
- `CompoundStrategy`: 10+ token paths (key differentiator)

### Simulation (`src/simulation/`)
- `PathSimulator`: Pre-execution validation
- `SlippageCalculator`: Impact estimation
- `ProfitProjector`: ROI calculations

## Smart Contracts

Uses contracts from `../contracts/`:
- `FlashArbitrage.sol`: Advanced Aave V3 integration
- `SimpleArbitrage.sol`: Capital-based execution

## Performance Targets

### Latency
- **Opportunity Detection**: <100ms from DEX event
- **Strategy Selection**: <10ms
- **Simulation**: <50ms
- **Flash Loan Execution**: <2 blocks (4 seconds)
- **Capital Execution**: <4 blocks (8 seconds)

### Success Metrics
- **Execution Success Rate**: >90%
- **Daily Profit Target**: $500+
- **Compound Arbitrage Share**: >60% of profits
- **Gas Efficiency**: <$2 per profitable trade

## Competitive Advantages

### Technical Moats
1. **Compound Arbitrage**: 10+ token path complexity
2. **Flash Loan Optimization**: Capital-free scaling
3. **Multi-Strategy Engine**: Adaptive opportunity capture
4. **Real-time Simulation**: High success rates
5. **Protocol Integration**: Unified message handling

### Economic Benefits
- **No Capital Requirements**: Flash loans eliminate funding needs
- **Unlimited Scaling**: Trade size limited only by DEX liquidity
- **Lower Costs**: 0.09% flash fee < rebalancing costs
- **Higher Profits**: Complex strategies yield 3-10x returns

## Migration Guide

### From Capital Arbitrage
1. Update imports: `use arbitrage::*` instead of `capital_arbitrage::*`
2. Change execution mode: Set `EXECUTION_MODE=flash_loan`
3. Deploy contracts: Use `ContractDeployer` for flash loan contracts
4. Update thresholds: Increase minimum profits for flash loan costs

### From Flash Loan Service
1. Strategy integration: Move custom strategies to `src/strategies/`
2. Aave client: Now part of `FlashLoanExecutor`
3. Configuration: Consolidate config files

## Security Considerations

### Flash Loan Risks
- **Contract Bugs**: Thorough testing and auditing required
- **MEV Attacks**: Use private mempools for large trades
- **Slippage**: Conservative estimates and deadline protection

### Capital Mode Risks
- **Private Key Security**: Never commit keys, use secure storage
- **Balance Management**: Monitor for unexpected drains
- **Gas Costs**: Set appropriate limits for profitability

## Testing

### Unit Tests
```bash
cargo test
```

### Integration Tests
```bash
cargo test --ignored
```

### Simulation Tests
```bash
SIMULATION_REQUIRED=true cargo test simulation
```

### Mainnet Validation
```bash
EXECUTION_MODE=capital cargo run --bin simple-arb -- --dry-run
```

## Monitoring

### Metrics Exported
- `arbitrage_opportunities_total`
- `arbitrage_executions_total`
- `arbitrage_profit_usd_total`
- `arbitrage_gas_cost_usd_total`
- `arbitrage_success_rate`

### Health Checks
- Contract deployment status
- Aave pool liquidity
- Wallet balance sufficiency
- DEX connectivity

## Next Steps

1. **Deploy Production**: Start with simple arbitrage, validate execution
2. **Enable Compound**: Gradually increase path complexity
3. **Scale Capital**: Add more DEXs and token pairs
4. **Optimize Performance**: Profile and optimize hot paths
5. **Add MEV Protection**: Integrate Flashbots or similar