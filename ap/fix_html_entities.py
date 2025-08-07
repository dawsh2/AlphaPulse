#!/usr/bin/env python3
"""
Fix HTML entities in JavaScript event handlers
"""

# Read the file
with open('/Users/daws/alphapulse/ui/research.html', 'r') as f:
    content = f.read()

# Replace problematic HTML entities in JavaScript contexts
replacements = [
    # Fix the filter select onchange handlers
    ('onchange="filterStrategies(&apos;type&apos;, this.value)"', 
     'onchange="filterStrategies(\'type\', this.value)"'),
    ('onchange="filterStrategies(&apos;complexity&apos;, this.value)"', 
     'onchange="filterStrategies(\'complexity\', this.value)"'),
]

for old, new in replacements:
    if old in content:
        content = content.replace(old, new)
        print(f"✅ Fixed: {old[:50]}...")
    else:
        print(f"❌ Not found: {old[:50]}...")

# Also check for any other &apos; in JavaScript contexts
import re
js_apos_pattern = r'(on\w+="[^"]*&apos;[^"]*")'
matches = re.findall(js_apos_pattern, content)
if matches:
    print(f"\n⚠️  Found {len(matches)} other instances of &apos; in event handlers:")
    for match in matches[:5]:  # Show first 5
        print(f"   {match}")

# Write the fixed content
with open('/Users/daws/alphapulse/ui/research.html', 'w') as f:
    f.write(content)

print("\n📋 Fixed HTML entities in JavaScript event handlers")
print("   This should resolve JavaScript errors preventing tabs from working")