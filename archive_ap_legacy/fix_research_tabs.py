#!/usr/bin/env python3
"""
Fix the broken tab functionality in research.html
"""

import re

# Read the file
with open('/Users/daws/alphapulse/ui/research.html', 'r') as f:
    content = f.read()

# Find the broken switchTab function and replace it
# The function appears to be incomplete/broken
broken_pattern = r'function switchTab\(tabName, element\) \{[^}]*?content\.classList\.remove\(\'active\'\);\s*\}(?!\s*\})'

# Complete working switchTab function
working_function = '''function switchTab(tabName, element) {
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
                    
                    // Special handling for catalogue tab
                    if (tabName === 'catalogue' && typeof loadStrategyCatalogue === 'function') {
                        loadStrategyCatalogue();
                    }
                } else {
                    console.error('Tab not found:', tabName + 'Tab');
                }
            } catch (error) {
                console.error('Error switching tab:', error);
            }
        }'''

# Try to find and replace the broken function
if 'function switchTab' in content:
    # Find the start of the function
    start_idx = content.find('function switchTab')
    if start_idx != -1:
        # Find the next function or major section after it
        # Look for common patterns that would indicate the end of this function
        end_markers = [
            '\n        function ',  # Next function at same indentation
            '\n\n        // Strategy Catalogue Functions',  # Comment section
            '\n    </script>',  # End of script
            '\n        window.',  # Window function assignment
        ]
        
        end_idx = -1
        for marker in end_markers:
            idx = content.find(marker, start_idx)
            if idx != -1 and (end_idx == -1 or idx < end_idx):
                end_idx = idx
        
        if end_idx != -1:
            # Replace the broken function
            content = content[:start_idx] + working_function + content[end_idx:]
            print("✅ Replaced broken switchTab function")
        else:
            print("❌ Could not find end of broken function")
    else:
        print("❌ Could not find start of switchTab function")
else:
    print("❌ switchTab function not found in file")

# Write the fixed content
with open('/Users/daws/alphapulse/ui/research.html', 'w') as f:
    f.write(content)

print("\n📋 Tab switching should now work for all tabs!")
print("   - Snippets, Templates, Notebooks, and Catalogue tabs should all be clickable")
print("   - Check browser console for any remaining errors")