#!/usr/bin/env python3
"""
Update research.html to default to Catalogue view with strategy cards
"""

# Read the current research.html
with open('/Users/daws/alphapulse/ui/research.html', 'r') as f:
    content = f.read()

# Find the main content area and replace it with catalogue
import re

# First, let's add the strategy card styles to the existing CSS
additional_styles = '''
        /* Strategy Card Styles */
        .strategy-grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
            gap: var(--space-6);
            padding: var(--space-4);
        }
        
        .strategy-section {
            margin-bottom: var(--space-8);
        }
        
        .section-title {
            font-family: var(--font-family-mono);
            font-size: var(--font-size-xl);
            font-weight: var(--font-weight-semibold);
            margin-bottom: var(--space-4);
            text-align: center;
            color: var(--color-text-secondary);
        }
        
        /* AlphaPulse Style Strategy Cards */
        .strategy-card {
            font-family: var(--font-family-sans);
            background: var(--color-bg-primary);
            border: 3px solid var(--color-text-primary);
            border-radius: var(--radius-lg);
            box-shadow: -3px 5px var(--color-text-primary);
            cursor: pointer;
            transition: all var(--transition-fast);
            padding: var(--space-4);
            text-align: left;
            position: relative;
            margin-bottom: 5px;
            margin-right: 5px;
            min-height: 140px;
            display: flex;
            flex-direction: column;
            width: 100%;
        }
        
        .strategy-card:hover {
            background: var(--color-text-primary);
            color: var(--color-bg-primary);
            transform: translate(2px, -2px);
            box-shadow: -5px 7px var(--color-text-primary);
        }
        
        .strategy-card:hover .strategy-title,
        .strategy-card:hover .strategy-description,
        .strategy-card:hover .tag {
            color: var(--color-bg-primary) !important;
        }
        
        .strategy-card:hover .tag {
            background: rgba(255, 255, 255, 0.2);
            border-color: var(--color-bg-primary);
        }
        
        .strategy-card:active {
            transform: translate(-1px, 2px);
            box-shadow: -2px 3px var(--color-text-primary);
        }
        
        .strategy-title {
            font-family: var(--font-family-mono);
            font-size: var(--font-size-lg);
            font-weight: var(--font-weight-semibold);
            margin-bottom: var(--space-2);
            line-height: 1.2;
            transition: color var(--transition-fast);
        }
        
        .strategy-description {
            font-size: var(--font-size-sm);
            color: var(--color-text-secondary);
            line-height: 1.4;
            margin-bottom: var(--space-3);
            flex-grow: 1;
            transition: color var(--transition-fast);
        }
        
        .strategy-tags {
            display: flex;
            flex-wrap: wrap;
            gap: var(--space-1);
            margin-top: auto;
        }
        
        .tag {
            font-family: var(--font-family-mono);
            font-size: var(--font-size-xs);
            font-weight: var(--font-weight-medium);
            padding: 2px var(--space-2);
            background: var(--color-bg-secondary);
            color: var(--color-text-primary);
            border: 1px solid var(--color-border);
            border-radius: var(--radius-md);
            text-transform: uppercase;
            letter-spacing: 0.05em;
            transition: all var(--transition-fast);
        }
        
        /* Strategy Card Colors */
        .color-blue {
            background: #89CDF1 !important;
            border-color: #5BA7D1 !important;
            color: #1a1a1a !important;
        }
        
        .color-blue:hover {
            background: #1a1a1a !important;
            color: #89CDF1 !important;
        }
        
        .color-blue .strategy-description {
            color: #2a2a2a !important;
        }
        
        .color-orange {
            background: #FF9500 !important;
            border-color: #CC7700 !important;
            color: #1a1a1a !important;
        }
        
        .color-orange:hover {
            background: #1a1a1a !important;
            color: #FF9500 !important;
        }
        
        .color-orange .strategy-description {
            color: #2a2a2a !important;
        }
        
        .color-green {
            background: #4CAF50 !important;
            border-color: #388E3C !important;
            color: #1a1a1a !important;
        }
        
        .color-green:hover {
            background: #1a1a1a !important;
            color: #4CAF50 !important;
        }
        
        .color-green .strategy-description {
            color: #2a2a2a !important;
        }
        
        .color-purple {
            background: #9C27B0 !important;
            border-color: #7B1FA2 !important;
            color: #ffffff !important;
        }
        
        .color-purple:hover {
            background: #ffffff !important;
            color: #9C27B0 !important;
        }
        
        .color-purple .strategy-description {
            color: #e0e0e0 !important;
        }
        
        .color-red {
            background: #F44336 !important;
            border-color: #D32F2F !important;
            color: #ffffff !important;
        }
        
        .color-red:hover {
            background: #ffffff !important;
            color: #F44336 !important;
        }
        
        .color-red .strategy-description {
            color: #ffcdd2 !important;
        }
        
        .color-teal {
            background: #009688 !important;
            border-color: #00695C !important;
            color: #ffffff !important;
        }
        
        .color-teal:hover {
            background: #ffffff !important;
            color: #009688 !important;
        }
        
        .color-teal .strategy-description {
            color: #b2dfdb !important;
        }
        
        .color-indigo {
            background: #3F51B5 !important;
            border-color: #303F9F !important;
            color: #ffffff !important;
        }
        
        .color-indigo:hover {
            background: #ffffff !important;
            color: #3F51B5 !important;
        }
        
        .color-indigo .strategy-description {
            color: #c5cae9 !important;
        }
        
        .color-pink {
            background: #E91E63 !important;
            border-color: #C2185B !important;
            color: #ffffff !important;
        }
        
        .color-pink:hover {
            background: #ffffff !important;
            color: #E91E63 !important;
        }
        
        .color-pink .strategy-description {
            color: #f8bbd9 !important;
        }
        
        .color-cyan {
            background: #00BCD4 !important;
            border-color: #0097A7 !important;
            color: #1a1a1a !important;
        }
        
        .color-cyan:hover {
            background: #1a1a1a !important;
            color: #00BCD4 !important;
        }
        
        .color-cyan .strategy-description {
            color: #2a2a2a !important;
        }
        
        /* Coming Soon Overlay */
        .coming-soon::after {
            content: 'Coming Soon';
            position: absolute;
            top: var(--space-2);
            right: var(--space-2);
            font-family: var(--font-family-mono);
            font-size: var(--font-size-xs);
            font-weight: var(--font-weight-semibold);
            padding: var(--space-1) var(--space-2);
            background: #ffd700;
            color: #333;
            border: 2px solid #333;
            border-radius: var(--radius-md);
            text-transform: uppercase;
            letter-spacing: 0.05em;
            z-index: 10;
        }
        
        .coming-soon {
            opacity: 0.8;
        }
        
        /* Search bar for catalogue */
        .catalogue-search {
            max-width: 500px;
            margin: 0 auto var(--space-6) auto;
            padding: 0 var(--space-4);
        }
        
        .catalogue-search input {
            width: 100%;
            padding: var(--space-3) var(--space-4);
            font-family: var(--font-family-mono);
            font-size: var(--font-size-base);
            background: var(--color-bg-primary);
            border: 3px solid var(--color-text-primary);
            border-radius: var(--radius-md);
            color: var(--color-text-primary);
            box-shadow: -3px 5px var(--color-text-primary);
            transition: all var(--transition-fast);
        }
        
        .catalogue-search input:focus {
            outline: none;
            transform: translate(2px, -2px);
            box-shadow: -5px 7px var(--color-text-primary);
        }
'''

