# Capital-Based Arbitrage Bot

A simpler, safer arbitrage bot that uses your own capital instead of flash loans. This approach is ideal for testing and debugging before moving to more complex flash loan strategies.

## Overview

This bot:
- Connects to the AlphaPulse relay server to receive arbitrage opportunities
- Uses your own wallet balance to execute trades
- Performs two-step arbitrage (buy low, sell high)
- Includes simulation mode for safe testing
- Provides detailed metrics and logging

## Key Differences from Flash Loan Bot

| Feature | Capital-Based | Flash Loan |
|---------|--------------|------------|
| Capital Required | Yes (your own funds) | No (borrowed) |
| Risk Level | Lower | Higher |
| Complexity | Simple (2 transactions) | Complex (1 atomic transaction) |
| Gas Costs | Higher (2 separate txs) | Lower (1 tx) |
| Profit Margin | Lower (due to capital limits) | Higher (unlimited capital) |
| Testing | Easier to debug | Harder to debug |

## Setup

1. **Install dependencies:**
```bash
cd backend/services/capital_arb_bot
cargo build
```

2. **Configure environment:**
```bash
cp .env.example .env
# Edit .env with your configuration
```

3. **Fund your wallet:**
- Transfer MATIC for gas fees
- Transfer trading tokens (USDC, WMATIC, etc.)

## Configuration

Key settings in `.env`:

- `PRIVATE_KEY`: Your wallet's private key (never commit!)
- `MIN_PROFIT_USD`: Minimum profit threshold (default: $5)
- `SIMULATION_MODE`: Run simulations before real trades (default: true)
- `MAX_TRADE_PERCENTAGE`: Max % of balance to use per trade (default: 50%)
- `SLIPPAGE_TOLERANCE`: Max acceptable slippage (default: 0.5%)

## Running the Bot

### Simulation Mode (Recommended for Testing)
```bash
SIMULATION_MODE=true cargo run
```

### Production Mode
```bash
SIMULATION_MODE=false cargo run
```

### With Custom Config
```bash
MIN_PROFIT_USD=10 MAX_TRADE_PERCENTAGE=0.3 cargo run
```

## How It Works

1. **Opportunity Detection:**
   - Receives arbitrage opportunities from Polygon collector via relay server
   - Filters by age, profit threshold, and available balance

2. **Simulation (if enabled):**
   - Queries DEX routers for expected outputs
   - Calculates expected profit after fees and slippage
   - Only proceeds if simulation shows profit

3. **Execution:**
   - Step 1: Buy token on cheaper DEX
   - Step 2: Sell token on expensive DEX
   - Calculate and log actual profit

4. **Safety Features:**
   - Maximum trade size limits
   - Slippage protection
   - Gas price checks
   - Opportunity age validation

## Testing

Run the test suite:
```bash
cargo test
```

Run integration tests (requires RPC connection):
```bash
cargo test -- --ignored
```

## Monitoring

The bot logs detailed metrics every 60 seconds:
- Opportunities received
- Opportunities simulated
- Opportunities executed
- Failed executions
- Total profit
- Success rate

## Profit Calculation

```
Gross Profit = (Sell Price - Buy Price) × Trade Size
Fees = (Buy Fee + Sell Fee) × Trade Size
Gas Cost = Gas Used × Gas Price × MATIC Price
Net Profit = Gross Profit - Fees - Gas Cost
```

## Common Issues

### Insufficient Balance
- Solution: Fund your wallet with trading tokens

### High Gas Prices
- Solution: Adjust `MAX_GAS_PRICE_GWEI` or wait for lower gas

### Low Success Rate
- Check `MIN_PROFIT_USD` threshold
- Verify `MAX_OPPORTUNITY_AGE_MS` setting
- Ensure good RPC connection

### Slippage Errors
- Increase `SLIPPAGE_TOLERANCE` (but reduces profit)
- Reduce `MAX_TRADE_PERCENTAGE` for less impact

## Migration to Flash Loans

Once comfortable with capital-based arbitrage:

1. Collect performance metrics
2. Identify profitable patterns
3. Deploy flash loan contract
4. Switch to flash loan bot

Benefits of starting with capital-based:
- Understand DEX dynamics
- Test profit calculations
- Debug execution logic
- Build confidence

## Security Considerations

- **Never commit private keys**
- Use a dedicated trading wallet
- Start with small amounts
- Monitor gas costs carefully
- Keep simulation mode on initially

## Performance Optimization

1. **Reduce Latency:**
   - Use private RPC endpoints
   - Colocate with validators
   - Optimize network routing

2. **Improve Success Rate:**
   - Fine-tune opportunity filters
   - Adjust slippage tolerance
   - Monitor DEX liquidity

3. **Maximize Profit:**
   - Optimize trade sizing
   - Bundle transactions when possible
   - Use MEV protection

## Next Steps

After successful testing:
1. Deploy flash loan contract (`../contracts/`)
2. Switch to flash loan bot (`../arbitrage_bot/`)
3. Scale up trading volume
4. Add more DEX integrations