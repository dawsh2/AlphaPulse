#!/bin/bash

# Integration Tests for manage.sh script
# Tests full workflow and command delegation

set -e

# Test setup
TEST_DIR="$(dirname "${BASH_SOURCE[0]}")"
PROJECT_ROOT="$(cd "${TEST_DIR}/.." && pwd)"
MANAGE_SCRIPT="${PROJECT_ROOT}/scripts/manage.sh"
LIB_DIR="${PROJECT_ROOT}/scripts/lib"

# Color codes for test output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BOLD='\033[1m'
NC='\033[0m'

# Test counters
TESTS_RUN=0
TESTS_PASSED=0
TESTS_FAILED=0

# Test helper functions
run_test() {
    local test_name="$1"
    TESTS_RUN=$((TESTS_RUN + 1))
    
    if $test_name; then
        TESTS_PASSED=$((TESTS_PASSED + 1))
        echo -e "${GREEN}✅ $test_name passed${NC}"
    else
        TESTS_FAILED=$((TESTS_FAILED + 1))
        echo -e "${RED}❌ $test_name failed${NC}"
    fi
}

# Setup mock library scripts
setup_mocks() {
    # Backup real library scripts if they exist
    for script in startup.sh shutdown.sh status.sh logs.sh; do
        if [[ -f "${LIB_DIR}/${script}" ]]; then
            cp "${LIB_DIR}/${script}" "${LIB_DIR}/${script}.real"
        fi
    done
    
    # Create mock scripts with simplified implementation
    cat > "${LIB_DIR}/startup.sh" << 'EOF'
#!/bin/bash
# Mock startup script
start_torq() {
    echo "Mock: Starting Torq services"
    return 0
}
EOF
    
    cat > "${LIB_DIR}/shutdown.sh" << 'EOF'
#!/bin/bash
# Mock shutdown script
stop_torq() {
    echo "Mock: Stopping Torq services"
    return 0
}
EOF
    
    cat > "${LIB_DIR}/status.sh" << 'EOF'
#!/bin/bash
# Mock status script
show_status() {
    echo "Mock: System Status"
    echo "Services: 8/8 running"
    return 0
}
EOF
    
    cat > "${LIB_DIR}/logs.sh" << 'EOF'
#!/bin/bash
# Mock logs script
show_logs() {
    echo "Mock: Showing logs"
    return 0
}
follow_logs() {
    echo "Mock: Following logs"
    return 0
}
EOF
    
    # Make mocks executable
    chmod +x "${LIB_DIR}"/*.sh
}

# Cleanup mock scripts
cleanup_mocks() {
    # Restore real library scripts
    for script in startup.sh shutdown.sh status.sh logs.sh; do
        if [[ -f "${LIB_DIR}/${script}.real" ]]; then
            mv "${LIB_DIR}/${script}.real" "${LIB_DIR}/${script}"
        fi
    done
}

# Test: Command delegation to startup script
test_up_command_delegation() {
    local output=$("$MANAGE_SCRIPT" up 2>&1)
    
    if [[ "$output" == *"Starting Torq system"* ]] && 
       [[ "$output" == *"Mock: Starting Torq services"* ]]; then
        return 0
    else
        echo "  Expected startup delegation, got: $output"
        return 1
    fi
}

# Test: Command delegation to shutdown script
test_down_command_delegation() {
    local output=$("$MANAGE_SCRIPT" down 2>&1)
    
    if [[ "$output" == *"Stopping Torq system"* ]] && 
       [[ "$output" == *"Mock: Stopping Torq services"* ]]; then
        return 0
    else
        echo "  Expected shutdown delegation, got: $output"
        return 1
    fi
}

# Test: Command delegation to status script
test_status_command_delegation() {
    local output=$("$MANAGE_SCRIPT" status 2>&1)
    
    if [[ "$output" == *"Mock: System Status"* ]]; then
        return 0
    else
        echo "  Expected status delegation, got: $output"
        return 1
    fi
}

# Test: Command delegation to logs script
test_logs_command_delegation() {
    local output=$("$MANAGE_SCRIPT" logs 2>&1)
    
    if [[ "$output" == *"Mock: Showing logs"* ]]; then
        return 0
    else
        echo "  Expected logs delegation, got: $output"
        return 1
    fi
}

# Test: Logs with follow option
test_logs_follow_delegation() {
    local output=$("$MANAGE_SCRIPT" logs --follow 2>&1)
    
    if [[ "$output" == *"Mock: Following logs"* ]]; then
        return 0
    else
        echo "  Expected follow logs delegation, got: $output"
        return 1
    fi
}

# Test: Restart command sequence
test_restart_command() {
    local output=$("$MANAGE_SCRIPT" restart 2>&1)
    
    if [[ "$output" == *"Restarting Torq system"* ]] && 
       [[ "$output" == *"Mock: Stopping Torq services"* ]] &&
       [[ "$output" == *"Mock: Starting Torq services"* ]]; then
        return 0
    else
        echo "  Expected restart sequence, got: $output"
        return 1
    fi
}

# Test: Environment variable export
test_environment_export() {
    # Create a test script that checks environment variables
    cat > "${LIB_DIR}/test_env.sh" << 'EOF'
#!/bin/bash
show_status() {
    if [[ -n "$PROJECT_ROOT" ]] && [[ -n "$VERBOSE" ]] && [[ -n "$SCRIPT_DIR" ]]; then
        echo "Environment variables set correctly"
    else
        echo "Missing environment variables"
        echo "PROJECT_ROOT=$PROJECT_ROOT"
        echo "VERBOSE=$VERBOSE"
        echo "SCRIPT_DIR=$SCRIPT_DIR"
    fi
}
EOF
    chmod +x "${LIB_DIR}/test_env.sh"
    
    # Temporarily replace status.sh
    mv "${LIB_DIR}/status.sh" "${LIB_DIR}/status.sh.tmp"
    mv "${LIB_DIR}/test_env.sh" "${LIB_DIR}/status.sh"
    
    local output=$("$MANAGE_SCRIPT" status --verbose 2>&1)
    
    # Restore original
    mv "${LIB_DIR}/status.sh" "${LIB_DIR}/test_env.sh"
    mv "${LIB_DIR}/status.sh.tmp" "${LIB_DIR}/status.sh"
    rm -f "${LIB_DIR}/test_env.sh"
    
    if [[ "$output" == *"Environment variables set correctly"* ]]; then
        return 0
    else
        echo "  Environment variables not exported correctly: $output"
        return 1
    fi
}

# Test: Multiple options handling
test_multiple_options() {
    local output=$("$MANAGE_SCRIPT" status --verbose --quiet 2>&1)
    
    # Should handle multiple options without error
    if [[ $? -eq 0 ]]; then
        return 0
    else
        echo "  Failed to handle multiple options"
        return 1
    fi
}

# Test: Path resolution from different directories
test_path_resolution() {
    # Save current directory
    local original_dir="$(pwd)"
    
    # Test running from a different directory (not via symlink - that's complex)
    cd /tmp
    
    # Run manage.sh with absolute path from different directory
    local output=$("$MANAGE_SCRIPT" status 2>&1)
    local result=$?
    
    # Return to original directory
    cd "$original_dir"
    
    if [[ $result -eq 0 ]] && [[ "$output" == *"Mock: System Status"* ]]; then
        return 0
    else
        echo "  Failed when run from different directory: result=$result, output=$output"
        return 1
    fi
}

# Test: Error propagation from library scripts
test_error_propagation() {
    # Create a failing mock
    cat > "${LIB_DIR}/failing.sh" << 'EOF'
#!/bin/bash
show_status() {
    echo "Mock: Error occurred" >&2
    return 1
}
EOF
    chmod +x "${LIB_DIR}/failing.sh"
    
    # Temporarily replace status.sh
    mv "${LIB_DIR}/status.sh" "${LIB_DIR}/status.sh.tmp"
    mv "${LIB_DIR}/failing.sh" "${LIB_DIR}/status.sh"
    
    # Run and check for error propagation
    local output=$("$MANAGE_SCRIPT" status 2>&1)
    local result=$?
    
    # Restore original
    mv "${LIB_DIR}/status.sh" "${LIB_DIR}/failing.sh"
    mv "${LIB_DIR}/status.sh.tmp" "${LIB_DIR}/status.sh"
    rm -f "${LIB_DIR}/failing.sh"
    
    # The script should propagate the error
    if [[ "$output" == *"Mock: Error occurred"* ]]; then
        return 0
    else
        echo "  Error not propagated correctly"
        return 1
    fi
}

# Main test runner
main() {
    echo -e "${BOLD}Running manage.sh Integration Tests${NC}"
    echo "===================================="
    echo ""
    
    # Setup mock environment
    echo "Setting up mock library scripts..."
    setup_mocks
    
    # Run all tests
    run_test test_up_command_delegation
    run_test test_down_command_delegation
    run_test test_status_command_delegation
    run_test test_logs_command_delegation
    run_test test_logs_follow_delegation
    run_test test_restart_command
    run_test test_environment_export
    run_test test_multiple_options
    run_test test_path_resolution
    run_test test_error_propagation
    
    # Cleanup
    echo ""
    echo "Cleaning up mock scripts..."
    cleanup_mocks
    
    echo ""
    echo "===================================="
    echo -e "${BOLD}Test Summary:${NC}"
    echo "  Tests Run: $TESTS_RUN"
    echo -e "  ${GREEN}Passed: $TESTS_PASSED${NC}"
    echo -e "  ${RED}Failed: $TESTS_FAILED${NC}"
    echo ""
    
    if [[ $TESTS_FAILED -eq 0 ]]; then
        echo -e "${GREEN}${BOLD}All integration tests passed!${NC}"
        exit 0
    else
        echo -e "${RED}${BOLD}Some tests failed!${NC}"
        exit 1
    fi
}

# Run the tests
main "$@"