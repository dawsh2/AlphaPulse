// Add this script to research.html to enable the Catalogue tab
// This can be added at the end of the existing <script> section

// Fix the switchTab function if it's broken
window.switchTab = function(tabName, element) {
    try {
        console.log('Switching to tab:', tabName);
        
        // Update tab buttons
        document.querySelectorAll('.sidebar-tab').forEach(tab => {
            tab.classList.remove('active');
        });
        element.classList.add('active');
        
        // Update tab content
        document.querySelectorAll('.tab-content').forEach(content => {
            content.classList.remove('active');
        });
        
        const targetTab = document.getElementById(tabName + 'Tab');
        if (targetTab) {
            targetTab.classList.add('active');
            console.log('Tab switched successfully to:', tabName);
            
            // Load catalogue content when switching to it
            if (tabName === 'catalogue') {
                loadStrategyCatalogue();
            }
        } else {
            console.error('Tab not found:', tabName + 'Tab');
        }
    } catch (error) {
        console.error('Error switching tab:', error);
    }
};

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
        },
        {
            id: 'twap',
            name: 'TWAP Execution',
            type: 'execution',
            complexity: 'intermediate',
            icon: '⏱️',
            description: 'Time-weighted average price algorithm for large order execution.',
            metrics: {
                'Slippage': '-0.02%',
                'Completion Rate': '99.8%',
                'Market Impact': 'Low',
                'Typical Size': '$100k+'
            },
            tags: ['Execution', 'Institutional', 'Large Orders']
        },
        {
            id: 'bollinger_mr',
            name: 'Bollinger Bands MR',
            type: 'meanreversion',
            complexity: 'beginner',
            icon: '📉',
            description: 'Trades price reversions when price touches Bollinger Bands.',
            metrics: {
                'Win Rate': '62%',
                'Avg Trade': '+0.8%',
                'Trades/Month': '25',
                'Recovery Time': '3 days'
            },
            tags: ['Simple', 'Range Markets', 'Stocks']
        }
    ];
    
    const grid = document.getElementById('strategyGrid');
    if (!grid) {
        console.error('Strategy grid not found');
        return;
    }
    
    grid.innerHTML = strategies.map(strategy => createStrategyCard(strategy)).join('');
}

function createStrategyCard(strategy) {
    const metricsHtml = Object.entries(strategy.metrics).map(([label, value]) => {
        const isPositive = value.toString().includes('+') || (value.toString().includes('%') && !value.toString().includes('-'));
        const isNegative = value.toString().includes('-');
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
    
    const typeDisplay = strategy.type.replace('meanreversion', 'Mean Reversion')
                                    .replace('marketmaking', 'Market Making')
                                    .replace('ml', 'Machine Learning')
                                    .split(' ')
                                    .map(word => word.charAt(0).toUpperCase() + word.slice(1))
                                    .join(' ');
    
    return `
        <div class="strategy-card ${strategy.featured ? 'featured' : ''}" 
             data-type="${strategy.type}" 
             data-complexity="${strategy.complexity}">
            ${strategy.featured ? '<div class="strategy-badge">Featured</div>' : ''}
            <div class="strategy-header">
                <div class="strategy-icon">${strategy.icon}</div>
                <div class="strategy-info">
                    <h3>${strategy.name}</h3>
                    <div class="strategy-type">${typeDisplay}</div>
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

window.filterStrategies = function(filterType, value) {
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

window.sortStrategies = function(sortBy) {
    const grid = document.getElementById('strategyGrid');
    const cards = Array.from(grid.children);
    
    cards.sort((a, b) => {
        switch(sortBy) {
            case 'name':
                const nameA = a.querySelector('h3').textContent;
                const nameB = b.querySelector('h3').textContent;
                return nameA.localeCompare(nameB);
            
            case 'performance':
                const getFirstMetricValue = (card) => {
                    const firstMetric = card.querySelector('.metric-value');
                    return parseFloat(firstMetric.textContent.replace(/[^0-9.-]/g, '')) || 0;
                };
                return getFirstMetricValue(b) - getFirstMetricValue(a);
            
            default:
                return 0;
        }
    });
    
    cards.forEach(card => grid.appendChild(card));
}

window.viewStrategy = function(strategyName) {
    console.log('Viewing strategy:', strategyName);
    // Navigate to develop page with strategy
    window.location.href = `/develop#strategy=${strategyName}`;
}

window.backtest = function(strategyName) {
    console.log('Backtesting strategy:', strategyName);
    alert(`Loading ${strategyName} into backtesting environment...`);
    // TODO: Implement actual backtesting navigation
}

// Initialize catalogue if the tab is already active
document.addEventListener('DOMContentLoaded', function() {
    const catalogueTab = document.getElementById('catalogueTab');
    if (catalogueTab && catalogueTab.classList.contains('active')) {
        loadStrategyCatalogue();
    }
});