#!/usr/bin/env python3
"""
Fix explore.html - remove emojis, fix styling, fix hover colors
"""

# Read the file
with open('/Users/daws/alphapulse/ui/explore.html', 'r') as f:
    content = f.read()

# 1. Remove all emoji icons
import re
emoji_pattern = r'<div class="strategy-icon">.*?</div>'
content = re.sub(emoji_pattern, '', content, flags=re.DOTALL)

# 2. Fix the shared.css path - it should be looking for it in the same directory
content = content.replace('<link rel="stylesheet" href="shared.css">', '')

# 3. Copy the CSS variables and base styles from research.html
# Read research.html to get the proper styles
with open('/Users/daws/alphapulse/ui/research.html', 'r') as f:
    research_content = f.read()

# Extract the CSS variables and base styles
style_match = re.search(r'<style>\s*:root\s*{.*?}.*?body\s*{.*?}', research_content, re.DOTALL)
if style_match:
    base_styles = style_match.group(0)
    # Insert these base styles into explore.html
    content = content.replace('<style>', '<style>\n' + base_styles + '\n')

# 4. Fix hover colors to use theme colors instead of hardcoded black
hover_fixes = {
    # Light background colors should keep their color on hover
    '.color-blue:hover {': '''.color-blue:hover {
            background: var(--color-text-primary);
            color: #89CDF1;
            transform: translate(2px, -2px);
            box-shadow: -6px 8px var(--color-text-primary);''',
    
    '.color-orange:hover {': '''.color-orange:hover {
            background: var(--color-text-primary);
            color: #FF9500;
            transform: translate(2px, -2px);
            box-shadow: -6px 8px var(--color-text-primary);''',
    
    '.color-green:hover {': '''.color-green:hover {
            background: var(--color-text-primary);
            color: #4CAF50;
            transform: translate(2px, -2px);
            box-shadow: -6px 8px var(--color-text-primary);''',
    
    '.color-cyan:hover {': '''.color-cyan:hover {
            background: var(--color-text-primary);
            color: #00BCD4;
            transform: translate(2px, -2px);
            box-shadow: -6px 8px var(--color-text-primary);''',
    
    # Dark background colors should go to white on hover
    '.color-purple:hover {': '''.color-purple:hover {
            background: var(--color-bg-primary);
            color: #9C27B0;
            transform: translate(2px, -2px);
            box-shadow: -6px 8px var(--color-text-primary);''',
    
    '.color-red:hover {': '''.color-red:hover {
            background: var(--color-bg-primary);
            color: #F44336;
            transform: translate(2px, -2px);
            box-shadow: -6px 8px var(--color-text-primary);''',
    
    '.color-teal:hover {': '''.color-teal:hover {
            background: var(--color-bg-primary);
            color: #009688;
            transform: translate(2px, -2px);
            box-shadow: -6px 8px var(--color-text-primary);''',
    
    '.color-indigo:hover {': '''.color-indigo:hover {
            background: var(--color-bg-primary);
            color: #3F51B5;
            transform: translate(2px, -2px);
            box-shadow: -6px 8px var(--color-text-primary);''',
    
    '.color-pink:hover {': '''.color-pink:hover {
            background: var(--color-bg-primary);
            color: #E91E63;
            transform: translate(2px, -2px);
            box-shadow: -6px 8px var(--color-text-primary);'''
}

for old, new in hover_fixes.items():
    pattern = old + r'[^}]*}'
    replacement = new + '\n        }'
    content = re.sub(pattern, replacement, content)

# 5. Update card padding since we removed icons
content = content.replace('padding: var(--space-6);', 'padding: var(--space-4);')

# 6. Fix the header to match other pages
header_html = '''    <!-- Header -->
    <header class="header">
        <nav class="nav-container">
            <a href="index.html" class="logo-link">
                <span class="logo-text">AlphaPulse</span>
            </a>
            <div class="nav-links">
                <a href="live-trading.html" class="nav-link">Live Trading</a>
                <a href="research.html" class="nav-link">Research</a>
                <a href="explore.html" class="nav-link active">Explore</a>
                <a href="develop.html" class="nav-link">Develop</a>
                <a href="deploy.html" class="nav-link">Deploy</a>
                <a href="replay.html" class="nav-link">Replay</a>
            </div>
        </nav>
    </header>'''

# Replace the header
header_pattern = r'<!-- Header -->.*?</header>'
content = re.sub(header_pattern, header_html, content, flags=re.DOTALL)

# 7. Remove logo-icon references
content = content.replace('<span class="logo-icon">📊</span>\n                ', '')

# 8. Also remove the notebook icon
content = content.replace('<span class="notebook-icon">📓</span>\n                    ', '')

# Write the updated content
with open('/Users/daws/alphapulse/ui/explore.html', 'w') as f:
    f.write(content)

print("✅ Removed all emojis")
print("✅ Fixed hover colors to match theme")
print("✅ Connected proper styling")
print("✅ Fixed header to match other pages")
print("\n✨ Explore page should now look consistent with the rest of the site!")