#!/usr/bin/env python3
"""
Fix the incomplete switchTab function
"""

# Read the current file
with open('/Users/daws/alphapulse/ui/research.html', 'r') as f:
    content = f.read()

# Find the broken switchTab function and fix it
import re

# The function appears to be cut off. Let's find it and replace with a complete version
broken_pattern = r'function switchTab\(tabName, element\) \{[^}]*content\.classList\.remove\(\'active\'\);\s*\}\s*// Strategy Catalogue Functions'

fixed_function = '''function switchTab(tabName, element) {
            try {
                console.log('Switching to tab:', tabName);
                
                // Update tab buttons
                document.querySelectorAll('.sidebar-tab').forEach(tab => {
                    tab.classList.remove('active');
                });
                element.classList.add('active');
                
                // Update tab content
                document.querySelectorAll('.tab-content').forEach(content => {
                    content.classList.remove('active');
                });
                
                const targetTab = document.getElementById(tabName + 'Tab');
                if (targetTab) {
                    targetTab.classList.add('active');
                    console.log('Tab switched successfully to:', tabName);
                    
                    // Load catalogue content when switching to it
                    if (tabName === 'catalogue') {
                        loadStrategyCatalogue();
                    }
                } else {
                    console.error('Tab not found:', tabName + 'Tab');
                }
            } catch (error) {
                console.error('Error switching tab:', error);
            }
        }
        
        // Strategy Catalogue Functions'''

content = re.sub(broken_pattern, fixed_function, content, flags=re.DOTALL)

# Also ensure the tabs array includes catalogueTab
if "'catalogueTab']" not in content:
    content = content.replace(
        "const tabs = ['snippetsTab', 'templatesTab', 'notebooksTab'];",
        "const tabs = ['snippetsTab', 'templatesTab', 'notebooksTab', 'catalogueTab'];"
    )

# Write the updated content
with open('/Users/daws/alphapulse/ui/research.html', 'w') as f:
    f.write(content)

print("✅ Fixed switchTab function")
print("📋 Issues resolved:")
print("   - Completed the truncated switchTab function")
print("   - Added proper error handling")
print("   - Ensured catalogue loading when tab is clicked")
print("   - Tab should now be clickable and functional")