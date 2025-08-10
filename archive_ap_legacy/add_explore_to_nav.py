#!/usr/bin/env python3
"""
Add Explore link to navigation on all pages
"""

import os
import re

# List of HTML files to update
html_files = [
    '/Users/daws/alphapulse/ui/index.html',
    '/Users/daws/alphapulse/ui/research.html',
    '/Users/daws/alphapulse/ui/develop.html',
    '/Users/daws/alphapulse/ui/deploy.html',
    '/Users/daws/alphapulse/ui/replay.html',
    '/Users/daws/alphapulse/ui/explore.html'
]

for file_path in html_files:
    if os.path.exists(file_path):
        with open(file_path, 'r') as f:
            content = f.read()
        
        # Check if Explore link already exists
        if 'href="explore.html"' not in content or file_path.endswith('explore.html'):
            # Find the Research link and add Explore after it
            research_pattern = r'(<a href="research.html" class="nav-link[^"]*">Research</a>)'
            
            if file_path.endswith('explore.html'):
                # For explore.html, update the active class
                content = re.sub(
                    r'<a href="research.html" class="nav-link[^"]*">Research</a>',
                    '<a href="research.html" class="nav-link">Research</a>\n                <a href="explore.html" class="nav-link active">Explore</a>',
                    content
                )
            else:
                # For other pages, just add the link
                replacement = r'\1\n                <a href="explore.html" class="nav-link">Explore</a>'
                content = re.sub(research_pattern, replacement, content)
            
            # Write back
            with open(file_path, 'w') as f:
                f.write(content)
            
            print(f"✅ Updated {os.path.basename(file_path)}")
        else:
            print(f"ℹ️  Explore link already exists in {os.path.basename(file_path)}")
    else:
        print(f"❌ File not found: {file_path}")

print("\n✨ Navigation links updated on all pages!")