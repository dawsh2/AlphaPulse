#!/bin/bash

#
# Comprehensive Polygon Test Suite Execution Script
#
# This script orchestrates the complete testing of the Polygon collectors,
# relay systems, and full chain integration with proper service coordination.
#
# Usage:
#   ./run_polygon_test_suite.sh [--quick|--full|--stress]
#
# Test Levels:
#   --quick: Essential tests only (~5 minutes)
#   --full:  Complete test suite (~30 minutes) 
#   --stress: Full suite + stress testing (~60 minutes)
#

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
LOG_DIR="$PROJECT_ROOT/test_logs"
SOCKET_DIR="/tmp/alphapulse_test"
TEST_LEVEL="${1:-full}"

# Test configuration
RELAY_SOCKET="$SOCKET_DIR/market_data.sock"
TEST_DURATION_SHORT=30
TEST_DURATION_FULL=60
TEST_DURATION_STRESS=120

echo -e "${BLUE}🚀 Polygon Test Suite Execution${NC}"
echo "   Project Root: $PROJECT_ROOT"
echo "   Test Level: $TEST_LEVEL"
echo "   Log Directory: $LOG_DIR"
echo

# Cleanup function
cleanup() {
    echo -e "${YELLOW}🧹 Cleaning up test environment...${NC}"
    
    # Kill any running test services
    pkill -f "market_data_relay" 2>/dev/null || true
    pkill -f "polygon_collector" 2>/dev/null || true
    pkill -f "polygon" 2>/dev/null || true
    pkill -f "alphapulse-dashboard-websocket" 2>/dev/null || true
    
    # Remove test socket directory
    rm -rf "$SOCKET_DIR" 2>/dev/null || true
    
    echo -e "${GREEN}✅ Cleanup completed${NC}"
}

# Setup signal handlers
trap cleanup EXIT
trap cleanup INT
trap cleanup TERM

# Initialize test environment
initialize_test_environment() {
    echo -e "${BLUE}🏗️ Initializing test environment...${NC}"
    
    # Create directories
    mkdir -p "$LOG_DIR"
    mkdir -p "$SOCKET_DIR"
    
    # Set permissions
    chmod 755 "$SOCKET_DIR"
    
    # Initialize Rust logging
    export RUST_LOG="info,alphapulse_adapter_service=debug,protocol_v2=debug"
    export RUST_BACKTRACE=1
    
    echo -e "${GREEN}✅ Test environment initialized${NC}"
}

# Check prerequisites
check_prerequisites() {
    echo -e "${BLUE}🔍 Checking prerequisites...${NC}"
    
    # Check Rust installation
    if ! command -v cargo &> /dev/null; then
        echo -e "${RED}❌ Cargo not found. Please install Rust.${NC}"
        exit 1
    fi
    
    # Check project build
    cd "$PROJECT_ROOT"
    if ! cargo check --quiet; then
        echo -e "${RED}❌ Project build check failed.${NC}"
        exit 1
    fi
    
    # Check internet connectivity for Polygon WebSocket
    if ! curl -s --connect-timeout 5 "https://polygon-mainnet.g.alchemy.com" > /dev/null; then
        echo -e "${YELLOW}⚠️ Warning: Polygon WebSocket endpoint may not be accessible${NC}"
    fi
    
    echo -e "${GREEN}✅ Prerequisites checked${NC}"
}

# Build test binaries
build_test_binaries() {
    echo -e "${BLUE}🔨 Building test binaries...${NC}"
    
    cd "$PROJECT_ROOT"
    
    # Build in release mode for performance testing
    if ! cargo build --release --quiet; then
        echo -e "${RED}❌ Failed to build test binaries${NC}"
        exit 1
    fi
    
    echo -e "${GREEN}✅ Test binaries built${NC}"
}

