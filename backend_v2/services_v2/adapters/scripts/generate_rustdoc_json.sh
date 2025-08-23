#!/bin/bash
# Generate JSON documentation from rustdoc for AI/tooling consumption
# This creates machine-readable API docs that auto-update with code changes

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}🔧 Generating rustdoc JSON for AI/tooling consumption...${NC}"
echo ""

# Check for nightly Rust
if ! command -v rustup &> /dev/null; then
    echo -e "${RED}❌ rustup not found. Please install Rust.${NC}"
    exit 1
fi

if ! rustup toolchain list | grep -q nightly; then
    echo -e "${YELLOW}📦 Installing nightly Rust for JSON generation...${NC}"
    rustup toolchain install nightly
fi

# Create output directory
OUTPUT_DIR="target/doc/json"
mkdir -p "$OUTPUT_DIR"

# Function to generate JSON docs for a package
generate_package_json() {
    local package_name="$1"
    local package_path="$2"
    local description="$3"
    
    echo -e "${YELLOW}📖 Generating JSON for $package_name...${NC}"
    
    # Generate JSON documentation
    cargo +nightly rustdoc \
        --package "$package_name" \
        --manifest-path "$package_path/Cargo.toml" \
        -- -Z unstable-options \
        --output-format json \
        --document-private-items \
        -o "$OUTPUT_DIR" 2>/dev/null || {
        echo -e "${RED}⚠️  Failed to generate JSON for $package_name (continuing...)${NC}"
        return 1
    }
    
    # Rename output to more readable name
    if [ -f "$OUTPUT_DIR/${package_name}.json" ]; then
        mv "$OUTPUT_DIR/${package_name}.json" "$OUTPUT_DIR/${package_name}_api.json"
        echo -e "${GREEN}✅ $package_name → ${package_name}_api.json${NC}"
    fi
    
    return 0
}

echo -e "${BLUE}Generating JSON documentation for core packages...${NC}"
echo ""

# Generate for Protocol V2 (core API)
generate_package_json "alphapulse_protocol_v2" "../../protocol_v2" "Core Protocol V2 API"

# Generate for Adapters (this package)
generate_package_json "alphapulse-adapter-service" "." "Adapter Service API"

# Try to generate for related packages if they exist
if [ -d "../../libs/adapters" ]; then
    generate_package_json "alphapulse_adapters" "../../libs/adapters" "Adapter Utilities"
fi

# Create consolidated API index
echo -e "${BLUE}📋 Creating API index...${NC}"

cat > "$OUTPUT_DIR/api_index.json" << 'EOF'
{
  "generated_at": "AUTO_TIMESTAMP",
  "description": "Auto-generated API documentation from rustdoc",
  "packages": {
    "alphapulse_protocol_v2": {
      "file": "alphapulse_protocol_v2_api.json",
      "description": "Core Protocol V2 API - InstrumentId, TLV types, message building",
      "key_types": [
        "InstrumentId",
        "TradeTLV",
        "QuoteTLV", 
        "PoolSwapTLV",
        "TLVMessageBuilder",
        "VenueId"
      ],
      "common_methods": {
        "InstrumentId": ["coin", "stock", "ethereum_token", "polygon_token"],
        "TradeTLV": ["new", "from_bytes", "as_bytes", "to_tlv_message"],
        "QuoteTLV": ["new", "from_bytes", "as_bytes"],
        "TLVMessageBuilder": ["new", "add_tlv", "build"]
      }
    },
    "alphapulse_adapter_service": {
      "file": "alphapulse-adapter-service_api.json",
      "description": "Adapter Service - Input/output adapters for exchanges",
      "key_types": [
        "CoinbaseCollector",
        "InputAdapter",
        "ValidationFramework", 
        "ConnectionManager",
        "AdapterMetrics"
      ],
      "reference_implementations": [
        "CoinbaseCollector - CEX WebSocket adapter",
        "PolygonDEXCollector - DEX event adapter"
      ]
    }
  },
  "usage": {
    "command": "Use this JSON data to provide accurate API information",
    "update_frequency": "Auto-updates when code changes",
    "ai_friendly": true,
    "human_readable": "Run: cargo doc --open"
  }
}
EOF

# Replace timestamp
if command -v date &> /dev/null; then
    TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
    sed -i.bak "s/AUTO_TIMESTAMP/$TIMESTAMP/" "$OUTPUT_DIR/api_index.json" && rm "$OUTPUT_DIR/api_index.json.bak" 2>/dev/null || true
