#!/bin/bash
# Start all three domain relay services for live arbitrage pipeline

set -e

echo "🚀 Starting AlphaPulse Domain Relay Services"
echo ""

# Create relay directory
mkdir -p /tmp/alphapulse

# Build relay binaries
echo "🔨 Building relay services..."
cd "$(dirname "$0")/.."
cargo build --release --bin start_market_data_relay 2>/dev/null || {
    echo "   Building market data relay from scripts/"
    rustc --edition 2021 -L target/release/deps scripts/start_market_data_relay.rs -o target/release/start_market_data_relay --extern tokio=target/release/deps/libtokio*.rlib --extern tracing=target/release/deps/libtracing*.rlib
}

cargo build --release --bin start_signal_relay 2>/dev/null || {
    echo "   Building signal relay from scripts/"
    rustc --edition 2021 -L target/release/deps scripts/start_signal_relay.rs -o target/release/start_signal_relay --extern tokio=target/release/deps/libtokio*.rlib --extern tracing=target/release/deps/libtracing*.rlib
}

cargo build --release --bin start_execution_relay 2>/dev/null || {
    echo "   Building execution relay from scripts/"
    rustc --edition 2021 -L target/release/deps scripts/start_execution_relay.rs -o target/release/start_execution_relay --extern tokio=target/release/deps/libtokio*.rlib --extern tracing=target/release/deps/libtracing*.rlib
}

echo "✅ Relay binaries built"
echo ""

# Function to check if socket exists
check_socket() {
    if [[ -S "$1" ]]; then
        echo "✅ $1 ready"
        return 0
    else
        echo "❌ $1 not ready"
        return 1
    fi
}

# Start MarketDataRelay (Domain 1) 
echo "📡 Starting MarketDataRelay..."
RUST_LOG=info cargo run --release --bin start_market_data_relay > /tmp/alphapulse/market_data_relay.log 2>&1 &
MARKET_DATA_PID=$!
echo "   PID: $MARKET_DATA_PID"

# Start SignalRelay (Domain 2)
echo "🔔 Starting SignalRelay..."
RUST_LOG=info cargo run --release --bin start_signal_relay > /tmp/alphapulse/signal_relay.log 2>&1 &
SIGNAL_PID=$!
echo "   PID: $SIGNAL_PID"

# Start ExecutionRelay (Domain 3)
echo "⚡ Starting ExecutionRelay..."
RUST_LOG=info cargo run --release --bin start_execution_relay > /tmp/alphapulse/execution_relay.log 2>&1 &
EXECUTION_PID=$!
echo "   PID: $EXECUTION_PID"

echo ""
echo "⏳ Waiting for relay services to initialize..."
sleep 3

# Check if all sockets are ready
echo ""
echo "🔍 Checking relay status:"
check_socket "/tmp/alphapulse/market_data.sock"
check_socket "/tmp/alphapulse/signals.sock" 
check_socket "/tmp/alphapulse/execution.sock"

echo ""
echo "💾 Process IDs:"
echo "   MarketDataRelay: $MARKET_DATA_PID"
echo "   SignalRelay: $SIGNAL_PID"
echo "   ExecutionRelay: $EXECUTION_PID"

# Save PIDs for cleanup
echo "$MARKET_DATA_PID" > /tmp/alphapulse/market_data.pid
echo "$SIGNAL_PID" > /tmp/alphapulse/signal.pid
echo "$EXECUTION_PID" > /tmp/alphapulse/execution.pid

echo ""
echo "📋 Log files:"
echo "   MarketData: /tmp/alphapulse/market_data_relay.log"
echo "   Signal: /tmp/alphapulse/signal_relay.log"
echo "   Execution: /tmp/alphapulse/execution_relay.log"

echo ""
echo "✅ All domain relay services started!"
echo "   MarketDataRelay ready for Polygon publisher"
echo "   SignalRelay ready for flash arbitrage strategy"
echo "   ExecutionRelay ready for execution commands"
echo ""
echo "🎯 Next: Start Polygon publisher and flash arbitrage strategy"