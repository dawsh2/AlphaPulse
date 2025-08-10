#!/usr/bin/env python3
"""
Complete fix for Catalogue tab
"""

# Read the current file
with open('/Users/daws/alphapulse/ui/research.html', 'r') as f:
    content = f.read()

# The main issue is likely that the onchange handlers have nested quotes that break the HTML
# Let's replace the entire catalogue tab content with properly escaped version

# Find and replace the catalogue tab content
import re

# First, let's find the catalogueTab div and replace its content
catalogue_pattern = r'<div class="tab-content" id="catalogueTab">.*?</div>\s*</div>\s*</aside>'

new_catalogue_content = '''<div class="tab-content" id="catalogueTab">
                        <div class="catalogue-filters">
                            <div class="filter-group">
                                <label class="filter-label">Type:</label>
                                <select class="filter-select" onchange="filterStrategies(&apos;type&apos;, this.value)">
                                    <option value="all">All Types</option>
                                    <option value="trend">Trend Following</option>
                                    <option value="meanreversion">Mean Reversion</option>
                                    <option value="marketmaking">Market Making</option>
                                    <option value="arbitrage">Arbitrage</option>
                                    <option value="ml">Machine Learning</option>
                                </select>
                            </div>
                            <div class="filter-group">
                                <label class="filter-label">Complexity:</label>
                                <select class="filter-select" onchange="filterStrategies(&apos;complexity&apos;, this.value)">
                                    <option value="all">All Levels</option>
                                    <option value="beginner">Beginner</option>
                                    <option value="intermediate">Intermediate</option>
                                    <option value="advanced">Advanced</option>
                                </select>
                            </div>
                            <div class="filter-group">
                                <label class="filter-label">Sort:</label>
                                <select class="filter-select" onchange="sortStrategies(this.value)">
                                    <option value="name">Name</option>
                                    <option value="performance">Performance</option>
                                    <option value="popularity">Most Used</option>
                                    <option value="recent">Recently Added</option>
                                </select>
                            </div>
                        </div>
                        
                        <div class="strategy-grid" id="strategyGrid">
                            <!-- Strategy cards will be dynamically loaded here -->
                        </div>
                    </div>
                </div>
            </aside>'''

# Replace the problematic section
match = re.search(catalogue_pattern, content, re.DOTALL)
if match:
    content = content[:match.start()] + new_catalogue_content + content[match.end():]
    print("✅ Replaced catalogue tab content with fixed version")
else:
    print("❌ Could not find catalogue tab content to replace")

# Make sure switchTab can handle 'catalogue'
if "tabName === 'catalogue'" not in content:
    # Find the switchTab function and update it
    switch_pattern = r'(if \(targetTab\) \{[^}]+targetTab\.classList\.add\(\'active\'\);[^}]+\})'
    
    replacement = r'''\1
                
                // Load catalogue content when switching to it
                if (tabName === 'catalogue') {
                    loadStrategyCatalogue();
                }'''
    
    content = re.sub(switch_pattern, replacement, content, flags=re.DOTALL)
    print("✅ Added catalogue loading to switchTab function")

# Write the updated content
with open('/Users/daws/alphapulse/ui/research.html', 'w') as f:
    f.write(content)

print("\n📋 Complete fix applied:")
print("   - Fixed quote escaping using &apos; entities")
print("   - Ensured loadStrategyCatalogue is called")
print("   - Tab should now be fully functional")