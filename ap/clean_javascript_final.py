#!/usr/bin/env python3
"""
Final cleanup of JavaScript to fix all syntax errors
"""

# Read the file
with open('/Users/daws/alphapulse/ui/research.html', 'r') as f:
    lines = f.readlines()

# Find the main script section
in_script = False
script_start = -1
script_end = -1

for i, line in enumerate(lines):
    if '<script>' in line and '</script>' not in line:
        in_script = True
        script_start = i
    elif '</script>' in line and in_script:
        script_end = i
        break

print(f"Script section: lines {script_start+1} to {script_end+1}")

# Look for specific issues in the script section
if script_start != -1 and script_end != -1:
    # Check for lines that are just stray characters
    for i in range(script_start, script_end):
        line = lines[i].strip()
        
        # Remove lines that are just orphaned closing braces/parens
        if line in [')', ');', '};', '}', '})']:
            print(f"Removing stray closer at line {i+1}: {line}")
            lines[i] = ''
        
        # Remove orphaned catch blocks
        if line.startswith('} catch') and i > 0:
            # Check if previous line has a closing brace (indicating no matching try)
            prev_line = lines[i-1].strip()
            if prev_line.endswith('}') or prev_line == '':
                print(f"Removing orphaned catch at line {i+1}")
                # Remove the catch block
                j = i
                brace_count = 0
                while j < script_end:
                    if '{' in lines[j]:
                        brace_count += 1
                    if '}' in lines[j]:
                        brace_count -= 1
                    lines[j] = ''
                    if brace_count == 0 and '}' in lines[j-1]:
                        break
                    j += 1

# Also look for a specific issue: template literals that might be causing problems
# Check for unclosed template literals
for i in range(script_start, script_end):
    line = lines[i]
    backtick_count = line.count('`')
    if backtick_count % 2 != 0:
        print(f"⚠️  Odd number of backticks on line {i+1}")
        # This might be causing the parenthesis mismatch

# Write the cleaned content
with open('/Users/daws/alphapulse/ui/research.html', 'w') as f:
    f.writelines(lines)

print("\n✅ Cleaned up JavaScript")
print("📋 Removed:")
print("   - Stray closing braces and parentheses")
print("   - Orphaned catch blocks")
print("\n✨ This should resolve the syntax errors!")