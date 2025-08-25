#!/bin/bash
# E2E Pipeline: Polygon → Market Data Relay → Dashboard

echo "🚀 Starting E2E Pipeline..."

# Clean up old logs
rm -f /tmp/alphapulse/logs/*.log

# Start relays first
echo "📡 Starting relays..."
target/release/market_data_relay > /tmp/alphapulse/logs/market_data_relay.log 2>&1 &
target/release/signal_relay > /tmp/alphapulse/logs/signal_relay.log 2>&1 &
target/release/execution_relay > /tmp/alphapulse/logs/execution_relay.log 2>&1 &
sleep 2

# Start polygon collector
echo "🔷 Starting Polygon collector..."
target/release/polygon > /tmp/alphapulse/logs/polygon_collector.log 2>&1 &
sleep 2

# Start dashboard WebSocket server with debug logging
echo "📊 Starting dashboard WebSocket server..."
RUST_LOG=debug target/release/alphapulse-dashboard-websocket > /tmp/alphapulse/logs/dashboard_websocket.log 2>&1 &
sleep 2

echo "✅ E2E Pipeline started!"
echo ""
echo "📍 Dashboard: http://localhost:8080"
echo "📍 Frontend: Run 'npm run dev:dashboard' in frontend directory"
echo ""
echo "📋 Logs:"
echo "  tail -f /tmp/alphapulse/logs/polygon_collector.log"
echo "  tail -f /tmp/alphapulse/logs/market_data_relay.log"
echo "  tail -f /tmp/alphapulse/logs/dashboard_websocket.log"
echo ""
echo "To stop: pkill -f polygon && pkill -f relay && pkill -f dashboard"