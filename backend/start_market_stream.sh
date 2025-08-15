#!/bin/bash

echo "Building Rust services..."
cargo build --release

echo "Creating Unix socket directory..."
mkdir -p /tmp/alphapulse

echo "Starting relay server..."
./target/release/relay-server &
RELAY_PID=$!

sleep 2

echo "Starting WebSocket bridge..."
./target/release/ws-bridge &
WS_BRIDGE_PID=$!

sleep 1

echo "Starting Kraken collector..."
EXCHANGE_NAME=kraken ./target/release/exchange-collector &
KRAKEN_PID=$!

echo "Starting Coinbase collector..."
EXCHANGE_NAME=coinbase ./target/release/exchange-collector &
COINBASE_PID=$!

echo ""
echo "Services started:"
echo "  Relay Server PID: $RELAY_PID"
echo "  WebSocket Bridge PID: $WS_BRIDGE_PID"
echo "  Kraken Collector PID: $KRAKEN_PID"
echo "  Coinbase Collector PID: $COINBASE_PID"
echo ""
echo "WebSocket endpoint: ws://localhost:8765/stream"
echo "Metrics endpoint: http://localhost:9090/metrics"
echo ""
echo "To test: Open test_market_stream.html in a browser"
echo ""
echo "Press Ctrl+C to stop all services"

trap "echo 'Stopping services...'; kill $RELAY_PID $WS_BRIDGE_PID $KRAKEN_PID $COINBASE_PID 2>/dev/null; rm -f /tmp/alphapulse/*.sock; exit" INT TERM

wait