#!/usr/bin/env python3
"""
Script to slim down DevelopPage.tsx by replacing the massive openFile function
"""

# Read the file
with open('DevelopPage.tsx', 'r') as f:
    lines = f.readlines()

# Build the new file
new_lines = []

# Add lines before openFile (0-561)
new_lines.extend(lines[:562])

# Add the slim openFile replacement
new_lines.append('  const openFile = async (filePath: string, fileName: string) => {\n')
new_lines.append('    await generateFileContent(filePath, fileName, {\n')
new_lines.append('      tabs,\n')
new_lines.append('      setTabs,\n')
new_lines.append('      setActiveTab,\n')
new_lines.append('      setEditorHidden\n')
new_lines.append('    });\n')
new_lines.append('  };\n')
new_lines.append('\n')

# Add lines after openFile (1575 onwards)
new_lines.extend(lines[1575:])

# Also need to add the import at the top
# Find the line with imports
for i, line in enumerate(new_lines):
    if 'import CodeEditor from' in line:
        new_lines.insert(i+1, "import { generateFileContent } from '../services/fileContentGenerator';\n")
        break

# Write the new file
with open('DevelopPage.tsx', 'w') as f:
    f.writelines(new_lines)

print(f"Original lines: {len(lines)}")
print(f"New lines: {len(new_lines)}")
print(f"Lines saved: {len(lines) - len(new_lines)}")