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
    echo -e "  ${GREEN}up${NC}         Start all AlphaPulse services"
    echo -e "  ${GREEN}down${NC}       Stop all services gracefully"
    echo -e "  ${GREEN}restart${NC}    Stop and start all services"
    echo -e "  ${GREEN}status${NC}     Show status of all services"
    echo -e "  ${GREEN}logs${NC}       Stream logs from all services"
    echo ""
    echo -e "  ${BLUE}validate${NC}   Run validation checks"
    echo -e "  ${BLUE}test${NC}       Run test suite"
    echo -e "  ${BLUE}demo${NC}       Run demonstration scripts"
    echo -e "  ${BLUE}deploy${NC}     Deploy relay services"
    echo ""
    echo -e "  ${GREEN}help${NC}       Show this help message"
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
    
    validate)
        print_info "Running validation checks..."
        
        # Run precision violation detection
        print_info "Checking for precision violations..."
        python3 "${LIB_DIR}/python/detect_precision_violations.py" "${PROJECT_ROOT}" || {
            print_warning "Precision violations found - see output above"
        }
        
        # Run other validation scripts
        "${LIB_DIR}/validation/run_all_validation.sh" || {
            print_error "Creating validation runner script"
            cat > "${LIB_DIR}/validation/run_all_validation.sh" << 'EOF'
#!/bin/bash
set -e
echo "Running all validation checks..."
find "${SCRIPT_DIR}/lib/validation" -name "*.sh" -executable -exec echo "Running {}" \; -exec {} \;
EOF
            chmod +x "${LIB_DIR}/validation/run_all_validation.sh"
            "${LIB_DIR}/validation/run_all_validation.sh"
        }
        ;;
    
    test)
        print_info "Running test suite..."
        if [[ -f "${LIB_DIR}/test/run_all_tests.sh" ]]; then
            source "${LIB_DIR}/test/run_all_tests.sh"
        else
            print_info "Running available test scripts..."
            find "${LIB_DIR}/test" -name "*.sh" -executable -exec echo "Running {}" \; -exec {} \;
        fi
        ;;
    
    demo)
        print_info "Running demonstration..."
        if [[ $# -gt 0 ]]; then
            DEMO_SCRIPT="${1}"
            case "${DEMO_SCRIPT}" in
                arbitrage|arb)
                    print_info "Starting demo arbitrage data generator..."
                    python3 "${LIB_DIR}/python/send_demo_arbitrage.py"
                    ;;
                mock-relay|relay)
                    print_info "Starting mock relay server..."
                    python3 "${LIB_DIR}/python/mock_relay.py"
                    ;;
                tlv-info|tlv)
                    print_info "Querying TLV type information..."
                    python3 "${LIB_DIR}/python/query_tlv_info.py"
                    ;;
                *)
                    # Try to find shell script demos
                    if [[ -f "${LIB_DIR}/demo/${DEMO_SCRIPT}.sh" ]]; then
                        source "${LIB_DIR}/demo/${DEMO_SCRIPT}.sh"
                    else
                        print_error "Demo script not found: ${DEMO_SCRIPT}"
                        print_info "Available Python demos:"
                        echo "  arbitrage    - Demo arbitrage data generator"
                        echo "  mock-relay   - Mock relay server for testing"
                        echo "  tlv-info     - TLV type information query"
                        print_info "Shell script demos:"
                        ls "${LIB_DIR}/demo/"*.sh 2>/dev/null | xargs -n1 basename | sed 's/.sh$//' | sed 's/^/  /' || echo "  No shell demo scripts found"
                        exit 1
                    fi
                    ;;
            esac
        else
            print_info "Available Python demos:"
            echo "  arbitrage    - Demo arbitrage data generator"
            echo "  mock-relay   - Mock relay server for testing"  
            echo "  tlv-info     - TLV type information query"
            print_info "Shell script demos:"
            ls "${LIB_DIR}/demo/"*.sh 2>/dev/null | xargs -n1 basename | sed 's/.sh$//' | sed 's/^/  /' || echo "  No shell demo scripts found"
        fi
        ;;
    
    deploy)
        print_info "Deploying relay services..."
        if [[ -f "${LIB_DIR}/start_domain_relays.sh" ]]; then
            source "${LIB_DIR}/start_domain_relays.sh"
        else
            print_error "Deployment script not found: ${LIB_DIR}/start_domain_relays.sh"
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