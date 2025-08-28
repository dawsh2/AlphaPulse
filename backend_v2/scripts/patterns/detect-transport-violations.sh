#!/bin/bash

# Torq Transport Usage Violation Detection
# Detects direct UnixSocketTransport::new usage outside approved factory locations

set -euo pipefail

# Script configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WHITELIST_FILE="${SCRIPT_DIR}/transport-whitelist.txt"

# Colors for output
RED='\033[0;31m'
YELLOW='\033[1;33m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
BOLD='\033[1m'
NC='\033[0m' # No Color

# Default whitelist patterns (approved locations for direct transport usage)
DEFAULT_WHITELIST=(
    "src/transport/factory.rs"
    "src/transport/builder.rs" 
    "libs/transport/src/factory.rs"
    "libs/transport/src/lib.rs"
    "network/transport/src/factory.rs"
    "network/transport/src/unix.rs"
    "services_v2/adapters/src/transport/"
)

# Function to check if file is whitelisted
is_whitelisted() {
    local file_path="$1"
    local normalized_path
    
    # Normalize path to relative form
    normalized_path=$(echo "$file_path" | sed 's|^.*/backend_v2/||' | sed 's|^.*/tool-002-worktree/||')
    
    # Check against whitelist file if it exists
    if [[ -f "$WHITELIST_FILE" ]]; then
        while IFS= read -r pattern; do
            # Skip comments and empty lines
            [[ "$pattern" =~ ^#.*$ ]] && continue
            [[ -z "$pattern" ]] && continue
            
            if [[ "$normalized_path" == *"$pattern"* ]]; then
                return 0  # Whitelisted
            fi
        done < "$WHITELIST_FILE"
    fi
    
    # Check against default whitelist
    for pattern in "${DEFAULT_WHITELIST[@]}"; do
        if [[ "$normalized_path" == *"$pattern"* ]]; then
            return 0  # Whitelisted
        fi
    done
    
    return 1  # Not whitelisted
}

# Function to detect violations in a single file
detect_violations_in_file() {
    local file_path="$1"
    local violations_found=0
    local line_number=1
    
    # Skip binary files and non-Rust files
    if ! [[ "$file_path" =~ \.(rs|toml)$ ]]; then
        return 0
    fi
    
    # Check if file is whitelisted
    if is_whitelisted "$file_path"; then
        return 0
    fi
    
    # Search for UnixSocketTransport::new usage, excluding comments and strings
    while IFS= read -r line; do
        # Skip empty lines
        [[ -z "$line" ]] && { ((line_number++)); continue; }
        
        # Skip comment lines (// or /* */ style)
        if [[ "$line" =~ ^[[:space:]]*// ]] || [[ "$line" =~ ^[[:space:]]*\* ]]; then
            ((line_number++))
            continue
        fi
        
        # Skip lines where pattern is in strings (basic detection)
        if [[ "$line" =~ \".*UnixSocketTransport::new.*\" ]]; then
            ((line_number++))
            continue
        fi
        
        # Check for violation pattern
        if [[ "$line" =~ UnixSocketTransport::new ]]; then
            echo -e "${RED}VIOLATION${NC}: Direct UnixSocketTransport usage in ${BOLD}$file_path${NC}:${BLUE}$line_number${NC}"
            echo -e "  ${YELLOW}Found:${NC} $(echo "$line" | xargs)"
            echo -e "  ${GREEN}Suggestion:${NC} Use TransportFactory::create() instead"
            echo -e "  ${GREEN}Example:${NC} let transport = TransportFactory::create(\"/tmp/socket\")?;"
            echo ""
            violations_found=1
        fi
        
        ((line_number++))
    done < "$file_path"
    
    return $violations_found
}

# Function to scan directory recursively
scan_directory() {
    local target_dir="$1"
    local found_violations=false
    
    # Find all Rust files
    while IFS= read -r -d '' file; do
        if detect_violations_in_file "$file"; then
            found_violations=true
            # Don't exit early - we want to report all violations
        fi
    done < <(find "$target_dir" -name "*.rs" -type f -print0 2>/dev/null)
    
    # Return 1 if any violations found, 0 if none
    if [[ "$found_violations" == "true" ]]; then
        return 1
    else
        return 0
    fi
}

# Function to display usage
show_usage() {
    echo "Usage: $0 <file_or_directory>"
    echo ""
    echo "Detects direct UnixSocketTransport::new usage outside approved locations."
    echo ""
    echo "Options:"
    echo "  file_or_directory    Path to scan for violations"
    echo ""
    echo "Examples:"
    echo "  $0 src/main.rs              # Check single file"
    echo "  $0 src/                     # Check directory"
    echo "  $0 .                        # Check current directory"
    echo ""
    echo "Whitelist file: $WHITELIST_FILE"
}

# Main function
main() {
    if [[ $# -eq 0 ]]; then
        show_usage
        exit 1
    fi
    
    local target="$1"
    local exit_code=0
    
    if [[ ! -e "$target" ]]; then
        echo -e "${RED}Error:${NC} Path '$target' does not exist"
        exit 1
    fi
    
    echo -e "${BOLD}Torq Transport Usage Violation Detection${NC}"
    echo -e "${BOLD}============================================${NC}"
    echo ""
    
    if [[ -f "$target" ]]; then
        echo "Scanning file: $target"
        echo ""
        # detect_violations_in_file returns 1 when violations found, 0 when none found
        # We want exit_code=1 when violations found
        detect_violations_in_file "$target"
        if [[ $? -ne 0 ]]; then
            exit_code=1
        fi
    elif [[ -d "$target" ]]; then
        echo "Scanning directory: $target"
        echo ""
        # scan_directory returns non-zero when violations found
        scan_directory "$target"
        if [[ $? -ne 0 ]]; then
            exit_code=1
        fi
    else
        echo -e "${RED}Error:${NC} '$target' is not a file or directory"
        exit 1
    fi
    
    if [[ $exit_code -eq 0 ]]; then
        echo -e "${GREEN}✅ No transport usage violations found${NC}"
    else
        echo -e "${RED}❌ Transport usage violations detected${NC}"
        echo ""
        echo -e "${YELLOW}To fix these violations:${NC}"
        echo "1. Replace direct UnixSocketTransport::new() calls with TransportFactory::create()"
        echo "2. If this is a legitimate factory usage, add the file to the whitelist:"
        echo "   echo 'src/your/file.rs' >> $WHITELIST_FILE"
    fi
    
    exit $exit_code
}

# Run main function
main "$@"