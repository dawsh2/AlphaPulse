#!/bin/bash
# AlphaPulse Daemon Management Script (macOS launchd)
# This script manages AlphaPulse services using macOS-specific launchd
# For Linux systems, an equivalent script using systemd would be needed

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PLIST_DIR="$SCRIPT_DIR"
LAUNCHD_DIR="$HOME/Library/LaunchAgents"

# Service definitions
SERVICES=(
    "com.alphapulse.relay"
    "com.alphapulse.exchange-collector-kraken"
    "com.alphapulse.exchange-collector-coinbase" 
    "com.alphapulse.exchange-collector-alpaca"
    "com.alphapulse.ws-bridge"
)

function install_services() {
    echo "Installing AlphaPulse services (macOS launchd)..."
    
    # Create LaunchAgents directory if it doesn't exist
    mkdir -p "$LAUNCHD_DIR"
    
    # Create logs directory
    mkdir -p "$SCRIPT_DIR/../logs"
    
    for service in "${SERVICES[@]}"; do
        plist_file="$PLIST_DIR/$service.plist"
        if [ -f "$plist_file" ]; then
            # Copy plist to LaunchAgents
            cp "$plist_file" "$LAUNCHD_DIR/"
            echo "Installed: $service"
        else
            echo "Warning: $plist_file not found"
        fi
    done
    
    echo ""
    echo "Services installed. Use 'start' command to launch them."
}

function uninstall_services() {
    echo "Uninstalling AlphaPulse services..."
    
    # Stop services first
    stop_services
    
    for service in "${SERVICES[@]}"; do
        rm -f "$LAUNCHD_DIR/$service.plist"
        echo "Uninstalled: $service"
    done
}

function start_services() {
    echo "Starting AlphaPulse services..."
    echo "Note: Services start in dependency order with delays"
    
    # Start relay server first (it creates the socket)
    launchctl load "$LAUNCHD_DIR/com.alphapulse.relay.plist" 2>/dev/null
    echo "Started: relay server"
    
    # Wait for relay socket
    echo "Waiting for relay socket..."
    for i in {1..10}; do
        if [ -S /tmp/alphapulse/relay.sock ]; then
            echo "Relay socket ready"
            break
        fi
        sleep 1
    done
    
    # Start exchange collectors (they connect to relay)
    sleep 2
    for collector in "kraken" "coinbase" "alpaca"; do
        launchctl load "$LAUNCHD_DIR/com.alphapulse.exchange-collector-$collector.plist" 2>/dev/null
        echo "Started: exchange-collector-$collector"
    done
    
    # Start ws-bridge last (it connects to relay)
    sleep 2
    launchctl load "$LAUNCHD_DIR/com.alphapulse.ws-bridge.plist" 2>/dev/null
    echo "Started: ws-bridge"
    
    echo ""
    echo "All services started. Check status with './daemon-manager.sh status'"
}

function stop_services() {
    echo "Stopping AlphaPulse services..."
    
    # Stop in reverse order
    for service in "${SERVICES[@]}"; do
        launchctl unload "$LAUNCHD_DIR/$service.plist" 2>/dev/null
        echo "Stopped: $service"
    done
    
    # Clean up any orphaned processes
    pkill -f "relay-server" 2>/dev/null
    pkill -f "exchange-collector" 2>/dev/null
    pkill -f "ws-bridge" 2>/dev/null
    
    echo "All services stopped"
}

function restart_services() {
    stop_services
    sleep 2
    start_services
}

function service_status() {
    echo "AlphaPulse Service Status (launchd)"
    echo "===================================="
    echo ""
    
    for service in "${SERVICES[@]}"; do
        if launchctl list | grep -q "$service"; then
            pid=$(launchctl list | grep "$service" | awk '{print $1}')
            if [ "$pid" != "-" ]; then
                echo "✅ $service: RUNNING (PID: $pid)"
            else
                echo "⚠️  $service: LOADED but NOT RUNNING"
            fi
        else
            echo "❌ $service: NOT LOADED"
        fi
    done
    
    echo ""
    echo "Socket Status:"
    if [ -S /tmp/alphapulse/relay.sock ]; then
        echo "✅ Relay socket exists"
    else
        echo "❌ Relay socket missing"
    fi
    
    # Check exchange-specific sockets
    for exchange in "kraken" "coinbase" "alpaca"; do
        if [ -S "/tmp/alphapulse/$exchange.sock" ]; then
            echo "✅ $exchange socket exists"
        else
            echo "❌ $exchange socket missing"
        fi
    done
    
    echo ""
    echo "WebSocket Endpoint:"
    if curl -s -o /dev/null -w "%{http_code}" http://localhost:8765 | grep -q "426"; then
        echo "✅ WebSocket server responding on ws://localhost:8765/stream"
    else
        echo "❌ WebSocket server not responding"
    fi
}

function show_logs() {
    service="$2"
    
    if [ -z "$service" ]; then
        echo "Usage: $0 logs <service-name>"
        echo "Available services: relay, exchange-collector-kraken, exchange-collector-coinbase, exchange-collector-alpaca, ws-bridge"
        return
    fi
    
    log_file="$SCRIPT_DIR/../logs/com.alphapulse.$service.log"
    error_file="$SCRIPT_DIR/../logs/com.alphapulse.$service.error.log"
    
    if [ -f "$log_file" ]; then
        echo "=== Standard Output (last 50 lines) ==="
        tail -50 "$log_file"
    fi
    
    if [ -f "$error_file" ]; then
        echo ""
        echo "=== Error Output (last 50 lines) ==="
        tail -50 "$error_file"
    fi
}

function tail_logs() {
    echo "Tailing all service logs (Ctrl+C to stop)..."
    tail -f "$SCRIPT_DIR"/../logs/*.log
}

# Main command handling
case "$1" in
    install)
        install_services
        ;;
    uninstall)
        uninstall_services
        ;;
    start)
        start_services
        ;;
    stop)
        stop_services
        ;;
    restart)
        restart_services
        ;;
    status)
        service_status
        ;;
    logs)
        show_logs "$@"
        ;;
    tail)
        tail_logs
        ;;
    *)
        echo "AlphaPulse Daemon Manager (macOS launchd)"
        echo "========================================="
        echo ""
        echo "Usage: $0 {install|uninstall|start|stop|restart|status|logs|tail}"
        echo ""
        echo "Commands:"
        echo "  install    - Install service configurations to ~/Library/LaunchAgents"
        echo "  uninstall  - Remove service configurations"
        echo "  start      - Start all services in dependency order"
        echo "  stop       - Stop all services"
        echo "  restart    - Restart all services"
        echo "  status     - Show service status"
        echo "  logs <svc> - Show logs for specific service"
        echo "  tail       - Tail all service logs"
        echo ""
        echo "Note: This script uses macOS-specific launchd for service management."
        echo "      For Linux systems, use systemd equivalents."
        exit 1
        ;;
esac