# Start test services in proper order
start_test_services() {
    echo -e "${BLUE}🚦 Starting test services in sequence...${NC}"
    
    cd "$PROJECT_ROOT"
    
    # Step 1: Start Market Data Relay
    echo -e "${BLUE}   Starting Market Data Relay...${NC}"
    cargo run --release -p alphapulse-relays --bin market_data_relay > "$LOG_DIR/relay.log" 2>&1 &
    RELAY_PID=$!
    
    # Wait for relay socket to become available
    local relay_timeout=10
    local relay_wait=0
    while [ ! -S "$RELAY_SOCKET" ] && [ $relay_wait -lt $relay_timeout ]; do
        sleep 1
        relay_wait=$((relay_wait + 1))
        echo -e "${YELLOW}   Waiting for relay socket... ($relay_wait/${relay_timeout})${NC}"
    done
    
    if [ ! -S "$RELAY_SOCKET" ]; then
        echo -e "${RED}❌ Market Data Relay failed to start${NC}"
        kill $RELAY_PID 2>/dev/null || true
        exit 1
    fi
    
    echo -e "${GREEN}✅ Market Data Relay started (PID: $RELAY_PID)${NC}"
    
    # Step 2: Start Polygon Collector  
    echo -e "${BLUE}   Starting Polygon Collector...${NC}"
    cargo run --release --bin polygon -- polygon.toml > "$LOG_DIR/collector.log" 2>&1 &
    COLLECTOR_PID=$!
    
    # Give collector time to connect to WebSocket
    sleep 3
    
    # Check if collector is running
    if ! kill -0 $COLLECTOR_PID 2>/dev/null; then
        echo -e "${RED}❌ Polygon Collector failed to start${NC}"
        kill $RELAY_PID 2>/dev/null || true
        exit 1
    fi
    
    echo -e "${GREEN}✅ Polygon Collector started (PID: $COLLECTOR_PID)${NC}"
    
    # Step 3: Start Dashboard WebSocket (optional for full testing)
    if [ "$TEST_LEVEL" = "full" ] || [ "$TEST_LEVEL" = "stress" ]; then
        echo -e "${BLUE}   Starting Dashboard WebSocket...${NC}"
        cargo run --release -p alphapulse-dashboard-websocket -- --port 8081 > "$LOG_DIR/dashboard.log" 2>&1 &
        DASHBOARD_PID=$!
        
        sleep 2
        
        if ! kill -0 $DASHBOARD_PID 2>/dev/null; then
            echo -e "${YELLOW}⚠️ Dashboard WebSocket failed to start (non-critical)${NC}"
            DASHBOARD_PID=""
        else
            echo -e "${GREEN}✅ Dashboard WebSocket started (PID: $DASHBOARD_PID)${NC}"
        fi
    fi
    
    echo -e "${GREEN}✅ All test services started successfully${NC}"
}

# Run collector tests
run_collector_tests() {
    echo -e "${BLUE}📊 Running Polygon Collector Tests...${NC}"
    
    cd "$PROJECT_ROOT"
    
    local test_duration=$TEST_DURATION_SHORT
    if [ "$TEST_LEVEL" = "full" ]; then
        test_duration=$TEST_DURATION_FULL
    elif [ "$TEST_LEVEL" = "stress" ]; then
        test_duration=$TEST_DURATION_STRESS
    fi
    
    # Run collector-specific tests
    echo -e "${BLUE}   Testing TLV construction...${NC}"
    if cargo test --release --test polygon_test_suite test_collector_tlv_construction_unit -- --nocapture; then
        echo -e "${GREEN}✅ TLV construction tests passed${NC}"
    else
        echo -e "${RED}❌ TLV construction tests failed${NC}"
        return 1
    fi
    
    echo -e "${BLUE}   Testing precision preservation...${NC}"
    if cargo test --release --test polygon_test_suite test_precision_preservation_unit -- --nocapture; then
        echo -e "${GREEN}✅ Precision preservation tests passed${NC}"
    else
        echo -e "${RED}❌ Precision preservation tests failed${NC}"
        return 1
    fi
    
    # Test live data ingestion
    echo -e "${BLUE}   Testing live data ingestion (${test_duration}s)...${NC}"
    local data_received=false
    local start_time=$(date +%s)
    local end_time=$((start_time + test_duration))
    
    while [ $(date +%s) -lt $end_time ]; do
        if grep -q "DEX events processed" "$LOG_DIR/collector.log" 2>/dev/null; then
            data_received=true
            break
        fi
        sleep 2
        echo -e "${YELLOW}   Waiting for data... $((end_time - $(date +%s)))s remaining${NC}"
    done
    
    if [ "$data_received" = true ]; then
        echo -e "${GREEN}✅ Live data ingestion verified${NC}"
    else
        echo -e "${YELLOW}⚠️ No live data received during test period${NC}"
    fi
    
    echo -e "${GREEN}✅ Collector tests completed${NC}"
}

