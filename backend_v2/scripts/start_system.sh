#!/bin/bash
# AlphaPulse System Orchestration Script
# Manages the complete system including all services, relays, and strategies

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
PID_FILE="/tmp/alphapulse_system.pids"
LOG_DIR="/tmp/alphapulse_logs"
SOCKET_DIR="/tmp/alphapulse"

# Service configuration
SERVICES=(
    "market_data_relay:$PROJECT_ROOT/protocol_v2/target/release/market_data_relay"
    "signal_relay:$PROJECT_ROOT/protocol_v2/target/release/signal_relay"
    "execution_relay:$PROJECT_ROOT/protocol_v2/target/release/execution_relay"
    "kraken_collector:$PROJECT_ROOT/services_v2/adapters/target/release/kraken_collector"
    "kraken_strategy:$PROJECT_ROOT/services_v2/strategies/kraken_signals/target/release/kraken_signals"
    "dashboard_server:$PROJECT_ROOT/services_v2/dashboard/websocket_server/target/release/alphapulse-dashboard-websocket"
)

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if service is running
is_service_running() {
    local service_name="$1"
    local pid_file="$LOG_DIR/$service_name.pid"
    
    if [[ -f "$pid_file" ]]; then
        local pid=$(cat "$pid_file")
        if ps -p "$pid" > /dev/null 2>&1; then
            return 0
        else
            rm -f "$pid_file"
        fi
    fi
    return 1
}

# Start a service
start_service() {
    local service_def="$1"
    local service_name="${service_def%%:*}"
    local service_binary="${service_def#*:}"
    
    if is_service_running "$service_name"; then
        log_warning "Service $service_name is already running"
        return 0
    fi
    
    if [[ ! -f "$service_binary" ]]; then
        log_error "Service binary not found: $service_binary"
        log_info "Please build the project first: cargo build --release --workspace"
        return 1
    fi
    
    log_info "Starting $service_name..."
    
    local log_file="$LOG_DIR/$service_name.log"
    local pid_file="$LOG_DIR/$service_name.pid"
    
    # Start service in background
    case "$service_name" in
        "market_data_relay")
            RUST_LOG=info "$service_binary" --socket-path "$SOCKET_DIR/market_data.sock" \
                > "$log_file" 2>&1 &
            ;;
        "signal_relay")
            RUST_LOG=info "$service_binary" --socket-path "$SOCKET_DIR/signals.sock" \
                > "$log_file" 2>&1 &
            ;;
        "execution_relay")
            RUST_LOG=info "$service_binary" --socket-path "$SOCKET_DIR/execution.sock" \
                > "$log_file" 2>&1 &
            ;;
        "kraken_collector")
            RUST_LOG=info "$service_binary" \
                --market-data-relay "$SOCKET_DIR/market_data.sock" \
                > "$log_file" 2>&1 &
            ;;
        "kraken_strategy")
            RUST_LOG=info "$service_binary" \
                --market-data-relay "$SOCKET_DIR/market_data.sock" \
                --signal-relay "$SOCKET_DIR/signals.sock" \
                > "$log_file" 2>&1 &
            ;;
        "dashboard_server")
            "$service_binary" \
                --market-data-relay "$SOCKET_DIR/market_data.sock" \
                --signal-relay "$SOCKET_DIR/signals.sock" \
                --execution-relay "$SOCKET_DIR/execution.sock" \
                --port 8080 \
                --enable-cors \
                > "$log_file" 2>&1 &
            ;;
        *)
            log_error "Unknown service: $service_name"
            return 1
            ;;
    esac
    
    local pid=$!
    echo "$pid" > "$pid_file"
    echo "$service_name:$pid" >> "$PID_FILE"
    
    # Wait a moment and check if service started successfully
    sleep 1
    if ps -p "$pid" > /dev/null 2>&1; then
        log_success "Started $service_name (PID: $pid)"
        return 0
    else
        log_error "Failed to start $service_name"
        rm -f "$pid_file"
        return 1
    fi
}

# Stop a service
stop_service() {
    local service_name="$1"
    local pid_file="$LOG_DIR/$service_name.pid"
    
    if [[ ! -f "$pid_file" ]]; then
        log_warning "Service $service_name is not running"
        return 0
    fi
    
    local pid=$(cat "$pid_file")
    
    if ps -p "$pid" > /dev/null 2>&1; then
        log_info "Stopping $service_name (PID: $pid)..."
        
        # Try graceful shutdown first
        kill -TERM "$pid"
        
        # Wait up to 10 seconds for graceful shutdown
        local count=0
        while ps -p "$pid" > /dev/null 2>&1 && [[ $count -lt 10 ]]; do
            sleep 1
            ((count++))
        done
        
        # Force kill if still running
        if ps -p "$pid" > /dev/null 2>&1; then
            log_warning "Force killing $service_name"
            kill -KILL "$pid"
        fi
        
        log_success "Stopped $service_name"
    fi
    
    rm -f "$pid_file"
}

# Setup environment
setup_environment() {
    log_info "Setting up environment..."
    
    # Create directories
    mkdir -p "$LOG_DIR"
    mkdir -p "$SOCKET_DIR"
    
    # Initialize PID file
    : > "$PID_FILE"
    
    log_success "Environment setup completed"
}

# Build project
build_project() {
    log_info "Building project..."
    
    cd "$PROJECT_ROOT"
    
    # Build all packages
    if ! cargo build --release --workspace; then
        log_error "Failed to build project"
        return 1
    fi
    
    log_success "Project built successfully"
}

