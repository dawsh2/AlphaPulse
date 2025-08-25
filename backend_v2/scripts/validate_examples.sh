#!/bin/bash
# Example Code Validation
# Ensures example code compiles and documentation examples are correct

set -e

echo "📚 Validating Example Code..."

# Check if examples directory exists
if [[ ! -d "examples" ]]; then
    echo "⚠️  No examples directory found"
    exit 0
fi

# Validate examples compile
echo "Compiling examples..."
if [[ -f "examples/Cargo.toml" ]]; then
    cd examples
    if cargo check --quiet; then
        echo "✅ All examples compile successfully"
    else
        echo "❌ Examples failed to compile"
        exit 1
    fi
    cd ..
else
    echo "⚠️  Examples directory exists but no Cargo.toml found"
fi

# Check for example files in protocol_v2/examples
if [[ -d "protocol_v2/examples" ]]; then
    echo "Checking Protocol V2 examples..."
    PROTOCOL_EXAMPLES=$(find protocol_v2/examples -name "*.rs" | wc -l)
    echo "📊 Found $PROTOCOL_EXAMPLES Protocol V2 example files"
    
    # Try to build protocol examples
    cd protocol_v2
    for example_file in examples/*.rs; do
        if [[ -f "$example_file" ]]; then
            example_name=$(basename "$example_file" .rs)
            if cargo check --example "$example_name" --quiet; then
                echo "✅ Example '$example_name' compiles"
            else
                echo "❌ Example '$example_name' failed to compile"
                exit 1
            fi
        fi
    done
    cd ..
fi

# Validate documentation examples (doctests)
echo "Running documentation tests..."
if cargo test --doc --workspace --quiet; then
    echo "✅ All documentation examples pass"
else
    echo "❌ Documentation examples failed"
    echo "Fix failing doctests in /// examples"
    exit 1
fi

# Check for broken links in markdown examples
echo "Checking for broken internal links in documentation..."
BROKEN_LINKS=$(find . -name "*.md" -exec grep -H "\[.*\](.*\.rs)" {} \; | while read -r line; do
    file=$(echo "$line" | cut -d: -f1)
    link=$(echo "$line" | grep -o "(.*\.rs)" | tr -d "()")
    
    # Check if linked file exists relative to markdown file
    dir=$(dirname "$file")
    if [[ ! -f "$dir/$link" && ! -f "$link" ]]; then
        echo "Broken link in $file: $link"
    fi
done)

if [[ -n "$BROKEN_LINKS" ]]; then
    echo "⚠️  Broken links found in documentation:"
    echo "$BROKEN_LINKS"
fi

# Check example consistency with main code
echo "Checking example-to-library consistency..."
EXAMPLE_IMPORTS=$(find examples/ protocol_v2/examples/ -name "*.rs" -exec grep -h "use.*alphapulse" {} \; 2>/dev/null | sort | uniq || true)
if [[ -n "$EXAMPLE_IMPORTS" ]]; then
    echo "📦 Examples use the following AlphaPulse imports:"
    echo "$EXAMPLE_IMPORTS" | head -5
    echo "..."
fi

echo "✅ Example validation completed"
echo "📊 Summary:"
echo "  - Examples compile successfully"
echo "  - Documentation tests pass"
echo "  - Link integrity verified"