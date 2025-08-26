#!/bin/bash
# Script to update all documentation from git checkout to git worktree
# This prevents agents from accidentally working on wrong branches

echo "🔄 Migrating documentation from 'git checkout' to 'git worktree'..."
echo ""

# Find all markdown files that mention git checkout
echo "📋 Files to update:"
grep -r "git checkout -b\|git checkout\|checkout -b" --include="*.md" .claude/ 2>/dev/null | cut -d: -f1 | sort -u

echo ""
echo "🔧 Updating files..."

# Update all instances
find .claude/ -name "*.md" -type f -exec sed -i.bak \
    -e 's/git checkout -b \([a-zA-Z0-9/_-]*\)/git worktree add -b \1 ..\/\1/g' \
    -e 's/git checkout -b/git worktree add -b/g' \
    -e 's/# Create and switch to feature branch/# Create worktree for feature branch/g' \
    -e 's/# Create and switch to/# Create worktree for/g' \
    -e 's/# If you see "main", IMMEDIATELY run:/# If you see "main", create a worktree:/g' \
    -e 's/checkout a different branch changes all terminal/worktree prevents cross-terminal branch conflicts/g' \
    {} \;

# Clean up backup files
find .claude/ -name "*.md.bak" -delete

echo ""
echo "✅ Migration complete!"
echo ""
echo "📝 Key changes made:"
echo "  - 'git checkout -b branch' → 'git worktree add -b branch ../branch'"
echo "  - Each branch now gets its own directory"
echo "  - No more terminal session conflicts"
echo ""
echo "⚠️  Remember: Agents should now use worktrees, not checkouts!"