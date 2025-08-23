# DeFi Scripts

Organized DeFi-related scripts for AlphaPulse arbitrage system.

## Directory Structure

### `/arbitrage/` - Arbitrage Detection & Execution
- `arb` - Main arbitrage scanner with sophisticated math (the golden reference)
- `*_arb*.py` - Various arbitrage strategies and scanners
- `verify_*.py` - Arbitrage opportunity validation scripts
- `execute_*.py` - Trade execution scripts
- `*usdc*.py` - USDC/USDC.e cross-token arbitrage

### `/deployment/` - Smart Contract Deployment
- `*.sol` - Flash loan and arbitrage smart contracts
- `deploy_*.py` - Deployment scripts for contracts
- `*Flash*.sol` - Various flash loan implementations

### `/analysis/` - Market Analysis & Research
- `analyze_*.py` - Market analysis and research tools
- `check_*.py` - Pool and token analysis scripts
- `*token*.py` - Token-specific analysis

### `/monitoring/` - System Monitoring & Health
- `*monitor*.py` - Real-time monitoring interfaces
- `*health*.sh` - Health check scripts
- `start-defi-services.sh` - Service startup

## Key Files

### Core Arbitrage
- **`arbitrage/arb`** - The definitive arbitrage scanner with precise math
- **`arbitrage/execute_arbitrage.py`** - Main execution engine
- **`arbitrage/verify_arbitrage.py`** - Opportunity validation

### Flash Loans
- **`deployment/FlashLoanArbitrage.sol`** - Main flash loan contract
- **`deployment/deploy_flash_arb.py`** - Contract deployment

### Analysis
- **`analysis/check_pools.py`** - Pool analysis and discovery
- **`analysis/analyze_dystopia.py`** - DEX-specific analysis

## Integration with Services

These scripts work with the main DeFi services:

```
Services:                    Scripts:
├── defi/scanner/       →   ├── arbitrage/ (detection)
├── defi/flash_loan/    →   ├── deployment/ (contracts)
├── defi/capital_arbitrage/ → analysis/ (research)
└── defi/arbitrage_bot/ →   └── monitoring/ (health)
```

## Usage

Most scripts expect to be run from the `/backend/scripts/` directory:

```bash
# Run main arbitrage scanner
cd backend/scripts
./defi/arbitrage/arb

# Deploy flash loan contracts
python defi/deployment/deploy_flash_arb.py

# Analyze pool liquidity
python defi/analysis/check_pools.py
```

## Migration Notes

This directory was created by consolidating 50+ scattered scripts from `/backend/scripts/` root. All DeFi functionality is now centralized in `/services/defi/` and `/scripts/defi/`.