# AlphaPulse Directory Cleanup Plan

## Current Structure Analysis

### 1. **pulse-engine/** 
- **Status**: Can be removed
- **Reason**: Contains Flask backend that we're replacing with NautilusTrader
- **Action**: Copy any unique code to `ap/` first, then delete

### 2. **nautilus-trader/** 
- **Status**: Git submodule - Should be removed
- **Reason**: We don't need a local copy since we use the pip package
- **Action**: Remove submodule, use installed package instead

### 3. **ap/** 
- **Status**: Keep - This is our main working directory
- **Contents**: 
  - Alpaca adapter development
  - Data fetching scripts
  - Catalog with stored data
  - Our custom adapters package

### 4. **ui/**
- **Status**: Keep
- **Contents**: Frontend web interface

### 5. **venv/**
- **Status**: Can be removed
- **Reason**: Old virtual environment, we're using pulse-engine/venv

## Recommended Final Structure

```
alphapulse/
├── .git/
├── .gitignore
├── .env
├── README.md
├── CLAUDE.md
├── ap/                           # Main development directory
│   ├── alphapulse-adapters/     # Custom NT adapters
│   ├── catalog/                  # ParquetDataCatalog storage
│   ├── examples/                 # Example scripts
│   ├── strategies/               # Trading strategies (to be created)
│   └── *.py                      # Utility scripts
├── ui/               # Frontend
└── docs/                         # Documentation

```

## For NautilusTrader Development

You do NOT need a local copy of NT for:
- Using existing functionality
- Creating custom adapters
- Writing strategies
- Running backtests

You WOULD need NT source only if:
- Contributing to NT core
- Debugging NT internals
- Modifying NT behavior

## Strategy Development

Strategies in NT are just Python classes that:
1. Inherit from `Strategy` base class
2. Implement trading logic
3. Can live in your own codebase

Example structure:
```python
# ap/strategies/my_strategy.py
from nautilus_trader.trading.strategy import Strategy

class MyStrategy(Strategy):
    def on_start(self):
        # Strategy logic here
        pass
```

## Cleanup Commands

```bash
# 1. Remove git submodule
git submodule deinit -f nautilus-trader
git rm -f nautilus-trader
rm -rf .git/modules/nautilus-trader

# 2. Remove pulse-engine (after verifying no unique code)
rm -rf pulse-engine/

# 3. Remove old venv
rm -rf venv/

# 4. Update .gitignore
echo "catalog/" >> .gitignore  # Don't commit market data
```