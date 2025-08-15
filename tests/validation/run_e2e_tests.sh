#!/bin/bash

# AlphaPulse E2E Test Runner
# This script orchestrates the complete end-to-end testing suite

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test configuration
CAPTURE_DURATION=${CAPTURE_DURATION:-60}
TEST_MODE=${TEST_MODE:-"full"}  # full, quick, or services-only

echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}AlphaPulse E2E Data Validation Pipeline${NC}"
echo -e "${GREEN}========================================${NC}"

# Function to print colored output
print_status() {
    echo -e "${YELLOW}[$(date '+%H:%M:%S')] $1${NC}"
}

print_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
}

# Check Python environment
print_status "Checking Python environment..."
if ! python3 --version &> /dev/null; then
    print_error "Python 3 is required but not found"
    exit 1
fi

# Install required Python packages if needed
print_status "Checking Python dependencies..."
pip3 install -q asyncio websockets aiohttp dataclasses 2>/dev/null || true

# Check if services are already running
check_services() {
    if pgrep -f "relay_server" > /dev/null; then
        return 0
    else
        return 1
    fi
}

# Clean up function
cleanup() {
    print_status "Cleaning up..."
    # Kill any running services
    pkill -f "relay_server" 2>/dev/null || true
    pkill -f "ws_bridge" 2>/dev/null || true
    pkill -f "exchange_collector" 2>/dev/null || true
}

# Set trap for cleanup on exit
trap cleanup EXIT

# Main test execution
run_tests() {
    local test_failed=0
    
    # Test 1: Decimal Precision Tests
    print_status "Running decimal precision tests..."
    if python3 test_decimal_precision.py; then
        print_success "Decimal precision tests passed"
    else
        print_error "Decimal precision tests failed"
        test_failed=1
    fi
    
    # Test 2: Protocol Validation (standalone)
    if [ "$TEST_MODE" != "services-only" ]; then
        print_status "Running protocol validation tests..."
        if python3 -c "
from protocol_validator import BinaryProtocolReader
reader = BinaryProtocolReader()
# Quick self-test
print('Protocol validator initialized successfully')
        "; then
            print_success "Protocol validator ready"
        else
            print_error "Protocol validator initialization failed"
            test_failed=1
        fi
    fi
    
    # Test 3: Full E2E Test with Services
    if [ "$TEST_MODE" == "full" ]; then
        print_status "Starting full E2E test with live services..."
        print_status "This will capture data for ${CAPTURE_DURATION} seconds"
        
        # Check if services are already running
        if check_services; then
            print_status "Services already running, using existing instances"
            python3 test_orchestrator.py --duration ${CAPTURE_DURATION} --no-services
        else
            print_status "Starting AlphaPulse services..."
            python3 test_orchestrator.py --duration ${CAPTURE_DURATION}
        fi
        
        if [ $? -eq 0 ]; then
            print_success "E2E integration tests passed"
        else
            print_error "E2E integration tests failed"
            test_failed=1
        fi
    fi
    
    # Test 4: Generate comprehensive report
    print_status "Generating test reports..."
    
    # Find the most recent test report
    LATEST_REPORT=$(ls -t e2e_test_report_*.json 2>/dev/null | head -1)
    
    if [ -n "$LATEST_REPORT" ]; then
        print_status "Latest report: $LATEST_REPORT"
        
        # Extract summary from JSON report
        python3 -c "
import json
with open('$LATEST_REPORT', 'r') as f:
    report = json.load(f)
    summary = report.get('summary', {})
    print(f\"Total Binary Messages: {summary.get('total_binary_messages', 0)}\")
    print(f\"Total WS Messages: {summary.get('total_ws_messages', 0)}\")
    print(f\"Overall Status: {summary.get('overall_status', 'UNKNOWN')}\")
        "
    fi
    
    return $test_failed
}

# Main execution
main() {
    echo ""
    print_status "Test Configuration:"
    echo "  - Capture Duration: ${CAPTURE_DURATION}s"
    echo "  - Test Mode: ${TEST_MODE}"
    echo ""
    
    # Change to test directory
    cd "$(dirname "$0")"
    
    # Run the tests
    if run_tests; then
        echo ""
        print_success "All E2E tests completed successfully! 🎉"
        echo -e "${GREEN}========================================${NC}"
        exit 0
    else
        echo ""
        print_error "Some tests failed. Check the logs above for details."
        echo -e "${RED}========================================${NC}"
        exit 1
    fi
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --duration)
            CAPTURE_DURATION="$2"
            shift 2
            ;;
        --mode)
            TEST_MODE="$2"
            shift 2
            ;;
        --help)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --duration SECONDS   Set capture duration (default: 60)"
            echo "  --mode MODE         Set test mode: full, quick, services-only (default: full)"
            echo "  --help              Show this help message"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Run main function
main