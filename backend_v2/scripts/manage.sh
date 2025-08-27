#!/bin/bash

# AlphaPulse System Management Interface
# Unified control script for system lifecycle management
# Usage: ./scripts/manage.sh [command] [options]

set -euo pipefail

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
BOLD='\033[1m'
NC='\033[0m' # No Color

# Get the script's directory (works from any location)
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$( cd "${SCRIPT_DIR}/.." && pwd )"

# Ensure we have required directories
mkdir -p "${PROJECT_ROOT}/logs"
mkdir -p "${PROJECT_ROOT}/.pids"

# Source library scripts
LIB_DIR="${SCRIPT_DIR}/lib"

# Function to display usage
show_usage() {
    echo -e "${BOLD}AlphaPulse System Management${NC}"
    echo -e "${BOLD}============================${NC}"
    echo ""
    echo "Usage: $0 <command> [options]"
    echo ""
    echo -e "${BOLD}Commands:${NC}"
    echo -e "  ${GREEN}up${NC}       Start all AlphaPulse services"
    echo -e "  ${GREEN}down${NC}     Stop all services gracefully"
    echo -e "  ${GREEN}restart${NC}  Stop and start all services"
    echo -e "  ${GREEN}status${NC}   Show status of all services"
    echo -e "  ${GREEN}logs${NC}     Stream logs from all services"
    echo -e "  ${GREEN}help${NC}     Show this help message"
    echo ""
    echo -e "${BOLD}Options:${NC}"
    echo "  -v, --verbose    Enable verbose output"
    echo "  -q, --quiet      Suppress non-error output"
    echo "  -f, --follow     Follow log output (for logs command)"
    echo ""
    echo -e "${BOLD}Examples:${NC}"
    echo "  $0 up           # Start the system"
    echo "  $0 down         # Stop the system"
    echo "  $0 status       # Check system status"
    echo "  $0 logs -f      # Follow system logs"
    echo ""
}

# Function to print colored messages
print_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1" >&2
}

# Parse command line arguments
COMMAND="${1:-help}"
shift || true

VERBOSE=false
QUIET=false
FOLLOW=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        -v|--verbose)
            VERBOSE=true
            shift
            ;;
        -q|--quiet)
            QUIET=true
            shift
            ;;
        -f|--follow)
            FOLLOW=true
            shift
            ;;
        *)
            print_error "Unknown option: $1"
            show_usage
            exit 1
            ;;
    esac
done

# Export environment variables for sub-scripts
export PROJECT_ROOT
export VERBOSE
export QUIET
export SCRIPT_DIR
export LIB_DIR

# Main command dispatcher
case "${COMMAND}" in
    up|start)
        print_info "Starting AlphaPulse system..."
        if [[ -f "${LIB_DIR}/startup.sh" ]]; then
            source "${LIB_DIR}/startup.sh"
            start_alphapulse
        else
            print_error "Startup script not found: ${LIB_DIR}/startup.sh"
            exit 1
        fi
        ;;
    
    down|stop)
        print_info "Stopping AlphaPulse system..."
        if [[ -f "${LIB_DIR}/shutdown.sh" ]]; then
            source "${LIB_DIR}/shutdown.sh"
            stop_alphapulse
        else
            print_error "Shutdown script not found: ${LIB_DIR}/shutdown.sh"
            exit 1
        fi
        ;;
    
    restart)
        print_info "Restarting AlphaPulse system..."
        $0 down
        sleep 2
        $0 up
        ;;
    
    status)
        if [[ -f "${LIB_DIR}/status.sh" ]]; then
            source "${LIB_DIR}/status.sh"
            show_status
        else
            print_error "Status script not found: ${LIB_DIR}/status.sh"
            exit 1
        fi
        ;;
    
    logs)
        if [[ -f "${LIB_DIR}/logs.sh" ]]; then
            source "${LIB_DIR}/logs.sh"
            if [[ "${FOLLOW}" == "true" ]]; then
                follow_logs
            else
                show_logs
            fi
        else
            print_error "Logs script not found: ${LIB_DIR}/logs.sh"
            exit 1
        fi
        ;;
    
    help|--help|-h)
        show_usage
        exit 0
        ;;
    
    *)
        print_error "Unknown command: ${COMMAND}"
        echo ""
        show_usage
        exit 1
        ;;
esac