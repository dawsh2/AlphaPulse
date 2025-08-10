#!/usr/bin/env python3
"""
Fix explore.html to properly link to shared.css
"""

# Read the file
with open('/Users/daws/alphapulse/ui/explore.html', 'r') as f:
    content = f.read()

# Remove all the CSS variables we manually added - they're in shared.css
import re

# Remove the entire CSS variables section we added
css_pattern = r':root\s*{[^}]+}.*?\.btn-primary:hover\s*{[^}]+}\s*'
content = re.sub(css_pattern, '', content, flags=re.DOTALL)

# Add the proper shared.css link
shared_css_link = '''    <!-- Shared styles with Full Stack Open design -->
    <link rel="stylesheet" href="shared.css">
    
    <!-- Page-specific styles -->'''

# Replace the empty shared styles comment
content = content.replace('    <!-- Shared styles -->\n    \n    ', shared_css_link)

# Write the updated content
with open('/Users/daws/alphapulse/ui/explore.html', 'w') as f:
    f.write(content)

print("✅ Added proper link to shared.css")
print("✅ Removed duplicate CSS variables")
print("✨ Explore page should now use the same theme as other pages!")