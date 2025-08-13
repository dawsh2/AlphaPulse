#!/bin/bash
# Build script for AlphaPulse Rust services

set -e

echo "🚀 Building AlphaPulse Rust Services"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    print_error "Rust/Cargo not found. Please install Rust from https://rustup.rs/"
    exit 1
fi

print_status "Rust version: $(cargo --version)"

# Check if Redis is running (for integration tests)
if ! command -v redis-cli &> /dev/null || ! redis-cli ping &> /dev/null; then
    print_warning "Redis not running. Some integration tests may fail."
fi

# Build in release mode for performance
print_status "Building in release mode..."
cargo build --release

if [ $? -eq 0 ]; then
    print_success "Build completed successfully"
else
    print_error "Build failed"
    exit 1
fi

# Run tests
print_status "Running tests..."
cargo test

if [ $? -eq 0 ]; then
    print_success "All tests passed"
else
    print_error "Tests failed"
    exit 1
fi

# Check formatting
print_status "Checking code formatting..."
cargo fmt --check

if [ $? -eq 0 ]; then
    print_success "Code formatting is correct"
else
    print_warning "Code formatting issues found. Run 'cargo fmt' to fix."
fi

# Run clippy for linting
print_status "Running Clippy linter..."
cargo clippy -- -D warnings

if [ $? -eq 0 ]; then
    print_success "No Clippy warnings"
else
    print_warning "Clippy warnings found. Please address them."
fi

# Display binary information
print_status "Built binaries:"
ls -la target/release/alphapulse-*

print_success "🎉 Build completed successfully!"
echo
print_status "To run the services:"
echo "  Collectors: ./target/release/alphapulse-collectors"
echo "  API Server: ./target/release/alphapulse-api-server"
echo
print_status "To run with Docker:"
echo "  docker-compose -f ../docker-compose.yml -f ../docker-compose.rust.yml up"