#!/bin/bash

# AlphaPulse Arbitrage Detection System Startup Script
# This script starts all components needed for real-time arbitrage detection

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Get the backend directory (parent of scripts)
BACKEND_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$BACKEND_DIR"

echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}    🚀 AlphaPulse Arbitrage Detection System${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
echo ""

# Function to cleanup on exit
cleanup() {
    echo -e "\n${YELLOW}Shutting down services...${NC}"
    
    # Kill all background processes
    jobs -p | xargs -r kill 2>/dev/null || true
    
    # Clean up socket files
    rm -f /tmp/alphapulse/*.sock 2>/dev/null || true
    
    echo -e "${GREEN}✅ All services stopped${NC}"
}

# Set up trap to cleanup on script exit
trap cleanup EXIT INT TERM

# Create socket directory if it doesn't exist
mkdir -p /tmp/alphapulse

# Check if cargo is available
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}❌ Error: cargo not found. Please install Rust.${NC}"
    exit 1
fi

# Build all components first
echo -e "${YELLOW}📦 Building components...${NC}"
cargo build --release --bin relay-server 2>&1 | grep -E "Compiling|Finished" || true
cargo build --release --bin exchange-collector 2>&1 | grep -E "Compiling|Finished" || true
(cd services/defi/scanner && cargo build --release 2>&1 | grep -E "Compiling|Finished" || true)
echo -e "${GREEN}✅ Build complete${NC}\n"

# Start Relay Server
echo -e "${BLUE}1. Starting Relay Server...${NC}"
RUST_LOG=info ./target/release/relay-server > logs/relay.log 2>&1 &
RELAY_PID=$!
sleep 2

# Check if relay is running
if ! kill -0 $RELAY_PID 2>/dev/null; then
    echo -e "${RED}❌ Failed to start Relay Server${NC}"
    exit 1
fi
echo -e "${GREEN}   ✅ Relay Server started (PID: $RELAY_PID)${NC}\n"

# Start Polygon Collector
echo -e "${BLUE}2. Starting Polygon DEX Collector...${NC}"
RUST_LOG=info ./target/release/exchange-collector --exchange polygon > logs/polygon_collector.log 2>&1 &
COLLECTOR_PID=$!
sleep 3

# Check if collector is running
if ! kill -0 $COLLECTOR_PID 2>/dev/null; then
    echo -e "${RED}❌ Failed to start Polygon Collector${NC}"
    exit 1
fi
echo -e "${GREEN}   ✅ Polygon Collector started (PID: $COLLECTOR_PID)${NC}\n"

# Start DeFi Scanner with price monitoring displayed in terminal
echo -e "${BLUE}3. Starting Arbitrage Scanner with Price Monitoring...${NC}"
echo -e "${GREEN}   ✅ Scanner will display live price data below${NC}\n"

echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}✨ All services are running!${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
echo ""
echo -e "${YELLOW}📊 Monitoring:${NC}"
echo -e "   • Relay logs:     tail -f logs/relay.log"
echo -e "   • Collector logs: tail -f logs/polygon_collector.log"
echo -e "   • Scanner output: Will appear below"
echo ""
echo -e "${YELLOW}📌 To stop all services: Press Ctrl+C${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
echo ""

# Create logs directory if it doesn't exist
mkdir -p logs

# Wait and show scanner output
echo -e "${GREEN}🔍 Live Price Monitoring & Arbitrage Detection:${NC}"
echo -e "${YELLOW}═══════════════════════════════════════════════════════════════${NC}"
echo ""

# Start scanner in foreground to show live price monitoring
# Only set HUFF_CONTRACT_ADDRESS if it has a value
if [ -n "$HUFF_CONTRACT_ADDRESS" ]; then
    echo -e "${GREEN}   🎯 Using Huff gas estimation with contract: $HUFF_CONTRACT_ADDRESS${NC}"
    ALCHEMY_RPC_URL=${ALCHEMY_RPC_URL:-https://polygon-rpc.com} \
    HUFF_CONTRACT_ADDRESS="$HUFF_CONTRACT_ADDRESS" \
    BOT_ADDRESS=${BOT_ADDRESS:-0x742d35Cc6634C0532925a3b8D9B5b7C3B5F6c8f7} \
    RUST_LOG=info \
    ./target/release/defi-scanner
else
    echo -e "${YELLOW}   ⚠️  Using fallback gas estimation (set HUFF_CONTRACT_ADDRESS for precise estimates)${NC}"
    ALCHEMY_RPC_URL=${ALCHEMY_RPC_URL:-https://polygon-rpc.com} \
    BOT_ADDRESS=${BOT_ADDRESS:-0x742d35Cc6634C0532925a3b8D9B5b7C3B5F6c8f7} \
    RUST_LOG=info \
    ./target/release/defi-scanner
fi