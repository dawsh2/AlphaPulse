#!/bin/bash
#
# AlphaPulse Development Tools Installation Script
# 
# This script installs the standard Rust tooling required for development:
# - cargo-deny: Security advisory and license checking
# - cargo-udeps: Unused dependency detection  
# - cargo-sort: Cargo.toml formatting
#

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}🔧 Installing AlphaPulse Development Tools${NC}"
echo ""

# Function to check if a command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Install cargo-deny
echo -e "${YELLOW}📦 Installing cargo-deny...${NC}"
if command_exists cargo-deny; then
    echo "✅ cargo-deny already installed"
    cargo-deny --version
else
    cargo install --locked cargo-deny
    echo "✅ cargo-deny installed successfully"
fi
echo ""

# Install cargo-sort
echo -e "${YELLOW}📦 Installing cargo-sort...${NC}"
if command_exists cargo-sort; then
    echo "✅ cargo-sort already installed"
    cargo-sort --version
else
    cargo install --locked cargo-sort
    echo "✅ cargo-sort installed successfully"
fi
echo ""

# Install nightly toolchain for cargo-udeps
echo -e "${YELLOW}🌙 Installing Rust nightly toolchain...${NC}"
rustup toolchain install nightly
echo "✅ Rust nightly toolchain installed"
echo ""

# Install cargo-udeps (requires nightly)
echo -e "${YELLOW}📦 Installing cargo-udeps...${NC}"
if cargo +nightly udeps --version >/dev/null 2>&1; then
    echo "✅ cargo-udeps already installed"
    cargo +nightly udeps --version
else
    cargo +nightly install --locked cargo-udeps
    echo "✅ cargo-udeps installed successfully"
fi
echo ""

echo -e "${GREEN}🎉 All development tools installed successfully!${NC}"
echo ""
echo -e "${YELLOW}Usage:${NC}"
echo "  cargo deny check           # Check security advisories and licenses"
echo "  cargo +nightly udeps       # Find unused dependencies"
echo "  cargo sort --workspace     # Format all Cargo.toml files"
echo "  cargo sort --check         # Check if Cargo.toml files are formatted"
echo ""
echo -e "${YELLOW}CI Integration:${NC}"
echo "  These tools are automatically run in GitHub Actions"
echo "  See .github/workflows/rust-tooling.yml for details"