#!/bin/bash
# AlphaPulse DeFi Services Startup Script

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
BACKEND_DIR="$( cd "$SCRIPT_DIR/.." && pwd )"

echo "Starting AlphaPulse DeFi services..."

# Load environment variables if .env exists
if [ -f "$BACKEND_DIR/.env" ]; then
    echo "Loading environment variables from .env..."
    export $(cat "$BACKEND_DIR/.env" | grep -v '^#' | xargs)
fi

# Validate required environment variables
if [ -z "$ALCHEMY_API_KEY" ]; then
    echo "Warning: ALCHEMY_API_KEY not set - DeFi services will run in demo mode"
fi

if [ -z "$PRIVATE_KEY" ]; then
    echo "Warning: PRIVATE_KEY not set - Flash loan execution will be disabled"
fi

# Create necessary directories
mkdir -p /tmp/alphapulse
mkdir -p "$BACKEND_DIR/logs"

# Kill any existing DeFi services
echo "Cleaning up old DeFi processes..."
pkill -f "defi-scanner|flash-loan-bot|defi-relay|capital-arbitrage" 2>/dev/null || true
sleep 1

# Build DeFi services if not already built
if [ ! -f "$BACKEND_DIR/target/release/defi-scanner" ]; then
    echo "Building DeFi services..."
    cd "$BACKEND_DIR/services/defi"
    cargo build --release --workspace
    cd "$BACKEND_DIR"
fi

# Ensure main relay server is running (reuse existing infrastructure)
if ! pgrep -f "relay-server" > /dev/null; then
    echo "Starting main relay server..."
    ./target/release/relay-server > logs/relay-server.log 2>&1 &
    RELAY_PID=$!
    echo "Relay server started with PID $RELAY_PID"
    sleep 2
    
    # Wait for relay socket to be created
    while [ ! -S /tmp/alphapulse/relay.sock ]; do
        echo "Waiting for relay socket..."
        sleep 1
    done
else
    echo "Main relay server already running"
    RELAY_PID=$(pgrep -f "relay-server")
fi

# Start DeFi Scanner (opportunity detection)
echo "Starting DeFi scanner..."
RUST_LOG=defi_scanner=info ./target/release/defi-scanner > logs/defi-scanner.log 2>&1 &
SCANNER_PID=$!
echo "DeFi scanner started with PID $SCANNER_PID"
sleep 2

# Start Capital Arbitrage Bot (simple strategies)
echo "Starting capital arbitrage bot..."
./target/release/capital-arbitrage > logs/capital-arbitrage.log 2>&1 &
CAPITAL_PID=$!
echo "Capital arbitrage bot started with PID $CAPITAL_PID"
sleep 2

# Start Flash Loan Bot (advanced strategies) - only if private key is available
if [ -n "$PRIVATE_KEY" ]; then
    echo "Starting flash loan bot..."
    ./target/release/flash-loan-bot > logs/flash-loan-bot.log 2>&1 &
    FLASH_PID=$!
    echo "Flash loan bot started with PID $FLASH_PID"
else
    echo "Skipping flash loan bot (PRIVATE_KEY not set)"
    FLASH_PID=""
fi

# Save PIDs for shutdown script
cat > "$BACKEND_DIR/scripts/defi-pids.txt" <<EOF
RELAY_PID=$RELAY_PID
SCANNER_PID=$SCANNER_PID
CAPITAL_PID=$CAPITAL_PID
FLASH_PID=$FLASH_PID
EOF

echo ""
echo "DeFi services started successfully!"
echo "Logs available in: $BACKEND_DIR/logs/"
echo ""

# Show running processes
echo "Service status:"
ps aux | grep -E "defi-scanner|flash-loan-bot|relay-server|capital-arbitrage" | grep -v grep

echo ""
echo "Service endpoints:"
echo "  - Main Relay: Unix socket /tmp/alphapulse/relay.sock"
echo "  - DeFi Scanner: Integrated with main relay"
echo "  - Metrics: http://localhost:9091/metrics"

echo ""
echo "Integration with existing services:"
echo "  - CEX Data: Relay server at /tmp/alphapulse/relay.sock"
echo "  - Dashboard: WebSocket bridge at ws://localhost:8765/stream"
echo "  - API: FastAPI at http://localhost:8000"

echo ""
echo "To stop DeFi services: $SCRIPT_DIR/stop-defi-services.sh"
echo "To monitor health: $SCRIPT_DIR/monitor_defi_health.sh"

echo ""
echo "Getting started:"
echo "1. Monitor logs: tail -f logs/defi-scanner.log"
echo "2. Check opportunities: curl http://localhost:9091/metrics | grep arbitrage"
echo "3. View dashboard: Open frontend with DeFi arbitrage component"

if [ -z "$PRIVATE_KEY" ]; then
    echo ""
    echo "⚠️  To enable flash loan execution:"
    echo "   export PRIVATE_KEY='your_wallet_private_key'"
    echo "   $SCRIPT_DIR/restart-defi-services.sh"
fi