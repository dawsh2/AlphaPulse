#!/bin/bash
# AlphaPulse Quality Gates Setup
# One-time setup for architectural quality enforcement

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

echo "🏗️  Setting up AlphaPulse architectural quality gates..."

# ==============================================================================
# PRE-COMMIT SETUP
# ==============================================================================

echo "🔧 Installing pre-commit hooks..."

# Install pre-commit if not present
if ! command -v pre-commit &> /dev/null; then
    echo "Installing pre-commit..."
    if command -v pip &> /dev/null; then
        pip install pre-commit
    elif command -v brew &> /dev/null; then
        brew install pre-commit
    else
        echo "❌ Please install pre-commit manually: https://pre-commit.com/#install"
        exit 1
    fi
fi

# Install the hooks
cd "$PROJECT_ROOT"
pre-commit install

# ==============================================================================
# SCRIPT PERMISSIONS
# ==============================================================================

echo "🔑 Setting script permissions..."

chmod +x "$SCRIPT_DIR/check_service_boundaries.sh"
chmod +x "$SCRIPT_DIR/check_readme_consistency.sh"
chmod +x "$SCRIPT_DIR/check_zero_copy_violations.sh"

# ==============================================================================
# CARGO ALIAS UPDATES  
# ==============================================================================

echo "⚙️  Updating cargo aliases..."

# Add quality check aliases to .cargo/config.toml
cat >> "$PROJECT_ROOT/.cargo/config.toml" << 'EOF'

# Quality gate aliases
quality-check = "run --bin quality_check"
arch-check = "run --bin architectural_check"
boundaries = "run --bin check_boundaries"
doc-check = "run --bin documentation_check"

# Combined quality gates
quality-full = "run --bin run_all_quality_checks"
pre-commit-check = "run --bin pre_commit_validation"
EOF

# ==============================================================================
# VALIDATION
# ==============================================================================

echo "✅ Running initial validation..."

# Test that all scripts are executable
"$SCRIPT_DIR/check_service_boundaries.sh" || echo "⚠️  Service boundary check has issues to fix"
"$SCRIPT_DIR/check_readme_consistency.sh" || echo "⚠️  Documentation has issues to fix"

# Test clippy configuration
cargo clippy --version > /dev/null || (echo "❌ Clippy not available" && exit 1)

# Test pre-commit installation  
pre-commit --version > /dev/null || (echo "❌ Pre-commit not properly installed" && exit 1)

echo ""
echo "🎉 Quality gates setup complete!"
echo ""
echo "📋 What you can do now:"
echo "  • cargo clippy-all - Run enhanced clippy checks"
echo "  • ./scripts/check_service_boundaries.sh - Check architectural integrity"
echo "  • ./scripts/check_readme_consistency.sh - Validate documentation"
echo "  • pre-commit run --all-files - Run all quality checks"
echo ""
echo "🔒 Pre-commit hooks will now automatically:"
echo "  • Check code formatting and style"
echo "  • Validate service boundaries"  
echo "  • Ensure documentation consistency"
echo "  • Test builds before commits"
echo ""
echo "🚀 Push to GitHub to trigger full architectural quality pipeline!"