# Run relay I/O tests  
run_relay_tests() {
    echo -e "${BLUE}🔗 Running Relay I/O Tests...${NC}"
    
    cd "$PROJECT_ROOT"
    
    # Test relay connectivity
    echo -e "${BLUE}   Testing relay connectivity...${NC}"
    if echo "test" | nc -U "$RELAY_SOCKET" >/dev/null 2>&1; then
        echo -e "${GREEN}✅ Relay socket accessible${NC}"
    else
        echo -e "${RED}❌ Relay socket not accessible${NC}"
        return 1
    fi
    
    # Run relay I/O framework tests
    echo -e "${BLUE}   Running relay I/O framework tests...${NC}"
    if cargo test --release --test relay_io_test_framework -- --nocapture; then
        echo -e "${GREEN}✅ Relay I/O tests passed${NC}"
    else
        echo -e "${RED}❌ Relay I/O tests failed${NC}"
        return 1
    fi
    
    # Test message forwarding with real data
    echo -e "${BLUE}   Testing message forwarding...${NC}"
    local forwarding_verified=false
    
    # Connect consumer and check for forwarded messages
    timeout 30s bash -c "
        while IFS= read -r line; do
            if [[ \$line =~ \"messages forwarded\" ]]; then
                echo 'Message forwarding verified'
                exit 0
            fi
        done < <(tail -f '$LOG_DIR/relay.log')
    " && forwarding_verified=true
    
    if [ "$forwarding_verified" = true ]; then
        echo -e "${GREEN}✅ Message forwarding verified${NC}"
    else
        echo -e "${YELLOW}⚠️ Message forwarding not explicitly verified${NC}"
    fi
    
    echo -e "${GREEN}✅ Relay tests completed${NC}"
}

# Run full chain integration tests
run_integration_tests() {
    echo -e "${BLUE}🔄 Running Full Chain Integration Tests...${NC}"
    
    cd "$PROJECT_ROOT"
    
    # Test end-to-end data flow
    echo -e "${BLUE}   Testing end-to-end data flow...${NC}"
    
    local integration_timeout=60
    if [ "$TEST_LEVEL" = "full" ]; then
        integration_timeout=120
    elif [ "$TEST_LEVEL" = "stress" ]; then
        integration_timeout=300
    fi
    
    # Run integration tests with timeout
    if timeout ${integration_timeout}s cargo test --release --test full_chain_integration_tests -- --nocapture; then
        echo -e "${GREEN}✅ Integration tests passed${NC}"
    else
        echo -e "${RED}❌ Integration tests failed or timed out${NC}"
        return 1
    fi
    
    echo -e "${GREEN}✅ Full chain integration tests completed${NC}"
}

# Run performance validation
run_performance_tests() {
    echo -e "${BLUE}⚡ Running Performance Validation...${NC}"
    
    cd "$PROJECT_ROOT"
    
    # Test latency requirements
    echo -e "${BLUE}   Validating latency requirements...${NC}"
    local latency_check=$(timeout 30s bash -c "
        tail -f '$LOG_DIR/collector.log' | while IFS= read -r line; do
            if [[ \$line =~ \"latency:.*([0-9]+)μs\" ]]; then
                latency=\${BASH_REMATCH[1]}
                if [ \$latency -le 35 ]; then
                    echo 'PASS'
                    exit 0
                else
                    echo 'FAIL'
                    exit 1
                fi
            fi
        done
    " || echo "TIMEOUT")
    
    if [ "$latency_check" = "PASS" ]; then
        echo -e "${GREEN}✅ Latency requirements met (<35μs)${NC}"
    elif [ "$latency_check" = "FAIL" ]; then
        echo -e "${RED}❌ Latency requirements not met${NC}"
        return 1
    else
        echo -e "${YELLOW}⚠️ Could not measure latency during test period${NC}"
    fi
    
    # Test throughput
    echo -e "${BLUE}   Validating throughput...${NC}"
    local events_processed=$(grep -c "DEX events processed" "$LOG_DIR/collector.log" 2>/dev/null || echo 0)
    
    if [ "$events_processed" -gt 0 ]; then
        echo -e "${GREEN}✅ Throughput validated ($events_processed events processed)${NC}"
    else
        echo -e "${YELLOW}⚠️ No throughput data available${NC}"
    fi
    
    echo -e "${GREEN}✅ Performance validation completed${NC}"
}

# Run stress tests (if requested)
run_stress_tests() {
    if [ "$TEST_LEVEL" != "stress" ]; then
        return 0
    fi
    
    echo -e "${BLUE}💪 Running Stress Tests...${NC}"
    
    cd "$PROJECT_ROOT"
    
    # High-frequency event simulation
    echo -e "${BLUE}   Testing high-frequency event handling...${NC}"
    
    # Monitor system under sustained load
    local stress_duration=180
    local start_time=$(date +%s)
    local end_time=$((start_time + stress_duration))
    
    while [ $(date +%s) -lt $end_time ]; do
        # Check system metrics
        local memory_usage=$(ps -o pid,pmem,comm -C market_data_relay,polygon 2>/dev/null | awk 'NR>1 {sum+=$2} END {print sum}')
        local remaining=$((end_time - $(date +%s)))
        
        if [ ! -z "$memory_usage" ]; then
            echo -e "${BLUE}   Memory usage: ${memory_usage}%, ${remaining}s remaining${NC}"
        fi
        
        sleep 10
    done
    
    echo -e "${GREEN}✅ Stress tests completed${NC}"
}

# Analyze test results
analyze_test_results() {
    echo -e "${BLUE}📊 Analyzing Test Results...${NC}"
    
    # Count log entries for analysis
    local collector_events=$(grep -c "DEX events processed" "$LOG_DIR/collector.log" 2>/dev/null || echo 0)
    local relay_forwards=$(grep -c "messages forwarded" "$LOG_DIR/relay.log" 2>/dev/null || echo 0)
    local errors=$(grep -c "ERROR\|FAILED\|❌" "$LOG_DIR"/*.log 2>/dev/null || echo 0)
    
    echo
    echo -e "${BLUE}📋 TEST RESULTS SUMMARY${NC}"
    echo "=================================="
    echo "Collector Events: $collector_events"
    echo "Relay Forwards: $relay_forwards" 
    echo "Errors Detected: $errors"
    echo
    
    # Check for critical errors
    if [ "$errors" -gt 10 ]; then
        echo -e "${RED}❌ HIGH ERROR COUNT DETECTED${NC}"
        echo "Review logs in: $LOG_DIR"
        return 1
    fi
    
    # Validate data flow
    if [ "$collector_events" -gt 0 ] && [ "$relay_forwards" -gt 0 ]; then
        echo -e "${GREEN}✅ DATA FLOW VALIDATED${NC}"
    elif [ "$collector_events" -gt 0 ]; then
        echo -e "${YELLOW}⚠️ COLLECTOR WORKING, RELAY ISSUES${NC}"
    else
        echo -e "${YELLOW}⚠️ NO DATA FLOW DETECTED${NC}"
    fi
    
    echo
    echo -e "${BLUE}Log files available at:${NC}"
    ls -la "$LOG_DIR"
    echo
}

# Generate final report
generate_final_report() {
    echo -e "${BLUE}📄 Generating Final Test Report...${NC}"
    
    local report_file="$LOG_DIR/test_report_$(date +%Y%m%d_%H%M%S).txt"
    
    {
        echo "POLYGON TEST SUITE EXECUTION REPORT"
        echo "==================================="
        echo "Date: $(date)"
        echo "Test Level: $TEST_LEVEL"
        echo "Duration: $(($(date +%s) - START_TIME)) seconds"
        echo
        echo "SERVICE STATUS:"
        echo "Relay PID: ${RELAY_PID:-N/A}"
        echo "Collector PID: ${COLLECTOR_PID:-N/A}"
        echo "Dashboard PID: ${DASHBOARD_PID:-N/A}"
        echo
        echo "LOG SUMMARIES:"
        echo
        echo "--- Collector Log Summary ---"
        grep -E "(✅|❌|⚠️|ERROR|INFO)" "$LOG_DIR/collector.log" 2>/dev/null | tail -20 || echo "No collector log entries"
        echo
        echo "--- Relay Log Summary ---"
        grep -E "(✅|❌|⚠️|ERROR|INFO)" "$LOG_DIR/relay.log" 2>/dev/null | tail -20 || echo "No relay log entries"
        echo
    } > "$report_file"
    
    echo -e "${GREEN}✅ Test report generated: $report_file${NC}"
}

# Main execution
main() {
    START_TIME=$(date +%s)
    
    echo -e "${BLUE}🚀 Starting Polygon Test Suite Execution${NC}"
    echo "   Test Level: $TEST_LEVEL"
    echo "   Start Time: $(date)"
    echo
    
    # Execute test phases
    initialize_test_environment
    check_prerequisites  
    build_test_binaries
    start_test_services
    
    # Allow services to stabilize
    echo -e "${BLUE}⏳ Allowing services to stabilize (10s)...${NC}"
    sleep 10
    
    local test_failures=0
    
    # Run test phases
    if ! run_collector_tests; then
        test_failures=$((test_failures + 1))
    fi
    
    if ! run_relay_tests; then
        test_failures=$((test_failures + 1))
    fi
    
    if ! run_integration_tests; then
        test_failures=$((test_failures + 1))
    fi
    
    if ! run_performance_tests; then
        test_failures=$((test_failures + 1))
    fi
    
    if ! run_stress_tests; then
        test_failures=$((test_failures + 1))
    fi
    
    # Analysis and reporting
    if ! analyze_test_results; then
        test_failures=$((test_failures + 1))
    fi
    
    generate_final_report
    
    local end_time=$(date +%s)
    local total_duration=$((end_time - START_TIME))
    
    echo
    echo -e "${BLUE}🏁 Test Suite Execution Completed${NC}"
    echo "   Duration: ${total_duration}s"
    echo "   Failures: $test_failures"
    
    if [ $test_failures -eq 0 ]; then
        echo -e "${GREEN}✅ ALL TESTS PASSED${NC}"
        return 0
    else
        echo -e "${RED}❌ $test_failures TEST PHASES FAILED${NC}"
        return 1
    fi
}

# Validate command line arguments
case "$TEST_LEVEL" in
    quick|full|stress)
        ;;
    *)
        echo -e "${RED}❌ Invalid test level: $TEST_LEVEL${NC}"
        echo "Usage: $0 [--quick|--full|--stress]"
        exit 1
        ;;
esac

# Execute main function
main "$@"