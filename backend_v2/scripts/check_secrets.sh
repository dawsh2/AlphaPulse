#!/bin/bash
# Secret Detection with Ethereum Event Signature Exclusions
# Prevents committing actual secrets while allowing legitimate blockchain data

set -e

# Common secret patterns to detect
PATTERNS=(
    "password.*=.*['\"][^'\"]{8,}"     # password = "something"
    "api[_-]?key.*=.*['\"][^'\"]{16,}" # api_key = "long_string"
    "secret.*=.*['\"][^'\"]{16,}"      # secret = "long_string"
    "token.*=.*['\"][^'\"]{20,}"       # token = "long_string" 
    "private[_-]?key.*['\"][^'\"]{32,}" # private_key = "long_string"
    "BEGIN.*PRIVATE.*KEY"              # PEM private keys
    "[A-Z0-9]{32,}"                    # Long uppercase alphanumeric (possible API keys)
)

# Load exclusions from .secretsignore
EXCLUSIONS=()
if [[ -f ".secretsignore" ]]; then
    while IFS= read -r line; do
        # Skip comments and empty lines
        if [[ ! "$line" =~ ^#.*$ ]] && [[ -n "$line" ]]; then
            EXCLUSIONS+=("$line")
        fi
    done < .secretsignore
fi

echo "🔍 Checking for secrets (with Ethereum event signature exclusions)..."

SECRETS_FOUND=false

# Check each file passed as argument
for file in "$@"; do
    if [[ ! -f "$file" ]]; then
        continue
    fi
    
    # Skip binary files
    if file "$file" | grep -q "binary"; then
        continue
    fi
    
    # Check for each pattern
    for pattern in "${PATTERNS[@]}"; do
        matches=$(grep -Hn "$pattern" "$file" 2>/dev/null || true)
        
        if [[ -n "$matches" ]]; then
            # Check if any matches should be excluded
            while IFS= read -r match; do
                if [[ -n "$match" ]]; then
                    excluded=false
                    
                    # Check against exclusions
                    for exclusion in "${EXCLUSIONS[@]}"; do
                        if echo "$match" | grep -q "$exclusion"; then
                            excluded=true
                            break
                        fi
                    done
                    
                    if [[ "$excluded" == false ]]; then
                        echo "⚠️  Potential secret in $match"
                        SECRETS_FOUND=true
                    fi
                fi
            done <<< "$matches"
        fi
    done
    
    # Special check for long hex strings (but exclude known Ethereum signatures)
    hex_matches=$(grep -Hn "0x[a-fA-F0-9]{32,}" "$file" 2>/dev/null || true)
    if [[ -n "$hex_matches" ]]; then
        while IFS= read -r match; do
            if [[ -n "$match" ]]; then
                excluded=false
                
                # Extract the hex value
                hex_value=$(echo "$match" | grep -o "0x[a-fA-F0-9]*")
                
                # Check against known Ethereum signatures
                for exclusion in "${EXCLUSIONS[@]}"; do
                    if [[ "$hex_value" == "$exclusion" ]]; then
                        excluded=true
                        break
                    fi
                done
                
                if [[ "$excluded" == false && ${#hex_value} -gt 34 ]]; then
                    echo "⚠️  Long hex string (possible secret) in $match"
                    SECRETS_FOUND=true
                fi
            fi
        done <<< "$hex_matches"
    fi
done

if [[ "$SECRETS_FOUND" == true ]]; then
    echo ""
    echo "❌ Potential secrets detected in commit"
    echo "If these are legitimate values:"
    echo "1. Add them to .secretsignore"
    echo "2. Or use environment variables instead"
    echo "3. Or commit with --no-verify (NOT recommended)"
    exit 1
else
    echo "✅ No secrets detected"
    echo "📊 Ethereum event signatures properly excluded"
fi