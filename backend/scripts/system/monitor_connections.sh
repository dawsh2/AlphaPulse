#!/bin/bash
# monitor_connections.sh - Monitor TCP connections for AlphaPulse services
# Usage: ./monitor_connections.sh [PID] [service_name]

if [ $# -eq 0 ]; then
    echo "Usage: $0 <PID> [service_name]"
    echo "       $0 auto - Monitor all AlphaPulse services"
    echo ""
    echo "Examples:"
    echo "  $0 1234 polygon-collector"
    echo "  $0 auto"
    exit 1
fi

if [ "$1" == "auto" ]; then
    echo "Auto-monitoring AlphaPulse services..."
    echo "Finding PIDs for active services..."
    
    # Find all AlphaPulse service PIDs
    POLYGON_PID=$(pgrep -f "exchange-collector" | head -1)
    RELAY_PID=$(pgrep -f "relay-server" | head -1)
    BRIDGE_PID=$(pgrep -f "ws-bridge" | head -1)
    
    if [ -z "$POLYGON_PID" ] && [ -z "$RELAY_PID" ] && [ -z "$BRIDGE_PID" ]; then
        echo "No AlphaPulse services found running!"
        exit 1
    fi
    
    echo "Monitoring connections every 5 seconds (Ctrl+C to stop)..."
    echo "Service thresholds: Polygon=50, Relay=20, Bridge=10"
    echo ""
    
    while true; do
        timestamp=$(date "+%H:%M:%S")
        total_connections=0
        
        if [ -n "$POLYGON_PID" ]; then
            polygon_conn=$(lsof -p $POLYGON_PID 2>/dev/null | grep -E "(TCP|UDP)" | wc -l)
            total_connections=$((total_connections + polygon_conn))
            status=""
            if [ $polygon_conn -gt 50 ]; then
                status=" ⚠️ HIGH"
            elif [ $polygon_conn -gt 20 ]; then
                status=" ⚡"
            fi
            echo "$timestamp Polygon($POLYGON_PID): $polygon_conn connections$status"
        fi
        
        if [ -n "$RELAY_PID" ]; then
            relay_conn=$(lsof -p $RELAY_PID 2>/dev/null | grep -E "(TCP|UDP)" | wc -l)
            total_connections=$((total_connections + relay_conn))
            status=""
            if [ $relay_conn -gt 20 ]; then
                status=" ⚠️ HIGH"
            fi
            echo "$timestamp Relay($RELAY_PID): $relay_conn connections$status"
        fi
        
        if [ -n "$BRIDGE_PID" ]; then
            bridge_conn=$(lsof -p $BRIDGE_PID 2>/dev/null | grep -E "(TCP|UDP)" | wc -l)
            total_connections=$((total_connections + bridge_conn))
            status=""
            if [ $bridge_conn -gt 10 ]; then
                status=" ⚠️ HIGH"
            fi
            echo "$timestamp Bridge($BRIDGE_PID): $bridge_conn connections$status"
        fi
        
        echo "$timestamp TOTAL: $total_connections connections"
        
        # Overall system warning
        if [ $total_connections -gt 100 ]; then
            echo "🚨 CRITICAL: Total connections exceed 100! Possible connection leak detected."
        fi
        
        echo "---"
        sleep 5
    done
else
    # Monitor specific PID
    PID=$1
    SERVICE_NAME=${2:-"Process"}
    
    if ! kill -0 $PID 2>/dev/null; then
        echo "Error: Process $PID not found or not accessible"
        exit 1
    fi
    
    echo "Monitoring $SERVICE_NAME (PID: $PID) connections..."
    echo "Press Ctrl+C to stop"
    echo ""
    
    while true; do
        if ! kill -0 $PID 2>/dev/null; then
            echo "Process $PID has terminated"
            break
        fi
        
        tcp_connections=$(lsof -p $PID 2>/dev/null | grep TCP | wc -l)
        udp_connections=$(lsof -p $PID 2>/dev/null | grep UDP | wc -l)
        total_connections=$((tcp_connections + udp_connections))
        
        timestamp=$(date "+%Y-%m-%d %H:%M:%S")
        echo "$timestamp: $total_connections total (TCP: $tcp_connections, UDP: $udp_connections)"
        
        # Connection leak warnings
        if [ $total_connections -gt 100 ]; then
            echo "🚨 CRITICAL: $total_connections connections detected! Investigating..."
            
            # Show detailed connection breakdown
            echo "Top connection destinations:"
            lsof -p $PID 2>/dev/null | grep -E "(TCP|UDP)" | awk '{print $9}' | sort | uniq -c | sort -nr | head -5
            
        elif [ $total_connections -gt 50 ]; then
            echo "⚠️ WARNING: High connection count ($total_connections)"
        fi
        
        sleep 5
    done
fi

echo "Connection monitoring stopped."