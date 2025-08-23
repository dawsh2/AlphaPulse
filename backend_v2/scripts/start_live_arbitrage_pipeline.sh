#!/bin/bash
# Complete startup script for live arbitrage detection pipeline
# This script starts all services in the correct order for end-to-end live data flow

set -e

echo "🚀 Starting AlphaPulse Live Arbitrage Detection Pipeline"
echo "════════════════════════════════════════════════════════"
echo ""

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Create directories
echo "📁 Creating necessary directories..."
mkdir -p /tmp/alphapulse
mkdir -p /tmp/alphapulse/logs

# Function to check if service is ready
wait_for_service() {
    local socket_path=$1
    local service_name=$2
    local max_attempts=30
    local attempt=0
    
    echo -n "⏳ Waiting for $service_name to be ready..."
    while [[ $attempt -lt $max_attempts ]]; do
        if [[ -S "$socket_path" ]]; then
            echo -e " ${GREEN}✅ Ready${NC}"
            return 0
        fi
        sleep 1
        attempt=$((attempt + 1))
        echo -n "."
    done
    echo -e " ${RED}❌ Failed${NC}"
    return 1
}

# Function to start service in background
start_service() {
    local service_path=$1
    local service_name=$2
    local log_file=$3
    local pid_file=$4
    
    echo "🔄 Starting $service_name..."
    cd /Users/daws/alphapulse/backend_v2
    
    # Check if this is a relay service
    if [[ "$service_path" == *"relay"* ]]; then
        RUST_LOG=info cargo run --release -p alphapulse-relays --bin "$service_path" > "$log_file" 2>&1 &
    else
        RUST_LOG=info cargo run --release --bin "$service_path" > "$log_file" 2>&1 &
    fi
    
    local pid=$!
    echo "$pid" > "$pid_file"
    echo "   PID: $pid, Log: $log_file"
}

# Step 1: Start Domain Relay Services
echo -e "${BLUE}Step 1: Domain Relay Services${NC}"
echo "─────────────────────────────"

# Check if relays are already running
if [[ -S "/tmp/alphapulse/market_data.sock" ]] && [[ -S "/tmp/alphapulse/signals.sock" ]] && [[ -S "/tmp/alphapulse/execution.sock" ]]; then
    echo "✅ All relay services already running"
else
    echo "Starting relay services..."
    cd /Users/daws/alphapulse/backend_v2/scripts
    
    # Start MarketDataRelay
    start_service "market_data_relay" "MarketDataRelay" "/tmp/alphapulse/logs/market_data_relay.log" "/tmp/alphapulse/market_data.pid"
    
    # Start SignalRelay  
    start_service "signal_relay" "SignalRelay" "/tmp/alphapulse/logs/signal_relay.log" "/tmp/alphapulse/signal.pid"
    
    # Start ExecutionRelay
    start_service "execution_relay" "ExecutionRelay" "/tmp/alphapulse/logs/execution_relay.log" "/tmp/alphapulse/execution.pid"
    
    # Wait for all relays to be ready
    wait_for_service "/tmp/alphapulse/market_data.sock" "MarketDataRelay" || exit 1
    wait_for_service "/tmp/alphapulse/signals.sock" "SignalRelay" || exit 1  
    wait_for_service "/tmp/alphapulse/execution.sock" "ExecutionRelay" || exit 1
fi

echo ""

# Step 2: Start Polygon Publisher (Real DEX Data)
echo -e "${BLUE}Step 2: Polygon DEX Publisher${NC}"
echo "─────────────────────────────"
echo "📡 Connecting to live Polygon blockchain..."
echo "   Subscribes to real DEX events via WebSocket"
echo "   Forwards TLV messages to MarketDataRelay"

cd /Users/daws/alphapulse/backend_v2/services_v2/adapters
start_service "polygon_publisher" "Polygon DEX Publisher" "/tmp/alphapulse/logs/polygon_publisher.log" "/tmp/alphapulse/polygon_publisher.pid"

# Give publisher time to establish WebSocket connection
echo "⏳ Waiting for Polygon blockchain connection (15 seconds)..."
sleep 15

echo ""

# Step 3: Start Flash Arbitrage Strategy
echo -e "${BLUE}Step 3: Flash Arbitrage Strategy${NC}"
echo "──────────────────────────────"
echo "🎯 Starting arbitrage opportunity detector..."
echo "   Consumes market data from MarketDataRelay"  
echo "   Detects arbitrage opportunities with native precision"
echo "   Sends signals to SignalRelay"

