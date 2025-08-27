#!/bin/bash

# AlphaPulse Timestamp Migration Tool
# Finds and lists all instances of unsafe timestamp conversion pattern
# Usage: ./scripts/migrate_unsafe_timestamps.sh [--fix]

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "🔍 AlphaPulse Unsafe Timestamp Pattern Scanner"
echo "=============================================="
echo ""

# Pattern to search for
UNSAFE_PATTERN="as_nanos\(\) as u64"

# Find all Rust files with the unsafe pattern
echo "Searching for unsafe timestamp conversions..."
echo ""

# Create temporary file for results
RESULTS_FILE=$(mktemp)

# Search and save results
rg "${UNSAFE_PATTERN}" \
    --type rust \
    --line-number \
    --with-filename \
    --no-heading \
    > "$RESULTS_FILE" 2>/dev/null || true

# Count occurrences
TOTAL_COUNT=$(cat "$RESULTS_FILE" | wc -l)

if [[ $TOTAL_COUNT -eq 0 ]]; then
    echo -e "${GREEN}✅ No unsafe timestamp patterns found!${NC}"
    rm "$RESULTS_FILE"
    exit 0
fi

echo -e "${YELLOW}⚠️  Found ${TOTAL_COUNT} instances of unsafe timestamp conversion${NC}"
echo ""

# Group by file
echo "Files containing unsafe patterns:"
echo "---------------------------------"
cat "$RESULTS_FILE" | cut -d':' -f1 | sort -u | while read -r file; do
    COUNT=$(grep "^${file}:" "$RESULTS_FILE" | wc -l)
    echo -e "  ${file}: ${RED}${COUNT} instances${NC}"
done

echo ""
echo "Detailed locations:"
echo "-------------------"

# Show each instance with context
while IFS= read -r line; do
    FILE=$(echo "$line" | cut -d':' -f1)
    LINE_NUM=$(echo "$line" | cut -d':' -f2)
    CODE=$(echo "$line" | cut -d':' -f3-)
    
    echo -e "${YELLOW}${FILE}:${LINE_NUM}${NC}"
    echo "  ${CODE}"
    echo ""
done < "$RESULTS_FILE"

# Migration instructions
echo "📋 Migration Instructions:"
echo "========================="
echo ""
echo "For each instance above, replace the unsafe pattern with:"
echo ""
echo "1. For system timestamps:"
echo -e "${GREEN}   // BEFORE:${NC}"
echo "   let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos() as u64;"
echo ""
echo -e "${GREEN}   // AFTER:${NC}"
echo "   use alphapulse_transport::safe_system_timestamp_ns;"
echo "   let timestamp = safe_system_timestamp_ns();"
echo ""
echo "2. For duration conversions:"
echo -e "${GREEN}   // BEFORE:${NC}"
echo "   let ns = duration.as_nanos() as u64;"
echo ""
echo -e "${GREEN}   // AFTER:${NC}"
echo "   use alphapulse_transport::safe_duration_to_ns;"
echo "   let ns = safe_duration_to_ns(duration);"
echo ""

# Check if automatic fix was requested
if [[ "${1:-}" == "--fix" ]]; then
    echo -e "${YELLOW}⚠️  Automatic fix requested but NOT implemented${NC}"
    echo "Manual review is required for each instance to determine the appropriate fix."
    echo "Some conversions may need safe_system_timestamp_ns() while others need safe_duration_to_ns()."
fi

# Create a checklist file
CHECKLIST_FILE="timestamp_migration_checklist.md"
echo "# Timestamp Migration Checklist" > "$CHECKLIST_FILE"
echo "" >> "$CHECKLIST_FILE"
echo "Generated: $(date)" >> "$CHECKLIST_FILE"
echo "Total instances: ${TOTAL_COUNT}" >> "$CHECKLIST_FILE"
echo "" >> "$CHECKLIST_FILE"
echo "## Files to migrate:" >> "$CHECKLIST_FILE"
echo "" >> "$CHECKLIST_FILE"

cat "$RESULTS_FILE" | cut -d':' -f1 | sort -u | while read -r file; do
    COUNT=$(grep "^${file}:" "$RESULTS_FILE" | wc -l)
    echo "- [ ] \`${file}\` (${COUNT} instances)" >> "$CHECKLIST_FILE"
done

echo "" >> "$CHECKLIST_FILE"
echo "## Detailed locations:" >> "$CHECKLIST_FILE"
echo "" >> "$CHECKLIST_FILE"

while IFS= read -r line; do
    FILE=$(echo "$line" | cut -d':' -f1)
    LINE_NUM=$(echo "$line" | cut -d':' -f2)
    echo "- [ ] \`${FILE}:${LINE_NUM}\`" >> "$CHECKLIST_FILE"
done < "$RESULTS_FILE"

echo ""
echo -e "${GREEN}✅ Migration checklist saved to: ${CHECKLIST_FILE}${NC}"

# Cleanup
rm "$RESULTS_FILE"

echo ""
echo "⚠️  IMPORTANT: Each instance requires manual review to determine:"
echo "   1. Whether to use safe_system_timestamp_ns() or safe_duration_to_ns()"
echo "   2. How to handle the import statements"
echo "   3. Whether error handling needs adjustment"
echo ""
echo "Run with --fix flag for automated assistance (not yet implemented)."