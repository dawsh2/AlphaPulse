#!/usr/bin/env python3
"""
Update explore.html to show centered strategy catalogue
"""

# Read the file
with open('/Users/daws/alphapulse/ui/explore.html', 'r') as f:
    content = f.read()

# Replace the notebook container with the strategy catalogue
import re

# Find the notebook container and replace it with catalogue content
notebook_pattern = r'<!-- Notebook Container -->.*?</div>\s*</div>\s*(?=\s*</div>\s*<script|\s*</div>\s*</body>)'

catalogue_content = '''            <!-- Catalogue Container -->
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
            </div>'''

# Replace the notebook container
content = re.sub(notebook_pattern, catalogue_content, content, flags=re.DOTALL)

# Add the catalogue-specific styles
catalogue_styles = '''
        /* Catalogue Styles */
        .catalogue-container {
            flex: 1;
            padding: var(--space-8) var(--space-6);
            overflow-y: auto;
        }
        
        .catalogue-header {
            text-align: center;
            margin-bottom: var(--space-8);
        }
        
        .catalogue-title {
            font-size: var(--font-size-4xl);
            font-weight: var(--font-weight-bold);
            margin-bottom: var(--space-2);
            background: linear-gradient(135deg, var(--color-primary) 0%, var(--color-secondary) 100%);
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
            background-clip: text;
        }
        
        .catalogue-subtitle {
            font-size: var(--font-size-lg);
            color: var(--color-text-secondary);
        }
        
        .catalogue-search {
            max-width: 600px;
            margin: 0 auto var(--space-8) auto;
        }
        
        .search-input {
            width: 100%;
            padding: var(--space-3) var(--space-4);
            font-family: var(--font-family-mono);
            font-size: var(--font-size-base);
            background: var(--color-bg-primary);
            border: 2px solid var(--color-border-primary);
            border-radius: var(--radius-md);
            color: var(--color-text-primary);
            outline: none;
            transition: all var(--transition-fast);
        }
        
        .search-input:focus {
            border-color: var(--color-primary);
            box-shadow: 0 0 0 3px rgba(9, 105, 218, 0.1);
        }
        
        .strategy-section {
            margin-bottom: var(--space-12);
        }
        
        .section-title {
            font-family: var(--font-family-mono);
            font-size: var(--font-size-xl);
            font-weight: var(--font-weight-semibold);
            margin-bottom: var(--space-6);
            text-align: center;
            color: var(--color-text-secondary);
            text-transform: lowercase;
        }
        
        .strategy-grid {
            display: grid;
            grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
            gap: var(--space-4);
            max-width: 1000px;
            margin: 0 auto;
        }
        
        .strategy-card {
            aspect-ratio: 1;
            background: var(--color-bg-secondary);
            border: 2px solid var(--color-border-primary);
            border-radius: var(--radius-lg);
            cursor: pointer;
            transition: all var(--transition-fast);
            padding: var(--space-4);
            display: flex;
            flex-direction: column;
            align-items: center;
            justify-content: center;
            text-align: center;
            position: relative;
        }
        
        .strategy-card:hover {
            transform: translateY(-2px);
            box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
            border-color: var(--color-primary);
        }
        
        .strategy-title {
            font-family: var(--font-family-mono);
            font-size: var(--font-size-lg);
            font-weight: var(--font-weight-semibold);
            margin-bottom: var(--space-2);
        }
        
        .strategy-description {
            font-size: var(--font-size-sm);
            color: var(--color-text-secondary);
        }
        
        /* Color variants */
        .color-blue {
            background: rgba(9, 105, 218, 0.1);
            border-color: rgba(9, 105, 218, 0.3);
        }
        
        .color-orange {
            background: rgba(255, 149, 0, 0.1);
            border-color: rgba(255, 149, 0, 0.3);
        }
        
        .color-green {
            background: rgba(26, 127, 55, 0.1);
            border-color: rgba(26, 127, 55, 0.3);
        }
        
        .color-purple {
            background: rgba(130, 80, 223, 0.1);
            border-color: rgba(130, 80, 223, 0.3);
        }
        
        .color-red {
            background: rgba(207, 34, 46, 0.1);
            border-color: rgba(207, 34, 46, 0.3);
        }
        
        .color-teal {
            background: rgba(0, 150, 136, 0.1);
            border-color: rgba(0, 150, 136, 0.3);
        }
        
        .color-indigo {
            background: rgba(63, 81, 181, 0.1);
            border-color: rgba(63, 81, 181, 0.3);
        }
        
        .color-pink {
            background: rgba(233, 30, 99, 0.1);
            border-color: rgba(233, 30, 99, 0.3);
        }
        
        .coming-soon {
            opacity: 0.6;
        }
        
        .coming-soon::after {
            content: 'Soon';
            position: absolute;
            top: var(--space-2);
            right: var(--space-2);
            font-family: var(--font-family-mono);
            font-size: var(--font-size-xs);
            font-weight: var(--font-weight-bold);
            padding: 2px var(--space-2);
            background: var(--color-warning);
            color: white;
            border-radius: var(--radius-sm);
            text-transform: uppercase;
        }'''

# Insert the styles before </style>
content = content.replace('    </style>', catalogue_styles + '\n    </style>')

# Write the updated content
with open('/Users/daws/alphapulse/ui/explore.html', 'w') as f:
    f.write(content)

print("✅ Updated explore.html with centered catalogue content")
print("✅ Added proper catalogue styles")
print("✅ Title and subtitle are now centered")