cd /Users/daws/alphapulse/backend_v2/services_v2/strategies/flash_arbitrage
start_service "alphapulse-flash-arbitrage" "Flash Arbitrage Strategy" "/tmp/alphapulse/logs/flash_arbitrage.log" "/tmp/alphapulse/flash_arbitrage.pid"

# Give strategy time to connect and subscribe
echo "⏳ Waiting for arbitrage strategy initialization (10 seconds)..."
sleep 10

echo ""

# Step 4: Start Dashboard WebSocket Server  
echo -e "${BLUE}Step 4: Dashboard WebSocket Server${NC}"
echo "────────────────────────────────"
echo "📊 Starting dashboard server..."
echo "   Subscribes to MarketDataRelay for pool state"
echo "   Subscribes to SignalRelay for arbitrage opportunities" 
echo "   Provides WebSocket API for frontend dashboard"

cd /Users/daws/alphapulse/backend_v2/services_v2/dashboard/websocket_server
start_service "alphapulse-dashboard-websocket" "Dashboard WebSocket Server" "/tmp/alphapulse/logs/dashboard_websocket.log" "/tmp/alphapulse/dashboard_websocket.pid"

# Wait for dashboard to be ready
echo "⏳ Waiting for dashboard server (5 seconds)..."
sleep 5

echo ""

# Step 5: Pipeline Status & Verification
echo -e "${BLUE}Step 5: Pipeline Status${NC}"
echo "───────────────────────"

echo -e "${GREEN}✅ Live Arbitrage Detection Pipeline Started!${NC}"
echo ""
echo "📋 Service Status:"
echo "   📡 MarketDataRelay: $(test -S /tmp/alphapulse/market_data.sock && echo '✅ Running' || echo '❌ Failed')"
echo "   🔔 SignalRelay: $(test -S /tmp/alphapulse/signals.sock && echo '✅ Running' || echo '❌ Failed')"
echo "   ⚡ ExecutionRelay: $(test -S /tmp/alphapulse/execution.sock && echo '✅ Running' || echo '❌ Failed')"
echo "   🌐 Polygon Publisher: $(test -f /tmp/alphapulse/polygon_publisher.pid && echo '✅ Running' || echo '❌ Failed')"
echo "   🎯 Flash Arbitrage: $(test -f /tmp/alphapulse/flash_arbitrage.pid && echo '✅ Running' || echo '❌ Failed')"
echo "   📊 Dashboard Server: $(test -f /tmp/alphapulse/dashboard_websocket.pid && echo '✅ Running' || echo '❌ Failed')"

echo ""
echo "📊 Live Data Flow:"
echo "   Polygon DEX Events → MarketDataRelay → Flash Arbitrage Strategy"
echo "   Arbitrage Signals → SignalRelay → Dashboard WebSocket Server"
echo "   Pool State Updates → MarketDataRelay → Dashboard WebSocket Server"

echo ""
echo "🌐 Endpoints:"
echo "   Dashboard WebSocket: ws://localhost:8080/ws"
echo "   Dashboard HTTP: http://localhost:8080/health"

echo ""
echo "📋 Log Files:"
echo "   MarketData Relay: /tmp/alphapulse/logs/market_data_relay.log"
echo "   Signal Relay: /tmp/alphapulse/logs/signal_relay.log"
echo "   Execution Relay: /tmp/alphapulse/logs/execution_relay.log"
echo "   Polygon Publisher: /tmp/alphapulse/logs/polygon_publisher.log"
echo "   Flash Arbitrage: /tmp/alphapulse/logs/flash_arbitrage.log"
echo "   Dashboard Server: /tmp/alphapulse/logs/dashboard_websocket.log"

echo ""
echo "🎯 Next Steps:"
echo "   1. Open frontend dashboard to view live arbitrage opportunities"
echo "   2. Monitor logs for real-time DEX events and arbitrage detection"
echo "   3. Verify end-to-end data flow with live market data"

echo ""
echo -e "${YELLOW}💡 To stop all services:${NC}"
echo "   ./scripts/stop_arbitrage_pipeline.sh"

echo ""
echo -e "${GREEN}🚀 Live arbitrage detection pipeline is ready!${NC}"