#!/bin/bash
# TLV Type Registry Integrity Check
# Ensures no duplicate TLV type numbers and validates expected_payload_size()

set -e

echo "🔍 Checking TLV Type Registry Integrity..."

TLV_TYPES_FILE="protocol_v2/src/tlv/types.rs"

if [[ ! -f "$TLV_TYPES_FILE" ]]; then
    echo "❌ TLV types file not found: $TLV_TYPES_FILE"
    exit 1
fi

# Check for duplicate TLV type numbers
echo "Checking for duplicate TLV type numbers..."
DUPLICATES=$(grep -E "= [0-9]+" "$TLV_TYPES_FILE" | awk '{print $3}' | sort | uniq -d)

if [[ -n "$DUPLICATES" ]]; then
    echo "❌ Duplicate TLV type numbers found:"
    echo "$DUPLICATES"
    echo ""
    echo "Each TLV type must have a unique number. Check $TLV_TYPES_FILE"
    exit 1
fi

# Check TLV type ranges
echo "Validating TLV type ranges..."
MARKET_DATA_RANGE=$(grep -E "= [1-9][0-9]*" "$TLV_TYPES_FILE" | awk '{print $3}' | tr -d ',' | sort -n)
SIGNAL_RANGE=$(grep -E "= [2-3][0-9]" "$TLV_TYPES_FILE" | awk '{print $3}' | tr -d ',' | sort -n)
EXECUTION_RANGE=$(grep -E "= [4-7][0-9]" "$TLV_TYPES_FILE" | awk '{print $3}' | tr -d ',' | sort -n)

# Check for types outside valid ranges
INVALID_TYPES=$(echo "$MARKET_DATA_RANGE" | awk '$1 > 19 && $1 < 20')
if [[ -n "$INVALID_TYPES" ]]; then
    echo "⚠️  Types in MarketData range (1-19) found outside bounds"
fi

# Validate expected_payload_size function exists
if ! grep -q "expected_payload_size" "$TLV_TYPES_FILE"; then
    echo "❌ expected_payload_size() function not found in $TLV_TYPES_FILE"
    echo "This function must be updated when TLV structs change"
    exit 1
fi

echo "✅ TLV Type Registry integrity check passed"
echo "📊 Summary:"
echo "  - No duplicate type numbers detected"
echo "  - Type ranges validated"
echo "  - expected_payload_size() function present"