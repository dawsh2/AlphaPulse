#!/bin/bash
# Project Structure Validation
# Ensures proper organization and service boundaries

set -e

echo "📁 Validating Project Structure..."

# Define expected directories
EXPECTED_DIRS=(
    "protocol_v2"
    "libs"
    "services_v2"
    "relays" 
    "network"
    "tests"
    "scripts"
    "config"
    ".github/workflows"
    ".claude"
)

# Check for required directories
echo "Checking required directories..."
for dir in "${EXPECTED_DIRS[@]}"; do
    if [[ ! -d "$dir" ]]; then
        echo "❌ Missing required directory: $dir"
        exit 1
    fi
done

# Validate service boundaries
echo "Validating service boundaries..."

# Check that libs don't import from services
INVALID_LIB_IMPORTS=$(find libs/ -name "*.rs" -exec grep -l "use.*services_v2::" {} \; 2>/dev/null || true)
if [[ -n "$INVALID_LIB_IMPORTS" ]]; then
    echo "❌ Libraries importing from services (violates separation):"
    echo "$INVALID_LIB_IMPORTS"
    exit 1
fi

# Check that protocol_v2 doesn't import from services
INVALID_PROTOCOL_IMPORTS=$(find protocol_v2/ -name "*.rs" -exec grep -l "use.*services_v2::" {} \; 2>/dev/null || true)
if [[ -n "$INVALID_PROTOCOL_IMPORTS" ]]; then
    echo "❌ Protocol V2 importing from services (violates isolation):"
    echo "$INVALID_PROTOCOL_IMPORTS"
    exit 1
fi

# Check for proper Cargo.toml structure
echo "Checking Cargo workspace structure..."
if ! grep -q "\\[workspace\\]" Cargo.toml; then
    echo "❌ Root Cargo.toml missing [workspace] section"
    exit 1
fi

# Validate that each service has its own Cargo.toml
SERVICES_WITHOUT_CARGO=$(find services_v2/ -maxdepth 2 -type d -name "*" | while read -r dir; do
    if [[ -d "$dir" && ! -f "$dir/Cargo.toml" && "$dir" != "services_v2" ]]; then
        echo "$dir"
    fi
done)

if [[ -n "$SERVICES_WITHOUT_CARGO" ]]; then
    echo "⚠️  Services without Cargo.toml (may need individual manifests):"
    echo "$SERVICES_WITHOUT_CARGO"
fi

# Check for proper README files
echo "Checking documentation structure..."
DIRS_WITHOUT_README=$(find libs/ services_v2/ -maxdepth 2 -type d | while read -r dir; do
    if [[ -d "$dir" && ! -f "$dir/README.md" && "$dir" != "libs" && "$dir" != "services_v2" ]]; then
        echo "$dir"
    fi
done)

if [[ -n "$DIRS_WITHOUT_README" ]]; then
    echo "⚠️  Directories missing README.md files:"
    echo "$DIRS_WITHOUT_README"
    echo "Consider adding README.md files for documentation"
fi

# Check for scattered test files in root
ROOT_TEST_FILES=$(find . -maxdepth 1 -name "test_*.rs" -o -name "*_test.rs" | head -5)
if [[ -n "$ROOT_TEST_FILES" ]]; then
    echo "⚠️  Test files in project root (consider organizing in tests/ directory):"
    echo "$ROOT_TEST_FILES"
    echo "..."
fi

echo "✅ Project structure validation completed"
echo "📊 Summary:"
echo "  - All required directories present"
echo "  - Service boundaries respected"  
echo "  - Cargo workspace properly configured"
echo "  - Documentation structure validated"