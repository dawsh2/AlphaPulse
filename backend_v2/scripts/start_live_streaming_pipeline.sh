#!/bin/bash
# Live Streaming Pipeline Startup Script
# Starts all services in the correct order for the E2E pipeline

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
BASE_DIR="$( cd "$SCRIPT_DIR/.." && pwd )"
LOG_DIR="/tmp/alphapulse/logs"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}🚀 Starting AlphaPulse Live Streaming Pipeline${NC}"

# Create log directory if it doesn't exist
mkdir -p "$LOG_DIR"

# Function to check if a process is running
is_running() {
    pgrep -f "$1" > /dev/null 2>&1
}

# Function to start a service
start_service() {
    local name=$1
    local binary=$2
    local log_file=$3
    
    if is_running "$binary"; then
        echo -e "${YELLOW}⚠️  $name is already running, skipping...${NC}"
    else
        echo -e "${GREEN}Starting $name...${NC}"
        nohup "$BASE_DIR/target/release/$binary" > "$LOG_DIR/$log_file" 2>&1 &
        sleep 1
        
        if is_running "$binary"; then
            echo -e "${GREEN}✅ $name started successfully${NC}"
        else
            echo -e "${RED}❌ Failed to start $name${NC}"
            echo -e "${RED}   Check log at: $LOG_DIR/$log_file${NC}"
            exit 1
        fi
    fi
}

# Kill existing services if requested
if [[ "$1" == "--restart" ]]; then
    echo -e "${YELLOW}Stopping existing services...${NC}"
    pkill -f market_data_relay || true
    pkill -f signal_relay || true
    pkill -f execution_relay || true
    pkill -f "polygon$" || true
    pkill -f alphapulse-dashboard-websocket || true
    sleep 2
fi

# 1. Start Relays (ORDER MATTERS!)
echo -e "\n${GREEN}📡 Starting Relay Services...${NC}"
start_service "Market Data Relay" "market_data_relay" "market_data_relay.log"
start_service "Signal Relay" "signal_relay" "signal_relay.log"
start_service "Execution Relay" "execution_relay" "execution_relay.log"

# 2. Start Polygon Collector
echo -e "\n${GREEN}🔌 Starting Polygon Collector...${NC}"
start_service "Polygon Collector" "polygon" "polygon_collector.log"

# Wait for polygon to connect
echo -e "${YELLOW}Waiting for Polygon to connect to relays...${NC}"
for i in {1..10}; do
    if grep -q "Connected to MarketData relay" "$LOG_DIR/polygon_collector.log" 2>/dev/null; then
        echo -e "${GREEN}✅ Polygon connected to Market Data Relay${NC}"
        break
    fi
    sleep 1
done

# 3. Start Dashboard WebSocket Server
echo -e "\n${GREEN}🖥️  Starting Dashboard WebSocket Server...${NC}"
start_service "Dashboard WebSocket" "alphapulse-dashboard-websocket" "dashboard_websocket.log"

# Wait for dashboard to connect to relays
echo -e "${YELLOW}Waiting for Dashboard to connect to relays...${NC}"
for i in {1..10}; do
    if grep -q "Connected to MarketData relay" "$LOG_DIR/dashboard_websocket.log" 2>/dev/null; then
        echo -e "${GREEN}✅ Dashboard connected to relays${NC}"
        break
    fi
    sleep 1
done

# Verification
echo -e "\n${GREEN}🔍 Verifying Pipeline Status...${NC}"

# Check if polygon is processing events
sleep 3
if tail -n 10 "$LOG_DIR/polygon_collector.log" | grep -q "Processed.*DEX events"; then
    EVENT_COUNT=$(tail -n 10 "$LOG_DIR/polygon_collector.log" | grep -oP "Processed \K\d+" | tail -1)
    echo -e "${GREEN}✅ Polygon processing live events (${EVENT_COUNT} events so far)${NC}"
else
    echo -e "${YELLOW}⚠️  Polygon not yet processing events${NC}"
fi

# Show running services
echo -e "\n${GREEN}📊 Running Services:${NC}"
ps aux | grep -E '(market_data_relay|signal_relay|execution_relay|polygon|alphapulse-dashboard-websocket)' | grep -v grep | awk '{print "  - " $11}'

echo -e "\n${GREEN}📝 Log Locations:${NC}"
echo "  Market Data Relay: $LOG_DIR/market_data_relay.log"
echo "  Signal Relay: $LOG_DIR/signal_relay.log"
echo "  Execution Relay: $LOG_DIR/execution_relay.log"
echo "  Polygon Collector: $LOG_DIR/polygon_collector.log"
echo "  Dashboard WebSocket: $LOG_DIR/dashboard_websocket.log"

echo -e "\n${GREEN}🌐 Frontend:${NC}"
echo "  Dashboard WebSocket: ws://localhost:8080/ws"
echo "  To start frontend: cd ../frontend && npm run dev:dashboard"

echo -e "\n${GREEN}✨ Live Streaming Pipeline Ready!${NC}"
echo -e "${YELLOW}Tip: Monitor live events with: tail -f $LOG_DIR/polygon_collector.log | grep Processed${NC}"