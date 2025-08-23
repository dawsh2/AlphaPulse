#!/bin/bash
# AlphaPulse End-to-End Test Script
# Orchestrates the complete system test including service startup, test execution, and cleanup

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
TEST_DIR="$PROJECT_ROOT/tests/e2e"
LOG_DIR="/tmp/alphapulse_e2e_logs"
PID_FILE="/tmp/alphapulse_e2e.pids"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Functions
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

cleanup() {
    log_info "Cleaning up test environment..."
    
    # Stop all test processes
    if [[ -f "$PID_FILE" ]]; then
        while IFS= read -r pid; do
            if ps -p "$pid" > /dev/null 2>&1; then
                log_info "Stopping process $pid"
                kill -TERM "$pid" 2>/dev/null || true
                sleep 1
                if ps -p "$pid" > /dev/null 2>&1; then
                    kill -KILL "$pid" 2>/dev/null || true
                fi
            fi
        done < "$PID_FILE"
        rm -f "$PID_FILE"
    fi
    
    # Clean up test sockets and directories
    rm -rf /tmp/alphapulse/e2e_*
    rm -rf /tmp/alphapulse_e2e_*
    
    log_success "Cleanup completed"
}

# Trap for cleanup on exit
trap cleanup EXIT

check_dependencies() {
    log_info "Checking dependencies..."
    
    # Check Rust/Cargo
    if ! command -v cargo &> /dev/null; then
        log_error "cargo not found. Please install Rust."
        exit 1
    fi
    
    # Check if project builds
    cd "$PROJECT_ROOT"
    if ! cargo check --workspace --quiet; then
        log_error "Project does not compile. Please fix build errors first."
        exit 1
    fi
    
    log_success "Dependencies check passed"
}

build_project() {
    log_info "Building project..."
    cd "$PROJECT_ROOT"
    
    # Build the main protocol and services
    if ! cargo build --release --package alphapulse-protocol; then
        log_error "Failed to build alphapulse-protocol"
        exit 1
    fi
    
    if ! cargo build --release --package alphapulse-adapters; then
        log_error "Failed to build alphapulse-adapters"
        exit 1
    fi
    
    if ! cargo build --release --package alphapulse-kraken-signals; then
        log_error "Failed to build alphapulse-kraken-signals"
        exit 1
    fi
    
    if ! cargo build --release --package alphapulse-dashboard-websocket; then
        log_error "Failed to build alphapulse-dashboard-websocket"
        exit 1
    fi
    
    # Build E2E tests
    cd "$TEST_DIR"
    if ! cargo build --release; then
        log_error "Failed to build E2E tests"
        exit 1
    fi
    
    log_success "Build completed"
}

setup_test_environment() {
    log_info "Setting up test environment..."
    
    # Create log directory
    mkdir -p "$LOG_DIR"
    
    # Create socket directories
    mkdir -p /tmp/alphapulse
    
    # Initialize PID file
    : > "$PID_FILE"
    
    log_success "Test environment setup completed"
}

run_basic_tests() {
    log_info "Running basic connectivity tests..."
    cd "$TEST_DIR"
    
    if cargo run --release --bin e2e_runner -- --scenario kraken --timeout 120 --validation basic --output "$LOG_DIR/basic_test_results.json"; then
        log_success "Basic tests passed"
        return 0
    else
        log_error "Basic tests failed"
        return 1
    fi
}

run_comprehensive_tests() {
    log_info "Running comprehensive tests..."
    cd "$TEST_DIR"
    
    if cargo run --release --bin e2e_runner -- --scenario all --timeout 300 --validation comprehensive --output "$LOG_DIR/comprehensive_test_results.json" --verbose; then
        log_success "Comprehensive tests passed"
        return 0
    else
        log_error "Comprehensive tests failed"
        return 1
    fi
}

run_polygon_arbitrage_tests() {
    log_info "Running Polygon arbitrage tests..."
    cd "$TEST_DIR"
    
    if cargo run --release --bin e2e_runner -- --scenario polygon --timeout 300 --validation comprehensive --live-data --output "$LOG_DIR/polygon_arbitrage_results.json" --verbose; then
        log_success "Polygon arbitrage tests passed"
        return 0
    else
        log_error "Polygon arbitrage tests failed"
        return 1
    fi
}