fi

# Generate API method extraction script
echo -e "${BLUE}📝 Creating API method extractor...${NC}"

cat > "$OUTPUT_DIR/extract_methods.py" << 'EOF'
#!/usr/bin/env python3
"""
Extract API methods from rustdoc JSON for quick reference
Usage: python extract_methods.py <package_name>
"""

import json
import sys
from pathlib import Path

def extract_methods(package_file):
    """Extract public methods from rustdoc JSON"""
    try:
        with open(package_file) as f:
            data = json.load(f)
    except FileNotFoundError:
        print(f"❌ File not found: {package_file}")
        return
    
    if 'index' not in data:
        print("❌ Invalid JSON format - no 'index' key")
        return
        
    methods = {}
    for item_id, item in data['index'].items():
        if item.get('kind') == 'struct':
            struct_name = item.get('name', 'Unknown')
            methods[struct_name] = []
            
            # Look for impl blocks
            if 'inner' in item and 'impls' in item['inner']:
                for impl_id in item['inner']['impls']:
                    impl_item = data['index'].get(str(impl_id), {})
                    if 'inner' in impl_item and 'items' in impl_item['inner']:
                        for method_id in impl_item['inner']['items']:
                            method_item = data['index'].get(str(method_id), {})
                            if method_item.get('kind') == 'function':
                                method_name = method_item.get('name', 'unknown')
                                methods[struct_name].append(method_name)
    
    # Print results
    for struct_name, method_list in methods.items():
        if method_list:  # Only show structs with methods
            print(f"\n## {struct_name}")
            for method in sorted(method_list):
                print(f"  - {method}()")

if __name__ == "__main__":
    if len(sys.argv) != 2:
        print("Usage: python extract_methods.py <json_file>")
        sys.exit(1)
    
    extract_methods(sys.argv[1])
EOF

chmod +x "$OUTPUT_DIR/extract_methods.py"

# Summary
echo ""
echo -e "${GREEN}✅ JSON documentation generation complete!${NC}"
echo ""
echo -e "${BLUE}Generated files:${NC}"
for file in "$OUTPUT_DIR"/*.json; do
    if [ -f "$file" ]; then
        SIZE=$(du -h "$file" | cut -f1)
        echo -e "  📄 $(basename "$file") (${SIZE})"
    fi
done

echo ""
echo -e "${BLUE}📖 Usage:${NC}"
echo -e "  ${YELLOW}For AI/tools:${NC} Parse JSON files in $OUTPUT_DIR/"
echo -e "  ${YELLOW}For humans:${NC}   cargo doc --package alphapulse-adapter-service --open"
echo -e "  ${YELLOW}Method extraction:${NC} python $OUTPUT_DIR/extract_methods.py <json_file>"
echo ""
echo -e "${GREEN}🎯 Benefits:${NC}"
echo -e "  - Auto-updates with code changes (no stale docs!)"
echo -e "  - Machine-readable API information"
echo -e "  - Fast AI assistant responses"
echo -e "  - Searchable documentation"
echo ""

# Test JSON validity
echo -e "${BLUE}🔍 Validating JSON files...${NC}"
VALID_COUNT=0
TOTAL_COUNT=0
for file in "$OUTPUT_DIR"/*.json; do
    if [ -f "$file" ]; then
        TOTAL_COUNT=$((TOTAL_COUNT + 1))
        if python3 -m json.tool "$file" > /dev/null 2>&1; then
            VALID_COUNT=$((VALID_COUNT + 1))
            echo -e "  ✅ $(basename "$file")"
        else
            echo -e "  ❌ $(basename "$file") - Invalid JSON"
        fi
    fi
done

echo ""
if [ $VALID_COUNT -eq $TOTAL_COUNT ]; then
    echo -e "${GREEN}🎉 All $TOTAL_COUNT JSON files are valid!${NC}"
else
    echo -e "${YELLOW}⚠️  $VALID_COUNT/$TOTAL_COUNT JSON files are valid${NC}"
fi

echo ""
echo -e "${BLUE}💡 Next steps:${NC}"
echo -e "  1. Test the JSON with your AI tools"
echo -e "  2. Run 'cargo doc --open' to see HTML version"
echo -e "  3. Add this script to CI/CD for continuous updates"
echo -e "  4. Use the JSON to answer API questions accurately"
EOF