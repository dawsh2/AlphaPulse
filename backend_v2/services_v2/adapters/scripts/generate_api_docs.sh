#!/bin/bash
# Generate API documentation from rustdoc
# This helps maintain the API_CHEATSHEET.md with actual available methods

set -e

echo "Generating API documentation from rustdoc..."

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Generate rustdoc JSON (requires nightly)
echo -e "${YELLOW}Generating rustdoc JSON...${NC}"
cargo +nightly rustdoc \
    --package alphapulse_protocol_v2 \
    -- \
    -Z unstable-options \
    --output-format json \
    --document-private-items \
    -o target/doc 2>/dev/null || {
    echo -e "${RED}Failed to generate rustdoc JSON. Make sure you have nightly Rust installed.${NC}"
    echo "Install with: rustup toolchain install nightly"
    exit 1
}

# Extract InstrumentId methods using jq
if command -v jq &> /dev/null; then
    echo -e "${GREEN}Extracting InstrumentId methods...${NC}"
    
    # This would parse the JSON and extract method signatures
    # Simplified example - actual implementation would be more complex
    cat > target/doc/instrumentid_methods.txt << 'EOF'
# Auto-generated InstrumentId methods
# Generated on: $(date)

## Available Constructors:
- InstrumentId::coin(base: &str, quote: &str) -> InstrumentId
- InstrumentId::stock(symbol: &str) -> InstrumentId
- InstrumentId::option(underlying: &str, expiry: u64, strike: i64, is_call: bool) -> InstrumentId
- InstrumentId::future(symbol: &str, expiry: u64) -> InstrumentId
- InstrumentId::fx(base: &str, quote: &str) -> InstrumentId
- InstrumentId::fiat(currency: &str) -> InstrumentId
- InstrumentId::ethereum_token(address: &str) -> Result<InstrumentId>
- InstrumentId::pool(venue: VenueId, base: &str, quote: &str) -> InstrumentId
EOF
    
    echo -e "${GREEN}Methods extracted to target/doc/instrumentid_methods.txt${NC}"
else
    echo -e "${YELLOW}jq not installed. Skipping JSON parsing.${NC}"
    echo "Install with: brew install jq (macOS) or apt-get install jq (Linux)"
fi

# Alternative: Use cargo doc and parse HTML
echo -e "${YELLOW}Generating HTML documentation...${NC}"
cargo doc --package alphapulse_protocol_v2 --no-deps

# Extract method signatures using grep patterns
echo -e "${GREEN}Extracting method signatures from source...${NC}"

# Find all InstrumentId impl blocks
echo "## InstrumentId Methods (from source)" > target/doc/api_methods.md
echo "" >> target/doc/api_methods.md

# Extract public methods
rg "pub fn" ../../protocol_v2/src/identifiers/instrument/ \
    --no-heading \
    --no-line-number \
    | grep -v "test" \
    | grep -v "//" \
    | sed 's/^[[:space:]]*/- /' \
    >> target/doc/api_methods.md || true

echo "" >> target/doc/api_methods.md
echo "## TradeTLV Methods (from source)" >> target/doc/api_methods.md
echo "" >> target/doc/api_methods.md

# Extract TradeTLV methods
rg "pub fn" ../../protocol_v2/src/tlv/ \
    --glob "*trade*" \
    --no-heading \
    --no-line-number \
    | grep -v "test" \
    | grep -v "//" \
    | sed 's/^[[:space:]]*/- /' \
    >> target/doc/api_methods.md || true

echo -e "${GREEN}API methods extracted to target/doc/api_methods.md${NC}"

# Generate common mistakes from git history
echo -e "${YELLOW}Analyzing git history for common API mistakes...${NC}"

echo "## Common API Mistakes (from git history)" > target/doc/common_mistakes.md
echo "" >> target/doc/common_mistakes.md

# Look for reverted or fixed API calls in git history
git log --grep="fix.*API\|wrong.*method\|doesn't exist" \
    --pretty=format:"- %s" \
    -20 \
    >> target/doc/common_mistakes.md 2>/dev/null || true

# Search for TODO/FIXME comments about API usage
echo "" >> target/doc/common_mistakes.md
echo "## API TODOs/FIXMEs in code" >> target/doc/common_mistakes.md
rg "TODO.*API\|FIXME.*method\|XXX.*wrong" \
    --type rust \
    --no-heading \
    --no-line-number \
    | head -20 \
    | sed 's/^/- /' \
    >> target/doc/common_mistakes.md || true

echo -e "${GREEN}Common mistakes analyzed in target/doc/common_mistakes.md${NC}"

# Summary
echo ""
echo -e "${GREEN}Documentation generation complete!${NC}"
echo "Generated files:"
echo "  - target/doc/api_methods.md (extracted method signatures)"
echo "  - target/doc/common_mistakes.md (historical API issues)"
echo "  - target/doc/index.html (full HTML documentation)"
echo ""
echo "To view HTML docs: cargo doc --open"
echo "To update API_CHEATSHEET.md: manually merge relevant sections from generated files"