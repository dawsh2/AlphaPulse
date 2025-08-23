#!/bin/bash
# Check status of the live arbitrage detection pipeline

echo "🔍 AlphaPulse Pipeline Status Check"
echo "══════════════════════════════════"

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo ""

# Check Domain Relay Services
echo -e "${BLUE}Domain Relay Services:${NC}"
echo "──────────────────────"

check_socket() {
    local socket_path=$1
    local service_name=$2
    
    if [[ -S "$socket_path" ]]; then
        echo -e "   $service_name: ${GREEN}✅ Running${NC}"
        return 0
    else
        echo -e "   $service_name: ${RED}❌ Not running${NC}"
        return 1
    fi
}

relay_count=0
check_socket "/tmp/alphapulse/market_data.sock" "MarketDataRelay" && relay_count=$((relay_count + 1))
check_socket "/tmp/alphapulse/signals.sock" "SignalRelay" && relay_count=$((relay_count + 1))
check_socket "/tmp/alphapulse/execution.sock" "ExecutionRelay" && relay_count=$((relay_count + 1))

echo ""

# Check Service Processes
echo -e "${BLUE}Service Processes:${NC}"
echo "─────────────────"

check_process() {
    local pid_file=$1
    local service_name=$2
    
    if [[ -f "$pid_file" ]]; then
        local pid=$(cat "$pid_file")
        if kill -0 "$pid" 2>/dev/null; then
            echo -e "   $service_name: ${GREEN}✅ Running (PID: $pid)${NC}"
            return 0
        else
            echo -e "   $service_name: ${RED}❌ PID file exists but process dead${NC}"
            return 1
        fi
    else
        echo -e "   $service_name: ${RED}❌ Not started${NC}"
        return 1
    fi
}

service_count=0
check_process "/tmp/alphapulse/polygon_publisher.pid" "Polygon Publisher" && service_count=$((service_count + 1))
check_process "/tmp/alphapulse/flash_arbitrage.pid" "Flash Arbitrage Strategy" && service_count=$((service_count + 1))
check_process "/tmp/alphapulse/dashboard_websocket.pid" "Dashboard WebSocket Server" && service_count=$((service_count + 1))

echo ""

# Check Recent Log Activity
echo -e "${BLUE}Recent Log Activity:${NC}"
echo "──────────────────"

check_recent_logs() {
    local log_file=$1
    local service_name=$2
    
    if [[ -f "$log_file" ]]; then
        local recent_lines=$(tail -n 1 "$log_file" 2>/dev/null | wc -l)
        local file_age=$(( $(date +%s) - $(stat -f %m "$log_file" 2>/dev/null || echo 0) ))
        
        if [[ $file_age -lt 300 ]]; then  # Less than 5 minutes old
            echo -e "   $service_name: ${GREEN}✅ Recent activity${NC}"
        else
            echo -e "   $service_name: ${YELLOW}⚠️ No recent activity (${file_age}s ago)${NC}"
        fi
    else
        echo -e "   $service_name: ${RED}❌ No log file${NC}"
    fi
}

if [[ -d "/tmp/alphapulse/logs" ]]; then
    check_recent_logs "/tmp/alphapulse/logs/market_data_relay.log" "MarketData Relay"
    check_recent_logs "/tmp/alphapulse/logs/signal_relay.log" "Signal Relay"  
    check_recent_logs "/tmp/alphapulse/logs/execution_relay.log" "Execution Relay"
    check_recent_logs "/tmp/alphapulse/logs/polygon_publisher.log" "Polygon Publisher"
    check_recent_logs "/tmp/alphapulse/logs/flash_arbitrage.log" "Flash Arbitrage"
    check_recent_logs "/tmp/alphapulse/logs/dashboard_websocket.log" "Dashboard Server"
else
    echo -e "   ${RED}❌ Log directory not found${NC}"
fi

echo ""

# Overall Status
echo -e "${BLUE}Overall Pipeline Status:${NC}"
echo "───────────────────────"

total_services=6
running_services=$((relay_count + service_count))

if [[ $running_services -eq $total_services ]]; then
    echo -e "   ${GREEN}✅ All services running ($running_services/$total_services)${NC}"
    echo -e "   ${GREEN}🚀 Live arbitrage detection pipeline is operational${NC}"
elif [[ $running_services -gt 3 ]]; then
    echo -e "   ${YELLOW}⚠️ Partially running ($running_services/$total_services)${NC}"
    echo -e "   ${YELLOW}Some services may need attention${NC}"
else
    echo -e "   ${RED}❌ Most services stopped ($running_services/$total_services)${NC}"
    echo -e "   ${RED}Pipeline needs to be started${NC}"
fi

echo ""

# Quick Actions
echo -e "${BLUE}Quick Actions:${NC}"
echo "─────────────"
echo "   Start pipeline: ./scripts/start_live_arbitrage_pipeline.sh"
echo "   Stop pipeline:  ./scripts/stop_arbitrage_pipeline.sh"
echo "   View logs:      tail -f /tmp/alphapulse/logs/*.log"

echo ""

# Data Flow Status  
if [[ $relay_count -eq 3 ]]; then
    echo -e "${BLUE}Data Flow:${NC}"
    echo "─────────"
    echo -e "   ${GREEN}Polygon DEX → MarketDataRelay → Flash Arbitrage${NC}"
    echo -e "   ${GREEN}Arbitrage Signals → SignalRelay → Dashboard${NC}"
else
    echo -e "${RED}Data Flow: ❌ Relay services not ready${NC}"
fi