#!/bin/bash

# Kill any existing services
pkill -f "exchange-collector|relay-server|ws-bridge"
sleep 1

# Start relay server with debug logging
echo "Starting relay server..."
RUST_LOG=debug ./target/release/relay-server 2>&1 | grep -E "L2|snapshot" &
RELAY_PID=$!
sleep 2

# Start collector which will send snapshots
echo "Starting coinbase collector..."
RUST_LOG=debug EXCHANGE_NAME=coinbase ./target/release/exchange-collector 2>&1 | grep -E "L2|snapshot" &
COLLECTOR_PID=$!

# Wait and capture output
sleep 10

# Kill processes
kill $RELAY_PID $COLLECTOR_PID 2>/dev/null
pkill -f "exchange-collector|relay-server"

echo "Test complete"