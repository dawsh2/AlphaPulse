#!/usr/bin/env bash
# AlphaPulse Service Boundary Validation
# Ensures proper architectural separation between services

set -euo pipefail

VIOLATIONS_FOUND=0
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

echo "🏗️  Validating service architectural boundaries..."

# ==============================================================================
# SERVICE BOUNDARY DEFINITIONS (Bash 3.2 compatible)
# ==============================================================================

get_allowed_deps() {
    case "$1" in
        "protocol_v2") echo "" ;;
        "libs") echo "protocol_v2" ;;
        "services_v2") echo "protocol_v2,libs" ;;
        "relays") echo "protocol_v2,libs" ;;
        "network") echo "protocol_v2" ;;
        *) echo "" ;;
    esac
}

# ==============================================================================
# CHECK IMPORT VIOLATIONS
# ==============================================================================

echo "📦 Checking import boundaries..."

check_imports() {
    local service="$1"
    local service_path="$PROJECT_ROOT/$service"
    
    if [[ ! -d "$service_path" ]]; then
        return
    fi
    
    echo "  🔍 Checking $service service boundaries..."
    
    # Get allowed dependencies for this service
    local allowed
    allowed=$(get_allowed_deps "$service")
    
    # Find use statements importing from our internal services only
    # Look for imports starting with our service names: protocol_v2, libs, services_v2, relays, network
    local violations
    violations=$(find "$service_path" -name "*.rs" -not -path "*/target/*" -exec grep -Hn "^use \(protocol_v2\|libs\|services_v2\|relays\|network\|alphapulse_\)::" {} \; || true)
    
    if [[ -n "$violations" ]]; then
        while IFS= read -r line; do
            local imported_service
            imported_service=$(echo "$line" | sed -n 's/.*use \(protocol_v2\|libs\|services_v2\|relays\|network\|alphapulse_[^:]*\)::.*/\1/p')
            
            # Check if this import is allowed (bash 3.2 compatible)
            local is_allowed=false
            if [[ -n "$allowed" && -n "$imported_service" ]]; then
                # Split allowed dependencies by comma and check each
                IFS=',' read -r allowed_dep_list <<< "$allowed"
                for allowed_dep in $allowed_dep_list; do
                    if [[ "$imported_service" == "$allowed_dep" || "$imported_service" == *"$allowed_dep"* ]]; then
                        is_allowed=true
                        break
                    fi
                done
            fi
            
            if [[ "$is_allowed" == false && -n "$imported_service" ]]; then
                echo "    ❌ Forbidden import in $service: $line"
                VIOLATIONS_FOUND=$((VIOLATIONS_FOUND + 1))
            fi
        done <<< "$violations"
    fi
}

# Check each service
for service in protocol_v2 libs services_v2 relays network; do
    check_imports "$service"
done

# ==============================================================================
# CHECK CIRCULAR DEPENDENCIES
# ==============================================================================

echo "🔄 Checking for circular dependencies..."

# Protocol V2 should never import from higher-level services
PROTOCOL_VIOLATIONS=$(find "$PROJECT_ROOT/protocol_v2" -name "*.rs" -exec grep -Hn "use.*services_v2\|use.*relays\|use.*libs" {} \; || true)
if [[ -n "$PROTOCOL_VIOLATIONS" ]]; then
    echo "  ❌ Protocol V2 importing from higher-level services:"
    echo "$PROTOCOL_VIOLATIONS"
    VIOLATIONS_FOUND=$((VIOLATIONS_FOUND + 1))
fi

# Libs should not import from services or relays
LIBS_VIOLATIONS=$(find "$PROJECT_ROOT/libs" -name "*.rs" -exec grep -Hn "use.*services_v2\|use.*relays" {} \; || true)
if [[ -n "$LIBS_VIOLATIONS" ]]; then
    echo "  ❌ Libs importing from application services:"
    echo "$LIBS_VIOLATIONS"
    VIOLATIONS_FOUND=$((VIOLATIONS_FOUND + 1))
fi

# ==============================================================================
# CHECK DIRECT FILE ACCESS VIOLATIONS  
# ==============================================================================

echo "📁 Checking for inappropriate file access patterns..."

# Services shouldn't directly access other service internals
DIRECT_ACCESS_VIOLATIONS=$(find "$PROJECT_ROOT/services_v2" -name "*.rs" -exec grep -Hn "\.\./" {} \; | \
    grep -v "test\|example\|config" | head -10 || true)
if [[ -n "$DIRECT_ACCESS_VIOLATIONS" ]]; then
    echo "  ⚠️  Relative path access found (potential boundary violation):"
    echo "$DIRECT_ACCESS_VIOLATIONS"
fi

# ==============================================================================
# CHECK SHARED STATE VIOLATIONS
# ==============================================================================

echo "🔒 Checking for shared mutable state..."

# Global mutable state should be avoided
GLOBAL_STATE_VIOLATIONS=$(find "$PROJECT_ROOT" -name "*.rs" -exec grep -Hn "static mut\|lazy_static!\|once_cell" {} \; | \
    grep -v "test\|benchmark\|config" || true)
if [[ -n "$GLOBAL_STATE_VIOLATIONS" ]]; then
    echo "  ⚠️  Global mutable state found:"
    echo "$GLOBAL_STATE_VIOLATIONS" | head -5
fi

# ==============================================================================
# VALIDATE SERVICE INTERFACE CONTRACTS
# ==============================================================================

echo "📋 Validating service interfaces..."

# Check that services expose proper public APIs
for service_dir in "$PROJECT_ROOT"/services_v2/*/; do
    if [[ -d "$service_dir" ]]; then
        service_name=$(basename "$service_dir")
        lib_file="$service_dir/src/lib.rs"
        
        if [[ -f "$lib_file" ]]; then
            # Check for proper module documentation
            if ! grep -q "//!" "$lib_file"; then
                echo "  ⚠️  Service $service_name missing module documentation"
            fi
            
            # Check for public API exports
            if ! grep -q "pub use\|pub mod" "$lib_file"; then
                echo "  ⚠️  Service $service_name not exporting public API"
            fi
        fi
    fi
done

# ==============================================================================
# SUMMARY
# ==============================================================================

echo ""
if [[ $VIOLATIONS_FOUND -eq 0 ]]; then
    echo "✅ All service boundaries are properly maintained!"
    echo "🏗️  Architecture integrity validated"
    exit 0
else
    echo "❌ Found $VIOLATIONS_FOUND architectural boundary violations"
    echo ""
    echo "💡 Fix recommendations:"
    echo "  - Move shared code to libs/ directory"
    echo "  - Use proper service interfaces instead of direct imports"
    echo "  - Avoid circular dependencies between services"
    echo "  - Follow the dependency hierarchy: protocol_v2 → libs → services"
    exit 1
fi