# Check system health
check_health() {
    log_info "Checking system health..."
    
    local healthy=true
    
    # Check if services are running
    for service_def in "${SERVICES[@]}"; do
        local service_name="${service_def%%:*}"
        if is_service_running "$service_name"; then
            log_success "$service_name is running"
        else
            log_error "$service_name is not running"
            healthy=false
        fi
    done
    
    # Check socket files
    for socket in market_data.sock signals.sock execution.sock; do
        if [[ -S "$SOCKET_DIR/$socket" ]]; then
            log_success "Socket $socket exists"
        else
            log_warning "Socket $socket missing"
        fi
    done
    
    # Check dashboard endpoint
    if curl -s http://localhost:8080/health > /dev/null 2>&1; then
        log_success "Dashboard health endpoint responding"
    else
        log_warning "Dashboard health endpoint not responding"
    fi
    
    if $healthy; then
        log_success "System health check passed"
        return 0
    else
        log_error "System health check failed"
        return 1
    fi
}

# Show system status
show_status() {
    echo "┌─────────────────────────────────────────────────────────────────┐"
    echo "│                    AlphaPulse System Status                     │"
    echo "└─────────────────────────────────────────────────────────────────┘"
    echo
    
    printf "%-20s %-10s %-10s %-30s\n" "SERVICE" "STATUS" "PID" "LOG FILE"
    echo "───────────────────────────────────────────────────────────────────"
    
    for service_def in "${SERVICES[@]}"; do
        local service_name="${service_def%%:*}"
        local pid_file="$LOG_DIR/$service_name.pid"
        local log_file="$LOG_DIR/$service_name.log"
        
        if is_service_running "$service_name"; then
            local pid=$(cat "$pid_file")
            printf "%-20s %-10s %-10s %-30s\n" "$service_name" "RUNNING" "$pid" "$log_file"
        else
            printf "%-20s %-10s %-10s %-30s\n" "$service_name" "STOPPED" "-" "$log_file"
        fi
    done
    
    echo
    echo "Socket Directory: $SOCKET_DIR"
    echo "Log Directory: $LOG_DIR"
    echo "Dashboard URL: http://localhost:8080"
}

# Start all services
start_all() {
    log_info "Starting AlphaPulse system..."
    
    setup_environment
    
    # Start services in order (relays first, then collectors, then strategies)
    local success=true
    
    for service_def in "${SERVICES[@]}"; do
        if ! start_service "$service_def"; then
            success=false
            break
        fi
        
        # Small delay between service starts
        sleep 2
    done
    
    if $success; then
        log_success "All services started successfully"
        show_status
        echo
        log_info "Dashboard available at: http://localhost:8080"
        log_info "Health check endpoint: http://localhost:8080/health"
        log_info "Use '$0 status' to check system status"
        log_info "Use '$0 stop' to stop all services"
    else
        log_error "Failed to start all services"
        stop_all
        return 1
    fi
}

# Stop all services
stop_all() {
    log_info "Stopping AlphaPulse system..."
    
    # Stop services in reverse order
    local reversed_services=()
    for ((i=${#SERVICES[@]}-1; i>=0; i--)); do
        reversed_services+=("${SERVICES[i]}")
    done
    
    for service_def in "${reversed_services[@]}"; do
        local service_name="${service_def%%:*}"
        stop_service "$service_name"
    done
    
    # Clean up PID file
    rm -f "$PID_FILE"
    
    # Clean up socket files
    rm -f "$SOCKET_DIR"/*.sock
    
    log_success "All services stopped"
}

# Restart all services
restart_all() {
    log_info "Restarting AlphaPulse system..."
    stop_all
    sleep 2
    start_all
}

# Show logs
show_logs() {
    local service_name="${1:-}"
    
    if [[ -z "$service_name" ]]; then
        log_info "Available services:"
        for service_def in "${SERVICES[@]}"; do
            echo "  - ${service_def%%:*}"
        done
        return 0
    fi
    
    local log_file="$LOG_DIR/$service_name.log"
    
    if [[ -f "$log_file" ]]; then
        log_info "Showing logs for $service_name (tail -f $log_file)"
        tail -f "$log_file"
    else
        log_error "Log file not found: $log_file"
        return 1
    fi
}

# Cleanup function
cleanup() {
    if [[ -f "$PID_FILE" ]]; then
        log_info "Cleaning up on exit..."
        stop_all
    fi
}

# Main function
main() {
    local command="${1:-start}"
    
    case "$command" in
        "start")
            start_all
            ;;
        "stop")
            stop_all
            ;;
        "restart")
            restart_all
            ;;
        "status")
            show_status
            ;;
        "health")
            check_health
            ;;
        "build")
            build_project
            ;;
        "logs")
            show_logs "${2:-}"
            ;;
        "help"|"-h"|"--help")
            cat << EOF
AlphaPulse System Orchestration Script

Usage: $0 [COMMAND] [OPTIONS]

Commands:
    start       Start all system services
    stop        Stop all system services
    restart     Restart all system services
    status      Show system status
    health      Check system health
    build       Build the project
    logs [SVC]  Show logs for a service (or list services)
    help        Show this help message

Services:
    market_data_relay   - Market data relay service
    signal_relay        - Trading signal relay service
    execution_relay     - Execution relay service
    kraken_collector    - Kraken data collector
    kraken_strategy     - Kraken trading strategy
    dashboard_server    - WebSocket dashboard server

Examples:
    $0 start                # Start all services
    $0 status               # Show status
    $0 logs kraken_strategy # Show strategy logs
    $0 health               # Check system health

EOF
            ;;
        *)
            log_error "Unknown command: $command"
            log_info "Use '$0 help' for usage information"
            exit 1
            ;;
    esac
}

# Trap for cleanup on exit
trap cleanup EXIT

# Run main function
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi