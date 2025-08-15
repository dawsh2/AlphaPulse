#!/bin/bash

echo "🧪 Testing POL price calculation with clean environment"
echo "======================================================"

# Ensure clean environment
pkill -f "exchange_collector|relay_server|ws_bridge" 2>/dev/null || true
sleep 1

# Clean up sockets
rm -f /tmp/alphapulse/*.sock 2>/dev/null || true
mkdir -p /tmp/alphapulse

# Build fresh binaries
echo "📦 Building fresh binaries..."
cd /Users/daws/alphapulse/backend
cargo build --release --bin relay-server --bin exchange-collector

echo "🚀 Starting relay server..."
RUST_LOG=debug ./target/release/relay-server &
RELAY_PID=$!
sleep 2

echo "🔗 Starting ONLY Polygon collector..."
RUST_LOG=debug EXCHANGE_NAME=polygon ./target/release/exchange-collector &
POLYGON_PID=$!

echo "📊 Monitoring POL price calculations..."
echo "PIDs: Relay=$RELAY_PID, Polygon=$POLYGON_PID"
echo "Press Ctrl+C to stop"

# Set up cleanup trap
cleanup() {
    echo "🛑 Cleaning up..."
    kill $POLYGON_PID $RELAY_PID 2>/dev/null
    sleep 1
    pkill -f "exchange_collector|relay_server" 2>/dev/null
    exit 0
}
trap cleanup INT TERM

# Wait for processes
wait