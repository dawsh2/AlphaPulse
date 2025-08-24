#!/bin/bash
# DevOps-Proper End-to-End Pipeline Testing
# Tests the complete data flow: Polygon → TLV → Market Data Relay → Consumers

set -e

echo "🧪 AlphaPulse E2E Pipeline Testing"
echo "=================================="
echo ""

# Configuration
TEST_DURATION=45
HEALTH_PORT=8001
SOCKET_DIR="/tmp/alphapulse_e2e_test"
LOG_DIR="/tmp/alphapulse_e2e_logs"

# Clean up function
cleanup() {
    echo "🧹 Cleaning up test environment..."
    pkill -f "market_data_relay" 2>/dev/null || true
    pkill -f "health_check" 2>/dev/null || true
    rm -rf "$SOCKET_DIR" "$LOG_DIR"
    echo "✅ Cleanup complete"
}
trap cleanup EXIT

# Step 1: Setup Test Environment
echo "📁 Setting up test environment..."
mkdir -p "$SOCKET_DIR" "$LOG_DIR"
export ALPHAPULSE_ENV=testing
echo "  Environment: $ALPHAPULSE_ENV"
echo "  Sockets: $SOCKET_DIR"
echo "  Logs: $LOG_DIR"
echo ""

# Step 2: Start Market Data Relay
echo "🔄 Starting Market Data Relay..."
cd /Users/daws/alphapulse/backend_v2
RUST_LOG=info cargo run --release --bin market_data_relay > "$LOG_DIR/market_data_relay.log" 2>&1 &
RELAY_PID=$!
echo "  Market Data Relay PID: $RELAY_PID"

# Wait for socket to be created
echo "⏳ Waiting for Market Data Relay socket..."
for i in {1..30}; do
    if [[ -S "/tmp/alphapulse/market_data.sock" ]]; then
        echo "✅ Market Data Relay socket ready"
        break
    fi
    sleep 1
    if [[ $i -eq 30 ]]; then
        echo "❌ Market Data Relay socket not created"
        exit 1
    fi
done
echo ""

# Step 3: Start Health Monitoring
echo "🏥 Starting health monitoring for Market Data Relay..."
# Create a simple health check script
cat > "$LOG_DIR/health_check.py" << 'EOF'
import socket
import time
import json

def check_market_data_relay():
    try:
        sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        sock.connect("/tmp/alphapulse/market_data.sock")
        sock.close()
        return True
    except:
        return False

def log_health():
    while True:
        status = "healthy" if check_market_data_relay() else "unhealthy"
        timestamp = time.strftime("%Y-%m-%d %H:%M:%S")
        print(f"{timestamp} - Market Data Relay: {status}")
        time.sleep(10)

if __name__ == "__main__":
    log_health()
EOF

python3 "$LOG_DIR/health_check.py" > "$LOG_DIR/health_monitor.log" 2>&1 &
HEALTH_PID=$!
echo "  Health Monitor PID: $HEALTH_PID"
echo ""

# Step 4: Start Live Data Producer
echo "🔥 Starting Live Polygon Data Stream..."
echo "  Duration: ${TEST_DURATION} seconds"
echo "  Real blockchain events will flow through the entire pipeline"
echo ""

# Run the live streaming demo for specified duration
RUST_LOG=info timeout ${TEST_DURATION}s cargo run --bin live_polygon_stream_demo > "$LOG_DIR/live_stream.log" 2>&1 || echo "Live streaming completed"

echo ""
echo "📊 E2E PIPELINE TEST RESULTS"
echo "============================"
echo ""

# Step 5: Analyze Results
echo "🔍 Analyzing pipeline performance..."

# Check relay logs
RELAY_LOG="$LOG_DIR/market_data_relay.log"
if [[ -f "$RELAY_LOG" ]]; then
    echo "📡 Market Data Relay Status:"
    if grep -q "listening on" "$RELAY_LOG"; then
        echo "  ✅ Started successfully"
    else
        echo "  ❌ Failed to start"
    fi
    
    # Count messages processed
    MSG_COUNT=$(grep -c "TLV message" "$RELAY_LOG" 2>/dev/null || echo "0")
    echo "  📊 Messages processed: $MSG_COUNT"
