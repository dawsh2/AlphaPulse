#!/bin/bash
# AlphaPulse Frontend Cleanup Script
# Removes unneeded files and temporary artifacts

echo "🧹 Starting AlphaPulse Frontend cleanup..."

cd frontend || { echo "❌ Frontend directory not found"; exit 1; }

# Backup files (manual backups)
echo "🗑️  Removing backup files..."
if [ -f "src/pages/DevelopPage.tsx.backup" ]; then
    rm src/pages/DevelopPage.tsx.backup
    echo "   ✅ Removed DevelopPage.tsx.backup (88KB)"
fi

if [ -f "src/pages/ResearchPage.tsx.backup" ]; then
    rm src/pages/ResearchPage.tsx.backup
    echo "   ✅ Removed ResearchPage.tsx.backup"
fi

if [ -f "src/components/MonitorPage/MonitorPage.tsx.backup" ]; then
    rm src/components/MonitorPage/MonitorPage.tsx.backup
    echo "   ✅ Removed MonitorPage.tsx.backup"
fi

# Orphaned CSS files
echo "🗑️  Removing orphaned CSS files..."
if [ -f "src/components/features/Monitor/TrueRealtimeChart.module.css" ]; then
    rm src/components/features/Monitor/TrueRealtimeChart.module.css
    echo "   ✅ Removed TrueRealtimeChart.module.css (component was replaced)"
fi

# Editor temporary files
echo "🗑️  Removing editor temporary files..."
find docs/ -name "#*#" -delete 2>/dev/null || true
find docs/ -name ".#*" -delete 2>/dev/null || true
echo "   ✅ Removed Emacs temporary files"

# Development log
if [ -f "frontend.log" ]; then
    rm frontend.log
    echo "   ✅ Removed frontend.log (13KB)"
fi

echo ""
echo "🤔 MANUAL REVIEW NEEDED:"
echo "   📁 src/components/MonitorPage/ - Check if redundant with features/Monitor/"
echo "   📄 Multiple documentation files - Consider consolidating:"
echo "      - REFACTOR.md (9KB)"
echo "      - ui-state.md (13KB)" 
echo "      - api-todo.md (6KB)"
echo "      - docs/ directory"
echo ""
echo "   🏗️  Architecture consideration:"
echo "      - Dual dashboard structure (main app + developer dashboard)"
echo "      - Consider consolidating or clearly documenting purpose"
echo ""
echo "✅ Frontend cleanup complete!"
echo "   Space saved: ~100KB + temporary files"
