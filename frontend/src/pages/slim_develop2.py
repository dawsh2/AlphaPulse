#!/usr/bin/env python3
"""
Script to slim down DevelopPage.tsx by replacing the massive loadFiles function
"""

# Read the file
with open('DevelopPage.tsx', 'r') as f:
    lines = f.readlines()

# Build the new file
new_lines = []

# Add lines before loadFiles (0-228)
new_lines.extend(lines[:229])

# Add the slim loadFiles replacement
new_lines.append('  const loadFiles = async () => {\n')
new_lines.append('    const fileStructure = await loadFileStructure();\n')
new_lines.append('    setFiles(fileStructure);\n')
new_lines.append('  };\n')
new_lines.append('\n')

# Add lines after loadFiles (551 onwards)
new_lines.extend(lines[551:])

# Write the new file
with open('DevelopPage.tsx', 'w') as f:
    f.writelines(new_lines)

print(f"Original lines: {len(lines)}")
print(f"New lines: {len(new_lines)}")
print(f"Lines saved: {len(lines) - len(new_lines)}")