run_live_data_tests() {
    log_info "Running live data tests (requires internet connection)..."
    cd "$TEST_DIR"
    
    if cargo run --release --bin e2e_runner -- --scenario kraken --timeout 180 --validation comprehensive --live-data --output "$LOG_DIR/live_data_test_results.json"; then
        log_success "Live data tests passed"
        return 0
    else
        log_warning "Live data tests failed (may be due to network issues)"
        return 0  # Don't fail the entire suite for network issues
    fi
}

generate_test_report() {
    log_info "Generating test report..."
    
    cat > "$LOG_DIR/test_report.html" << EOF
<!DOCTYPE html>
<html>
<head>
    <title>AlphaPulse E2E Test Report</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 20px; }
        .header { background-color: #f0f0f0; padding: 20px; border-radius: 5px; }
        .test-section { margin: 20px 0; padding: 15px; border: 1px solid #ddd; border-radius: 5px; }
        .pass { background-color: #d4edda; border-color: #c3e6cb; }
        .fail { background-color: #f8d7da; border-color: #f5c6cb; }
        .warning { background-color: #fff3cd; border-color: #ffeaa7; }
        pre { background-color: #f8f9fa; padding: 10px; border-radius: 3px; overflow-x: auto; }
    </style>
</head>
<body>
    <div class="header">
        <h1>AlphaPulse E2E Test Report</h1>
        <p>Generated: $(date)</p>
        <p>Version: $(cd "$PROJECT_ROOT" && git describe --tags --always --dirty)</p>
    </div>
EOF

    # Add test results if they exist
    for result_file in "$LOG_DIR"/*.json; do
        if [[ -f "$result_file" ]]; then
            echo "    <div class=\"test-section\">" >> "$LOG_DIR/test_report.html"
            echo "        <h3>$(basename "$result_file" .json)</h3>" >> "$LOG_DIR/test_report.html"
            echo "        <pre>$(cat "$result_file" | jq '.' 2>/dev/null || cat "$result_file")</pre>" >> "$LOG_DIR/test_report.html"
            echo "    </div>" >> "$LOG_DIR/test_report.html"
        fi
    done

    cat >> "$LOG_DIR/test_report.html" << EOF
</body>
</html>
EOF

    log_success "Test report generated: $LOG_DIR/test_report.html"
}

# Main execution
main() {
    local test_mode="${1:-comprehensive}"
    
    echo "┌─────────────────────────────────────────┐"
    echo "│        AlphaPulse E2E Test Suite        │"
    echo "└─────────────────────────────────────────┘"
    echo
    
    log_info "Starting E2E test suite in '$test_mode' mode"
    
    check_dependencies
    setup_test_environment
    build_project
    
    local exit_code=0
    
    case "$test_mode" in
        "basic")
            run_basic_tests || exit_code=1
            ;;
        "comprehensive")
            run_basic_tests || exit_code=1
            run_comprehensive_tests || exit_code=1
            ;;
        "arbitrage")
            run_polygon_arbitrage_tests || exit_code=1
            ;;
        "full")
            run_basic_tests || exit_code=1
            run_comprehensive_tests || exit_code=1
            run_polygon_arbitrage_tests || exit_code=1
            run_live_data_tests || exit_code=1
            ;;
        *)
            log_error "Unknown test mode: $test_mode"
            log_info "Available modes: basic, comprehensive, arbitrage, full"
            exit 1
            ;;
    esac
    
    generate_test_report
    
    if [[ $exit_code -eq 0 ]]; then
        log_success "All tests completed successfully!"
        echo
        log_info "Test results available in: $LOG_DIR/"
        log_info "Test report: $LOG_DIR/test_report.html"
    else
        log_error "Some tests failed. Check logs in: $LOG_DIR/"
        exit 1
    fi
}

# Script usage
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "${1:-comprehensive}"
fi