#!/usr/bin/env python3
"""
Apply the catalogue patch to research.html
"""

# Read the patch JavaScript
with open('/Users/daws/alphapulse/ap/research_catalogue_patch.js', 'r') as f:
    patch_js = f.read()

# Read the current HTML file
with open('/Users/daws/alphapulse/ui/research.html', 'r') as f:
    content = f.read()

# Find the last </script> tag before </body>
import re

# Pattern to find the last script closing tag before body
pattern = r'(</script>\s*)(</body>)'

# Insert our patch before the closing script tag
if '</script>' in content and '</body>' in content:
    # Find the last occurrence
    last_script_pos = content.rfind('</script>')
    
    if last_script_pos != -1:
        # Insert the patch code before the closing script tag
        insert_pos = last_script_pos
        new_content = (
            content[:insert_pos] + 
            '\n\n        // CATALOGUE TAB FUNCTIONALITY\n' +
            patch_js + 
            '\n\n    ' +
            content[insert_pos:]
        )
        
        # Write the updated content
        with open('/Users/daws/alphapulse/ui/research.html', 'w') as f:
            f.write(new_content)
        
        print("✅ Successfully patched research.html with Catalogue functionality")
        print("\n📋 What was added:")
        print("   - Fixed switchTab function")
        print("   - loadStrategyCatalogue function")
        print("   - createStrategyCard function")
        print("   - filterStrategies function")
        print("   - sortStrategies function")
        print("   - viewStrategy and backtest functions")
        print("   - 6 example strategies with full details")
        print("\n✨ The Catalogue tab should now be fully functional!")
    else:
        print("❌ Could not find script tag to insert patch")
else:
    print("❌ Could not find proper location to insert patch")