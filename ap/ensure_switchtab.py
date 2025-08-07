#!/usr/bin/env python3
"""
Ensure switchTab function is defined early in the script
"""

# Read the file
with open('/Users/daws/alphapulse/ui/research.html', 'r') as f:
    content = f.read()

# Define a working switchTab function
switchtab_definition = '''
        // Define switchTab at the beginning to ensure it's available
        window.switchTab = function(tabName, element) {
            try {
                console.log('Switching to tab:', tabName);
                
                // Update tab buttons
                document.querySelectorAll('.sidebar-tab').forEach(tab => {
                    tab.classList.remove('active');
                });
                if (element) element.classList.add('active');
                
                // Update tab content
                document.querySelectorAll('.tab-content').forEach(content => {
                    content.classList.remove('active');
                });
                
                const targetTab = document.getElementById(tabName + 'Tab');
                if (targetTab) {
                    targetTab.classList.add('active');
                    console.log('Tab switched successfully to:', tabName);
                    
                    // Special handling for catalogue tab
                    if (tabName === 'catalogue') {
                        setTimeout(() => {
                            if (typeof loadStrategyCatalogue === 'function') {
                                loadStrategyCatalogue();
                            }
                        }, 100);
                    }
                } else {
                    console.error('Tab not found:', tabName + 'Tab');
                }
            } catch (error) {
                console.error('Error switching tab:', error);
            }
        };
'''

# Find the first <script> tag and add our function right after it
import re
script_pattern = r'<script>\s*'
match = re.search(script_pattern, content)

if match:
    insert_pos = match.end()
    content = content[:insert_pos] + switchtab_definition + '\n' + content[insert_pos:]
    print("✅ Added switchTab function at the beginning of script")
else:
    print("❌ Could not find script tag")

# Write the updated content
with open('/Users/daws/alphapulse/ui/research.html', 'w') as f:
    f.write(content)

print("\n📋 Changes made:")
print("   - Added switchTab function at the very beginning")
print("   - Made it a window property to ensure global access")
print("   - Added setTimeout for catalogue loading")
print("\n✨ switchTab should now be available when onclick handlers fire!")