#!/usr/bin/env python3
"""
Remove duplicate builder form content
"""

# Read the file
with open('/Users/daws/alphapulse/ui/explore.html', 'r') as f:
    lines = f.readlines()

# Find and remove the duplicate builder form section
# Look for the Builder Form comment and remove everything until the closing divs
in_duplicate_section = False
duplicate_start = -1
duplicate_end = -1
div_count = 0

for i, line in enumerate(lines):
    if '<!-- Builder Form -->' in line and duplicate_start == -1 and i > 4000:  # Only after line 4000
        duplicate_start = i
        in_duplicate_section = True
        div_count = 0
    elif in_duplicate_section:
        div_count += line.count('<div')
        div_count -= line.count('</div>')
        if div_count < 0 or (div_count == 0 and '</div>' in line):
            duplicate_end = i
            break

if duplicate_start != -1 and duplicate_end != -1:
    print(f"Found duplicate builder form from line {duplicate_start+1} to {duplicate_end+1}")
    # Remove these lines
    lines = lines[:duplicate_start] + lines[duplicate_end+1:]
    
    # Write back
    with open('/Users/daws/alphapulse/ui/explore.html', 'w') as f:
        f.writelines(lines)
    
    print("✅ Removed duplicate builder form content")
else:
    print("❌ Could not find duplicate builder form")