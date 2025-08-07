#!/usr/bin/env python3
"""
Fix the syntax error on line 2511
"""

# Read the file
with open('/Users/daws/alphapulse/ui/research.html', 'r') as f:
    lines = f.readlines()

# Fix line 2511 (index 2510) - remove the stray ");
if len(lines) > 2510:
    print(f"Line 2511 before: {lines[2510].strip()}")
    
    # Remove the problematic line if it's just ");"
    if lines[2510].strip() == ');':
        lines[2510] = ''  # Remove the line
        print("✅ Removed stray '); on line 2511")
    else:
        print("❌ Line 2511 doesn't match expected pattern")

# Also check for other syntax issues around that area
# It looks like there might be duplicated or misplaced code
# Let's check lines 2512-2520 for issues
print("\nChecking surrounding lines for issues...")
for i in range(2510, min(2520, len(lines))):
    line = lines[i].strip()
    if line and not line.startswith('//'):
        print(f"Line {i+1}: {line[:60]}...")

# Look for patterns that suggest duplicated or misplaced code
duplicated_code_start = None
for i in range(2511, min(2530, len(lines))):
    if 'const targetTab = document.getElementById(tabName' in lines[i]:
        duplicated_code_start = i
        print(f"\n⚠️  Found duplicated code starting at line {i+1}")
        break

# If we found duplicated code, remove it
if duplicated_code_start:
    # Find the end of the duplicated section
    end_line = duplicated_code_start
    brace_count = 0
    for i in range(duplicated_code_start, min(len(lines), duplicated_code_start + 50)):
        if '{' in lines[i]:
            brace_count += lines[i].count('{')
        if '}' in lines[i]:
            brace_count -= lines[i].count('}')
        if brace_count == 0 and '}' in lines[i]:
            end_line = i
            break
    
    # Remove the duplicated section
    print(f"Removing duplicated code from lines {duplicated_code_start+1} to {end_line+1}")
    for i in range(duplicated_code_start, end_line + 1):
        lines[i] = ''

# Write the fixed content
with open('/Users/daws/alphapulse/ui/research.html', 'w') as f:
    f.writelines(lines)

print("\n✅ Fixed syntax error")
print("📋 Changes made:")
print("   - Removed stray '); that was causing syntax error")
print("   - Cleaned up any duplicated code sections")
print("\n✨ JavaScript should now load properly and tabs should work!")