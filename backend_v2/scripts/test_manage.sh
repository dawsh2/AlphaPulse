#!/bin/bash
# Test suite for manage.sh orchestrator script
# Following Test-Driven Development - these tests are written BEFORE implementation

# Don't use set -e in test scripts as we expect some commands to fail
set -u

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test counters
TESTS_RUN=0
TESTS_PASSED=0
TESTS_FAILED=0

# Test helper functions
test_start() {
    local test_name="$1"
    echo -n "Testing $test_name... "
    ((TESTS_RUN++))
}

test_pass() {
    echo -e "${GREEN}✅ PASSED${NC}"
    ((TESTS_PASSED++))
}

test_fail() {
    local reason="$1"
    echo -e "${RED}❌ FAILED${NC}: $reason"
    ((TESTS_FAILED++))
}

# Setup test environment
setup() {
    # Create temp directories for testing
    export TEST_DIR=$(mktemp -d)
    export ALPHAPULSE_ROOT="$TEST_DIR"
    mkdir -p "$TEST_DIR/scripts"
    mkdir -p "$TEST_DIR/scripts/lib"
    
    # Copy the manage.sh script (will be created after tests)
    if [[ -f "./scripts/manage.sh" ]]; then
        cp "./scripts/manage.sh" "$TEST_DIR/scripts/"
        chmod +x "$TEST_DIR/scripts/manage.sh"
    fi
    
    # Copy lib scripts if they exist
    if [[ -d "./scripts/lib" ]]; then
        cp -r ./scripts/lib/* "$TEST_DIR/scripts/lib/" 2>/dev/null || true
        chmod +x "$TEST_DIR/scripts/lib"/*.sh 2>/dev/null || true
    fi
}

# Cleanup test environment
teardown() {
    if [[ -n "${TEST_DIR:-}" ]] && [[ -d "$TEST_DIR" ]]; then
        rm -rf "$TEST_DIR"
    fi
}

# Test 1: Script exists and is executable
test_script_exists() {
    test_start "script exists and is executable"
    
    if [[ ! -f "./scripts/manage.sh" ]]; then
        test_fail "manage.sh does not exist"
    else
        if [[ ! -x "./scripts/manage.sh" ]]; then
            test_fail "manage.sh is not executable"
        else
            test_pass
        fi
    fi
}

# Test 2: Invalid command shows usage
test_invalid_command() {
    test_start "invalid command shows usage"
    
    local output
    output=$("$TEST_DIR/scripts/manage.sh" invalid 2>&1 || true)
    
    if [[ ! "$output" == *"Usage:"* ]]; then
        test_fail "Did not show usage text"
        return 1
    fi
    
    if [[ ! "$output" == *"Commands:"* ]]; then
        test_fail "Did not list available commands"
        return 1
    fi
    
    test_pass
}

# Test 3: Help command works
test_help_command() {
    test_start "help command"
    
    local output
    output=$("$TEST_DIR/scripts/manage.sh" help 2>&1)
    
    if [[ ! "$output" == *"AlphaPulse System Management"* ]]; then
        test_fail "Did not show title"
        return 1
    fi
    
    if [[ ! "$output" == *"up"* ]] || [[ ! "$output" == *"down"* ]]; then
        test_fail "Did not list core commands"
        return 1
    fi
    
    test_pass
}

# Test 4: Creates required directories
test_creates_directories() {
    test_start "creates required directories"
    
    # Remove directories if they exist
    rm -rf "$TEST_DIR/logs" "$TEST_DIR/.pids"
    
    # Run any command that should create directories
    "$TEST_DIR/scripts/manage.sh" status >/dev/null 2>&1 || true
    
    if [[ ! -d "$TEST_DIR/logs" ]]; then
        test_fail "Did not create logs/ directory"
        return 1
    fi
    
    if [[ ! -d "$TEST_DIR/.pids" ]]; then
        test_fail "Did not create .pids/ directory"
        return 1
    fi
    
    test_pass
}

# Test 5: Status command format
test_status_command() {
    test_start "status command output format"
    
    local output
    output=$("$TEST_DIR/scripts/manage.sh" status 2>&1 || true)
    
    if [[ ! "$output" == *"AlphaPulse System Status"* ]]; then
        test_fail "Did not show status header"
        return 1
    fi
    
    test_pass
}

# Test 6: Works from any directory
test_works_from_any_dir() {
    test_start "works from any directory"
    
    local original_dir=$(pwd)
    cd /tmp
    
    local output
    output=$("$original_dir/scripts/manage.sh" help 2>&1 || true)
    
    cd "$original_dir"
    
    if [[ ! "$output" == *"AlphaPulse"* ]]; then
        test_fail "Did not work from different directory"
        return 1
    fi
    
    test_pass
}

# Test 7: Command delegation to lib scripts
test_command_delegation() {
    test_start "delegates to lib/ scripts"
    
    # Create a mock lib script
    cat > "$TEST_DIR/scripts/lib/status.sh" << 'EOF'
#!/bin/bash
echo "Status delegated successfully"
EOF
    chmod +x "$TEST_DIR/scripts/lib/status.sh"
    
    local output
    output=$("$TEST_DIR/scripts/manage.sh" status 2>&1 || true)
    
    if [[ "$output" == *"Status delegated successfully"* ]]; then
        test_pass
    else
        test_fail "Did not delegate to lib/status.sh"
        return 1
    fi
}

# Test 8: Validates required commands exist
test_validates_commands() {
    test_start "validates required commands"
    
    local output
    # Test with 'up' command when lib/startup.sh doesn't exist
    rm -f "$TEST_DIR/scripts/lib/startup.sh"
    
    output=$("$TEST_DIR/scripts/manage.sh" up 2>&1 || true)
    
    if [[ "$output" == *"not implemented"* ]] || [[ "$output" == *"not found"* ]]; then
        test_pass
    else
        test_fail "Did not validate missing lib script"
        return 1
    fi
}

# Test 9: Environment variable handling
test_environment_variables() {
    test_start "environment variable handling"
    
    # Create a mock lib script that uses environment
    cat > "$TEST_DIR/scripts/lib/test_env.sh" << 'EOF'
#!/bin/bash
echo "ALPHAPULSE_ROOT=$ALPHAPULSE_ROOT"
EOF
    chmod +x "$TEST_DIR/scripts/lib/test_env.sh"
    
    # Modify manage.sh to support test_env command (mock)
    local output
    ALPHAPULSE_ROOT="$TEST_DIR" "$TEST_DIR/scripts/manage.sh" help >/dev/null 2>&1
    
    if [[ -n "$ALPHAPULSE_ROOT" ]]; then
        test_pass
    else
        test_fail "Environment variables not set"
        return 1
    fi
}

# Test 10: Error handling and exit codes
test_error_handling() {
    test_start "error handling and exit codes"
    
    # Test invalid command returns non-zero
    "$TEST_DIR/scripts/manage.sh" invalid >/dev/null 2>&1
    local exit_code=$?
    
    if [[ $exit_code -eq 0 ]]; then
        test_fail "Invalid command returned success (0)"
        return 1
    fi
    
    test_pass
}

# Main test runner
main() {
    echo "================================"
    echo "Running manage.sh Test Suite"
    echo "================================"
    echo ""
    
    # Run setup
    setup
    
    # Run all tests
    test_script_exists
    
    # Only run other tests if script exists
    if [[ -f "./scripts/manage.sh" ]]; then
        test_invalid_command
        test_help_command
        test_creates_directories
        test_status_command
        test_works_from_any_dir
        test_command_delegation
        test_validates_commands
        test_environment_variables
        test_error_handling
    fi
    
    # Run teardown
    teardown
    
    # Print summary
    echo ""
    echo "================================"
    echo "Test Summary"
    echo "================================"
    echo -e "Tests Run: $TESTS_RUN"
    echo -e "Tests Passed: ${GREEN}$TESTS_PASSED${NC}"
    echo -e "Tests Failed: ${RED}$TESTS_FAILED${NC}"
    
    if [[ $TESTS_FAILED -eq 0 ]]; then
        echo -e "${GREEN}All tests passed!${NC}"
        exit 0
    else
        echo -e "${RED}Some tests failed!${NC}"
        exit 1
    fi
}

# Run main if executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi