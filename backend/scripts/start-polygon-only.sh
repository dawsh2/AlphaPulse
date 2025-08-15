#!/bin/bash
# AlphaPulse Polygon-Only Service Startup Script

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
BACKEND_DIR="$( cd "$SCRIPT_DIR/.." && pwd )"

echo "Starting AlphaPulse Polygon-only services..."

# Create necessary directories
mkdir -p /tmp/alphapulse
mkdir -p "$BACKEND_DIR/logs"

# Kill any existing services
echo "Cleaning up old processes..."
pkill -f "relay-server|exchange-collector|ws-bridge" 2>/dev/null || true
sleep 1

# Start relay server first (it creates the socket)
echo "Starting relay server..."
cd "$BACKEND_DIR"
./target/release/relay-server > logs/relay-server.log 2>&1 &
RELAY_PID=$!
echo "Relay server started with PID $RELAY_PID"
sleep 2

# Wait for relay socket to be created
while [ ! -S /tmp/alphapulse/relay.sock ]; do
    echo "Waiting for relay socket..."
    sleep 1
done

# Start ONLY Polygon collector with environment API key
if [ -n "$ALCHEMY_API_KEY" ]; then
    echo "Starting Polygon collector (Alchemy WebSocket with API key)..."
    EXCHANGE_NAME=polygon ./target/release/exchange-collector > logs/polygon-collector.log 2>&1 &
else
    echo "Starting Polygon collector (demo mode - no real-time events)..."
    ALCHEMY_API_KEY=demo EXCHANGE_NAME=polygon ./target/release/exchange-collector > logs/polygon-collector.log 2>&1 &
fi
POLYGON_PID=$!
echo "Polygon collector started with PID $POLYGON_PID"

sleep 2

# Start WebSocket bridge
echo "Starting WebSocket bridge..."
./target/release/ws-bridge > logs/ws-bridge.log 2>&1 &
WS_PID=$!
echo "WebSocket bridge started with PID $WS_PID"

# Save PIDs for shutdown script
cat > "$BACKEND_DIR/scripts/pids.txt" <<EOF
RELAY_PID=$RELAY_PID
POLYGON_PID=$POLYGON_PID
WS_PID=$WS_PID
EOF

echo ""
echo "Polygon-only services started successfully!"
echo "Logs available in: $BACKEND_DIR/logs/"
echo ""
echo "Service status:"
ps aux | grep -E "relay-server|exchange-collector|ws-bridge" | grep -v grep

echo ""
echo "WebSocket endpoint: ws://localhost:8765/stream"
echo "To stop services, run: $SCRIPT_DIR/stop.sh"
echo ""
echo "Note: Using OnFinality free endpoint (88ms latency) to avoid rate limits"