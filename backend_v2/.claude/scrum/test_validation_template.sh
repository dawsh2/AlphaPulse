#!/bin/bash
# Test Validation Template - Use this to validate PR testing requirements
# Usage: ./test_validation_template.sh [COMPONENT] [PACKAGE] [PR_NUMBER]

set -e

COMPONENT=${1:-"unknown"}
PACKAGE=${2:-"services_v2"}
PR_NUMBER=${3:-""}

echo "🧪 COMPREHENSIVE TEST VALIDATION FOR: $COMPONENT"
echo "Package: $PACKAGE"
echo "PR: #$PR_NUMBER"
echo "======================================================="

# Function to check test results
check_test_result() {
    local test_name="$1"
    local cmd="$2"

    echo ""
    echo "🔍 Running: $test_name"
    echo "Command: $cmd"
    echo "---"

    if eval "$cmd"; then
        echo "✅ PASSED: $test_name"
        return 0
    else
        echo "❌ FAILED: $test_name"
        return 1
    fi
}

# Track results
TESTS_PASSED=0
TESTS_FAILED=0

echo ""
echo "1️⃣ UNIT TESTS"
echo "=============="
if check_test_result "Unit Tests" "cargo test --package $PACKAGE --lib"; then
    ((TESTS_PASSED++))
else
    ((TESTS_FAILED++))
fi

echo ""
echo "2️⃣ INTEGRATION TESTS"
echo "==================="
if check_test_result "Integration Tests" "cargo test --package $PACKAGE --test integration"; then
    ((TESTS_PASSED++))
else
    ((TESTS_FAILED++))
fi

echo ""
echo "3️⃣ PERFORMANCE BENCHMARKS"
echo "========================="
if check_test_result "Performance Benchmarks" "cargo bench --package $PACKAGE"; then
    ((TESTS_PASSED++))
else
    ((TESTS_FAILED++))
fi

echo ""
echo "4️⃣ COMPILATION VALIDATION"
echo "========================="
if check_test_result "Compilation Check" "cargo check --package $PACKAGE"; then
    ((TESTS_PASSED++))
else
    ((TESTS_FAILED++))
fi

echo ""
echo "5️⃣ CLIPPY LINTING"
echo "================="
if check_test_result "Clippy Linting" "cargo clippy --package $PACKAGE -- -D warnings"; then
    ((TESTS_PASSED++))
else
    ((TESTS_FAILED++))
fi

echo ""
echo "6️⃣ REAL DATA TESTING (if applicable)"
echo "===================================="
# This section varies by component type
case $COMPONENT in
    "polygon"|"binance"|"kraken"|"exchange")
        echo "Testing with live exchange data..."
        if check_test_result "Live Data Test" "timeout 30s cargo run --package $PACKAGE --bin $COMPONENT -- --test-mode"; then
            ((TESTS_PASSED++))
        else
            ((TESTS_FAILED++))
        fi
        ;;
    "protocol"|"parser"|"tlv")
        echo "Testing with real message samples..."
        if check_test_result "Message Parsing Test" "cargo test --package $PACKAGE real_data"; then
            ((TESTS_PASSED++))
        else
            ((TESTS_FAILED++))
        fi
        ;;
    *)
        echo "ℹ️  No specific real data test for component type: $COMPONENT"
        ;;
esac

echo ""
echo "7️⃣ END-TO-END VALIDATION"
echo "========================"
# Look for e2e test script
if [ -f "./scripts/test_${COMPONENT}_e2e.sh" ]; then
    if check_test_result "E2E Validation" "./scripts/test_${COMPONENT}_e2e.sh"; then
        ((TESTS_PASSED++))
    else
        ((TESTS_FAILED++))
    fi
else
    echo "ℹ️  No E2E script found: ./scripts/test_${COMPONENT}_e2e.sh"
fi

echo ""
echo "📊 TEST VALIDATION SUMMARY"
echo "=========================="
echo "Tests Passed: $TESTS_PASSED"
echo "Tests Failed: $TESTS_FAILED"
echo "Total Tests:  $((TESTS_PASSED + TESTS_FAILED))"

if [ $TESTS_FAILED -eq 0 ]; then
    echo ""
    echo "🎉 ALL TESTS PASSED!"
    echo "✅ PR testing requirements satisfied"
    echo "✅ Component ready for code review"
    exit 0
else
    echo ""
    echo "❌ TESTING REQUIREMENTS NOT MET"
    echo "❌ $TESTS_FAILED test(s) failed"
    echo "❌ PR cannot be approved until all tests pass"
    exit 1
fi
