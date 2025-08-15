# AlphaPulse Flash Loan Arbitrage Contracts

## Overview
Smart contracts for executing flash loan arbitrage on Polygon DEXs using Aave V3.

## Setup

1. **Install dependencies:**
```bash
npm install
```

2. **Configure environment:**
```bash
cp .env.example .env
# Edit .env with your private key and RPC URL
```

3. **Compile contracts:**
```bash
npm run compile
```

## Deployment

### Deploy to Polygon Mainnet
```bash
npm run deploy
```

### Deploy to Mumbai Testnet (for testing)
```bash
npm run deploy:mumbai
```

## Contract Architecture

The `FlashArbitrage` contract:
1. Receives arbitrage opportunities from the bot
2. Borrows funds via Aave V3 flash loan
3. Executes buy on cheaper DEX
4. Executes sell on expensive DEX
5. Repays flash loan + fee
6. Sends profit to owner

## Key Features
- **No capital required**: Uses flash loans for trading capital
- **Slippage protection**: 0.5% max slippage on trades
- **Owner-only execution**: Only bot owner can trigger trades
- **Emergency withdraw**: Can recover stuck funds

## Gas Optimization
- Single contract deployment (used for all trades)
- Optimized swap routing
- Minimal storage operations

## Security Considerations
- Owner-only functions
- Reentrancy protection via Aave
- Slippage limits
- Emergency functions

## Testing Checklist
1. Deploy to Mumbai testnet first
2. Test with small amounts ($100)
3. Verify gas costs match estimates
4. Check slippage handling
5. Test emergency withdraw

## Bot Integration
After deployment, the contract address will be saved to:
- `deployments/polygon-deployment.json`
- `../services/arbitrage_bot/config.json`

The bot will automatically use the deployed contract address.

## Monitoring
Track your contract on Polygonscan:
```
https://polygonscan.com/address/<YOUR_CONTRACT_ADDRESS>
```

## Estimated Costs
- Deployment: ~0.5 MATIC
- Per arbitrage execution: ~0.02-0.05 MATIC
- Flash loan fee: 0.05% of borrowed amount (Aave)