# Insert the additional styles before the closing style tag
content = content.replace('    </style>', additional_styles + '\n    </style>')

# Create the new catalogue content
catalogue_content = '''
            <div class="main-content">
                <div class="content-header">
                    <h1>Strategy Catalogue</h1>
                    <p>Build, Test, and Deploy Trading Strategies</p>
                </div>
                
                <div class="catalogue-search">
                    <input 
                        type="text" 
                        placeholder="search strategies..." 
                        id="strategySearchInput"
                        onkeyup="searchStrategies()"
                    />
                </div>
                
                <section class="strategy-section">
                    <h2 class="section-title">core strategies</h2>
                    <div class="strategy-grid">
                        
                        <button class="strategy-card color-blue" onclick="selectStrategy('ema-cross')">
                            <h3 class="strategy-title">EMA Cross</h3>
                            <p class="strategy-description">Classic trend-following strategy using exponential moving average crossovers.</p>
                            <div class="strategy-tags">
                                <span class="tag">trend following</span>
                                <span class="tag">beginner</span>
                            </div>
                        </button>
                        
                        <button class="strategy-card color-orange" onclick="selectStrategy('mean-reversion')">
                            <h3 class="strategy-title">Mean Reversion</h3>
                            <p class="strategy-description">Statistical arbitrage strategy that profits from temporary price dislocations.</p>
                            <div class="strategy-tags">
                                <span class="tag">statistical arb</span>
                                <span class="tag">intermediate</span>
                            </div>
                        </button>
                        
                        <button class="strategy-card color-green" onclick="selectStrategy('momentum')">
                            <h3 class="strategy-title">Momentum</h3>
                            <p class="strategy-description">Trend-following strategies that capitalize on price continuations.</p>
                            <div class="strategy-tags">
                                <span class="tag">trend</span>
                                <span class="tag">beginner</span>
                            </div>
                        </button>
                        
                    </div>
                </section>
                
                <section class="strategy-section">
                    <h2 class="section-title">statistical & factor models</h2>
                    <div class="strategy-grid">
                        
                        <button class="strategy-card color-purple" onclick="selectStrategy('pairs-trading')">
                            <h3 class="strategy-title">Pairs Trading</h3>
                            <p class="strategy-description">Long-short strategies exploiting divergences between correlated assets.</p>
                            <div class="strategy-tags">
                                <span class="tag">market neutral</span>
                                <span class="tag">intermediate</span>
                            </div>
                        </button>
                        
                        <button class="strategy-card color-red" onclick="selectStrategy('volatility')">
                            <h3 class="strategy-title">Volatility Trading</h3>
                            <p class="strategy-description">Advanced strategies focused on volatility surface dynamics and options.</p>
                            <div class="strategy-tags">
                                <span class="tag">options</span>
                                <span class="tag">expert</span>
                            </div>
                        </button>
                        
                        <button class="strategy-card color-teal coming-soon" onclick="selectStrategy('statistical-arb')">
                            <h3 class="strategy-title">Statistical Arbitrage</h3>
                            <p class="strategy-description">Multi-asset statistical models using factor decomposition and PCA.</p>
                            <div class="strategy-tags">
                                <span class="tag">factor models</span>
                                <span class="tag">advanced</span>
                            </div>
                        </button>
                        
                    </div>
                </section>
                
                <section class="strategy-section">
                    <h2 class="section-title">machine learning & portfolio</h2>
                    <div class="strategy-grid">
                        
                        <button class="strategy-card color-indigo" onclick="selectStrategy('ml-predictive')">
                            <h3 class="strategy-title">ML Predictive Models</h3>
                            <p class="strategy-description">Ensemble models and deep learning for alpha generation.</p>
                            <div class="strategy-tags">
                                <span class="tag">deep learning</span>
                                <span class="tag">expert</span>
                            </div>
                        </button>
                        
                        <button class="strategy-card color-pink coming-soon" onclick="selectStrategy('sentiment')">
                            <h3 class="strategy-title">Sentiment Analysis</h3>
                            <p class="strategy-description">NLP-driven strategies using news and social media signals.</p>
                            <div class="strategy-tags">
                                <span class="tag">nlp</span>
                                <span class="tag">advanced</span>
                            </div>
                        </button>
                        
                        <button class="strategy-card color-cyan" onclick="selectStrategy('custom')">
                            <h3 class="strategy-title">Custom Strategy Builder</h3>
                            <p class="strategy-description">Visual drag-and-drop interface for building custom strategies.</p>
                            <div class="strategy-tags">
                                <span class="tag">visual builder</span>
                                <span class="tag">all levels</span>
                            </div>
                        </button>
                        
                    </div>
                </section>
            </div>
'''

