#!/bin/bash

# Build script for AlphaPulse UI

echo "🚀 Building AlphaPulse UI..."

# Build the React app
npm run build

echo "✅ Build complete!"
echo "📁 Build output in: dist/"
echo ""
echo "To deploy, run:"
echo "  cp -r dist/* ../ui/"
echo "  cd ../ui && ./deploy_to_site.sh"