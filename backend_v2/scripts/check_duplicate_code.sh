#!/bin/bash
# Duplicate Code Detection
# Uses simple heuristics to find potentially duplicated functions and logic

set -e

echo "🔍 Checking for Duplicate Code..."

# Check for duplicate function names across different modules
echo "Scanning for duplicate function names..."
DUPLICATE_FUNCTIONS=$(find . -name "*.rs" -exec grep -Hn "^pub fn \|^fn " {} \; | \
    sed 's/.*fn \([a-zA-Z_][a-zA-Z0-9_]*\).*/\1/' | \
    sort | uniq -d | head -10)

if [[ -n "$DUPLICATE_FUNCTIONS" ]]; then
    echo "⚠️  Functions with same names found across modules:"
    echo "$DUPLICATE_FUNCTIONS"
    echo "Review if these are intentional or could be consolidated"
fi

# Check for similar struct names
echo "Scanning for similar struct names..."
SIMILAR_STRUCTS=$(find . -name "*.rs" -exec grep -Hn "^pub struct \|^struct " {} \; | \
    sed 's/.*struct \([a-zA-Z_][a-zA-Z0-9_]*\).*/\1/' | \
    sort | uniq -d | head -5)

if [[ -n "$SIMILAR_STRUCTS" ]]; then
    echo "⚠️  Duplicate struct names found:"
    echo "$SIMILAR_STRUCTS"
fi

# Check for repeated error handling patterns
echo "Checking for repeated error patterns..."
ERROR_PATTERNS=$(find . -name "*.rs" -exec grep -c "\.context(" {} \; 2>/dev/null | \
    awk -F: '$2 > 5 {print $1 " (" $2 " instances)"}' | head -3)

if [[ -n "$ERROR_PATTERNS" ]]; then
    echo "📊 Files with many error context calls (consider error utilities):"
    echo "$ERROR_PATTERNS"
fi

# Look for duplicated constants
echo "Checking for duplicate constants..."
DUPLICATE_CONSTANTS=$(find . -name "*.rs" -exec grep -Hn "^const \|^pub const " {} \; | \
    sed 's/.*const \([A-Z_][A-Z0-9_]*\).*/\1/' | \
    sort | uniq -d | head -5)

if [[ -n "$DUPLICATE_CONSTANTS" ]]; then
    echo "⚠️  Duplicate constant names:"
    echo "$DUPLICATE_CONSTANTS"
    echo "Consider moving shared constants to a common module"
fi

# Check for similar imports across files
echo "Analyzing import patterns..."
COMMON_IMPORTS=$(find . -name "*.rs" -exec grep -h "^use " {} \; | \
    sort | uniq -c | sort -nr | head -5 | awk '$1 > 10 {print $1 " files: " substr($0, index($0,$2))}')

if [[ -n "$COMMON_IMPORTS" ]]; then
    echo "📦 Most common imports (candidates for prelude):"
    echo "$COMMON_IMPORTS"
fi

# Look for copy-pasted comment blocks
echo "Checking for duplicated comment blocks..."
COMMENT_BLOCKS=$(find . -name "*.rs" -exec grep -A 3 "^/// " {} \; | \
    grep -v "^--$" | sort | uniq -d | head -3)

if [[ -n "$COMMENT_BLOCKS" ]]; then
    echo "📝 Potentially duplicated documentation found"
fi

# Check for TODO/FIXME concentration
TODO_FILES=$(find . -name "*.rs" -exec grep -c "TODO\|FIXME\|XXX" {} \; 2>/dev/null | \
    awk -F: '$2 > 3 {print $1 " (" $2 " TODOs)"}' | head -3)

if [[ -n "$TODO_FILES" ]]; then
    echo "📋 Files with many TODOs (may need refactoring):"
    echo "$TODO_FILES"
fi

echo "✅ Duplicate code check completed"
echo "📊 Summary:"
echo "  - Function name analysis completed"
echo "  - Struct duplication checked"
echo "  - Error pattern analysis done"
echo "  - Import consolidation opportunities identified"
echo ""
echo "💡 This is a heuristic check. Manual review recommended for actual duplicates."