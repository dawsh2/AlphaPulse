#!/usr/bin/env python3
"""
Script to add Catalogue tab to research.html
"""

import re

# Read the current file
with open('/Users/daws/alphapulse/ui/research.html', 'r') as f:
    content = f.read()

# 1. Add CSS styles before </style>
css_to_add = """
        /* Strategy Catalogue Styles */
        .strategy-grid {
            display: grid;
            grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
            gap: var(--space-4);
            padding: var(--space-4);
        }

        .strategy-card {
            background: var(--color-bg-primary);
            border: 1px solid var(--color-border-primary);
            border-radius: var(--radius-md);
            padding: var(--space-4);
            transition: all var(--transition-fast);
            cursor: pointer;
            position: relative;
            overflow: hidden;
        }

        .strategy-card:hover {
            border-color: var(--color-border-hover);
            transform: translateY(-2px);
            box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
        }

        .strategy-card.featured {
            border-color: var(--color-primary);
            background: linear-gradient(135deg, var(--color-bg-primary) 0%, rgba(255, 204, 0, 0.05) 100%);
        }

        .strategy-badge {
            position: absolute;
            top: var(--space-2);
            right: var(--space-2);
            background: var(--color-primary);
            color: var(--color-bg-primary);
            padding: var(--space-1) var(--space-2);
            border-radius: var(--radius-xs);
            font-size: var(--font-size-xs);
            font-weight: var(--font-weight-medium);
        }

        .strategy-header {
            display: flex;
            align-items: center;
            gap: var(--space-3);
            margin-bottom: var(--space-3);
        }

        .strategy-icon {
            width: 48px;
            height: 48px;
            background: var(--color-bg-secondary);
            border-radius: var(--radius-sm);
            display: flex;
            align-items: center;
            justify-content: center;
            font-size: 24px;
        }

        .strategy-info h3 {
            margin: 0 0 var(--space-1) 0;
            font-size: var(--font-size-md);
            font-weight: var(--font-weight-semibold);
        }

        .strategy-type {
            color: var(--color-text-secondary);
            font-size: var(--font-size-xs);
        }

        .strategy-description {
            color: var(--color-text-secondary);
            font-size: var(--font-size-sm);
            line-height: 1.5;
            margin-bottom: var(--space-3);
        }

        .strategy-metrics {
            display: grid;
            grid-template-columns: 1fr 1fr;
            gap: var(--space-2);
            margin-bottom: var(--space-3);
        }

        .metric-item {
            display: flex;
            flex-direction: column;
            gap: var(--space-1);
        }

        .metric-label {
            font-size: var(--font-size-xs);
            color: var(--color-text-secondary);
        }

        .metric-value {
            font-size: var(--font-size-sm);
            font-weight: var(--font-weight-medium);
        }

        .metric-value.positive {
            color: var(--color-success);
        }

        .metric-value.negative {
            color: var(--color-danger);
        }

        .strategy-tags {
            display: flex;
            flex-wrap: wrap;
            gap: var(--space-2);
            margin-bottom: var(--space-3);
        }

        .strategy-tag {
            background: var(--color-bg-secondary);
            padding: var(--space-1) var(--space-2);
            border-radius: var(--radius-xs);
            font-size: var(--font-size-xs);
            color: var(--color-text-secondary);
        }

        .strategy-actions {
            display: flex;
            gap: var(--space-2);
            margin-top: auto;
        }

        .strategy-action {
            flex: 1;
            padding: var(--space-2);
            background: var(--color-bg-secondary);
            border: 1px solid var(--color-border-primary);
            border-radius: var(--radius-sm);
            font-size: var(--font-size-xs);
            text-align: center;
            cursor: pointer;
            transition: all var(--transition-fast);
        }

        .strategy-action:hover {
            background: var(--color-bg-tertiary);
            border-color: var(--color-border-hover);
        }

        .strategy-action.primary {
            background: var(--color-primary);
            color: var(--color-bg-primary);
            border-color: var(--color-primary);
        }

        .strategy-action.primary:hover {
            background: var(--color-primary-hover);
            border-color: var(--color-primary-hover);
        }

        .catalogue-filters {
            display: flex;
            gap: var(--space-3);
            padding: var(--space-4);
            border-bottom: 1px solid var(--color-border-primary);
        }

        .filter-group {
            display: flex;
            align-items: center;
            gap: var(--space-2);
        }

        .filter-label {
            font-size: var(--font-size-sm);
            color: var(--color-text-secondary);
        }

        .filter-select {
            padding: var(--space-2);
            background: var(--color-bg-primary);
            border: 1px solid var(--color-border-primary);
            border-radius: var(--radius-sm);
            font-size: var(--font-size-sm);
        }
"""

