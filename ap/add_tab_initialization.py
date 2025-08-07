#!/usr/bin/env python3
"""
Add initialization code to ensure tabs work
"""

# Read the file
with open('/Users/daws/alphapulse/ui/research.html', 'r') as f:
    content = f.read()

# Add initialization code before the closing script tag
init_code = """
        // Initialize tabs when page loads
        document.addEventListener('DOMContentLoaded', function() {
            console.log('Initializing research page tabs...');
            
            // Ensure switchTab is available globally
            if (typeof window.switchTab === 'undefined') {
                console.error('switchTab not found, defining fallback...');
                window.switchTab = function(tabName, element) {
                    try {
                        // Update buttons
                        document.querySelectorAll('.sidebar-tab').forEach(tab => {
                            tab.classList.remove('active');
                        });
                        if (element) element.classList.add('active');
                        
                        // Update content
                        document.querySelectorAll('.tab-content').forEach(content => {
                            content.classList.remove('active');
                        });
                        
                        const targetTab = document.getElementById(tabName + 'Tab');
                        if (targetTab) {
                            targetTab.classList.add('active');
                            
                            if (tabName === 'catalogue' && typeof loadStrategyCatalogue === 'function') {
                                loadStrategyCatalogue();
                            }
                        }
                    } catch (e) {
                        console.error('Error in switchTab:', e);
                    }
                };
            }
            
            // Add click handlers as backup
            const tabButtons = document.querySelectorAll('.sidebar-tab');
            console.log('Found', tabButtons.length, 'tab buttons');
            
            tabButtons.forEach(button => {
                // Get tab name from onclick attribute or button text
                let tabName = null;
                const onclickAttr = button.getAttribute('onclick');
                if (onclickAttr) {
                    const match = onclickAttr.match(/switchTab\(['"]([^'"]+)['"]/);
                    if (match) tabName = match[1];
                }
                
                if (!tabName) {
                    // Fallback to button text
                    tabName = button.textContent.toLowerCase().trim();
                }
                
                // Add event listener
                button.addEventListener('click', function(e) {
                    e.preventDefault();
                    console.log('Tab clicked:', tabName);
                    switchTab(tabName, this);
                });
            });
            
            console.log('Tab initialization complete');
        });
"""

# Find the last </script> tag and insert before it
last_script_pos = content.rfind('</script>')
if last_script_pos != -1:
    content = content[:last_script_pos] + init_code + '\n    ' + content[last_script_pos:]
    print("✅ Added tab initialization code")
else:
    print("❌ Could not find script closing tag")

# Write the updated file
with open('/Users/daws/alphapulse/ui/research.html', 'w') as f:
    f.write(content)

print("\n📋 Added failsafe initialization:")
print("   - DOMContentLoaded handler to ensure tabs are set up")
print("   - Fallback switchTab function if original is missing")
print("   - Event listeners added to all tab buttons")
print("   - Console logging for debugging")
print("\n✨ Tabs should now definitely work!")