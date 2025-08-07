#!/usr/bin/env python3
"""
Final comprehensive fix for the Catalogue tab
"""

# Read the current file
with open('/Users/daws/alphapulse/ui/research.html', 'r') as f:
    lines = f.readlines()

# Find the broken switchTab function (around line 1917)
fixed = False
for i in range(len(lines)):
    if 'content.classList.remove(\'active\');' in lines[i] and i < len(lines) - 1:
        # Check if the next line has the closing bracket missing
        if '}' not in lines[i] and '// Strategy Catalogue Functions' in lines[i+1]:
            # Fix by adding the missing closing brackets and completion
            lines[i] = lines[i].rstrip() + '\n                });\n                \n                const targetTab = document.getElementById(tabName + \'Tab\');\n                if (targetTab) {\n                    targetTab.classList.add(\'active\');\n                    console.log(\'Tab switched successfully to:\', tabName);\n                    \n                    // Load catalogue content when switching to it\n                    if (tabName === \'catalogue\') {\n                        loadStrategyCatalogue();\n                    }\n                } else {\n                    console.error(\'Tab not found:\', tabName + \'Tab\');\n                }\n            } catch (error) {\n                console.error(\'Error switching tab:\', error);\n            }\n        }\n'
            fixed = True
            print(f"✅ Fixed broken function at line {i+1}")
            break

# Write the fixed content
with open('/Users/daws/alphapulse/ui/research.html', 'w') as f:
    f.writelines(lines)

if fixed:
    print("\n📋 Successfully fixed:")
    print("   - Completed the switchTab function")
    print("   - Added missing closing brackets")
    print("   - Added catalogue loading logic")
    print("   - The Catalogue tab should now be fully clickable and functional!")
else:
    print("❌ Could not find the exact location to fix. Manual intervention may be needed.")