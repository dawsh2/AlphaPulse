#!/bin/bash
# .project/env.sh - Environment setup for .project/ configuration

# Set CARGO_HOME to use .project/cargo.toml
export CARGO_HOME="$(pwd)/.project"

# Add project-specific tool paths
export PATH="$CARGO_HOME/bin:$PATH"

# Make tools use .project configuration
alias pre-commit="pre-commit --config .project/pre-commit.yaml"

# Source this file: source .project/env.sh
echo "✅ .project/ environment configured"
echo "   - CARGO_HOME: $CARGO_HOME" 
echo "   - Git hooks: .project/git-hooks/"
echo "   - Pre-commit: .project/pre-commit.yaml"