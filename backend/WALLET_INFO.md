# 🔐 Your AlphaPulse Trading Wallet

## Wallet Details
- **Address**: `0x63587ab424AD1bfc493D423A032537274a5251c7`
- **Network**: Polygon (Chain ID: 137)
- **Created**: 2025-08-15

## ⚠️ IMPORTANT SECURITY NOTES
- Your private key and mnemonic are stored in `.env` files (gitignored)
- Backup is in `backend/.wallet-backups/` (also gitignored)
- **NEVER** share your private key or mnemonic with anyone
- **NEVER** commit these to git

## 📋 Next Steps to Start Trading

### 1. Fund Your Wallet

You need two things:
1. **MATIC for gas fees** (~5 MATIC recommended, ~$4)
2. **USDC for trading** (start with $10-50 for testing)

### 2. Send from Coinbase

**For MATIC:**
1. Buy MATIC on Coinbase (if you don't have any)
2. Go to Send → Enter address: `0x63587ab424AD1bfc493D423A032537274a5251c7`
3. Select "Polygon" network
4. Send 5-10 MATIC

**For USDC:**
1. Go to Send → Enter address: `0x63587ab424AD1bfc493D423A032537274a5251c7`
2. **CRITICAL**: Select "Polygon" network (NOT Ethereum!)
3. Start with small test amount ($10)
4. Once confirmed, send trading amount

### 3. Check Your Balance

**JavaScript version** (quick check):
```bash
cd backend/scripts
node check-balance.js
```

**Rust version** (integrated with bot):
```bash
cd backend/services/capital_arb_bot
cargo run --bin check-balance
```

### 4. Start Trading Bot (Simulation Mode)

```bash
cd backend/services/capital_arb_bot
cargo run

# The bot will:
# - Connect to relay server
# - Receive arbitrage opportunities
# - Simulate trades (SIMULATION_MODE=true)
# - Show potential profits
```

### 5. Go Live (After Testing)

Edit `backend/services/capital_arb_bot/.env`:
```bash
SIMULATION_MODE=false  # Change from true
MIN_PROFIT_USD=10.0    # Increase threshold
MAX_TRADE_PERCENTAGE=0.1  # Start conservative (10% of balance)
```

## 🔍 Monitor Your Wallet

- **Polygonscan**: https://polygonscan.com/address/0x63587ab424AD1bfc493D423A032537274a5251c7
- **Check balance**: `cargo run --bin check-balance`
- **View trades**: Check Polygonscan transaction history

## 📊 Trading Strategy

1. **Start Small**: Test with $10-50 USDC
2. **Monitor Gas**: Keep 5+ MATIC for gas fees
3. **Watch Metrics**: Bot shows success rate and profits
4. **Scale Gradually**: Increase trade size as confidence grows

## 🚨 Troubleshooting

**"Insufficient MATIC balance"**
- Send more MATIC from Coinbase

**"No arbitrage opportunities"**
- Normal during low volatility
- Check that all services are running:
  ```bash
  cd backend
  ./start_all_services.sh
  ```

**"Transaction failed"**
- Check gas price settings
- Verify token balances
- Review slippage settings

## 💡 Tips

- Always keep SIMULATION_MODE=true initially
- Start with small amounts until familiar
- Monitor gas costs vs profits
- Use the dashboard to visualize opportunities
- Keep private keys secure!

---

**Remember**: This is real money on a real blockchain. Start small, test thoroughly, and scale gradually!