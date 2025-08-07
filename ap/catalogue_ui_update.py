#!/usr/bin/env python3
"""
Update the Catalogue UI with better card design
"""

# Read the file
with open('/Users/daws/alphapulse/ui/research.html', 'r') as f:
    content = f.read()

# New CSS for strategy cards that match the design system
new_css = """
        /* Strategy Catalogue - Card Button Style */
        .strategy-grid {
            display: grid;
            grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
            gap: var(--space-4);
            padding: var(--space-4);
        }

        .strategy-card {
            background: var(--color-bg-primary);
            border: 2px solid var(--color-border-primary);
            border-radius: var(--radius-lg);
            padding: var(--space-5);
            transition: all var(--transition-base);
            cursor: pointer;
            position: relative;
            overflow: hidden;
            display: flex;
            flex-direction: column;
            min-height: 280px;
        }

        .strategy-card:hover {
            transform: translateY(-4px);
            box-shadow: var(--shadow-lg);
        }

        /* Color coding by strategy type */
        .strategy-card[data-type="trend"] {
            border-color: var(--color-success);
            background: linear-gradient(135deg, var(--color-bg-primary) 0%, rgba(16, 185, 129, 0.05) 100%);
        }

        .strategy-card[data-type="meanreversion"] {
            border-color: var(--color-accent-primary);
            background: linear-gradient(135deg, var(--color-bg-primary) 0%, rgba(59, 130, 246, 0.05) 100%);
        }

        .strategy-card[data-type="marketmaking"] {
            border-color: var(--color-warning);
            background: linear-gradient(135deg, var(--color-bg-primary) 0%, rgba(245, 158, 11, 0.05) 100%);
        }

        .strategy-card[data-type="ml"] {
            border-color: var(--color-danger);
            background: linear-gradient(135deg, var(--color-bg-primary) 0%, rgba(239, 68, 68, 0.05) 100%);
        }

        .strategy-card[data-type="execution"] {
            border-color: var(--color-text-secondary);
            background: linear-gradient(135deg, var(--color-bg-primary) 0%, rgba(107, 114, 128, 0.05) 100%);
        }

        /* Complexity badge */
        .complexity-badge {
            position: absolute;
            top: var(--space-3);
            right: var(--space-3);
            padding: var(--space-1) var(--space-3);
            border-radius: var(--radius-full);
            font-size: var(--font-size-xs);
            font-weight: var(--font-weight-semibold);
            text-transform: uppercase;
            letter-spacing: var(--letter-spacing-wide);
        }

        .complexity-badge[data-level="beginner"] {
            background: var(--color-success-emphasis);
            color: white;
        }

        .complexity-badge[data-level="intermediate"] {
            background: var(--color-warning);
            color: var(--color-text-primary);
        }

        .complexity-badge[data-level="advanced"] {
            background: var(--color-danger);
            color: white;
        }

        .strategy-icon-large {
            font-size: 48px;
            margin-bottom: var(--space-3);
            opacity: 0.8;
        }

        .strategy-name {
            font-size: var(--font-size-xl);
            font-weight: var(--font-weight-bold);
            margin-bottom: var(--space-2);
            color: var(--color-text-primary);
        }

        .strategy-type-label {
            font-size: var(--font-size-sm);
            color: var(--color-text-secondary);
            margin-bottom: var(--space-3);
            text-transform: capitalize;
        }

        .strategy-description {
            font-size: var(--font-size-sm);
            color: var(--color-text-secondary);
            line-height: var(--line-height-relaxed);
            margin-bottom: var(--space-4);
            flex-grow: 1;
        }

        .strategy-stats {
            display: grid;
            grid-template-columns: repeat(2, 1fr);
            gap: var(--space-3);
            padding-top: var(--space-3);
            border-top: 1px solid var(--color-border-secondary);
        }

        .stat-item {
            text-align: center;
        }

        .stat-value {
            font-size: var(--font-size-lg);
            font-weight: var(--font-weight-bold);
            color: var(--color-text-primary);
            display: block;
        }

        .stat-label {
            font-size: var(--font-size-xs);
            color: var(--color-text-tertiary);
            text-transform: uppercase;
            letter-spacing: var(--letter-spacing-wide);
        }

        /* Quick action buttons */
        .strategy-actions {
            position: absolute;
            bottom: var(--space-3);
            right: var(--space-3);
            display: flex;
            gap: var(--space-2);
            opacity: 0;
            transition: opacity var(--transition-fast);
        }

        .strategy-card:hover .strategy-actions {
            opacity: 1;
        }

        .quick-action {
            width: 32px;
            height: 32px;
            border-radius: var(--radius-full);
            background: var(--color-bg-primary);
            border: 1px solid var(--color-border-primary);
            display: flex;
            align-items: center;
            justify-content: center;
            cursor: pointer;
            transition: all var(--transition-fast);
        }

        .quick-action:hover {
            background: var(--color-bg-secondary);
            transform: scale(1.1);
        }

        /* Filter pills */
        .catalogue-filters {
            display: flex;
            gap: var(--space-3);
            padding: var(--space-4);
            border-bottom: 1px solid var(--color-border-primary);
            flex-wrap: wrap;
            align-items: center;
        }

        .filter-pills {
            display: flex;
            gap: var(--space-2);
        }

        .filter-pill {
            padding: var(--space-2) var(--space-3);
            border-radius: var(--radius-full);
            background: var(--color-bg-secondary);
            border: 1px solid var(--color-border-primary);
            font-size: var(--font-size-sm);
            cursor: pointer;
            transition: all var(--transition-fast);
        }

        .filter-pill:hover {
            background: var(--color-bg-tertiary);
        }

        .filter-pill.active {
            background: var(--color-accent-primary);
            color: white;
            border-color: var(--color-accent-primary);
        }
"""

