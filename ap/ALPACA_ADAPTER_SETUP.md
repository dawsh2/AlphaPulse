# Alpaca Adapter Development Setup

## Current Approach (Not Recommended)
We've been copying files directly into the installed NautilusTrader package. This is fragile and will break on updates.

## Recommended Approach

### Option 1: Separate Package (Best for Long-term)
Create your adapter as a separate package that depends on NautilusTrader:

```bash
# Structure
alphapulse-nautilus-adapters/
├── setup.py
├── nautilus_adapters/
│   ├── __init__.py
│   └── alpaca/
│       ├── __init__.py
│       ├── config.py
│       ├── data.py
│       └── execution.py
└── tests/
```

Install in development mode:
```bash
pip install -e ./alphapulse-nautilus-adapters
```

Then import as:
```python
from nautilus_adapters.alpaca import AlpacaDataClient
```

### Option 2: Fork and Maintain (For Contributing Back)
1. Fork NautilusTrader on GitHub
2. Add your adapters to the fork
3. Keep fork synced with upstream:
```bash
git remote add upstream https://github.com/nautechsystems/nautilus_trader.git
git fetch upstream
git checkout develop
git merge upstream/develop
```

### Option 3: Local Development Plugin (Quick Development)
Use Python path manipulation:

```python
# In your scripts
import sys
sys.path.insert(0, '/Users/daws/alphapulse/ap')

# Now you can import your local adapter
from nautilus_trader.adapters.alpaca import AlpacaDataClient
```

## Maintaining Your Additions

### If Using Option 1 (Separate Package):
- Your code is completely independent
- Update NautilusTrader anytime without conflicts
- Your adapter uses NT as a dependency

### If Using Option 2 (Fork):
- Regular syncs with upstream
- Conflicts only in files you modified
- Can contribute improvements back

### If Using Option 3 (Local Path):
- Quick for development
- No installation needed
- Must ensure path is set in all scripts

## Migration Steps

1. **Remove from installed location:**
```bash
rm -rf /Users/daws/alphapulse/pulse-engine/venv/lib/python3.13/site-packages/nautilus_trader/adapters/alpaca
```

2. **Choose approach above**

3. **Update imports in your code**