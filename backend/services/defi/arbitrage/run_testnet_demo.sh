#!/bin/bash

echo "=========================================="
echo "🚀 AlphaPulse Testnet Swap Demo"
echo "=========================================="
echo ""

# Check if private key is set
if [ -z "$TESTNET_PRIVATE_KEY" ]; then
    echo "❌ Error: TESTNET_PRIVATE_KEY environment variable not set"
    echo ""
    echo "📝 Setup instructions:"
    echo "  1. Get a testnet wallet private key"
    echo "  2. Export it: export TESTNET_PRIVATE_KEY=\"your_private_key_here\""
    echo "  3. Get test tokens from: https://faucet.polygon.technology/"
    echo ""
    exit 1
fi

echo "✅ Private key configured"
echo ""

# Function to run testnet command
run_testnet_cmd() {
    echo "🔄 Running: $1"
    echo "----------------------------------------"
    
    # Try to run the command, capture both stdout and stderr
    if cargo build --bin run-testnet-swaps --release 2>/dev/null; then
        if cargo run --bin run-testnet-swaps --release -- $1 2>&1; then
            echo "✅ Command completed successfully"
        else
            echo "❌ Command failed"
        fi
    else
        echo "⚠️ Binary not available - would run: cargo run --bin run-testnet-swaps -- $1"
    fi
    
    echo ""
}

echo "📊 Demo: Testnet Swap Execution Framework"
echo "=========================================="
echo ""

echo "1️⃣ Checking wallet balances on Mumbai testnet..."
run_testnet_cmd "balance --network mumbai"

echo "2️⃣ Simulating arbitrage cycle (WMATIC -> USDC -> WMATIC)..."
run_testnet_cmd "arbitrage --network mumbai --amount 0.05"

echo "3️⃣ Running comprehensive test suite..."
run_testnet_cmd "suite --network mumbai"

echo "=========================================="
echo "✅ Testnet demo completed!"
echo "=========================================="
echo ""

echo "💡 Available commands:"
echo "  cargo run --bin run-testnet-swaps -- balance --network mumbai"
echo "  cargo run --bin run-testnet-swaps -- swap --token-in WMATIC --token-out USDC --amount 0.1"
echo "  cargo run --bin run-testnet-swaps -- arbitrage --network mumbai --amount 0.1"
echo "  cargo run --bin run-testnet-swaps -- suite --network mumbai"
echo ""

echo "🔗 Useful links:"
echo "  Mumbai Faucet: https://faucet.polygon.technology/"
echo "  Mumbai Explorer: https://mumbai.polygonscan.com/"
echo "  QuickSwap (Mumbai): https://quickswap.exchange/"