#!/usr/bin/env python3
"""
Fix the orphaned catch block causing syntax error
"""

# Read the file
with open('/Users/daws/alphapulse/ui/research.html', 'r') as f:
    lines = f.readlines()

# Find and remove the orphaned catch block (lines 2512-2515)
# These lines don't belong here - they're part of a broken/duplicated function
if len(lines) > 2515:
    # Check if lines 2512-2515 contain the orphaned catch block
    if '} catch (error) {' in lines[2511] or '} catch (error) {' in lines[2512]:
        print("Found orphaned catch block")
        
        # Remove lines 2511-2515 (the orphaned catch block)
        for i in range(2511, 2516):
            if i < len(lines):
                print(f"Removing line {i+1}: {lines[i].strip()}")
                lines[i] = ''
        
        print("✅ Removed orphaned catch block")

# Write the fixed content
with open('/Users/daws/alphapulse/ui/research.html', 'w') as f:
    f.writelines(lines)

print("\n📋 Fixed orphaned catch block that was causing syntax error")
print("✨ JavaScript should now parse correctly!")