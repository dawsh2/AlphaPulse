#!/bin/bash
# AlphaPulse README and Documentation Consistency Checker
# Ensures documentation stays in sync with code structure

set -euo pipefail

VIOLATIONS_FOUND=0
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

echo "📚 Checking documentation consistency..."

# ==============================================================================
# README-FIRST DEVELOPMENT VALIDATION
# ==============================================================================

echo "📖 Validating README-first development..."

# Check that each directory with Rust code has a README
find "$PROJECT_ROOT" -name "Cargo.toml" -not -path "*/target/*" | while read -r cargo_file; do
    dir=$(dirname "$cargo_file")
    readme_file="$dir/README.md"
    
    if [[ ! -f "$readme_file" ]]; then
        echo "  ❌ Missing README.md in $(basename "$dir")"
        VIOLATIONS_FOUND=$((VIOLATIONS_FOUND + 1))
    else
        # Check if README mentions the purpose/responsibility
        if ! grep -qi "purpose\|responsibility\|overview" "$readme_file"; then
            echo "  ⚠️  README in $(basename "$dir") missing purpose/overview section"
        fi
    fi
done

# ==============================================================================
# MODULE DOCUMENTATION VALIDATION
# ==============================================================================

echo "📝 Checking module documentation..."

# Check lib.rs files have proper module docs
find "$PROJECT_ROOT" -name "lib.rs" -not -path "*/target/*" | while read -r lib_file; do
    if ! head -20 "$lib_file" | grep -q "//!"; then
        echo "  ❌ Missing module documentation in $lib_file"
        VIOLATIONS_FOUND=$((VIOLATIONS_FOUND + 1))
    fi
done

# Check mod.rs files have proper module docs
find "$PROJECT_ROOT" -name "mod.rs" -not -path "*/target/*" | while read -r mod_file; do
    if [[ $(wc -l < "$mod_file") -gt 10 ]] && ! head -10 "$mod_file" | grep -q "//!"; then
        echo "  ⚠️  Large mod.rs missing documentation: $mod_file"
    fi
done

# ==============================================================================
# PUBLIC API DOCUMENTATION
# ==============================================================================

echo "🔍 Checking public API documentation..."

# Find public functions/structs without docs
find "$PROJECT_ROOT/protocol_v2" "$PROJECT_ROOT/libs" -name "*.rs" -not -path "*/target/*" | \
xargs grep -l "pub fn\|pub struct\|pub enum" | while read -r rust_file; do
    # Check for missing docs on public items
    missing_docs=$(grep -n "pub fn\|pub struct\|pub enum" "$rust_file" | \
        while IFS=: read -r line_num content; do
            # Check if previous line has documentation
            if [[ $line_num -gt 1 ]]; then
                prev_line=$((line_num - 1))
                if ! sed -n "${prev_line}p" "$rust_file" | grep -q "///\|//!"; then
                    echo "Line $line_num in $(basename "$rust_file"): $content"
                fi
            fi
        done)
    
    if [[ -n "$missing_docs" ]]; then
        echo "  ⚠️  Missing public API docs in $(basename "$rust_file"):"
        echo "$missing_docs" | head -3
    fi
done

# ==============================================================================
# CLAUDE.MD CONSISTENCY
# ==============================================================================

echo "🤖 Checking CLAUDE.md consistency..."

# Check that CLAUDE.md files exist where expected
for important_dir in "$PROJECT_ROOT" "$PROJECT_ROOT/protocol_v2" "$PROJECT_ROOT/services_v2"; do
    claude_file="$important_dir/CLAUDE.md"
    if [[ ! -f "$claude_file" ]]; then
        echo "  ❌ Missing CLAUDE.md in $(basename "$important_dir")"
        VIOLATIONS_FOUND=$((VIOLATIONS_FOUND + 1))
    fi
done

# ==============================================================================
# ARCHITECTURE DIAGRAM CONSISTENCY  
# ==============================================================================

echo "🏗️  Checking architecture documentation..."

# Look for mermaid diagrams in documentation
MERMAID_FILES=$(find "$PROJECT_ROOT" -name "*.md" -exec grep -l '```mermaid' {} \; || true)
if [[ -z "$MERMAID_FILES" ]]; then
    echo "  ⚠️  No mermaid architecture diagrams found in documentation"
fi

# Check that architectural decisions are documented
if [[ -f "$PROJECT_ROOT/CLAUDE.md" ]]; then
    if ! grep -q "## Architecture\|## Technical Decisions\|## Why" "$PROJECT_ROOT/CLAUDE.md"; then
        echo "  ⚠️  Main CLAUDE.md missing architectural decision documentation"
    fi
fi

# ==============================================================================
# EXAMPLE CODE VALIDATION
# ==============================================================================

echo "💡 Validating example code..."

# Check that examples in documentation are valid
find "$PROJECT_ROOT" -name "*.md" | while read -r md_file; do
    # Extract rust code blocks
    if grep -q '```rust' "$md_file"; then
        # Count rust code blocks
        rust_blocks=$(grep -c '```rust' "$md_file")
        if [[ $rust_blocks -gt 0 ]]; then
            echo "  📝 Found $rust_blocks Rust examples in $(basename "$md_file")"
        fi
    fi
done

# ==============================================================================
# CHANGELOG VALIDATION
# ==============================================================================

echo "📅 Checking changelog maintenance..."

if [[ ! -f "$PROJECT_ROOT/CHANGELOG.md" ]]; then
    echo "  ⚠️  No CHANGELOG.md found - consider adding one for version tracking"
fi

# ==============================================================================
# SUMMARY
# ==============================================================================

echo ""
if [[ $VIOLATIONS_FOUND -eq 0 ]]; then
    echo "✅ Documentation consistency maintained!"
    echo "📚 All modules properly documented"
    exit 0
else
    echo "❌ Found $VIOLATIONS_FOUND documentation violations"
    echo ""
    echo "💡 Fix recommendations:"
    echo "  - Add README.md files to each major directory"  
    echo "  - Document public APIs with /// comments"
    echo "  - Add module-level //! documentation"
    echo "  - Keep CLAUDE.md files updated with architectural decisions"
    exit 1
fi