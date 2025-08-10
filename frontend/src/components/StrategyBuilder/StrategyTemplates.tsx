import React, { useState, useEffect } from 'react';
import styles from './StrategyTemplates.module.css';

interface StrategyTemplate {
  id: string;
  name: string;
  description: string;
  type: 'single' | 'ensemble';
  category: string;
  difficulty: 'beginner' | 'intermediate' | 'advanced';
  parameters: Record<string, any>;
  entryConditions: string[];
  exitConditions: string[];
  riskManagement: {
    stopLoss?: string;
    takeProfit?: string;
  };
  expectedPerformance?: {
    sharpe: number;
    maxDrawdown: number;
    winRate: number;
  };
  tags: string[];
  author?: string;
  downloads?: number;
  rating?: number;
}

interface StrategyTemplatesProps {
  onSelectTemplate: (template: StrategyTemplate) => void;
  onClose?: () => void;
}

export const StrategyTemplates: React.FC<StrategyTemplatesProps> = ({ onSelectTemplate, onClose }) => {
  const [templates, setTemplates] = useState<StrategyTemplate[]>([]);
  const [filteredTemplates, setFilteredTemplates] = useState<StrategyTemplate[]>([]);
  const [selectedCategory, setSelectedCategory] = useState<string>('all');
  const [selectedDifficulty, setSelectedDifficulty] = useState<string>('all');
  const [searchQuery, setSearchQuery] = useState('');
  const [loading, setLoading] = useState(false);
  const [viewMode, setViewMode] = useState<'grid' | 'list'>('grid');

  // Default templates
  const defaultTemplates: StrategyTemplate[] = [
    {
      id: 'template_1',
      name: 'Classic EMA Crossover',
      description: 'A simple yet effective trend-following strategy using exponential moving averages',
      type: 'single',
      category: 'trend',
      difficulty: 'beginner',
      parameters: {
        fast_ema: 12,
        slow_ema: 26,
        timeframe: '1h'
      },
      entryConditions: [
        'EMA(12) > EMA(26)',
        'Volume > Average(Volume, 20)'
      ],
      exitConditions: [
        'EMA(12) < EMA(26)'
      ],
      riskManagement: {
        stopLoss: '2%',
        takeProfit: '5%'
      },
      expectedPerformance: {
        sharpe: 1.5,
        maxDrawdown: 10,
        winRate: 55
      },
      tags: ['trend', 'ema', 'classic'],
      author: 'AlphaPulse Team',
      downloads: 1250,
      rating: 4.5
    },
    {
      id: 'template_2',
      name: 'RSI Mean Reversion',
      description: 'Capitalizes on oversold and overbought conditions using RSI',
      type: 'single',
      category: 'meanreversion',
      difficulty: 'beginner',
      parameters: {
        rsi_period: 14,
        oversold: 30,
        overbought: 70
      },
      entryConditions: [
        'RSI(14) < 30',
        'Price > SMA(200)'
      ],
      exitConditions: [
        'RSI(14) > 70'
      ],
      riskManagement: {
        stopLoss: '3%',
        takeProfit: '4%'
      },
      expectedPerformance: {
        sharpe: 1.8,
        maxDrawdown: 8,
        winRate: 62
      },
      tags: ['meanreversion', 'rsi', 'oscillator'],
      author: 'AlphaPulse Team',
      downloads: 980,
      rating: 4.3
    },
    {
      id: 'template_3',
      name: 'Bollinger Band Breakout',
      description: 'Trades breakouts from Bollinger Band compression zones',
      type: 'single',
      category: 'volatility',
      difficulty: 'intermediate',
      parameters: {
        bb_period: 20,
        bb_stddev: 2,
        atr_period: 14
      },
      entryConditions: [
        'Price > Upper_BB(20, 2)',
        'BB_Width < ATR(14) * 0.5',
        'Volume > Average(Volume, 20) * 1.5'
      ],
      exitConditions: [
        'Price < Middle_BB(20, 2)'
      ],
      riskManagement: {
        stopLoss: 'ATR(14) * 2',
        takeProfit: 'ATR(14) * 4'
      },
      expectedPerformance: {
        sharpe: 2.1,
        maxDrawdown: 12,
        winRate: 48
      },
      tags: ['volatility', 'breakout', 'bollinger'],
      author: 'QuantTrader',
      downloads: 756,
      rating: 4.6
    },
    {
      id: 'template_4',
      name: 'MACD Divergence Hunter',
      description: 'Identifies and trades MACD divergences for high-probability reversals',
      type: 'single',
      category: 'divergence',
      difficulty: 'advanced',
      parameters: {
        macd_fast: 12,
        macd_slow: 26,
        macd_signal: 9,
        divergence_lookback: 50
      },
      entryConditions: [
        'MACD_Bullish_Divergence(50)',
        'RSI(14) < 50',
        'Price > VWAP'
      ],
      exitConditions: [
        'MACD < Signal',
        'RSI(14) > 70'
      ],
      riskManagement: {
        stopLoss: 'Recent_Low - ATR(14)',
        takeProfit: 'Fibonacci(1.618)'
      },
      expectedPerformance: {
        sharpe: 2.3,
        maxDrawdown: 15,
        winRate: 42
      },
      tags: ['divergence', 'macd', 'reversal'],
      author: 'ProTrader',
      downloads: 523,
      rating: 4.7
    },
    {
      id: 'template_5',
      name: 'Adaptive Momentum Ensemble',
      description: 'Multi-strategy ensemble that adapts to market regimes',
      type: 'ensemble',
      category: 'ensemble',
      difficulty: 'advanced',
      parameters: {
        regime_lookback: 50,
        rebalance_frequency: 'daily',
        max_strategies: 3
      },
      entryConditions: [
        'Regime == "Trending" ? Use_Trend_Strategy',
        'Regime == "Ranging" ? Use_MeanReversion_Strategy',
        'Regime == "Volatile" ? Use_Volatility_Strategy'
      ],
      exitConditions: [
        'Strategy_Specific_Exit_Conditions'
      ],
      riskManagement: {
        stopLoss: 'Portfolio_VAR(95%)',
        takeProfit: 'Dynamic_Based_On_Regime'
      },
      expectedPerformance: {
        sharpe: 2.5,
        maxDrawdown: 10,
        winRate: 58
      },
      tags: ['ensemble', 'adaptive', 'regime'],
      author: 'AlphaPulse Team',
      downloads: 342,
      rating: 4.8
    },
    {
      id: 'template_6',
      name: 'Volume Profile Scalper',
      description: 'High-frequency scalping strategy using volume profile and order flow',
      type: 'single',
      category: 'scalping',
      difficulty: 'advanced',
      parameters: {
        volume_profile_period: 20,
        poc_threshold: 0.02,
        min_volume_ratio: 1.2
      },
      entryConditions: [
        'Price_Near_POC(0.02)',
        'Volume_Imbalance > 1.2',
        'Spread < 0.1%'
      ],
      exitConditions: [
        'Price_Move > 0.5%',
        'Volume_Exhaustion'
      ],
      riskManagement: {
        stopLoss: '0.3%',
        takeProfit: '0.5%'
      },
      expectedPerformance: {
        sharpe: 3.2,
        maxDrawdown: 5,
        winRate: 68
      },
      tags: ['scalping', 'volume', 'hft'],
      author: 'SpeedTrader',
      downloads: 289,
      rating: 4.4
    }
  ];

  useEffect(() => {
    loadTemplates();
  }, []);

  useEffect(() => {
    filterTemplates();
  }, [templates, selectedCategory, selectedDifficulty, searchQuery]);

  const loadTemplates = async () => {
    setLoading(true);
    try {
      // Try to load from API
      const response = await fetch('/api/strategies/templates');
      if (response.ok) {
        const data = await response.json();
        setTemplates([...defaultTemplates, ...data.data]);
      } else {
        // Fallback to default templates
        setTemplates(defaultTemplates);
      }
      
      // Also check localStorage for user's saved templates
      const localTemplates = JSON.parse(localStorage.getItem('alphapulse_templates') || '[]');
      if (localTemplates.length > 0) {
        setTemplates(prev => [...prev, ...localTemplates]);
      }
    } catch (error) {
      console.error('Error loading templates:', error);
      setTemplates(defaultTemplates);
    } finally {
      setLoading(false);
    }
  };

  const filterTemplates = () => {
    let filtered = [...templates];
    
    // Category filter
    if (selectedCategory !== 'all') {
      filtered = filtered.filter(t => t.category === selectedCategory);
    }
    
    // Difficulty filter
    if (selectedDifficulty !== 'all') {
      filtered = filtered.filter(t => t.difficulty === selectedDifficulty);
    }
    
    // Search filter
    if (searchQuery) {
      const query = searchQuery.toLowerCase();
      filtered = filtered.filter(t => 
        t.name.toLowerCase().includes(query) ||
        t.description.toLowerCase().includes(query) ||
        t.tags.some(tag => tag.toLowerCase().includes(query))
      );
    }
    
    setFilteredTemplates(filtered);
  };

  const categories = [
    { value: 'all', label: 'All Categories', icon: '🎯' },
    { value: 'trend', label: 'Trend Following', icon: '📈' },
    { value: 'meanreversion', label: 'Mean Reversion', icon: '🔄' },
    { value: 'volatility', label: 'Volatility', icon: '📊' },
    { value: 'divergence', label: 'Divergence', icon: '🔀' },
    { value: 'scalping', label: 'Scalping', icon: '⚡' },
    { value: 'ensemble', label: 'Ensemble', icon: '🎼' }
  ];

  const getDifficultyColor = (difficulty: string) => {
    switch (difficulty) {
      case 'beginner': return styles.beginner;
      case 'intermediate': return styles.intermediate;
      case 'advanced': return styles.advanced;
      default: return '';
    }
  };

  return (
    <div className={styles.templatesContainer}>
      <div className={styles.header}>
        <h2>📚 Strategy Templates Library</h2>
        {onClose && (
          <button className={styles.closeBtn} onClick={onClose}>×</button>
        )}
      </div>
      
      <div className={styles.controls}>
        <div className={styles.searchBar}>
          <input
            type="text"
            placeholder="Search templates..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className={styles.searchInput}
          />
        </div>
        
        <div className={styles.filters}>
          <select
            value={selectedCategory}
            onChange={(e) => setSelectedCategory(e.target.value)}
            className={styles.filterSelect}
          >
            {categories.map(cat => (
              <option key={cat.value} value={cat.value}>
                {cat.icon} {cat.label}
              </option>
            ))}
          </select>
          
          <select
            value={selectedDifficulty}
            onChange={(e) => setSelectedDifficulty(e.target.value)}
            className={styles.filterSelect}
          >
            <option value="all">All Levels</option>
            <option value="beginner">🟢 Beginner</option>
            <option value="intermediate">🟡 Intermediate</option>
            <option value="advanced">🔴 Advanced</option>
          </select>
          
          <div className={styles.viewToggle}>
            <button
              className={`${styles.viewBtn} ${viewMode === 'grid' ? styles.active : ''}`}
              onClick={() => setViewMode('grid')}
            >
              ⚏
            </button>
            <button
              className={`${styles.viewBtn} ${viewMode === 'list' ? styles.active : ''}`}
              onClick={() => setViewMode('list')}
            >
              ☰
            </button>
          </div>
        </div>
      </div>
      
      <div className={styles.stats}>
        <span>{filteredTemplates.length} templates found</span>
      </div>
      
      {loading ? (
        <div className={styles.loading}>
          <div className={styles.spinner}></div>
          <p>Loading templates...</p>
        </div>
      ) : (
        <div className={`${styles.templatesGrid} ${viewMode === 'list' ? styles.listView : ''}`}>
          {filteredTemplates.map(template => (
            <div key={template.id} className={styles.templateCard}>
              <div className={styles.cardHeader}>
                <h3>{template.name}</h3>
                <span className={`${styles.difficulty} ${getDifficultyColor(template.difficulty)}`}>
                  {template.difficulty}
                </span>
              </div>
              
              <p className={styles.description}>{template.description}</p>
              
              <div className={styles.metrics}>
                {template.expectedPerformance && (
                  <>
                    <div className={styles.metric}>
                      <span className={styles.metricLabel}>Sharpe</span>
                      <span className={styles.metricValue}>{template.expectedPerformance.sharpe}</span>
                    </div>
                    <div className={styles.metric}>
                      <span className={styles.metricLabel}>Max DD</span>
                      <span className={styles.metricValue}>{template.expectedPerformance.maxDrawdown}%</span>
                    </div>
                    <div className={styles.metric}>
                      <span className={styles.metricLabel}>Win Rate</span>
                      <span className={styles.metricValue}>{template.expectedPerformance.winRate}%</span>
                    </div>
                  </>
                )}
              </div>
              
              <div className={styles.tags}>
                {template.tags.map(tag => (
                  <span key={tag} className={styles.tag}>{tag}</span>
                ))}
              </div>
              
              <div className={styles.cardFooter}>
                <div className={styles.meta}>
                  {template.author && <span className={styles.author}>by {template.author}</span>}
                  {template.downloads && <span className={styles.downloads}>⬇ {template.downloads}</span>}
                  {template.rating && <span className={styles.rating}>⭐ {template.rating}</span>}
                </div>
                <button
                  className={styles.useBtn}
                  onClick={() => onSelectTemplate(template)}
                >
                  Use Template
                </button>
              </div>
            </div>
          ))}
        </div>
      )}
      
      {filteredTemplates.length === 0 && !loading && (
        <div className={styles.emptyState}>
          <p>No templates found matching your criteria</p>
          <button onClick={() => {
            setSelectedCategory('all');
            setSelectedDifficulty('all');
            setSearchQuery('');
          }}>
            Clear Filters
          </button>
        </div>
      )}
    </div>
  );
};

export default StrategyTemplates;