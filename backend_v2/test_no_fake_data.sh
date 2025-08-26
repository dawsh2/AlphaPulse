#!/bin/bash

echo "🔍 Scanning for hardcoded fake data patterns..."
echo "=============================================="

# Patterns that indicate fake/hardcoded data
fake_patterns=(
    '"0x1234...5678"'      # Fake pool address
    '"WETH".to_string()'   # Hardcoded token symbol
    '"USDC".to_string()'   # Hardcoded token symbol  
    '$2.50'                # Hardcoded gas cost
    '$3.50'                # Another hardcoded gas cost
    'gas_cost_q64.*2\.50'  # Hardcoded gas in Q64 format
    'venue_a.*QuickSwap'   # Hardcoded venue
    'venue_b.*SushiSwap'   # Hardcoded venue
    '"Below threshold"'    # Hardcoded status (when not in conditional)
    '150\.0.*profit'       # Hardcoded profit value
)

found_issues=0

echo -e "\nScanning flash_arbitrage strategy files..."

for pattern in "${fake_patterns[@]}"; do
    # Search in the strategy files, excluding test files
    result=$(grep -r "$pattern" services_v2/strategies/flash_arbitrage/src/ 2>/dev/null | grep -v "test" | grep -v "example" | grep -v "//" || true)
    
    if [ -n "$result" ]; then
        echo "❌ Found hardcoded pattern: $pattern"
        echo "   Location: $(echo "$result" | head -1 | cut -d: -f1)"
        ((found_issues++))
    fi
done

# Check for the specific deprecated function that was sending fake data
if grep -q "send_arbitrage_analysis" services_v2/strategies/flash_arbitrage/src/signal_output.rs; then
    # It exists, but check if it's disabled
    if ! grep -q "DISABLED\|deprecated" services_v2/strategies/flash_arbitrage/src/signal_output.rs; then
        echo "❌ send_arbitrage_analysis exists and is not marked as disabled!"
        ((found_issues++))
    else
        echo "✅ send_arbitrage_analysis is properly disabled"
    fi
fi

# Check that ArbitrageSignalTLV is being used instead of DemoDeFiArbitrageTLV
echo -e "\nChecking for proper TLV usage..."
if grep -r "DemoDeFiArbitrageTLV::new" services_v2/strategies/flash_arbitrage/src/ 2>/dev/null | grep -v "//" | grep -v "test"; then
    echo "❌ Still using DemoDeFiArbitrageTLV::new in production code!"
    ((found_issues++))
else
    echo "✅ Not using DemoDeFiArbitrageTLV in production code"
fi

if grep -q "TLVType::ArbitrageSignal" services_v2/strategies/flash_arbitrage/src/signal_output.rs; then
    echo "✅ Using proper ArbitrageSignal TLV type (32)"
else
    echo "⚠️  Not using ArbitrageSignal TLV type - verify signal output"
fi

echo -e "\n=============================================="
if [ $found_issues -eq 0 ]; then
    echo "✅ NO FAKE DATA PATTERNS FOUND!"
    echo "The system appears to be using real data only."
else
    echo "❌ Found $found_issues potential fake data issues"
    echo "Please review and fix the hardcoded values above."
    exit 1
fi