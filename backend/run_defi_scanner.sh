#!/bin/bash

echo "Starting AlphaPulse DeFi Scanner with Real DEX Data"
echo "===================================================="
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Kill any existing processes
echo -e "${YELLOW}Cleaning up existing processes...${NC}"
pkill -f relay_server
pkill -f exchange_collector
pkill -f defi-scanner
sleep 1

# Start relay server
echo -e "${GREEN}1. Starting Relay Server...${NC}"
cd backend/services/relay_server
cargo build --release 2>/dev/null
./target/release/relay_server &
RELAY_PID=$!
echo "   Relay PID: $RELAY_PID"
sleep 2

# Start exchange collector 
echo -e "${GREEN}2. Starting Exchange Collector (Polygon DEX)...${NC}"
cd ../exchange_collector
cargo build --release 2>/dev/null
RUST_LOG=info ./target/release/exchange_collector &
COLLECTOR_PID=$!
echo "   Collector PID: $COLLECTOR_PID"
sleep 2

# Start scanner
echo -e "${GREEN}3. Starting DeFi Scanner...${NC}"
cd ../defi/scanner
cargo build --release 2>/dev/null
RUST_LOG=info ./target/release/defi-scanner &
SCANNER_PID=$!
echo "   Scanner PID: $SCANNER_PID"

echo ""
echo -e "${GREEN}✅ All services running!${NC}"
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Monitoring for arbitrage opportunities with:"
echo "  • Real DEX pool data (SwapEventMessages)"
echo "  • Closed-form optimal trade sizing"
echo "  • Accurate slippage calculation"
echo "  • Huff-optimized gas costs"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo -e "${YELLOW}Press Ctrl+C to stop all services${NC}"
echo ""

# Function to cleanup on exit
cleanup() {
    echo ""
    echo -e "${YELLOW}Shutting down services...${NC}"
    kill $SCANNER_PID 2>/dev/null
    kill $COLLECTOR_PID 2>/dev/null
    kill $RELAY_PID 2>/dev/null
    echo -e "${GREEN}Services stopped.${NC}"
    exit 0
}

# Set trap for Ctrl+C
trap cleanup INT

# Keep script running and show logs
tail -f /dev/null