# Find and replace the main content area
# Look for the pattern that contains the code editor
main_content_pattern = r'<div class="main-content">.*?</div>\s*</div>\s*</div>'
content = re.sub(main_content_pattern, catalogue_content, content, flags=re.DOTALL)

# Add the JavaScript functions for strategy selection and search
additional_js = '''
        // Strategy selection
        function selectStrategy(strategyType) {
            console.log(`Selected strategy: ${strategyType}`);
            
            // Strategy mapping
            const strategyInfo = {
                'ema-cross': {
                    name: 'EMA Cross Strategy',
                    template: 'ema_cross_template',
                    description: 'Classic exponential moving average crossover strategy'
                },
                'mean-reversion': {
                    name: 'Mean Reversion Strategy',
                    template: 'mean_reversion_template',
                    description: 'Statistical arbitrage based on price deviations'
                },
                'momentum': {
                    name: 'Momentum Strategy',
                    template: 'momentum_template',
                    description: 'Trend-following momentum-based strategy'
                },
                'pairs-trading': {
                    name: 'Pairs Trading Strategy',
                    template: 'pairs_trading_template',
                    description: 'Market-neutral pairs trading'
                },
                'volatility': {
                    name: 'Volatility Trading Strategy',
                    template: 'volatility_template',
                    description: 'Options and volatility-based strategies'
                },
                'ml-predictive': {
                    name: 'ML Predictive Model',
                    template: 'ml_template',
                    description: 'Machine learning alpha generation'
                },
                'custom': {
                    name: 'Custom Strategy Builder',
                    template: 'blank_template',
                    description: 'Build your own strategy from scratch'
                }
            };
            
            const strategy = strategyInfo[strategyType];
            if (!strategy) {
                alert('Coming soon!');
                return;
            }
            
            // Create a modal or navigate to strategy builder
            const modal = document.createElement('div');
            modal.style.cssText = `
                position: fixed;
                top: 50%;
                left: 50%;
                transform: translate(-50%, -50%);
                background: var(--color-bg-primary);
                border: 3px solid var(--color-text-primary);
                border-radius: var(--radius-lg);
                box-shadow: -5px 7px var(--color-text-primary);
                padding: var(--space-6);
                max-width: 500px;
                z-index: 1000;
            `;
            
            modal.innerHTML = `
                <h2 style="font-family: var(--font-family-mono); margin-bottom: var(--space-4);">${strategy.name}</h2>
                <p style="color: var(--color-text-secondary); margin-bottom: var(--space-4);">${strategy.description}</p>
                <div style="display: flex; gap: var(--space-2); margin-top: var(--space-6);">
                    <button onclick="loadStrategyTemplate('${strategy.template}')" class="btn btn-primary">
                        Load Template
                    </button>
                    <button onclick="this.parentElement.parentElement.remove()" class="btn">
                        Cancel
                    </button>
                </div>
            `;
            
            document.body.appendChild(modal);
            
            // Add backdrop
            const backdrop = document.createElement('div');
            backdrop.style.cssText = `
                position: fixed;
                top: 0;
                left: 0;
                width: 100%;
                height: 100%;
                background: rgba(0, 0, 0, 0.5);
                z-index: 999;
            `;
            backdrop.onclick = () => {
                modal.remove();
                backdrop.remove();
            };
            document.body.appendChild(backdrop);
        }
        
        // Load strategy template
        function loadStrategyTemplate(templateName) {
            // Remove modal
            document.querySelectorAll('div[style*="position: fixed"]').forEach(el => el.remove());
            
            // Switch to code editor view
            // This would load the appropriate template
            console.log(`Loading template: ${templateName}`);
            alert(`Loading ${templateName}...\\n\\nThis would switch to the code editor with the selected strategy template.`);
        }
        
        // Search strategies
        function searchStrategies() {
            const query = document.getElementById('strategySearchInput').value.toLowerCase();
            const cards = document.querySelectorAll('.strategy-card');
            
            cards.forEach(card => {
                const title = card.querySelector('.strategy-title').textContent.toLowerCase();
                const description = card.querySelector('.strategy-description').textContent.toLowerCase();
                const tags = Array.from(card.querySelectorAll('.tag')).map(tag => tag.textContent.toLowerCase()).join(' ');
                
                if (query === '' || title.includes(query) || description.includes(query) || tags.includes(query)) {
                    card.style.display = 'flex';
                    card.style.opacity = '1';
                    card.style.pointerEvents = 'auto';
                } else {
                    card.style.opacity = '0.3';
                    card.style.pointerEvents = 'none';
                }
            });
        }
'''

# Insert the JavaScript before the closing script tag
content = content.replace('    </script>', additional_js + '\n    </script>')

# Write the updated content
with open('/Users/daws/alphapulse/ui/research.html', 'w') as f:
    f.write(content)

print("✅ Updated research.html to default to Catalogue view")
print("✅ Added strategy cards with AlphaPulse button styling")
print("✅ Added color-coded cards for different strategy types")
print("✅ Added search functionality")
print("✅ Added strategy selection handlers")
print("\n🎨 Features implemented:")
print("   - 9 strategy cards in 3 categories")
print("   - Color-coded by strategy type")
print("   - Hover effects matching AlphaPulse design")
print("   - Search functionality")
print("   - Coming Soon badges for future strategies")