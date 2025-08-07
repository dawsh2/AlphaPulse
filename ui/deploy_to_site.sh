#!/bin/bash

# Script to deploy ui to alphapulse-site and push to GitHub
# This script preserves the CNAME file

set -e  # Exit on any error

echo "🚀 Deploying AlphaPulse Web to Site..."

# Check if source directory exists
if [ ! -d "/Users/daws/alphapulse/ui" ]; then
    echo "❌ Error: Source directory /Users/daws/alphapulse/ui not found"
    exit 1
fi

# Check if target directory exists
if [ ! -d "/Users/daws/alphapulse-site" ]; then
    echo "❌ Error: Target directory /Users/daws/alphapulse-site not found"
    exit 1
fi

# Save CNAME file if it exists
CNAME_BACKUP=""
if [ -f "/Users/daws/alphapulse-site/CNAME" ]; then
    echo "💾 Backing up CNAME file..."
    CNAME_BACKUP=$(cat /Users/daws/alphapulse-site/CNAME)
    echo "   CNAME content: $CNAME_BACKUP"
fi

# Navigate to target directory
cd /Users/daws/alphapulse-site

# Check if it's a git repository
if [ ! -d ".git" ]; then
    echo "❌ Error: /Users/daws/alphapulse-site is not a git repository"
    exit 1
fi

# Get git status
echo "📊 Current git status:"
git status --porcelain

# Remove all files except .git and CNAME
echo "🧹 Cleaning target directory (preserving .git and CNAME)..."
find . -mindepth 1 -maxdepth 1 ! -name '.git' ! -name 'CNAME' -exec rm -rf {} \;

# Copy new files from source
echo "📂 Copying new files from ui..."
cp -r /Users/daws/alphapulse/ui/* .

# Restore CNAME file if it was backed up
if [ ! -z "$CNAME_BACKUP" ]; then
    echo "🔄 Restoring CNAME file..."
    echo "$CNAME_BACKUP" > CNAME
    echo "   CNAME restored with content: $CNAME_BACKUP"
fi

# Add all changes to git
echo "📝 Adding changes to git..."
git add .

# Check if there are any changes to commit
if git diff --staged --quiet; then
    echo "✅ No changes to commit"
else
    # Show what's being committed
    echo "📋 Changes to be committed:"
    git diff --staged --name-status
    
    # Commit changes
    echo "💾 Committing changes..."
    git commit -m "Update AlphaPulse web interface

🚀 Generated with Claude Code

- Updated Develop tab with Monaco Editor integration
- Enhanced Replay tab with strategy selection interface  
- Improved Research tab with ADMF-PC principles
- Added Generate Signals functionality
- Fixed file loading and Monaco Editor initialization
- Enhanced UI with better error handling and user feedback

Co-Authored-By: Claude <noreply@anthropic.com>"

    # Push to remote
    echo "🌐 Pushing to remote repository..."
    git push

    echo "✅ Successfully deployed and pushed to GitHub!"
    echo "🎉 AlphaPulse site has been updated!"
fi

echo ""
echo "🔗 Site should be available at your GitHub Pages URL"
echo "📁 Local site directory: /Users/daws/alphapulse-site"