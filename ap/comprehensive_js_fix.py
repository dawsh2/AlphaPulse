#!/usr/bin/env python3
"""
Comprehensive JavaScript fix for research.html
"""

# Read the file
with open('/Users/daws/alphapulse/ui/research.html', 'r') as f:
    content = f.read()

# Fix 1: Replace all HTML entities in JavaScript
content = content.replace('&apos;', "'")
content = content.replace('&quot;', '"')
content = content.replace('&amp;', '&')

# Fix 2: Find and fix unmatched braces
# This is tricky, but let's check for common patterns

# Fix 3: Ensure all event handlers use proper quotes
import re

# Fix onchange handlers
content = re.sub(r'onchange="filterStrategies\(&apos;(\w+)&apos;, this\.value\)"', 
                 r'onchange="filterStrategies(\'\1\', this.value)"', content)

# Fix onclick handlers
content = re.sub(r'onclick="switchTab\(&apos;(\w+)&apos;, this\)"',
                 r'onclick="switchTab(\'\1\', this)"', content)

# Write the fixed content
with open('/Users/daws/alphapulse/ui/research.html', 'w') as f:
    f.write(content)

print("✅ Fixed HTML entities in JavaScript")
print("✅ Fixed event handler quotes")
print("✨ Basic fixes applied!")

# Now let's validate the structure
lines = content.split('\n')
print(f"\nTotal lines: {len(lines)}")

# Find script sections
in_script = False
script_lines = []
for i, line in enumerate(lines):
    if '<script>' in line:
        in_script = True
    elif '</script>' in line:
        in_script = False
    elif in_script:
        script_lines.append((i+1, line))

# Count braces in script sections
open_braces = 0
close_braces = 0
for line_num, line in script_lines:
    open_braces += line.count('{')
    close_braces += line.count('}')

print(f"\nJavaScript brace count:")
print(f"  Open braces: {open_braces}")
print(f"  Close braces: {close_braces}")
print(f"  Difference: {open_braces - close_braces}")