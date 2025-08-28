#!/bin/bash

# Torq Pattern Enforcement Runner
# Runs all architectural pattern checks for CI/CD integration

set -euo pipefail

# Colors for output
RED='\033[0;31m'
YELLOW='\033[1;33m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
BOLD='\033[1m'
NC='\033[0m' # No Color

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$(dirname "$SCRIPT_DIR")")"

# Configuration
TARGET_DIR="${1:-$PROJECT_ROOT}"
FAIL_FAST="${2:-false}"

# Pattern check scripts
PATTERN_SCRIPTS=(
    "detect-transport-violations.sh"
    "detect-precision-violations.py"
)

echo -e "${BOLD}Torq Architectural Pattern Enforcement${NC}"
echo -e "${BOLD}===========================================${NC}"
echo ""
echo "Target: $TARGET_DIR"
echo "Fail Fast: $FAIL_FAST"
echo ""

# Run each pattern check
total_violations=0
failed_checks=0

for script in "${PATTERN_SCRIPTS[@]}"; do
    script_path="$SCRIPT_DIR/$script"
    
    if [[ ! -f "$script_path" ]]; then
        echo -e "${YELLOW}⚠️  Skipping $script (not found)${NC}"
        continue
    fi
    
    echo -e "${BLUE}Running: $script${NC}"
    echo "----------------------------------------"
    
    # Run the pattern check
    if [[ "$script" == *.py ]]; then
        if python3 "$script_path" "$TARGET_DIR" --quiet; then
            echo -e "${GREEN}✅ $script: PASSED${NC}"
        else
            exit_code=$?
            failed_checks=$((failed_checks + 1))
            echo -e "${RED}❌ $script: FAILED (exit code: $exit_code)${NC}"
            
            # Run again with full output to show violations
            echo "Violations found:"
            python3 "$script_path" "$TARGET_DIR" || true
            
            if [[ "$FAIL_FAST" == "true" ]]; then
                echo -e "${RED}Failing fast due to violations${NC}"
                exit $exit_code
            fi
        fi
    else
        if bash "$script_path" "$TARGET_DIR" > /dev/null 2>&1; then
            echo -e "${GREEN}✅ $script: PASSED${NC}"
        else
            exit_code=$?
            failed_checks=$((failed_checks + 1))
            echo -e "${RED}❌ $script: FAILED (exit code: $exit_code)${NC}"
            
            # Run again with full output to show violations
            echo "Violations found:"
            bash "$script_path" "$TARGET_DIR" || true
            
            if [[ "$FAIL_FAST" == "true" ]]; then
                echo -e "${RED}Failing fast due to violations${NC}"
                exit $exit_code
            fi
        fi
    fi
    
    echo ""
done

# Summary
echo -e "${BOLD}Summary${NC}"
echo "======="
echo "Total pattern checks: ${#PATTERN_SCRIPTS[@]}"
echo "Failed checks: $failed_checks"

if [[ $failed_checks -eq 0 ]]; then
    echo -e "${GREEN}🎉 All pattern checks passed!${NC}"
    exit 0
else
    echo -e "${RED}💥 $failed_checks pattern check(s) failed${NC}"
    echo ""
    echo -e "${YELLOW}To fix violations:${NC}"
    echo "1. Review the output above for specific violations"
    echo "2. Follow the suggested fixes for each violation type"  
    echo "3. Re-run this script to verify fixes"
    echo "4. If violations are legitimate, update whitelists"
    exit 1
fi