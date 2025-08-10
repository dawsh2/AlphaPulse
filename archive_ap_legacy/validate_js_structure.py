#!/usr/bin/env python3
"""
Validate and fix JavaScript structure
"""

# Read the file
with open('/Users/daws/alphapulse/ui/research.html', 'r') as f:
    content = f.read()

# Find all script sections
import re

# Find the main script section
script_match = re.search(r'<script>(.*?)</script>', content, re.DOTALL)

if script_match:
    script_content = script_match.group(1)
    
    # Count braces to check for balance
    open_braces = script_content.count('{')
    close_braces = script_content.count('}')
    open_parens = script_content.count('(')
    close_parens = script_content.count(')')
    
    print(f"JavaScript structure check:")
    print(f"  Open braces {{: {open_braces}")
    print(f"  Close braces }}: {close_braces}")
    print(f"  Open parens (: {open_parens}")
    print(f"  Close parens ): {close_parens}")
    
    if open_braces != close_braces:
        print(f"❌ Brace mismatch! Difference: {open_braces - close_braces}")
    else:
        print("✅ Braces are balanced")
        
    if open_parens != close_parens:
        print(f"❌ Parenthesis mismatch! Difference: {open_parens - close_parens}")
    else:
        print("✅ Parentheses are balanced")

# Check if key functions exist
key_functions = ['switchTab', 'loadStrategyCatalogue', 'filterStrategies', 'toggleCategory']
print("\nChecking for key functions:")
for func in key_functions:
    pattern = f'function {func}'
    if pattern in content:
        print(f"✅ {func} function found")
        # Find the line number
        lines = content.split('\n')
        for i, line in enumerate(lines):
            if pattern in line:
                print(f"   at line {i+1}")
                break
    else:
        print(f"❌ {func} function NOT found")

# Look for any remaining syntax issues
print("\nChecking for common syntax issues:")

# Check for orphaned catch blocks
orphaned_catch = re.findall(r'^\s*\} catch', content, re.MULTILINE)
if orphaned_catch:
    print(f"❌ Found {len(orphaned_catch)} potential orphaned catch blocks")
else:
    print("✅ No orphaned catch blocks found")

# Check for double semicolons
double_semi = content.count(';;')
if double_semi:
    print(f"⚠️  Found {double_semi} double semicolons")

# Check for stray closing braces/parens at line start
stray_closers = re.findall(r'^\s*[})];\s*$', content, re.MULTILINE)
if stray_closers:
    print(f"⚠️  Found {len(stray_closers)} potential stray closing braces/parens")

print("\n📋 Summary:")
if open_braces == close_braces and open_parens == close_parens and not orphaned_catch:
    print("✅ JavaScript structure appears to be valid")
else:
    print("❌ JavaScript structure has issues that need fixing")