# AlphaPulse Arbitrage Bot - Production Setup

## 🚀 Quick Start

### 1. Test Mode (Recommended First)
```bash
# Set your private key
export PRIVATE_KEY="<your_private_key_here>"

# Run test script (finds and simulates one trade)
python3 test_arbitrage_execution.py
```

### 2. Production Mode (When Ready)

#### Option A: Python Bot (Simple)
```bash
# Configure
export PRIVATE_KEY="<your_private_key_here>"
export EXECUTE_TRADES=false  # Set to true when ready

# Run
python3 auto_arbitrage_bot.py
```

#### Option B: Rust Bot (Performance)
```bash
# Configure
export PRIVATE_KEY="<your_private_key_here>"
export EXECUTE_TRADES=false  # Set to true when ready

# Build and run
./start_arbitrage_bot.sh
```

## 📋 Prerequisites

1. **MATIC Balance**: Need ~10 MATIC for gas
2. **USDC Balance**: Need 10-100 USDC for trades (or use flash loans)
3. **Private Key**: Export as environment variable

## 🎯 Getting Your First Successful Trade

### Step 1: Get Test Funds
```bash
# Convert MATIC to USDC
python3 get_usdc.py

# Check balance
python3 -c "
from web3 import Web3
w3 = Web3(Web3.HTTPProvider('https://polygon-rpc.com'))
usdc = w3.eth.contract(
    address='0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174',
    abi='[{\"constant\":true,\"inputs\":[{\"name\":\"_owner\",\"type\":\"address\"}],\"name\":\"balanceOf\",\"outputs\":[{\"name\":\"balance\",\"type\":\"uint256\"}],\"type\":\"function\"}]'
)
balance = usdc.functions.balanceOf('YOUR_ADDRESS').call()
print(f'USDC Balance: {balance/1e6:.2f}')
"
```

### Step 2: Deploy Contracts
```bash
# Deploy flash loan arbitrage contract
python3 deploy_flash_loan_arbitrage.py

# Export contract address
export FLASH_LOAN_CONTRACT=0x...  # Address from deployment
```

### Step 3: Test with Small Trade
```bash
# Run test script
python3 test_arbitrage_execution.py

# When you see a profitable opportunity, it will show:
# 🎯 PROFITABLE OPPORTUNITY FOUND!
#    Profit: $1.23
# 
# In simulation mode, it shows what would be executed
# To execute real trades, edit the script and set:
# EXECUTE_REAL_TRADE = True
```

### Step 4: Go Live
```bash
# Edit test_arbitrage_execution.py
# Change: EXECUTE_REAL_TRADE = True

# Run again
python3 test_arbitrage_execution.py

# Watch for:
# ✅ Trade executed successfully!
```

## 🛡️ MEV Protection

### Option 1: High Gas (Simple)
The bot automatically uses 1.5x base gas price to avoid frontrunning

### Option 2: Flashbots (Better)
```bash
# For Polygon, use Marlin relay
export USE_FLASHBOTS=true
export FLASHBOTS_RELAY_URL=https://polygon-relay.marlin.org
```

### Option 3: Private RPC (Best)
```bash
# Use a private RPC endpoint
export POLYGON_RPC=wss://your-private-endpoint
```

## 📊 Monitoring

### Real-time Stats
The bot shows stats every minute:
```
📊 Statistics Report
  Runtime: 3600s
  Opportunities Found: 42
  Trades Executed: 5
  Success Rate: 80.0%
  Total Profit: $12.34
  Profit/Hour: $12.34
```

### Prometheus Metrics
Access metrics at http://localhost:9090/metrics

## 🔧 Configuration

### Environment Variables
```bash
# Required
export PRIVATE_KEY="<your_key>"

# Optional
export EXECUTE_TRADES=false       # true to execute real trades
export USE_FLASH_LOANS=true      # Use Aave flash loans
export MIN_PROFIT_USD=1.0        # Minimum profit to execute
export MAX_GAS_PRICE_GWEI=100    # Maximum gas price
export MAX_POSITION_SIZE_USD=10000  # Maximum trade size
```

### Config File (.env)
```env
PRIVATE_KEY="<your_key_here>"
EXECUTE_TRADES=false
USE_FLASH_LOANS=true
MIN_PROFIT_USD=1.0
MAX_GAS_PRICE_GWEI=100
```

## 🚨 Common Issues

### "No opportunities found"
- Market is efficient, spreads are tiny
- Try lowering MIN_PROFIT_USD to 0.50 for testing
- Check during high volatility periods

### "Gas too high"
- Polygon gas spikes during high activity
- Increase MAX_GAS_PRICE_GWEI
- Or wait for lower gas prices

### "Transaction reverted"
- Slippage too high - someone else took the opportunity
- Use MEV protection (Flashbots)
- Increase gas price for faster execution

### "Insufficient USDC"
- Get more USDC: `python3 get_usdc.py`
- Or enable flash loans: `USE_FLASH_LOANS=true`

## 📈 Expected Performance

| Metric | Value |
|--------|-------|
| Opportunities/Day | 20-50 |
| Success Rate | 60-80% |
| Avg Profit/Trade | $1-5 |
| Daily Profit | $20-100 |
| Required Capital | $100-1000 (or flash loans) |

## 🎯 Tips for Success

1. **Start Small**: Test with $10-50 trades first
2. **Monitor Gas**: High gas can eat profits
3. **Use Flash Loans**: No capital required
4. **MEV Protection**: Essential for consistent profits
5. **Fast RPC**: Use WebSocket, not HTTP
6. **Multiple DEXs**: More pools = more opportunities

## 🔒 Security

1. **Never share your private key**
2. **Use a dedicated arbitrage wallet**
3. **Keep minimal funds in hot wallet**
4. **Monitor for unusual activity**
5. **Test everything in simulation first**

## 📝 Next Steps

1. ✅ Get test execution working
2. ⬜ Deploy flash loan contract
3. ⬜ Enable MEV protection
4. ⬜ Scale up capital
5. ⬜ Add more DEXs
6. ⬜ Optimize gas usage

## 🆘 Support

Check logs for detailed errors:
```bash
# Python bot logs
tail -f arbitrage_executions.json

# Rust bot logs
RUST_LOG=debug cargo run

# Scanner output
./arb 2>&1 | tee scanner.log
```

## 🎉 Your First Profitable Trade

When you successfully execute your first profitable arbitrage:

1. **Screenshot the success** ✅
2. **Check the transaction on Polygonscan**
3. **Verify profit in your wallet**
4. **Scale up gradually**
5. **Consider flash loans for larger trades**

Good luck! 🚀