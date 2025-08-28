#!/bin/bash

# Install Torq Git hooks for architecture validation
# 
# This script sets up the git hooks directory and installs the pre-commit
# hook for architecture validation.

set -e

REPO_ROOT="$(git rev-parse --show-toplevel)"
HOOKS_DIR="$REPO_ROOT/.githooks"

echo "🔧 Installing Torq Git hooks..."

if [ ! -d "$HOOKS_DIR" ]; then
    echo "❌ Git hooks directory not found: $HOOKS_DIR"
    exit 1
fi

# Configure git to use our hooks directory
git config core.hooksPath "$HOOKS_DIR"

echo "✅ Git hooks installed successfully!"
echo ""
echo "The following hooks are now active:"
echo "  - pre-commit: Architecture validation checks"
echo ""
echo "To run architecture validation manually:"
echo "  cd tests/architecture_validation && cargo run"
echo ""
echo "To bypass hooks (not recommended):"
echo "  git commit --no-verify"
echo ""
echo "To disable hooks:"
echo "  git config --unset core.hooksPath"