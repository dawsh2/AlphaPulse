#!/usr/bin/env python3
"""
Remove the second duplicate builder-container
"""

# Read the file
with open('/Users/daws/alphapulse/ui/explore.html', 'r') as f:
    lines = f.readlines()

# Find the second builder-container (around line 4240)
duplicate_start = -1
for i, line in enumerate(lines):
    if i > 4200 and 'class="builder-container"' in line:
        duplicate_start = i - 1  # Include the opening div
        break

if duplicate_start != -1:
    # Find where this duplicate ends (look for </div> followed by </body>)
    duplicate_end = -1
    div_count = 0
    for i in range(duplicate_start, len(lines)):
        div_count += lines[i].count('<div')
        div_count -= lines[i].count('</div>')
        if div_count == 0 and i > duplicate_start:
            duplicate_end = i
            break
    
    if duplicate_end != -1:
        print(f"Found duplicate builder container from line {duplicate_start+1} to {duplicate_end+1}")
        # Remove these lines
        lines = lines[:duplicate_start] + lines[duplicate_end+1:]
        
        # Write back
        with open('/Users/daws/alphapulse/ui/explore.html', 'w') as f:
            f.writelines(lines)
        
        print("✅ Removed duplicate builder container")
    else:
        print("❌ Could not find end of duplicate")
else:
    print("❌ Could not find duplicate builder container")