#!/bin/bash
# AlphaPulse DeFi Services Health Monitor

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
BACKEND_DIR="$( cd "$SCRIPT_DIR/.." && pwd )"

echo "🔍 AlphaPulse DeFi Services Health Check"
echo "========================================"
echo ""

# Check if services are running
echo "📋 Service Status:"
echo "-----------------"

SERVICES=("defi-relay" "defi-scanner" "capital-arbitrage" "flash-loan-bot")
RUNNING_COUNT=0

for service in "${SERVICES[@]}"; do
    PID=$(pgrep -f "$service" | head -1)
    if [ -n "$PID" ]; then
        echo "✅ $service (PID: $PID)"
        RUNNING_COUNT=$((RUNNING_COUNT + 1))
    else
        echo "❌ $service (not running)"
    fi
done

echo ""
echo "Running: $RUNNING_COUNT/4 services"

# Check socket files
echo ""
echo "🔌 Socket Status:"
echo "-----------------"

SOCKETS=("/tmp/alphapulse/relay.sock" "/tmp/alphapulse/defi_relay.sock")
for socket in "${SOCKETS[@]}"; do
    if [ -S "$socket" ]; then
        echo "✅ $socket"
    else
        echo "❌ $socket (missing)"
    fi
done

# Check log files and recent activity
echo ""
echo "📄 Recent Log Activity:"
echo "----------------------"

LOG_DIR="$BACKEND_DIR/logs"
if [ -d "$LOG_DIR" ]; then
    for log_file in defi-relay.log defi-scanner.log capital-arbitrage.log flash-loan-bot.log; do
        log_path="$LOG_DIR/$log_file"
        if [ -f "$log_path" ]; then
            lines=$(wc -l < "$log_path" 2>/dev/null || echo "0")
            last_modified=$(stat -c %Y "$log_path" 2>/dev/null || echo "0")
            current_time=$(date +%s)
            age=$((current_time - last_modified))
            
            if [ $age -lt 60 ]; then
                echo "✅ $log_file ($lines lines, updated ${age}s ago)"
            elif [ $age -lt 300 ]; then
                echo "⚠️  $log_file ($lines lines, updated ${age}s ago)"
            else
                echo "❌ $log_file ($lines lines, updated ${age}s ago - stale)"
            fi
        else
            echo "❌ $log_file (missing)"
        fi
    done
else
    echo "❌ Log directory not found: $LOG_DIR"
fi

# Check for errors in logs
echo ""
echo "🚨 Recent Errors:"
echo "-----------------"

ERROR_COUNT=0
if [ -d "$LOG_DIR" ]; then
    for log_file in "$LOG_DIR"/*.log; do
        if [ -f "$log_file" ]; then
            errors=$(tail -100 "$log_file" 2>/dev/null | grep -i "error\|failed\|panic" | wc -l)
            if [ $errors -gt 0 ]; then
                echo "⚠️  $(basename "$log_file"): $errors recent errors"
                ERROR_COUNT=$((ERROR_COUNT + errors))
            fi
        fi
    done
fi

if [ $ERROR_COUNT -eq 0 ]; then
    echo "✅ No recent errors detected"
else
    echo "❌ Total recent errors: $ERROR_COUNT"
fi

# Check system resources
echo ""
echo "💻 System Resources:"
echo "-------------------"

# Memory usage
total_memory=$(free -m | awk 'NR==2{print $2}')
used_memory=$(free -m | awk 'NR==2{print $3}')
memory_percent=$((used_memory * 100 / total_memory))

echo "Memory: ${used_memory}MB / ${total_memory}MB (${memory_percent}%)"

# CPU load
load_avg=$(uptime | awk -F'load average:' '{print $2}' | cut -d',' -f1 | tr -d ' ')
echo "Load Average: $load_avg"

# Disk space for logs
if [ -d "$LOG_DIR" ]; then
    disk_usage=$(du -sh "$LOG_DIR" 2>/dev/null | cut -f1)
    echo "Log Directory Size: $disk_usage"
fi

# Network connectivity test
echo ""
echo "🌐 Network Connectivity:"
echo "-----------------------"

# Test Alchemy connection (if URL is set)
if [ -n "$ALCHEMY_RPC_URL" ]; then
    if curl -s --max-time 5 "$ALCHEMY_RPC_URL" > /dev/null 2>&1; then
        echo "✅ Alchemy RPC connection"
    else
        echo "❌ Alchemy RPC connection failed"
    fi
else
    echo "⚠️  ALCHEMY_RPC_URL not configured"
fi

# Check metrics endpoint
if curl -s --max-time 5 http://localhost:9091/metrics > /dev/null 2>&1; then
    echo "✅ Metrics endpoint (localhost:9091)"
else
    echo "❌ Metrics endpoint not responding"
fi

# Summary
echo ""
echo "📊 Health Summary:"
echo "-----------------"

if [ $RUNNING_COUNT -eq 4 ] && [ $ERROR_COUNT -eq 0 ]; then
    echo "🟢 HEALTHY - All systems operational"
elif [ $RUNNING_COUNT -ge 2 ] && [ $ERROR_COUNT -lt 5 ]; then
    echo "🟡 DEGRADED - Some issues detected"
else
    echo "🔴 UNHEALTHY - Significant problems detected"
fi

echo ""
echo "For detailed logs, run:"
echo "  tail -f $LOG_DIR/defi-scanner.log"
echo "  tail -f $LOG_DIR/flash-loan-bot.log"
echo ""
echo "To restart services:"
echo "  $SCRIPT_DIR/stop-defi-services.sh"
echo "  $SCRIPT_DIR/start-defi-services.sh"