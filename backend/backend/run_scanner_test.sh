#!/bin/bash

echo "Starting AlphaPulse DeFi Scanner Test"
echo "======================================"
echo ""

# Build the scanner
echo "Building scanner..."
cargo build --release --package defi-scanner 2>/dev/null

# Run just the scanner (it will connect to relay if available)
echo "Starting scanner..."
echo ""
echo "The scanner will:"
echo "  • Process SwapEventMessages from DEX pools"
echo "  • Calculate optimal trade sizes using closed-form solutions"
echo "  • Show accurate slippage and gas costs"
echo "  • Display opportunities like this:"
echo ""
echo "🚀 ARBITRAGE OPPORTUNITY DETECTED!"
echo "═══════════════════════════════════════════════════════════════"
echo "📊 PAIR: WMATIC → USDC"
echo "───────────────────────────────────────────────────────────────"
echo "💱 PRICES:"
echo "   Buy:  QuickSwap @ \$0.254337"
echo "   Sell: SushiSwap @ \$0.256244"
echo "   Spread: 0.750%"
echo "───────────────────────────────────────────────────────────────"
echo "📈 TRADE SIZING:"
echo "   Optimal Size: \$5234.67"
echo "   Buy Slippage:  0.234%"
echo "   Sell Slippage: 0.156%"
echo "   Total Impact: 0.390%"
echo "───────────────────────────────────────────────────────────────"
echo "💰 PROFITABILITY:"
echo "   Gross Profit: \$39.26 (0.750%)"
echo "   Gas Cost:     \$0.50 (Huff optimized)"
echo "   Net Profit:   \$38.76"
echo "───────────────────────────────────────────────────────────────"
echo "🎯 EXECUTION:"
echo "   Confidence: 95.0%"
echo "   Block: #64752981"
echo "   Pools: 0x6e7a5F → 0xcd353F"
echo "═══════════════════════════════════════════════════════════════"
echo ""
echo "Running scanner now..."
echo ""

# Run the scanner
RUST_LOG=info ./backend/services/defi/scanner/target/release/defi-scanner