#!/bin/bash
# AlphaPulse Service Shutdown Script

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

echo "Stopping AlphaPulse services..."

# Try to read PIDs from file
if [ -f "$SCRIPT_DIR/pids.txt" ]; then
    source "$SCRIPT_DIR/pids.txt"
    
    # Kill services in reverse order
    for pid_var in WS_PID ALPACA_PID COINBASE_PID RELAY_PID; do
        pid=${!pid_var}
        if [ ! -z "$pid" ] && kill -0 $pid 2>/dev/null; then
            echo "Stopping process $pid_var (PID: $pid)..."
            kill $pid
        fi
    done
    
    rm "$SCRIPT_DIR/pids.txt"
else
    echo "No PID file found, using pkill..."
fi

# Fallback: kill by name
echo "Cleaning up any remaining processes..."
pkill -f "ws-bridge" 2>/dev/null || true
pkill -f "exchange-collector" 2>/dev/null || true
pkill -f "relay-server" 2>/dev/null || true

# Clean up sockets
rm -f /tmp/alphapulse/*.sock 2>/dev/null || true

echo "All services stopped."