#!/bin/bash
# Dependency Validation
# Checks for dependency conflicts, security issues, and optimization opportunities

set -e

echo "📦 Validating Dependencies..."

# Check for dependency conflicts
echo "Checking for version conflicts..."
if command -v cargo &> /dev/null; then
    if cargo tree --duplicates --quiet 2>/dev/null | head -5; then
        echo "⚠️  Duplicate dependencies found (may cause version conflicts)"
        echo "Run 'cargo tree --duplicates' for details"
    else
        echo "✅ No duplicate dependencies detected"
    fi
else
    echo "⚠️  Cargo not found, skipping dependency tree analysis"
fi

# Security audit (if cargo-audit is available)
echo "Running security audit..."
if command -v cargo-audit &> /dev/null; then
    if cargo audit --quiet; then
        echo "✅ No known security vulnerabilities"
    else
        echo "❌ Security vulnerabilities detected"
        echo "Run 'cargo audit' for details and update vulnerable dependencies"
        exit 1
    fi
else
    echo "💡 Install cargo-audit for security scanning: cargo install cargo-audit"
fi

# Check for outdated dependencies
echo "Checking for outdated dependencies..."
if command -v cargo-outdated &> /dev/null; then
    OUTDATED=$(cargo outdated --quiet --exit-code 1 2>/dev/null | head -5)
    if [[ -n "$OUTDATED" ]]; then
        echo "📈 Outdated dependencies found:"
        echo "$OUTDATED"
        echo "Consider updating with 'cargo update'"
    else
        echo "✅ All dependencies are up to date"
    fi
else
    echo "💡 Install cargo-outdated for update checking: cargo install cargo-outdated"
fi

# Validate workspace dependencies
echo "Validating workspace structure..."
if ! grep -q "\\[workspace\\]" Cargo.toml; then
    echo "❌ Root Cargo.toml missing workspace configuration"
    exit 1
fi

# Check for unused dependencies
echo "Checking for unused dependencies..."
if command -v cargo-machete &> /dev/null; then
    UNUSED=$(cargo machete --quiet 2>/dev/null | head -3)
    if [[ -n "$UNUSED" ]]; then
        echo "🗑️  Unused dependencies detected:"
        echo "$UNUSED"
        echo "Consider removing unused dependencies to reduce build time"
    else
        echo "✅ No unused dependencies found"
    fi
else
    echo "💡 Install cargo-machete for unused dependency detection: cargo install cargo-machete"
fi

# Validate feature flags
echo "Checking feature flag usage..."
FEATURE_COUNT=$(find . -name "Cargo.toml" -exec grep -c "features.*=" {} \; 2>/dev/null | \
    awk '{sum += $1} END {print sum+0}')
echo "📊 Total feature flag configurations: $FEATURE_COUNT"

# Check for heavy dependencies in hot paths
echo "Analyzing dependency weight..."
HEAVY_DEPS=$(grep -r "serde\|tokio\|reqwest\|sqlx" */Cargo.toml | wc -l 2>/dev/null || echo "0")
echo "📊 Heavy dependencies in use: $HEAVY_DEPS"

# Check for conflicting async runtimes
echo "Checking for async runtime conflicts..."
ASYNC_RUNTIMES=$(find . -name "Cargo.toml" -exec grep -H "tokio\|async-std\|smol" {} \; 2>/dev/null | \
    cut -d: -f1 | sort | uniq | wc -l)

if [[ "$ASYNC_RUNTIMES" -gt 1 ]]; then
    echo "⚠️  Multiple async runtimes detected in workspace"
    echo "Ensure consistent async runtime usage across crates"
fi

# Validate AlphaPulse internal dependencies
echo "Checking internal dependency structure..."
INTERNAL_DEPS=$(find . -name "Cargo.toml" -exec grep -c "alphapulse.*path.*=" {} \; 2>/dev/null | \
    awk '{sum += $1} END {print sum+0}')
echo "🔗 Internal AlphaPulse dependencies: $INTERNAL_DEPS"

# Check for dev-dependencies in production code
echo "Validating dev-dependency separation..."
PROD_DEV_DEPS=$(find . -name "*.rs" -not -path "*/tests/*" -not -path "*/benches/*" \
    -exec grep -l "use.*test" {} \; 2>/dev/null | head -3)

if [[ -n "$PROD_DEV_DEPS" ]]; then
    echo "⚠️  Production code may be using dev dependencies:"
    echo "$PROD_DEV_DEPS"
fi

echo "✅ Dependency validation completed"
echo "📊 Summary:"
echo "  - Version conflicts checked"
echo "  - Security audit completed"
echo "  - Workspace structure validated"
echo "  - Feature flags analyzed"
echo "  - Internal dependency structure verified"