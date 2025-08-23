#!/bin/bash

echo "🚀 Starting AlphaPulse Arbitrage Bot"
echo "===================================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check for required environment variables
check_env() {
    if [ -z "${!1}" ]; then
        echo -e "${RED}❌ Missing required environment variable: $1${NC}"
        exit 1
    else
        echo -e "${GREEN}✅ $1 is set${NC}"
    fi
}

# Optional environment variables
check_optional_env() {
    if [ -z "${!1}" ]; then
        echo -e "${YELLOW}⚠️  Optional variable $1 not set (using default)${NC}"
    else
        echo -e "${GREEN}✅ $1 is set${NC}"
    fi
}

echo ""
echo "Checking environment..."
echo "-----------------------"

# Required
check_env "PRIVATE_KEY"

# Optional
check_optional_env "POLYGON_RPC"
check_optional_env "EXECUTE_TRADES"
check_optional_env "USE_FLASH_LOANS"
check_optional_env "MIN_PROFIT_USD"
check_optional_env "ARBITRAGE_CONTRACT"
check_optional_env "FLASH_LOAN_CONTRACT"

# Set defaults
export POLYGON_RPC=${POLYGON_RPC:-"wss://polygon-bor.publicnode.com"}
export EXECUTE_TRADES=${EXECUTE_TRADES:-"false"}
export USE_FLASH_LOANS=${USE_FLASH_LOANS:-"true"}
export MIN_PROFIT_USD=${MIN_PROFIT_USD:-"1.0"}

echo ""
echo "Configuration:"
echo "--------------"
echo "RPC: $POLYGON_RPC"
echo "Execute Trades: $EXECUTE_TRADES"
echo "Use Flash Loans: $USE_FLASH_LOANS"
echo "Min Profit: \$$MIN_PROFIT_USD"

if [ "$EXECUTE_TRADES" = "true" ]; then
    echo -e "${YELLOW}⚠️  WARNING: Bot will execute REAL trades!${NC}"
    echo "Press Ctrl+C within 5 seconds to cancel..."
    sleep 5
else
    echo -e "${GREEN}Running in SIMULATION mode${NC}"
fi

echo ""
echo "Building bot..."
echo "---------------"

cd arbitrage_bot
cargo build --release

if [ $? -ne 0 ]; then
    echo -e "${RED}❌ Build failed${NC}"
    exit 1
fi

echo -e "${GREEN}✅ Build successful${NC}"

echo ""
echo "Starting bot..."
echo "---------------"

# Run with proper logging
RUST_LOG=arbitrage_bot=info,ethers=warn cargo run --release