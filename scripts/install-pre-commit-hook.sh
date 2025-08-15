#!/bin/bash

# Install pre-commit hook for security checks
# Run this script after cloning the repository

HOOK_DIR=".git/hooks"
HOOK_FILE="$HOOK_DIR/pre-commit"

# Create hooks directory if it doesn't exist
mkdir -p "$HOOK_DIR"

# Create the pre-commit hook
cat > "$HOOK_FILE" << 'EOF'
#!/bin/bash

# Colors for output
RED='\033[0;31m'
YELLOW='\033[1;33m'
GREEN='\033[0;32m'
NC='\033[0m' # No Color

echo -e "${GREEN}Running security pre-commit checks...${NC}"

# Forbidden file patterns
FORBIDDEN_FILES="\.env$|\.env\.|private_key|secret_key|\.key$|\.pem$|\.p12$|\.pfx$|keystore|mnemonic\.txt|seed\.txt|wallet\.json"

# Check for forbidden files
FOUND_FILES=$(git diff --cached --name-only | grep -E "$FORBIDDEN_FILES")
if [ ! -z "$FOUND_FILES" ]; then
    echo -e "${RED}❌ ERROR: Attempting to commit sensitive files!${NC}"
    echo -e "${RED}The following files appear to contain sensitive data:${NC}"
    echo "$FOUND_FILES" | while read file; do
        echo -e "${RED}  - $file${NC}"
    done
    echo -e "${YELLOW}If these files don't contain secrets, consider renaming them.${NC}"
    echo -e "${YELLOW}If you must commit them, use: git commit --no-verify${NC}"
    exit 1
fi

# Patterns that might indicate secrets in file content
SECRET_PATTERNS="PRIVATE_KEY|API_KEY|API_SECRET|SECRET_KEY|PASSWORD|MNEMONIC|SEED_PHRASE|Bearer\s+[A-Za-z0-9\-_=]+|0x[a-fA-F0-9]{64}"

# Check for secret patterns in staged content
FOUND_SECRETS=$(git diff --cached | grep -iE "$SECRET_PATTERNS" | head -5)
if [ ! -z "$FOUND_SECRETS" ]; then
    echo -e "${YELLOW}⚠️  WARNING: Possible sensitive data detected in commit${NC}"
    echo -e "${YELLOW}Found the following suspicious patterns:${NC}"
    echo "$FOUND_SECRETS" | head -5
    echo ""
    read -p "Are you sure these are safe to commit? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo -e "${RED}Commit cancelled.${NC}"
        exit 1
    fi
fi

# Check for large files (might be data dumps with secrets)
LARGE_FILES=$(git diff --cached --name-only | xargs -I {} sh -c 'if [ -f "{}" ]; then wc -c "{}" | awk "\$1 > 1048576 {print \$2}"; fi' 2>/dev/null)
if [ ! -z "$LARGE_FILES" ]; then
    echo -e "${YELLOW}⚠️  WARNING: Large files detected (>1MB):${NC}"
    echo "$LARGE_FILES"
    read -p "Large files might contain data dumps. Continue? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

echo -e "${GREEN}✅ Security checks passed!${NC}"
EOF

# Make the hook executable
chmod +x "$HOOK_FILE"

echo "✅ Pre-commit hook installed successfully!"
echo ""
echo "The hook will:"
echo "  - Prevent committing .env files and private keys"
echo "  - Warn about potential secrets in code"
echo "  - Alert on large files that might contain data dumps"
echo ""
echo "To bypass the hook in emergencies (NOT RECOMMENDED):"
echo "  git commit --no-verify"
echo ""
echo "To uninstall the hook:"
echo "  rm .git/hooks/pre-commit"