# Find the closing style tag and insert new CSS
content = content.replace('    </style>', new_css + '\n    </style>')

# New JavaScript for the improved catalogue
new_js = """
        // Enhanced Strategy Catalogue
        function loadStrategyCatalogue() {
            const strategies = [
                {
                    id: 'ema_cross',
                    name: 'EMA Cross',
                    type: 'trend',
                    complexity: 'beginner',
                    icon: '📈',
                    description: 'Classic exponential moving average crossover strategy. Simple yet effective in trending markets.',
                    stats: {
                        'Win Rate': '45%',
                        'Sharpe': '1.35',
                        'Avg Trade': '+0.8%',
                        'Frequency': '15/mo'
                    }
                },
                {
                    id: 'market_maker',
                    name: 'Market Maker',
                    type: 'marketmaking',
                    complexity: 'advanced',
                    icon: '💹',
                    description: 'Provides liquidity by maintaining bid/ask quotes. Profits from spread capture.',
                    stats: {
                        'Fill Rate': '82%',
                        'Daily Vol': '$1.2M',
                        'Spread': '0.05%',
                        'Sharpe': '2.1'
                    }
                },
                {
                    id: 'bollinger_bands',
                    name: 'Bollinger Bands',
                    type: 'meanreversion',
                    complexity: 'beginner',
                    icon: '📊',
                    description: 'Mean reversion strategy using dynamic bands to identify overbought/oversold conditions.',
                    stats: {
                        'Win Rate': '62%',
                        'Sharpe': '1.8',
                        'Avg Trade': '+0.5%',
                        'Hold Time': '2.5 days'
                    }
                },
                {
                    id: 'ml_predictor',
                    name: 'ML Price Predictor',
                    type: 'ml',
                    complexity: 'advanced',
                    icon: '🤖',
                    description: 'Machine learning model using multiple features to predict short-term price movements.',
                    stats: {
                        'Accuracy': '68%',
                        'Sharpe': '2.5',
                        'Signals': '150/day',
                        'Win Rate': '58%'
                    }
                },
                {
                    id: 'twap_exec',
                    name: 'TWAP Execution',
                    type: 'execution',
                    complexity: 'intermediate',
                    icon: '⏱️',
                    description: 'Time-weighted average price algorithm for optimal execution of large orders.',
                    stats: {
                        'Slippage': '-0.02%',
                        'Complete': '99.8%',
                        'Avg Size': '$500K',
                        'Impact': 'Minimal'
                    }
                },
                {
                    id: 'pairs_trading',
                    name: 'Statistical Arbitrage',
                    type: 'meanreversion',
                    complexity: 'intermediate',
                    icon: '⚖️',
                    description: 'Exploits price relationships between correlated assets using cointegration.',
                    stats: {
                        'Win Rate': '71%',
                        'Sharpe': '2.2',
                        'Correlation': '0.85',
                        'Frequency': '40/mo'
                    }
                }
            ];
            
            const grid = document.getElementById('strategyGrid');
            if (!grid) return;
            
            // Create the new filter UI
            const catalogueTab = document.getElementById('catalogueTab');
            if (catalogueTab && !document.getElementById('catalogueFilters')) {
                const filtersHtml = `
                    <div class="catalogue-filters" id="catalogueFilters">
                        <span style="font-weight: var(--font-weight-semibold);">Type:</span>
                        <div class="filter-pills" data-filter="type">
                            <div class="filter-pill active" data-value="all">All</div>
                            <div class="filter-pill" data-value="trend">Trend</div>
                            <div class="filter-pill" data-value="meanreversion">Mean Reversion</div>
                            <div class="filter-pill" data-value="marketmaking">Market Making</div>
                            <div class="filter-pill" data-value="ml">Machine Learning</div>
                            <div class="filter-pill" data-value="execution">Execution</div>
                        </div>
                        
                        <span style="font-weight: var(--font-weight-semibold); margin-left: var(--space-4);">Level:</span>
                        <div class="filter-pills" data-filter="complexity">
                            <div class="filter-pill active" data-value="all">All</div>
                            <div class="filter-pill" data-value="beginner">Beginner</div>
                            <div class="filter-pill" data-value="intermediate">Intermediate</div>
                            <div class="filter-pill" data-value="advanced">Advanced</div>
                        </div>
                    </div>
                `;
                catalogueTab.innerHTML = filtersHtml + catalogueTab.innerHTML;
                
                // Add filter handlers
                document.querySelectorAll('.filter-pill').forEach(pill => {
                    pill.addEventListener('click', function() {
                        const filterGroup = this.parentElement;
                        filterGroup.querySelectorAll('.filter-pill').forEach(p => p.classList.remove('active'));
                        this.classList.add('active');
                        
                        const filterType = filterGroup.getAttribute('data-filter');
                        const filterValue = this.getAttribute('data-value');
                        filterStrategies(filterType, filterValue);
                    });
                });
            }
            
            // Create strategy cards
            grid.innerHTML = strategies.map(strategy => createEnhancedStrategyCard(strategy)).join('');
        }
        
        function createEnhancedStrategyCard(strategy) {
            const statsHtml = Object.entries(strategy.stats).map(([label, value]) => `
                <div class="stat-item">
                    <span class="stat-value">${value}</span>
                    <span class="stat-label">${label}</span>
                </div>
            `).join('');
            
            const typeLabel = strategy.type.replace('meanreversion', 'mean reversion')
                                          .replace('marketmaking', 'market making')
                                          .replace('ml', 'machine learning');
            
            return `
                <div class="strategy-card" 
                     data-type="${strategy.type}" 
                     data-complexity="${strategy.complexity}"
                     onclick="openStrategy('${strategy.id}')">
                    
                    <div class="complexity-badge" data-level="${strategy.complexity}">
                        ${strategy.complexity}
                    </div>
                    
                    <div class="strategy-icon-large">${strategy.icon}</div>
                    <h3 class="strategy-name">${strategy.name}</h3>
                    <div class="strategy-type-label">${typeLabel}</div>
                    
                    <p class="strategy-description">${strategy.description}</p>
                    
                    <div class="strategy-stats">
                        ${statsHtml}
                    </div>
                    
                    <div class="strategy-actions">
                        <div class="quick-action" onclick="event.stopPropagation(); viewStrategyCode('${strategy.id}')" title="View Code">
                            <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
                                <path d="M5.854 4.854a.5.5 0 1 0-.708-.708l-3.5 3.5a.5.5 0 0 0 0 .708l3.5 3.5a.5.5 0 0 0 .708-.708L2.707 8l3.147-3.146zm4.292 0a.5.5 0 0 1 .708-.708l3.5 3.5a.5.5 0 0 1 0 .708l-3.5 3.5a.5.5 0 0 1-.708-.708L13.293 8l-3.147-3.146z"/>
                            </svg>
                        </div>
                        <div class="quick-action" onclick="event.stopPropagation(); backtestStrategy('${strategy.id}')" title="Backtest">
                            <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
                                <path d="M11.251.068a.5.5 0 0 1 .227.58L9.677 6.5H13a.5.5 0 0 1 .364.843l-8 8.5a.5.5 0 0 1-.842-.49L6.323 9.5H3a.5.5 0 0 1-.364-.843l8-8.5a.5.5 0 0 1 .615-.09z"/>
                            </svg>
                        </div>
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
        
        function openStrategy(strategyId) {
            console.log('Opening strategy:', strategyId);
            // Navigate to strategy details or open in editor
            alert(`Opening ${strategyId} strategy...`);
        }
        
        function viewStrategyCode(strategyId) {
            console.log('Viewing code for:', strategyId);
            window.open(`/develop#strategy=${strategyId}`, '_blank');
        }
        
        function backtestStrategy(strategyId) {
            console.log('Backtesting:', strategyId);
            alert(`Starting backtest for ${strategyId}...`);
        }
"""

# Find where to insert the new JS (after the fixed switchTab function)
insert_marker = "} catch (error) {\n                console.error('Error switching tab:', error);\n            }\n        }"
insert_pos = content.find(insert_marker)
if insert_pos != -1:
    insert_pos += len(insert_marker)
    content = content[:insert_pos] + '\n' + new_js + content[insert_pos:]

# Write the updated file
with open('/Users/daws/alphapulse/ui/research.html', 'w') as f:
    f.write(content)

print("✅ Updated Catalogue UI with improved card design")
print("\n📋 New features:")
print("   - Large card buttons with color coding by strategy type")
print("   - Complexity badges (Beginner/Intermediate/Advanced)")
print("   - Clean stats grid showing key metrics")
print("   - Filter pills instead of dropdowns")
print("   - Quick action buttons on hover (Code/Backtest)")
print("   - Matches the existing design system (buttons, shadows, etc.)")
print("\n🎨 Color scheme:")
print("   - Trend Following: Green")
print("   - Mean Reversion: Blue")
print("   - Market Making: Yellow/Orange")
print("   - Machine Learning: Red")
print("   - Execution: Gray")