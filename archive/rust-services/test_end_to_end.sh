#!/bin/bash

# End-to-end test for Tokio Transport integration
echo "🚀 Starting end-to-end test of Tokio Transport..."

# Clean up any existing processes
echo "🧹 Cleaning up existing processes..."
pkill -f alphapulse-api-server || true
pkill -f alphapulse-collectors || true
sleep 2

# Set environment variables
export RUST_LOG=info,alphapulse_common::tokio_transport=debug
export USE_TOKIO_TRANSPORT=true
export REDIS_URL=redis://localhost:6379

# Build everything first
echo "🔨 Building services..."
cargo build --package alphapulse-api-server
cargo build --package alphapulse-collectors

echo "📊 Starting API server with Tokio transport..."
./target/debug/alphapulse-api-server &
API_PID=$!
sleep 3

echo "💹 Starting collectors..."
./target/debug/alphapulse-collectors &
COLLECTOR_PID=$!
sleep 3

echo "🌐 Testing WebSocket connection..."
# Use websocat or curl to test the WebSocket
if command -v websocat &> /dev/null; then
    echo "Testing with websocat..."
    timeout 10 websocat ws://localhost:3000/ws &
    WS_PID=$!
    sleep 5
else
    echo "websocat not found, using curl test..."
    curl -i -N -H "Connection: Upgrade" -H "Upgrade: websocket" -H "Sec-WebSocket-Key: test" -H "Sec-WebSocket-Version: 13" http://localhost:3000/ws
fi

echo "📈 Checking logs for data flow..."
sleep 5

echo "🛑 Stopping services..."
kill $API_PID 2>/dev/null || true
kill $COLLECTOR_PID 2>/dev/null || true
kill $WS_PID 2>/dev/null || true

echo "✅ Test complete! Check the logs above for:"
echo "  - Tokio transport initialization"
echo "  - Trade writes from collector"
echo "  - Event-driven reads in API server"
echo "  - WebSocket broadcasts"