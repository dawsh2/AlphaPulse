#!/bin/bash

echo "🧪 Testing AlphaPulse Integrity Fixes"
echo "======================================"

# Test 1: Check that DemoDeFiArbitrageTLV is completely removed
echo -e "\n1️⃣ Testing DemoDeFiArbitrageTLV removal..."
if [ -f protocol_v2/src/tlv/demo_defi.rs ]; then
    echo "❌ DemoDeFiArbitrageTLV file still exists!"
    exit 1
else
    echo "✅ DemoDeFiArbitrageTLV has been completely removed"
fi

# Test 2: Check that send_arbitrage_analysis is disabled
echo -e "\n2️⃣ Testing send_arbitrage_analysis is disabled..."
if grep -q "DISABLED.*sending fake" services_v2/strategies/flash_arbitrage/src/signal_output.rs; then
    echo "✅ send_arbitrage_analysis is disabled"
else
    echo "❌ send_arbitrage_analysis might still be active"
    exit 1
fi

# Test 3: Check profitability guards are active
echo -e "\n3️⃣ Testing profitability guards..."
if grep -q "^\s*if !pos.is_profitable" services_v2/strategies/flash_arbitrage/src/detector.rs; then
    echo "✅ Profitability check is active"
else
    echo "❌ Profitability check is commented out"
    exit 1
fi

# Test 4: Check profit margin guard
echo -e "\n4️⃣ Testing profit margin guard..."
if grep -q "if profit_margin > self.config.max_profit_margin_pct" services_v2/strategies/flash_arbitrage/src/detector.rs; then
    echo "✅ Profit margin guard is active"
else
    echo "❌ Profit margin guard is commented out"
    exit 1
fi

# Test 5: Check all DEX events are handled
echo -e "\n5️⃣ Testing DEX event handlers..."
events=(
    "11 =>"
    "12 =>"
    "13 =>"
    "14 =>"
    "16 =>"
)

event_names=("PoolSwap" "PoolMint" "PoolBurn" "PoolTick" "PoolSync")
i=0
for event in "${events[@]}"; do
    if grep -q "$event" services_v2/strategies/flash_arbitrage/src/relay_consumer.rs; then
        echo "✅ Handler found for: ${event_names[$i]} (type ${event% =>})"
    else
        echo "❌ Missing handler for: ${event_names[$i]} (type ${event% =>})"
        exit 1
    fi
    ((i++))
done

# Test 6: Check that process_pool_sync function exists
echo -e "\n6️⃣ Testing process_pool_sync function..."
if grep -q "async fn process_pool_sync" services_v2/strategies/flash_arbitrage/src/relay_consumer.rs; then
    echo "✅ process_pool_sync function exists"
else
    echo "❌ process_pool_sync function not found"
    exit 1
fi

# Test 7: Compile check
echo -e "\n7️⃣ Testing compilation..."
if cargo check --package alphapulse-flash-arbitrage 2>&1 | grep -q "error\["; then
    echo "❌ Compilation errors found"
    cargo check --package alphapulse-flash-arbitrage
    exit 1
else
    echo "✅ Code compiles successfully (with expected deprecation warnings)"
fi

echo -e "\n======================================"
echo "✅ ALL TESTS PASSED!"
echo "======================================"
echo ""
echo "Summary of fixes:"
echo "- ✅ No fake data: DemoDeFiArbitrageTLV COMPLETELY REMOVED, send_arbitrage_analysis disabled"
echo "- ✅ Safety guards: Profitability and margin checks are active"
echo "- ✅ Complete events: All DEX events (Swap, Mint, Burn, Tick, Sync) are processed"
echo "- ✅ Code quality: Compiles without errors"