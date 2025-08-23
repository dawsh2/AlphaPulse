#!/bin/bash

# Simple script to test the dashboard with all required services

set -e

echo "🚀 Starting DeFi Scanner Dashboard Test"
echo "========================================="

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Create socket directory
mkdir -p /tmp/alphapulse

# Check if binaries exist
if [ ! -f "./target/release/relay-server" ]; then
    echo -e "${RED}❌ relay-server binary not found. Run: cargo build --release --bin relay-server${NC}"
    exit 1
fi

if [ ! -f "./target/release/exchange-collector" ]; then
    echo -e "${RED}❌ exchange-collector binary not found. Run: cargo build --release --bin exchange-collector${NC}"
    exit 1
fi

if [ ! -f "./target/release/defi-scanner" ]; then
    echo -e "${RED}❌ defi-scanner binary not found. Run: cargo build --release --bin defi-scanner${NC}"
    exit 1
fi

# Function to cleanup on exit
cleanup() {
    echo -e "\n${YELLOW}🛑 Shutting down services...${NC}"
    jobs -p | xargs -r kill 2>/dev/null || true
    rm -f /tmp/alphapulse/*.sock 2>/dev/null || true
    echo -e "${GREEN}✅ Cleanup complete${NC}"
}

trap cleanup EXIT INT TERM

echo -e "${YELLOW}1. Starting Relay Server...${NC}"
./target/release/relay-server > /tmp/relay.log 2>&1 &
RELAY_PID=$!
sleep 2

# Check if relay is running
if ! kill -0 $RELAY_PID 2>/dev/null; then
    echo -e "${RED}❌ Failed to start relay server${NC}"
    exit 1
fi
echo -e "${GREEN}   ✅ Relay Server started (PID: $RELAY_PID)${NC}"

echo -e "${YELLOW}2. Starting Exchange Collector...${NC}"
./target/release/exchange-collector --exchange polygon > /tmp/collector.log 2>&1 &
COLLECTOR_PID=$!
sleep 3

# Check if collector is running
if ! kill -0 $COLLECTOR_PID 2>/dev/null; then
    echo -e "${RED}❌ Failed to start exchange collector${NC}"
    exit 1
fi
echo -e "${GREEN}   ✅ Exchange Collector started (PID: $COLLECTOR_PID)${NC}"

echo -e "${YELLOW}3. Waiting for socket setup...${NC}"
sleep 2

echo -e "${GREEN}✨ All services running! Starting Dashboard...${NC}"
echo "========================================="
echo ""

# Start the dashboard
ALCHEMY_RPC_URL=${ALCHEMY_RPC_URL:-https://polygon-rpc.com} \
ENABLE_DASHBOARD=true \
RUST_LOG=info \
./target/release/defi-scanner