# Find </style> and insert before it
content = content.replace('    </style>', css_to_add + '\n    </style>')

# 2. Add Catalogue tab button after Notebooks button
tab_button = '<button class="sidebar-tab" onclick="switchTab(\'catalogue\', this)">Catalogue</button>'
tabs_pattern = r'(<button class="sidebar-tab" onclick="switchTab\(\'notebooks\', this\)">Notebooks</button>)'
content = re.sub(tabs_pattern, r'\1\n                        ' + tab_button, content)

# 3. Add Catalogue tab content after notebooksTab
catalogue_content = '''
                    <!-- Catalogue Tab Content -->
                    <div class="tab-content" id="catalogueTab">
                        <div class="catalogue-filters">
                            <div class="filter-group">
                                <label class="filter-label">Type:</label>
                                <select class="filter-select" onchange="filterStrategies('type', this.value)">
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
                                <select class="filter-select" onchange="filterStrategies('complexity', this.value)">
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
                    </div>'''

# Find the closing tag of notebooksTab and add after it
notebooks_end_pattern = r'(</div>\s*</div>\s*</aside>)'
content = re.sub(notebooks_end_pattern, catalogue_content + '\n                ' + r'\1', content)

# 4. Add JavaScript functions
js_functions = '''
        // Strategy Catalogue Functions
        function loadStrategyCatalogue() {
            const strategies = [
                {
                    id: 'ema_cross',
                    name: 'EMA Cross',
                    type: 'trend',
                    complexity: 'beginner',
                    icon: '📈',
                    description: 'Classic exponential moving average crossover strategy. Enters long when fast EMA crosses above slow EMA.',
                    metrics: {
                        'Avg Return': '+12.5%',
                        'Win Rate': '45%',
                        'Max Drawdown': '-8.2%',
                        'Sharpe Ratio': '1.35'
                    },
                    tags: ['Simple', 'All Markets', 'Low Frequency'],
                    featured: true
                },
                {
                    id: 'market_maker',
                    name: 'Market Maker',
                    type: 'marketmaking',
                    complexity: 'advanced',
                    icon: '💹',
                    description: 'Provides liquidity by placing limit orders on both sides of the order book.',
                    metrics: {
                        'Daily Volume': '$1.2M',
                        'Spread Capture': '72%',
                        'Inventory Risk': 'Medium',
                        'Sharpe Ratio': '2.1'
                    },
                    tags: ['Complex', 'High Frequency', 'Crypto']
                },
                {
                    id: 'orderbook_imbalance',
                    name: 'Order Book Imbalance',
                    type: 'ml',
                    complexity: 'intermediate',
                    icon: '📊',
                    description: 'Analyzes order book depth to predict short-term price movements.',
                    metrics: {
                        'Signals/Day': '150+',
                        'Accuracy': '68%',
                        'Avg Hold Time': '3.5 min',
                        'Profit Factor': '1.8'
                    },
                    tags: ['ML Enhanced', 'Real-time', 'Futures']
                },
                {
                    id: 'volatility_market_maker',
                    name: 'Volatility Market Maker',
                    type: 'marketmaking',
                    complexity: 'advanced',
                    icon: '🎯',
                    description: 'Adjusts spread dynamically based on market volatility.',
                    metrics: {
                        'Avg Spread': '0.05%',
                        'Fill Rate': '82%',
                        'Vol Adjusted SR': '3.2',
                        'Max Position': '$50k'
                    },
                    tags: ['Adaptive', 'Risk Managed', 'Options']
                }
            ];
            
            const grid = document.getElementById('strategyGrid');
            grid.innerHTML = strategies.map(strategy => createStrategyCard(strategy)).join('');
        }
        
        function createStrategyCard(strategy) {
            const metricsHtml = Object.entries(strategy.metrics).map(([label, value]) => {
                const isPositive = value.includes('+') || (value.includes('%') && !value.includes('-'));
                const isNegative = value.includes('-');
                const valueClass = isPositive ? 'positive' : isNegative ? 'negative' : '';
                
                return `
                    <div class="metric-item">
                        <span class="metric-label">${label}</span>
                        <span class="metric-value ${valueClass}">${value}</span>
                    </div>
                `;
            }).join('');
            
            const tagsHtml = strategy.tags.map(tag => 
                `<span class="strategy-tag">${tag}</span>`
            ).join('');
            
            return `
                <div class="strategy-card ${strategy.featured ? 'featured' : ''}" 
                     data-type="${strategy.type}" 
                     data-complexity="${strategy.complexity}">
                    ${strategy.featured ? '<div class="strategy-badge">Featured</div>' : ''}
                    <div class="strategy-header">
                        <div class="strategy-icon">${strategy.icon}</div>
                        <div class="strategy-info">
                            <h3>${strategy.name}</h3>
                            <div class="strategy-type">${strategy.type.charAt(0).toUpperCase() + strategy.type.slice(1)}</div>
                        </div>
                    </div>
                    <div class="strategy-description">${strategy.description}</div>
                    <div class="strategy-metrics">${metricsHtml}</div>
                    <div class="strategy-tags">${tagsHtml}</div>
                    <div class="strategy-actions">
                        <button class="strategy-action" onclick="viewStrategy('${strategy.id}')">View Code</button>
                        <button class="strategy-action primary" onclick="backtest('${strategy.id}')">Backtest</button>
                    </div>
                </div>
            `;
        }

        function filterStrategies(filterType, value) {
            const cards = document.querySelectorAll('.strategy-card');
            
            cards.forEach(card => {
                if (value === 'all') {
                    card.style.display = 'block';
                } else {
                    const cardValue = card.getAttribute(`data-${filterType}`);
                    card.style.display = cardValue === value ? 'block' : 'none';
                }
            });
        }

        function sortStrategies(sortBy) {
            const grid = document.getElementById('strategyGrid');
            const cards = Array.from(grid.children);
            
            cards.sort((a, b) => {
                switch(sortBy) {
                    case 'name':
                        const nameA = a.querySelector('h3').textContent;
                        const nameB = b.querySelector('h3').textContent;
                        return nameA.localeCompare(nameB);
                    
                    case 'performance':
                        const perfA = parseFloat(a.querySelector('.metric-value').textContent.replace(/[^0-9.-]/g, ''));
                        const perfB = parseFloat(b.querySelector('.metric-value').textContent.replace(/[^0-9.-]/g, ''));
                        return perfB - perfA;
                    
                    default:
                        return 0;
                }
            });
            
            cards.forEach(card => grid.appendChild(card));
        }

        function viewStrategy(strategyName) {
            console.log('Viewing strategy:', strategyName);
            window.open(`/develop#strategy=${strategyName}`, '_blank');
        }

        function backtest(strategyName) {
            console.log('Backtesting strategy:', strategyName);
            alert(`Loading ${strategyName} into backtesting environment...`);
        }
'''

