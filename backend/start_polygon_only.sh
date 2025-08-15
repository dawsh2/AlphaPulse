#!/bin/bash

# AlphaPulse Polygon-Only Startup Script
# This script starts only the Polygon collector to isolate debugging

echo "🚀 Starting AlphaPulse with ONLY Polygon collector..."
echo "This will allow you to see only Polygon data without Coinbase/Alpaca interference."

# Kill any existing processes
echo "🛑 Stopping any existing processes..."
pkill -f "exchange-collector"
pkill -f "relay-server" 
pkill -f "ws-bridge"

# Clean up any existing sockets
echo "🧹 Cleaning up Unix sockets..."
rm -f /tmp/alphapulse/*.sock
mkdir -p /tmp/alphapulse

# Start relay server first
echo "🔄 Starting relay server..."
cd /Users/daws/alphapulse/backend
cargo run --bin relay-server &
RELAY_PID=$!

# Give relay server time to start
sleep 2

# Start only Polygon collector
echo "📊 Starting Polygon collector ONLY..."
EXCHANGE_NAME=polygon cargo run --bin exchange-collector &
POLYGON_PID=$!

# Give collector time to connect
sleep 2

# Start WS bridge
echo "🌐 Starting WebSocket bridge..."
cd services/ws_bridge
cargo run --release &
WS_BRIDGE_PID=$!

echo ""
echo "✅ AlphaPulse running with ONLY Polygon collector!"
echo "📊 Services:"
echo "   - Relay Server (PID: $RELAY_PID)"
echo "   - Polygon Collector (PID: $POLYGON_PID)" 
echo "   - WebSocket Bridge (PID: $WS_BRIDGE_PID)"
echo ""
echo "🌐 Dashboard available at: http://localhost:3000"
echo "🔌 WebSocket available at: ws://127.0.0.1:8765/stream"
echo ""
echo "Press Ctrl+C to stop all services..."

# Function to cleanup on exit
cleanup() {
    echo ""
    echo "🛑 Stopping all services..."
    kill $RELAY_PID $POLYGON_PID $WS_BRIDGE_PID 2>/dev/null
    rm -f /tmp/alphapulse/*.sock
    echo "✅ All services stopped"
    exit 0
}

# Set trap to cleanup on Ctrl+C
trap cleanup INT

# Wait for any process to exit
wait