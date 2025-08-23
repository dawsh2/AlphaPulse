#!/bin/bash
# AlphaPulse DeFi Services Stop Script

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
BACKEND_DIR="$( cd "$SCRIPT_DIR/.." && pwd )"

echo "Stopping AlphaPulse DeFi services..."

# Read PIDs if available
PID_FILE="$BACKEND_DIR/scripts/defi-pids.txt"
if [ -f "$PID_FILE" ]; then
    source "$PID_FILE"
    
    # Stop services gracefully
    if [ -n "$FLASH_PID" ]; then
        echo "Stopping flash loan bot (PID: $FLASH_PID)..."
        kill -TERM "$FLASH_PID" 2>/dev/null || true
    fi
    
    if [ -n "$CAPITAL_PID" ]; then
        echo "Stopping capital arbitrage bot (PID: $CAPITAL_PID)..."
        kill -TERM "$CAPITAL_PID" 2>/dev/null || true
    fi
    
    if [ -n "$SCANNER_PID" ]; then
        echo "Stopping DeFi scanner (PID: $SCANNER_PID)..."
        kill -TERM "$SCANNER_PID" 2>/dev/null || true
    fi
    
    if [ -n "$DEFI_RELAY_PID" ]; then
        echo "Stopping DeFi relay (PID: $DEFI_RELAY_PID)..."
        kill -TERM "$DEFI_RELAY_PID" 2>/dev/null || true
    fi
    
    # Wait for graceful shutdown
    sleep 3
    
    # Force kill if still running
    for pid in "$FLASH_PID" "$CAPITAL_PID" "$SCANNER_PID" "$DEFI_RELAY_PID"; do
        if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
            echo "Force killing process $pid..."
            kill -KILL "$pid" 2>/dev/null || true
        fi
    done
    
    # Remove PID file
    rm -f "$PID_FILE"
else
    echo "PID file not found, using process names..."
fi

# Kill any remaining DeFi processes by name
echo "Cleaning up any remaining DeFi processes..."
pkill -f "defi-scanner|flash-loan-bot|defi-relay|capital-arbitrage" 2>/dev/null || true

# Clean up sockets
echo "Cleaning up sockets..."
rm -f /tmp/alphapulse/defi_relay.sock 2>/dev/null || true

# Show final status
echo ""
echo "Checking for remaining DeFi processes..."
REMAINING=$(ps aux | grep -E "defi-scanner|flash-loan-bot|defi-relay|capital-arbitrage" | grep -v grep | wc -l)

if [ "$REMAINING" -eq 0 ]; then
    echo "✅ All DeFi services stopped successfully"
else
    echo "⚠️  Some DeFi processes may still be running:"
    ps aux | grep -E "defi-scanner|flash-loan-bot|defi-relay|capital-arbitrage" | grep -v grep
fi

echo ""
echo "DeFi services shutdown complete."