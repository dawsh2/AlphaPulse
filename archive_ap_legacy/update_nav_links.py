#!/usr/bin/env python3
"""
Update navigation links in all pages to include Explore
"""

import os
import re

# List of HTML files to update
html_files = [
    '/Users/daws/alphapulse/ui/index.html',
    '/Users/daws/alphapulse/ui/live-trading.html',
    '/Users/daws/alphapulse/ui/research.html',
    '/Users/daws/alphapulse/ui/develop.html',
    '/Users/daws/alphapulse/ui/deploy.html',
    '/Users/daws/alphapulse/ui/replay.html'
]

# Pattern to find the nav-links section
nav_pattern = r'(<div class="nav-links">)(.*?)(</div>)'

for file_path in html_files:
    if os.path.exists(file_path):
        with open(file_path, 'r') as f:
            content = f.read()
        
        # Check if Explore link already exists
        if 'href="explore.html"' not in content:
            # Find the nav-links section
            match = re.search(nav_pattern, content, re.DOTALL)
            if match:
                nav_start = match.group(1)
                nav_content = match.group(2)
                nav_end = match.group(3)
                
                # Find where to insert (after Research link)
                if 'href="research.html"' in nav_content:
                    # Split at Research link and insert Explore after it
                    parts = nav_content.split('</a>')
                    new_parts = []
                    for i, part in enumerate(parts):
                        if 'href="research.html"' in part:
                            new_parts.append(part + '</a>')
                            new_parts.append('                <a href="explore.html" class="nav-link">Explore</a>')
                        elif part.strip():  # Only add non-empty parts
                            new_parts.append(part + '</a>')
                    
                    # Reconstruct nav section
                    new_nav = nav_start + '\n'.join(new_parts) + '\n            ' + nav_end
                    
                    # Replace in content
                    content = content.replace(match.group(0), new_nav)
                    
                    # Write back
                    with open(file_path, 'w') as f:
                        f.write(content)
                    
                    print(f"✅ Updated {os.path.basename(file_path)}")
                else:
                    print(f"⚠️  Could not find Research link in {os.path.basename(file_path)}")
        else:
            print(f"ℹ️  Explore link already exists in {os.path.basename(file_path)}")
    else:
        print(f"❌ File not found: {file_path}")

print("\n✨ Navigation links updated!")