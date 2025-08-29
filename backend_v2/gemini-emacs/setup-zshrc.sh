#!/bin/bash

# Add Gemini API key to ~/.zshrc

ZSHRC="$HOME/.zshrc"
API_KEY="AIzaSyBsQn-UkNl_v-qVhGjPWwrRX2cEqBqCgPg"

# Check if the export already exists
if grep -q "GEMINI_API_KEY" "$ZSHRC" 2>/dev/null; then
    echo "GEMINI_API_KEY already exists in ~/.zshrc"
    echo "Updating to new value..."
    # Use sed to update the existing line
    sed -i.backup "s/export GEMINI_API_KEY=.*/export GEMINI_API_KEY=\"$API_KEY\"/" "$ZSHRC"
    echo "✓ Updated GEMINI_API_KEY in ~/.zshrc"
else
    # Add the export to the end of the file
    echo "" >> "$ZSHRC"
    echo "# Gemini API Key for gemini-code.el" >> "$ZSHRC"
    echo "export GEMINI_API_KEY=\"$API_KEY\"" >> "$ZSHRC"
    echo "✓ Added GEMINI_API_KEY to ~/.zshrc"
fi

echo ""
echo "To apply the changes:"
echo "  1. Run: source ~/.zshrc"
echo "  2. Or open a new terminal"
echo ""
echo "Your API key has been set to: ${API_KEY:0:10}..."