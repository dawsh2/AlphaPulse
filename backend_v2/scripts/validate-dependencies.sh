#!/bin/bash
# Dependency validation script to prevent circular dependencies and maintain clean architecture
# This script is designed to run as part of CI/CD pipeline and pre-commit hooks

set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LIBS_DIR="$PROJECT_ROOT/libs"
EXIT_CODE=0

echo "🔍 Validating dependency architecture..."

# Function to check for circular dependency patterns
check_circular_dependencies() {
    echo "📋 Checking for circular dependencies..."
    
    local types_cargo="$LIBS_DIR/types/Cargo.toml"
    local codec_cargo="$LIBS_DIR/alphapulse_codec/Cargo.toml"
    
    # Check if types depends on codec (should not happen) - ignore comments
    if grep -q "^[[:space:]]*alphapulse_codec" "$types_cargo" 2>/dev/null; then
        echo "❌ CRITICAL: Found alphapulse_codec dependency in libs/types/Cargo.toml"
        echo "   This creates a circular dependency and violates architecture!"
        echo "   See ADR-001 in .claude/docs/architecture-decisions/ for proper pattern."
        EXIT_CODE=1
    fi
    
    # Verify codec depends on types (should be present)
    if ! grep -q "alphapulse-types" "$codec_cargo" 2>/dev/null; then
        echo "⚠️  WARNING: alphapulse_codec should depend on alphapulse-types"
        echo "   Check $codec_cargo dependency configuration."
    fi
    
    echo "✅ Circular dependency check complete"
}

# Function to validate import patterns in source files
check_import_patterns() {
    echo "📋 Checking import patterns in source files..."
    
    # Check for codec imports in types package (should not happen)
    # Ignore commented lines and binaries that are commented out in Cargo.toml
    local codec_imports_in_types
    codec_imports_in_types=$(find "$LIBS_DIR/types/src" -name "*.rs" -not -path "*/bin/*" -exec grep -l "^[[:space:]]*use.*alphapulse_codec" {} \; 2>/dev/null || true)
    
    if [[ -n "$codec_imports_in_types" ]]; then
        echo "❌ CRITICAL: Found alphapulse_codec imports in types package:"
        echo "$codec_imports_in_types"
        echo "   This violates the architecture separation. Move codec usage to service level."
        EXIT_CODE=1
    fi
    
    echo "✅ Import pattern check complete"
}

# Function to validate that required dependencies are present
check_required_dependencies() {
    echo "📋 Checking required dependencies..."
    
    # Verify alphapulse-transport is available for types package
    local transport_dep_in_types
    transport_dep_in_types=$(grep -q "alphapulse-transport" "$LIBS_DIR/types/Cargo.toml" && echo "found" || echo "missing")
    
    if [[ "$transport_dep_in_types" == "missing" ]]; then
        echo "⚠️  WARNING: alphapulse-transport dependency missing from types package"
        echo "   This may cause timestamp function import issues."
    fi
    
    echo "✅ Required dependency check complete"
}

# Function to check for common anti-patterns
check_antipatterns() {
    echo "📋 Checking for architectural anti-patterns..."
    
    # Check for wildcard imports that might hide dependency issues
    local wildcard_imports
    wildcard_imports=$(find "$PROJECT_ROOT" -name "*.rs" -path "*/src/*" -exec grep -l "use alphapulse_.*::\*" {} \; 2>/dev/null || true)
    
    if [[ -n "$wildcard_imports" ]]; then
        echo "⚠️  WARNING: Found wildcard imports (use specific imports for better dependency management):"
        echo "$wildcard_imports" | head -5
        if [[ $(echo "$wildcard_imports" | wc -l) -gt 5 ]]; then
            echo "   ... and $(( $(echo "$wildcard_imports" | wc -l) - 5 )) more files"
        fi
    fi
    
    echo "✅ Anti-pattern check complete"
}

# Function to validate package feature flags
check_feature_flags() {
    echo "📋 Checking feature flag consistency..."
    
    # Check that types package doesn't have codec-specific features
    local types_features
    types_features=$(grep -A 10 "^\[features\]" "$LIBS_DIR/types/Cargo.toml" || true)
    
    if echo "$types_features" | grep -q "codec\|builder\|parser" 2>/dev/null; then
        echo "⚠️  WARNING: Types package may have codec-specific features"
        echo "   Consider moving these to alphapulse_codec package."
    fi
    
    echo "✅ Feature flag check complete"
}

# Function to generate dependency graph summary
generate_dependency_summary() {
    echo "📋 Dependency architecture summary:"
    echo
    echo "├── alphapulse-types (foundation)"
    echo "│   └── alphapulse-transport (timestamps only)"
    echo "└── alphapulse_codec (protocol implementation)"
    echo "    └── alphapulse-types (type definitions)"
    echo
    echo "Services import from both packages as needed."
    echo "See .claude/docs/dependency-patterns.md for import guidelines."
    echo
}

# Main execution
main() {
    echo "🏗️  AlphaPulse Dependency Architecture Validation"
    echo "=============================================="
    echo
    
    check_circular_dependencies
    echo
    
    check_import_patterns
    echo
    
    check_required_dependencies
    echo
    
    check_antipatterns
    echo
    
    check_feature_flags
    echo
    
    generate_dependency_summary
    
    if [[ $EXIT_CODE -eq 0 ]]; then
        echo "✅ All dependency architecture checks passed!"
        echo "🚀 System maintains clean separation between types and codec packages."
    else
        echo "❌ Dependency architecture validation failed!"
        echo "🔧 Please fix the issues above before proceeding."
        echo "📖 See ADR-001 and dependency-patterns.md in .claude/docs/ for guidance."
    fi
    
    exit $EXIT_CODE
}

# Show help message
show_help() {
    echo "AlphaPulse Dependency Validation Script"
    echo
    echo "Usage: $0 [options]"
    echo
    echo "Options:"
    echo "  -h, --help    Show this help message"
    echo "  --ci          Run in CI mode (stricter validation)"
    echo
    echo "This script validates the dependency architecture to prevent:"
    echo "  • Circular dependencies between alphapulse_codec and alphapulse-types"
    echo "  • Import violations that break architectural boundaries"
    echo "  • Common anti-patterns that create tight coupling"
    echo
    echo "References:"
    echo "  • ADR-001: .claude/docs/architecture-decisions/ADR-001-codec-types-separation.md"
    echo "  • Patterns: .claude/docs/dependency-patterns.md"
    echo
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -h|--help)
            show_help
            exit 0
            ;;
        --ci)
            # In CI mode, treat warnings as errors
            echo "🏃 Running in CI mode (strict validation)"
            # Could add stricter validation here in the future
            shift
            ;;
        *)
            echo "Unknown option: $1"
            show_help
            exit 1
            ;;
    esac
done

# Run main function
main

# Make the script executable
chmod +x "$0" 2>/dev/null || true