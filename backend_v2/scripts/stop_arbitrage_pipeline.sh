#!/bin/bash
# Stop all services in the live arbitrage detection pipeline

set -e

echo "🛑 Stopping AlphaPulse Live Arbitrage Detection Pipeline"
echo "════════════════════════════════════════════════════════"

# Colors for output  
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to stop service by PID file
stop_service() {
    local pid_file=$1
    local service_name=$2
    
    if [[ -f "$pid_file" ]]; then
        local pid=$(cat "$pid_file")
        echo -n "🛑 Stopping $service_name (PID: $pid)..."
        
        if kill "$pid" 2>/dev/null; then
            # Wait for process to terminate
            local attempts=0
            while [[ $attempts -lt 10 ]] && kill -0 "$pid" 2>/dev/null; do
                sleep 1
                attempts=$((attempts + 1))
                echo -n "."
            done
            
            if kill -0 "$pid" 2>/dev/null; then
                # Force kill if still running
                echo -n " force killing..."
                kill -9 "$pid" 2>/dev/null || true
            fi
            
            rm -f "$pid_file"
            echo -e " ${GREEN}✅ Stopped${NC}"
        else
            echo -e " ${YELLOW}⚠️ Already stopped${NC}"
            rm -f "$pid_file"
        fi
    else
        echo -e "${YELLOW}⚠️ $service_name PID file not found${NC}"
    fi
}

# Function to remove socket files
cleanup_socket() {
    local socket_path=$1
    local socket_name=$2
    
    if [[ -S "$socket_path" ]]; then
        rm -f "$socket_path"
        echo "🧹 Cleaned up $socket_name socket"
    fi
}

echo ""

# Stop services in reverse order (opposite of startup)
echo "Stopping services in reverse order..."

# Stop Dashboard WebSocket Server
stop_service "/tmp/alphapulse/dashboard_websocket.pid" "Dashboard WebSocket Server"

# Stop Flash Arbitrage Strategy
stop_service "/tmp/alphapulse/flash_arbitrage.pid" "Flash Arbitrage Strategy"

# Stop Polygon Collector
stop_service "/tmp/alphapulse/polygon_collector.pid" "Polygon DEX Collector"

# Stop Domain Relay Services
stop_service "/tmp/alphapulse/execution.pid" "ExecutionRelay"
stop_service "/tmp/alphapulse/signal.pid" "SignalRelay"
stop_service "/tmp/alphapulse/market_data.pid" "MarketDataRelay"

echo ""

# Cleanup socket files
echo "Cleaning up socket files..."
cleanup_socket "/tmp/alphapulse/execution.sock" "ExecutionRelay"
cleanup_socket "/tmp/alphapulse/signals.sock" "SignalRelay" 
cleanup_socket "/tmp/alphapulse/market_data.sock" "MarketDataRelay"

echo ""

# Additional cleanup - kill any remaining processes by name
echo "🧹 Additional process cleanup..."
pkill -f "market_data_relay" 2>/dev/null || true
pkill -f "signal_relay" 2>/dev/null || true  
pkill -f "execution_relay" 2>/dev/null || true
pkill -f "polygon.*adapter" 2>/dev/null || true
pkill -f "alphapulse-flash-arbitrage" 2>/dev/null || true
pkill -f "alphapulse-dashboard-websocket" 2>/dev/null || true

# Wait a moment for processes to terminate
sleep 2

# Final status check
echo "📋 Final Status Check:"
remaining_processes=$(ps aux | grep -E "(market_data_relay|signal_relay|execution_relay|polygon.*adapter|alphapulse-flash-arbitrage|alphapulse-dashboard-websocket)" | grep -v grep | wc -l)

if [[ $remaining_processes -eq 0 ]]; then
    echo -e "${GREEN}✅ All AlphaPulse services stopped successfully${NC}"
else
    echo -e "${RED}⚠️ $remaining_processes processes may still be running${NC}"
    echo "Remaining processes:"
    ps aux | grep -E "(market_data_relay|signal_relay|execution_relay|polygon.*adapter|alphapulse-flash-arbitrage|alphapulse-dashboard-websocket)" | grep -v grep || echo "None found"
    echo ""
    echo "💡 If needed, use: pkill -f alphapulse"
fi

echo ""
echo "📊 Log files preserved in /tmp/alphapulse/logs/ for debugging"
echo ""
echo -e "${GREEN}🎯 Pipeline shutdown complete${NC}"