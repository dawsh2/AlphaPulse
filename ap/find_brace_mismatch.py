#!/usr/bin/env python3
"""
Find where the brace mismatch is occurring
"""

# Read the file
with open('/Users/daws/alphapulse/ui/research.html', 'r') as f:
    lines = f.readlines()

# Find script sections and track braces
in_script = False
brace_count = 0
problem_areas = []

for i, line in enumerate(lines):
    if '<script>' in line:
        in_script = True
        continue
    elif '</script>' in line:
        in_script = False
        if brace_count != 0:
            print(f"⚠️  Script section ending at line {i+1} has brace count: {brace_count}")
        continue
    
    if in_script:
        open_count = line.count('{')
        close_count = line.count('}')
        
        if open_count > 0 or close_count > 0:
            prev_count = brace_count
            brace_count += open_count - close_count
            
            # Track significant changes
            if abs(open_count - close_count) > 2:
                problem_areas.append({
                    'line': i+1,
                    'content': line.strip()[:80],
                    'open': open_count,
                    'close': close_count,
                    'running_total': brace_count
                })

print(f"\nFinal brace count: {brace_count}")
print(f"\nPotential problem areas (lines with many braces):")
for area in problem_areas[-10:]:  # Show last 10
    print(f"  Line {area['line']}: +{area['open']} -{area['close']} = {area['running_total']} total")
    print(f"    {area['content']}...")

# Let's also check for functions that might not be closed properly
print("\n\nChecking for unclosed functions:")
in_script = False
current_function = None
function_stack = []

for i, line in enumerate(lines):
    if '<script>' in line:
        in_script = True
    elif '</script>' in line:
        in_script = False
    elif in_script:
        # Look for function definitions
        if 'function' in line and '{' in line:
            func_name = line.strip().split('function')[1].split('(')[0].strip()
            if func_name:
                function_stack.append((func_name, i+1))
        
        # Count braces for current line
        open_count = line.count('{')
        close_count = line.count('}')
        
        # If we have more closes than opens, we're closing functions
        if close_count > open_count and function_stack:
            for _ in range(close_count - open_count):
                if function_stack:
                    function_stack.pop()

if function_stack:
    print(f"\n❌ Potentially unclosed functions:")
    for func_name, line_num in function_stack:
        print(f"  Function '{func_name}' at line {line_num}")