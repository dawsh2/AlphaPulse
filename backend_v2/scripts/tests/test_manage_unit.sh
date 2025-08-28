#!/bin/bash

# Unit Tests for manage.sh script
# Tests command validation, directory creation, and basic functionality

set -e

# Test setup
TEST_DIR="$(dirname "${BASH_SOURCE[0]}")"
PROJECT_ROOT="$(cd "${TEST_DIR}/.." && pwd)"
MANAGE_SCRIPT="${PROJECT_ROOT}/scripts/manage.sh"

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

# Test: Invalid command handling
test_invalid_command() {
    local output=$("$MANAGE_SCRIPT" invalid_command 2>&1 || true)
    
    if [[ "$output" == *"Unknown command: invalid_command"* ]] && [[ "$output" == *"Usage:"* ]]; then
        return 0
    else
        echo "  Expected error message and usage, got: $output"
        return 1
    fi
}

# Test: Help command
test_help_command() {
    local output=$("$MANAGE_SCRIPT" help 2>&1)
    
    if [[ "$output" == *"Torq System Management"* ]] && 
       [[ "$output" == *"Commands:"* ]] && 
       [[ "$output" == *"Options:"* ]]; then
        return 0
    else
        echo "  Expected help text, got: $output"
        return 1
    fi
}

# Test: Directory auto-creation
test_directory_creation() {
    # Clean up test directories
    rm -rf "${PROJECT_ROOT}/logs" "${PROJECT_ROOT}/.pids"
    
    # Run status command which should trigger directory creation
    "$MANAGE_SCRIPT" status > /dev/null 2>&1 || true
    
    if [[ -d "${PROJECT_ROOT}/logs" && -d "${PROJECT_ROOT}/.pids" ]]; then
        return 0
    else
        echo "  Directories not created: logs=$(test -d ${PROJECT_ROOT}/logs && echo yes || echo no), .pids=$(test -d ${PROJECT_ROOT}/.pids && echo yes || echo no)"
        return 1
    fi
}

# Test: Script is executable
test_script_executable() {
    if [[ -x "$MANAGE_SCRIPT" ]]; then
        return 0
    else
        echo "  Script is not executable: $MANAGE_SCRIPT"
        return 1
    fi
}

# Test: Unknown option handling
test_unknown_option() {
    local output=$("$MANAGE_SCRIPT" up --invalid-option 2>&1 || true)
    
    if [[ "$output" == *"Unknown option: --invalid-option"* ]]; then
        return 0
    else
        echo "  Expected error for unknown option, got: $output"
        return 1
    fi
}

# Test: Valid options parsing (verbose)
test_verbose_option() {
    # This should work without error
    local output=$("$MANAGE_SCRIPT" status --verbose 2>&1)
    
    if [[ $? -eq 0 ]]; then
        return 0
    else
        echo "  Verbose option failed"
        return 1
    fi
}

# Test: Path independence
test_path_independence() {
    # Save current directory
    local original_dir="$(pwd)"
    
    # Change to temp directory
    cd /tmp
    
    # Run manage.sh from different location
    local output=$("$MANAGE_SCRIPT" help 2>&1)
    local result=$?
    
    # Return to original directory
    cd "$original_dir"
    
    if [[ $result -eq 0 ]] && [[ "$output" == *"Torq System Management"* ]]; then
        return 0
    else
        echo "  Script failed when run from different directory"
        return 1
    fi
}

# Test: Library script checking
test_library_script_check() {
    # Temporarily rename a library script to test error handling
    if [[ -f "${PROJECT_ROOT}/scripts/lib/status.sh" ]]; then
        mv "${PROJECT_ROOT}/scripts/lib/status.sh" "${PROJECT_ROOT}/scripts/lib/status.sh.bak"
    fi
    
    local output=$("$MANAGE_SCRIPT" status 2>&1 || true)
    local has_error=false
    
    if [[ "$output" == *"Status script not found"* ]]; then
        has_error=true
    fi
    
    # Restore the library script
    if [[ -f "${PROJECT_ROOT}/scripts/lib/status.sh.bak" ]]; then
        mv "${PROJECT_ROOT}/scripts/lib/status.sh.bak" "${PROJECT_ROOT}/scripts/lib/status.sh"
    fi
    
    if [[ "$has_error" == "true" ]]; then
        return 0
    else
        echo "  Expected error when library script missing"
        return 1
    fi
}

# Test: Quiet option
test_quiet_option() {
    local output=$("$MANAGE_SCRIPT" status --quiet 2>&1)
    
    # Quiet mode should still work
    if [[ $? -eq 0 ]]; then
        return 0
    else
        echo "  Quiet option failed"
        return 1
    fi
}

# Test: Follow option for logs
test_follow_option() {
    # Test that follow option is accepted (we can't test actual following in unit test)
    local output=$(timeout 1 "$MANAGE_SCRIPT" logs --follow 2>&1 || true)
    
    # Should not error on the option itself
    if [[ "$output" != *"Unknown option"* ]]; then
        return 0
    else
        echo "  Follow option not recognized"
        return 1
    fi
}

# Main test runner
main() {
    echo -e "${BOLD}Running manage.sh Unit Tests${NC}"
    echo "============================="
    echo ""
    
    # Run all tests
    run_test test_invalid_command
    run_test test_help_command
    run_test test_directory_creation
    run_test test_script_executable
    run_test test_unknown_option
    run_test test_verbose_option
    run_test test_path_independence
    run_test test_library_script_check
    run_test test_quiet_option
    run_test test_follow_option
    
    echo ""
    echo "============================="
    echo -e "${BOLD}Test Summary:${NC}"
    echo "  Tests Run: $TESTS_RUN"
    echo -e "  ${GREEN}Passed: $TESTS_PASSED${NC}"
    echo -e "  ${RED}Failed: $TESTS_FAILED${NC}"
    echo ""
    
    if [[ $TESTS_FAILED -eq 0 ]]; then
        echo -e "${GREEN}${BOLD}All unit tests passed!${NC}"
        exit 0
    else
        echo -e "${RED}${BOLD}Some tests failed!${NC}"
        exit 1
    fi
}

# Run the tests
main "$@"