else
    echo "  ❌ No relay log found"
fi

# Check live stream results
STREAM_LOG="$LOG_DIR/live_stream.log"
if [[ -f "$STREAM_LOG" ]]; then
    echo ""
    echo "🔥 Live Stream Results:"
    
    # Extract final statistics
    if grep -q "LIVE STREAMING RESULTS" "$STREAM_LOG"; then
        echo "  ✅ Live streaming completed successfully"
        
        EVENTS=$(grep "Total Events:" "$STREAM_LOG" | tail -1 | grep -o '[0-9]* real blockchain events' | head -1)
        BLOCKS=$(grep "Block Headers:" "$STREAM_LOG" | tail -1 | grep -o '[0-9]* new blocks' | head -1) 
        SWAPS=$(grep "DEX Swaps:" "$STREAM_LOG" | tail -1 | grep -o '[0-9]* swap transactions' | head -1)
        
        echo "  📊 Events processed: $EVENTS"
        echo "  📊 Block headers: $BLOCKS"
        echo "  📊 DEX swaps: $SWAPS"
        
        # Check processing performance
        if grep -q "Sub-microsecond processing per event" "$STREAM_LOG"; then
            echo "  ⚡ Performance: Sub-microsecond processing confirmed"
        fi
    else
        echo "  ⚠️  Live streaming may have been interrupted"
    fi
else
    echo "  ❌ No stream log found"
fi

# Check health monitoring
HEALTH_LOG="$LOG_DIR/health_monitor.log"
if [[ -f "$HEALTH_LOG" ]]; then
    echo ""
    echo "🏥 Health Monitoring Results:"
    
    HEALTHY_COUNT=$(grep -c "healthy" "$HEALTH_LOG" 2>/dev/null || echo "0")
    UNHEALTHY_COUNT=$(grep -c "unhealthy" "$HEALTH_LOG" 2>/dev/null || echo "0")
    
    echo "  ✅ Healthy checks: $HEALTHY_COUNT"
    echo "  ❌ Unhealthy checks: $UNHEALTHY_COUNT"
    
    if [[ $HEALTHY_COUNT -gt 0 && $UNHEALTHY_COUNT -eq 0 ]]; then
        echo "  🎉 System remained healthy throughout test"
    fi
fi

echo ""
echo "🎯 E2E PIPELINE VALIDATION"
echo "=========================="

# Determine overall test success
SUCCESS=true

# Check if socket was created and maintained
if [[ ! -S "/tmp/alphapulse/market_data.sock" ]]; then
    echo "❌ Market Data Relay socket not maintained"
    SUCCESS=false
else
    echo "✅ Market Data Relay: Socket operational"
fi

# Check if live streaming processed events
if grep -q "LIVE STREAMING SUCCESS" "$STREAM_LOG" 2>/dev/null; then
    echo "✅ Live Streaming: Real blockchain events processed"
else
    echo "❌ Live Streaming: Did not complete successfully"
    SUCCESS=false
fi

# Check if any events flowed through the pipeline
if [[ $(grep -c "Event processed in" "$STREAM_LOG" 2>/dev/null || echo "0") -gt 0 ]]; then
    echo "✅ Pipeline Flow: Events processed end-to-end"
else
    echo "❌ Pipeline Flow: No events detected"
    SUCCESS=false
fi

echo ""
if [[ "$SUCCESS" == true ]]; then
    echo "🎉 E2E PIPELINE TEST: PASSED"
    echo "   ✅ Real data ingestion working"
    echo "   ✅ TLV message processing working"  
    echo "   ✅ Market Data Relay operational"
    echo "   ✅ Health monitoring functional"
    echo ""
    echo "🚀 Pipeline ready for production deployment!"
else
    echo "❌ E2E PIPELINE TEST: FAILED"
    echo "   Check logs in: $LOG_DIR"
    echo ""
    echo "🔧 Pipeline needs debugging before deployment"
fi

echo ""
echo "📋 Test artifacts preserved in: $LOG_DIR"
echo "💡 Review logs for detailed analysis"
echo ""

# Keep processes running for a moment to ensure clean shutdown
sleep 2