#!/bin/bash
# Install git hooks for AlphaPulse Task System

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Get directories
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
GIT_ROOT=$(git rev-parse --show-toplevel)
GIT_HOOKS_DIR="$GIT_ROOT/.git/hooks"

echo -e "${BLUE}🔧 Installing AlphaPulse Task Validation Hooks${NC}"
echo "================================================"

# Check if .git/hooks directory exists
if [[ ! -d "$GIT_HOOKS_DIR" ]]; then
    echo -e "${RED}Error: Not in a git repository${NC}"
    exit 1
fi

# Install pre-commit hook
if [[ -f "$SCRIPT_DIR/pre-commit" ]]; then
    # Check if hook already exists
    if [[ -f "$GIT_HOOKS_DIR/pre-commit" ]]; then
        # Back up existing hook
        echo -e "${YELLOW}⚠️  Existing pre-commit hook found${NC}"
        backup_name="$GIT_HOOKS_DIR/pre-commit.backup.$(date +%Y%m%d_%H%M%S)"
        cp "$GIT_HOOKS_DIR/pre-commit" "$backup_name"
        echo "   Backed up to: $(basename "$backup_name")"
    fi
    
    # Copy new hook
    cp "$SCRIPT_DIR/pre-commit" "$GIT_HOOKS_DIR/pre-commit"
    chmod +x "$GIT_HOOKS_DIR/pre-commit"
    echo -e "${GREEN}✅ Installed pre-commit hook${NC}"
else
    echo -e "${RED}❌ pre-commit hook not found in $SCRIPT_DIR${NC}"
    exit 1
fi

echo ""
echo -e "${GREEN}🎉 Git hooks installed successfully!${NC}"
echo ""
echo "The pre-commit hook will:"
echo "  • Validate task metadata format"
echo "  • Check for required fields (task_id, status, priority)"
echo "  • Ensure dependencies are declared"
echo "  • Block commits with invalid task files"
echo ""
echo "To bypass the hook in emergencies (not recommended):"
echo "  git commit --no-verify"
echo ""
echo "To uninstall:"
echo "  rm $GIT_HOOKS_DIR/pre-commit"