#!/bin/bash

# AlphaPulse Backend Runner

echo "🚀 Starting AlphaPulse Backend..."

# Check for Python
if ! command -v python3 &> /dev/null; then
    echo "❌ Python 3 is not installed"
    exit 1
fi

# Check for required environment variables
if [ -z "$ALPACA_API_KEY" ] || [ -z "$ALPACA_API_SECRET" ]; then
    echo "⚠️  Warning: Alpaca API keys not set in environment"
    echo "   Add to ~/.zshrc or ~/.bashrc:"
    echo "   export ALPACA_API_KEY='your_key'"
    echo "   export ALPACA_API_SECRET='your_secret'"
fi

# Install dependencies if needed
if [ ! -d "venv" ]; then
    echo "📦 Creating virtual environment..."
    python3 -m venv venv
    source venv/bin/activate
    pip install -r requirements.txt
else
    source venv/bin/activate
fi

# Run the server
echo "✅ Starting server on http://localhost:5000"
python app.py