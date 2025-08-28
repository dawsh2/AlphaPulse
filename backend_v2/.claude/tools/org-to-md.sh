#!/bin/bash
# Org-mode to Markdown conversion script for documentation
# Converts all .org files in source/ to .md files in generated/

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SOURCE_DIR="$SCRIPT_DIR/../docs/source"
GENERATED_DIR="$SCRIPT_DIR/../docs/generated"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}Starting Org to Markdown conversion...${NC}"

# Create generated directory if it doesn't exist
mkdir -p "$GENERATED_DIR"

# Function to convert a single org file to markdown
convert_file() {
    local org_file="$1"
    local rel_path="${org_file#$SOURCE_DIR/}"
    local md_file="$GENERATED_DIR/${rel_path%.org}.md"
    local md_dir="$(dirname "$md_file")"
    
    # Create target directory if needed
    mkdir -p "$md_dir"
    
    # Check if org file is newer than md file
    if [ -f "$md_file" ] && [ "$org_file" -ot "$md_file" ]; then
        echo -e "  ${YELLOW}Skipping${NC} $rel_path (up to date)"
        return 0
    fi
    
    echo -e "  ${GREEN}Converting${NC} $rel_path"
    
    # Use emacs in batch mode to convert org to markdown
    emacs "$org_file" \
        --batch \
        --eval "(require 'ox-md)" \
        --eval "(setq org-export-with-toc nil)" \
        --eval "(setq org-export-with-author nil)" \
        --eval "(setq org-export-with-date nil)" \
        --eval "(setq org-export-with-title t)" \
        --eval "(setq org-export-headline-levels 4)" \
        --funcall org-md-export-to-markdown \
        2>/dev/null
    
    # Move the generated file to the correct location
    local generated_name="${org_file%.org}.md"
    if [ -f "$generated_name" ]; then
        mv "$generated_name" "$md_file"
        
        # Add generation notice at the top
        local temp_file=$(mktemp)
        {
            echo "<!-- GENERATED FROM $rel_path - DO NOT EDIT DIRECTLY -->"
            echo ""
            cat "$md_file"
        } > "$temp_file"
        mv "$temp_file" "$md_file"
        
        echo -e "    ✓ Generated $(basename "$md_file")"
    else
        echo -e "    ${RED}✗ Failed to generate${NC} $(basename "$md_file")"
        return 1
    fi
}

# Find all org files and convert them
find "$SOURCE_DIR" -name "*.org" -type f | while read -r org_file; do
    convert_file "$org_file"
done

# Clean up any orphaned markdown files (org source deleted)
find "$GENERATED_DIR" -name "*.md" -type f | while read -r md_file; do
    rel_path="${md_file#$GENERATED_DIR/}"
    org_file="$SOURCE_DIR/${rel_path%.md}.org"
    
    if [ ! -f "$org_file" ]; then
        echo -e "  ${RED}Removing orphaned${NC} $rel_path"
        rm "$md_file"
    fi
done

echo -e "${GREEN}Conversion complete!${NC}"

# Verify Rust can access the files
if command -v cargo &> /dev/null; then
    echo -e "\n${GREEN}Verifying Rust accessibility...${NC}"
    
    # Check if any Rust files use include_str! with our generated docs
    if grep -r 'include_str!.*generated/' --include="*.rs" "$SCRIPT_DIR/../../.." 2>/dev/null | head -1 > /dev/null; then
        echo -e "  ✓ Found Rust files using generated documentation"
        echo -e "  Run ${YELLOW}cargo doc${NC} to verify documentation rendering"
    else
        echo -e "  ${YELLOW}No Rust files currently using generated docs${NC}"
        echo -e "  Add ${YELLOW}#[doc = include_str!(\"path/to/generated.md\")]${NC} to your Rust code"
    fi
fi