# Find the existing switchTab function and add after it
switchTab_pattern = r'(function switchTab\(tabName, element\) {[^}]+}[^}]+})'
match = re.search(switchTab_pattern, content, re.DOTALL)
if match:
    end_pos = match.end()
    content = content[:end_pos] + '\n' + js_functions + content[end_pos:]

# 5. Update the tab list in JavaScript
content = content.replace(
    "const tabs = ['snippetsTab', 'templatesTab', 'notebooksTab'];",
    "const tabs = ['snippetsTab', 'templatesTab', 'notebooksTab', 'catalogueTab'];"
)

# 6. Add loadStrategyCatalogue call to the tab switch
switch_tab_content = '''
                if (tabName === 'catalogue' && targetTab) {
                    loadStrategyCatalogue();
                }
'''
content = content.replace(
    "targetTab.classList.add('active');\n                    console.log('Tab switched successfully to:', tabName);",
    "targetTab.classList.add('active');\n                    console.log('Tab switched successfully to:', tabName);\n" + switch_tab_content
)

# Write the updated content
with open('/Users/daws/alphapulse/ui/research.html', 'w') as f:
    f.write(content)

print("✅ Successfully added Catalogue tab to research.html")
print("📋 Features added:")
print("   - Catalogue tab button in sidebar")
print("   - Strategy card grid layout")
print("   - Filter by type and complexity")
print("   - Sort by name and performance")
print("   - 4 example strategy cards with metrics")
print("   - Interactive View Code and Backtest buttons")