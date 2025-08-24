#!/bin/bash
# AlphaPulse Zero-Copy Violation Detector
# Catches performance anti-patterns that break 50M msg/s design

set -euo pipefail

VIOLATIONS_FOUND=0
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

echo "🔍 Scanning for zero-copy violations in AlphaPulse codebase..."

# ==============================================================================
# HOT PATH ALLOCATION VIOLATIONS  
# ==============================================================================

echo "📊 Checking for allocation violations..."

# Vec allocations in hot paths (protocol_v2, relays, adapters)
HOT_PATHS=("protocol_v2" "relays" "services_v2/adapters" "libs")

for path in "${HOT_PATHS[@]}"; do
    if [[ -d "$PROJECT_ROOT/$path" ]]; then
        echo "  🔥 Scanning hot path: $path"
        
        # Dangerous Vec patterns
        VEC_VIOLATIONS=$(find "$PROJECT_ROOT/$path" -name "*.rs" -exec grep -Hn "Vec::new()\|vec!\[\]\|\.to_vec()" {} \; | grep -v "test\|bench\|example" || true)
        if [[ -n "$VEC_VIOLATIONS" ]]; then
            echo "    ❌ Vec allocation violations found:"
            echo "$VEC_VIOLATIONS" | head -10
            VIOLATIONS_FOUND=$((VIOLATIONS_FOUND + 1))
        fi
        
        # String allocations  
        STRING_VIOLATIONS=$(find "$PROJECT_ROOT/$path" -name "*.rs" -exec grep -Hn "String::from\|\.to_string()\|format!" {} \; | grep -v "test\|bench\|example\|debug\|error\|warn\|info" || true)
        if [[ -n "$STRING_VIOLATIONS" ]]; then
            echo "    ❌ String allocation violations found:"
            echo "$STRING_VIOLATIONS" | head -10
            VIOLATIONS_FOUND=$((VIOLATIONS_FOUND + 1))
        fi
        
        # HashMap insertions (potential rehashing)
        HASHMAP_VIOLATIONS=$(find "$PROJECT_ROOT/$path" -name "*.rs" -exec grep -Hn "HashMap::new()\|BTreeMap::new()\|\.insert(" {} \; | grep -v "test\|bench\|example\|config" || true)
        if [[ -n "$HASHMAP_VIOLATIONS" ]]; then
            echo "    ⚠️  HashMap operations found (check for hot path usage):"
            echo "$HASHMAP_VIOLATIONS" | head -5
        fi
    fi
done

# ==============================================================================
# ZERO-COPY TRAIT VIOLATIONS
# ==============================================================================

echo "📋 Checking zero-copy trait usage..."

# Find structs that should have zerocopy traits but don't
STRUCTS_WITHOUT_ZEROCOPY=$(find "$PROJECT_ROOT/protocol_v2" -name "*.rs" -exec grep -l "#\[repr(C" {} \; | xargs grep -L "AsBytes\|FromBytes" | head -5 || true)
if [[ -n "$STRUCTS_WITHOUT_ZEROCOPY" ]]; then
    echo "  ❌ Structs with #[repr(C)] missing zerocopy traits:"
    echo "$STRUCTS_WITHOUT_ZEROCOPY"
    VIOLATIONS_FOUND=$((VIOLATIONS_FOUND + 1))
fi

# ==============================================================================
# TLV BUILDER ANTI-PATTERNS
# ==============================================================================

echo "🏗️  Checking TLV builder patterns..."

# Check for builder allocations
BUILDER_VIOLATIONS=$(find "$PROJECT_ROOT/protocol_v2" -name "builder.rs" -exec grep -Hn "\.to_vec()\|Vec::with_capacity" {} \; || true)
if [[ -n "$BUILDER_VIOLATIONS" ]]; then
    echo "  ❌ TLV Builder allocation violations found:"
    echo "$BUILDER_VIOLATIONS"
    VIOLATIONS_FOUND=$((VIOLATIONS_FOUND + 1))
fi

# ==============================================================================
# PANIC/UNWRAP IN HOT PATHS
# ==============================================================================

echo "🚨 Checking for panics in hot paths..."

for path in "${HOT_PATHS[@]}"; do
    if [[ -d "$PROJECT_ROOT/$path" ]]; then
        PANIC_VIOLATIONS=$(find "$PROJECT_ROOT/$path" -name "*.rs" -exec grep -Hn "panic!\|\.unwrap()\|\.expect(" {} \; | grep -v "test\|bench\|example" || true)
        if [[ -n "$PANIC_VIOLATIONS" ]]; then
            echo "  ❌ Panic/unwrap violations in $path:"
            echo "$PANIC_VIOLATIONS" | head -5
            VIOLATIONS_FOUND=$((VIOLATIONS_FOUND + 1))
        fi
    fi
done

# ==============================================================================
# PERFORMANCE REGRESSION INDICATORS
# ==============================================================================

echo "⚡ Checking for performance regression indicators..."

# Look for synchronous I/O in async contexts
SYNC_IO_VIOLATIONS=$(find "$PROJECT_ROOT" -name "*.rs" -exec grep -Hn "std::fs::\|std::net::" {} \; | grep -v "test\|bench\|example\|config" | head -5 || true)
if [[ -n "$SYNC_IO_VIOLATIONS" ]]; then
    echo "  ⚠️  Synchronous I/O found (should be async in hot paths):"
    echo "$SYNC_IO_VIOLATIONS"
fi

# ==============================================================================
# SUMMARY
# ==============================================================================

echo ""
if [[ $VIOLATIONS_FOUND -eq 0 ]]; then
    echo "✅ No critical zero-copy violations found!"
    echo "🚀 Codebase appears optimized for 50M msg/s performance"
    exit 0
else
    echo "❌ Found $VIOLATIONS_FOUND critical performance violations"
    echo "🔧 Fix these before committing to maintain 50M msg/s performance"
    echo ""
    echo "💡 Quick fixes:"
    echo "  - Replace .to_vec() with zero-copy references"
    echo "  - Use pre-allocated buffers instead of Vec::new()"  
    echo "  - Add AsBytes/FromBytes traits to #[repr(C)] structs"
    echo "  - Replace unwrap() with proper error handling"
    exit 1
fi