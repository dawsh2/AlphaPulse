#!/usr/bin/env python3
"""
Ensure button styles are properly applied to strategy cards
"""

# Read the file
with open('/Users/daws/alphapulse/ui/research.html', 'r') as f:
    content = f.read()

# Add button reset styles for the strategy cards
button_styles = '''
        /* Reset button styles for strategy cards */
        .strategy-card {
            /* Reset button defaults */
            border: none;
            outline: none;
            background: none;
            font: inherit;
            color: inherit;
            cursor: pointer;
            
            /* Apply AlphaPulse button styles */
            font-family: var(--font-family-sans);
            background: var(--color-bg-primary);
            border: 3px solid var(--color-text-primary);
            border-radius: var(--radius-lg);
            box-shadow: -3px 5px var(--color-text-primary);
            transition: all var(--transition-fast);
            padding: var(--space-4);
            text-align: left;
            position: relative;
            margin-bottom: 5px;
            margin-right: 5px;
            min-height: 140px;
            display: flex;
            flex-direction: column;
            width: 100%;
        }
'''

# Find where to insert it (after the existing .strategy-card definition)
import re

# Look for the existing .strategy-card style
pattern = r'(\.strategy-card\s*{[^}]+})'
match = re.search(pattern, content, re.DOTALL)

if match:
    # Replace the existing definition with our enhanced one
    content = content.replace(match.group(1), button_styles)
    print("✅ Updated strategy-card button styles")
else:
    print("⚠️  Could not find existing .strategy-card style")

# Write the updated content
with open('/Users/daws/alphapulse/ui/research.html', 'w') as f:
    f.write(content)

print("✨ Strategy cards now have proper button styling")