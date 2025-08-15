#!/bin/bash
# AlphaPulse Service Status Script

echo "AlphaPulse Service Status"
echo "========================="
echo ""

# Check each service
services=("relay-server" "exchange-collector" "ws-bridge")

for service in "${services[@]}"; do
    count=$(ps aux | grep -E "$service" | grep -v grep | wc -l)
    if [ $count -gt 0 ]; then
        echo "✅ $service: RUNNING ($count instance(s))"
        ps aux | grep -E "$service" | grep -v grep | awk '{print "   PID:", $2, "CPU:", $3"%", "MEM:", $4"%"}'
    else
        echo "❌ $service: NOT RUNNING"
    fi
done

echo ""
echo "Socket Status:"
if [ -S /tmp/alphapulse/relay.sock ]; then
    echo "✅ Relay socket exists"
else
    echo "❌ Relay socket missing"
fi

echo ""
echo "WebSocket Endpoint:"
if curl -s -o /dev/null -w "%{http_code}" http://localhost:8765 | grep -q "426"; then
    echo "✅ WebSocket server responding on ws://localhost:8765/stream"
else
    echo "❌ WebSocket server not responding"
fi

echo ""
echo "Recent Logs:"
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
BACKEND_DIR="$( cd "$SCRIPT_DIR/.." && pwd )"

if [ -d "$BACKEND_DIR/logs" ]; then
    for log in "$BACKEND_DIR"/logs/*.log; do
        if [ -f "$log" ]; then
            filename=$(basename "$log")
            echo "  $filename: $(tail -1 "$log" 2>/dev/null | cut -c1-80)"
        fi
    done
fi