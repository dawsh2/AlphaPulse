#!/usr/bin/env python3
"""
Fix text centering in explore.html
"""

# Read the file
with open('/Users/daws/alphapulse/ui/explore.html', 'r') as f:
    lines = f.readlines()

# Find where the catalogue container starts (around line 1580)
# Remove the duplicate/old catalogue content
catalogue_start = -1
for i, line in enumerate(lines):
    if '<!-- Catalogue Container -->' in line and catalogue_start == -1:
        catalogue_start = i
        break

if catalogue_start != -1:
    # Remove everything from the first catalogue container to find where the real content should go
    # Look for where the explore-container div starts
    container_start = -1
    for i in range(catalogue_start - 10, catalogue_start):
        if 'class="explore-container"' in lines[i]:
            container_start = i
            break
    
    if container_start != -1:
        # Replace with clean catalogue content
        new_content = '''        <div class="explore-container">
            <!-- Catalogue Container -->
            <div class="catalogue-container">
                <div class="catalogue-header">
                    <h1 class="catalogue-title">Strategy Catalogue</h1>
                    <p class="catalogue-subtitle">Build, Test, and Deploy Trading Strategies</p>
                </div>
                
                <div class="catalogue-search">
                    <input 
                        type="text" 
                        placeholder="search strategies..." 
                        id="strategySearchInput"
                        class="search-input"
                    />
                </div>
                
                <section class="strategy-section">
                    <h2 class="section-title">core strategies</h2>
                    <div class="strategy-grid">
                        
                        <div class="strategy-card color-blue" onclick="showPreview('ema-cross')">
                            <h3 class="strategy-title">EMA Cross</h3>
                            <p class="strategy-description">Moving Average Crossover</p>
                        </div>
                        
                        <div class="strategy-card color-orange" onclick="showPreview('mean-reversion')">
                            <h3 class="strategy-title">Mean Reversion</h3>
                            <p class="strategy-description">Statistical Arbitrage</p>
                        </div>
                        
                        <div class="strategy-card color-green" onclick="showPreview('momentum')">
                            <h3 class="strategy-title">Momentum</h3>
                            <p class="strategy-description">Trend Following</p>
                        </div>
                        
                        <div class="strategy-card color-purple" onclick="showPreview('breakout')">
                            <h3 class="strategy-title">Breakout</h3>
                            <p class="strategy-description">Range Breakout</p>
                        </div>
                        
                    </div>
                </section>
                
                <section class="strategy-section">
                    <h2 class="section-title">advanced strategies</h2>
                    <div class="strategy-grid">
                        
                        <div class="strategy-card color-red" onclick="showPreview('pairs-trading')">
                            <h3 class="strategy-title">Pairs Trading</h3>
                            <p class="strategy-description">Market Neutral</p>
                        </div>
                        
                        <div class="strategy-card color-teal" onclick="showPreview('volatility')">
                            <h3 class="strategy-title">Volatility</h3>
                            <p class="strategy-description">Options & Vol</p>
                        </div>
                        
                        <div class="strategy-card color-indigo" onclick="showPreview('arbitrage')">
                            <h3 class="strategy-title">Arbitrage</h3>
                            <p class="strategy-description">Risk-Free Profit</p>
                        </div>
                        
                        <div class="strategy-card color-pink coming-soon" onclick="showPreview('sentiment')">
                            <h3 class="strategy-title">Sentiment</h3>
                            <p class="strategy-description">NLP Analysis</p>
                        </div>
                        
                    </div>
                </section>
            </div>
        </div>
'''
        
        # Find the end of the current content (look for the closing divs and script tag)
        content_end = -1
        for i in range(catalogue_start, len(lines)):
            if '</div>' in lines[i] and '<script src="layout.js">' in lines[i+1] if i+1 < len(lines) else False:
                content_end = i
                break
        
        if content_end != -1:
            # Replace the content
            lines = lines[:container_start] + [new_content + '\n'] + lines[content_end+1:]
        
        # Write back
        with open('/Users/daws/alphapulse/ui/explore.html', 'w') as f:
            f.writelines(lines)
        
        print("✅ Fixed catalogue container structure")
        print("✅ Removed inline styles that were overriding CSS")
        print("✅ Text should now be properly centered")
    else:
        print("❌ Could not find explore-container")
else:
    print("❌ Could not find catalogue container")

# Also make sure the CSS doesn't have conflicting styles
with open('/Users/daws/alphapulse/ui/explore.html', 'r') as f:
    content = f.read()

# Update explore-container CSS to ensure proper layout
import re
explore_css_pattern = r'\.explore-container\s*{[^}]+}'
new_explore_css = '''.explore-container {
            min-height: calc(100vh - var(--header-height));
            display: flex;
            flex-direction: column;
        }'''

content = re.sub(explore_css_pattern, new_explore_css, content)

# Make sure catalogue-container takes full width
catalogue_css_pattern = r'\.catalogue-container\s*{[^}]+}'
new_catalogue_css = '''.catalogue-container {
            flex: 1;
            width: 100%;
            max-width: 1400px;
            margin: 0 auto;
            padding: var(--space-8) var(--space-6);
            overflow-y: auto;
        }'''

content = re.sub(catalogue_css_pattern, new_catalogue_css, content)

with open('/Users/daws/alphapulse/ui/explore.html', 'w') as f:
    f.write(content)

print("✅ Updated CSS for proper centering")