#!/usr/bin/env python3
"""
Remove the sidebar from explore.html
"""

# Read the file
with open('/Users/daws/alphapulse/ui/explore.html', 'r') as f:
    content = f.read()

# Update the title
content = content.replace('<title>Research - AlphaPulse</title>', '<title>Explore - AlphaPulse</title>')

# Remove the entire snippets sidebar section
import re

# Find and remove the snippets sidebar
sidebar_pattern = r'<!-- Snippets Sidebar -->.*?</aside>'
content = re.sub(sidebar_pattern, '', content, flags=re.DOTALL)

# Remove the research-container flex layout since we don't need it without sidebar
content = content.replace('class="research-container"', 'class="explore-container"')

# Update CSS to remove flex layout
content = content.replace(
    '.research-container {\n            display: flex;\n            height: calc(100vh - var(--header-height));\n            overflow: hidden;\n        }',
    '.explore-container {\n            min-height: calc(100vh - var(--header-height));\n            padding: var(--space-4);\n        }'
)

# Write the updated content
with open('/Users/daws/alphapulse/ui/explore.html', 'w') as f:
    f.write(content)

print("✅ Removed sidebar from explore.html")
print("✅ Updated title and container class")
print("✅ Adjusted layout for full-width content")