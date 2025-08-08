import React, { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import styles from './ExplorePage.module.css';

interface Strategy {
  id: string;
  title: string;
  description: string;
  color: string;
  tags: string[];
  creator?: string; // Username of strategy creator
  comingSoon?: boolean;
  metrics?: {
    sharpe: number;
    annualReturn: number;
    maxDrawdown: number;
    winRate: number;
  };
  behavior?: 'trending' | 'meanrev' | 'breakout' | 'volatility';
  risk?: 'conservative' | 'moderate' | 'aggressive';
  timeframe?: 'intraday' | 'swing' | 'position';
}

const coreStrategies: Strategy[] = [
  {
    id: 'ema-cross',
    title: 'EMA Cross',
    description: 'Classic trend-following strategy using exponential moving average crossovers.',
    color: 'blue',
    tags: ['MA cross', 'simple', 'S&P-500', 'NASDAQ', 'Russell-2000', 'SPY', 'QQQ', 'IWM'],
    creator: 'alexchen',
    behavior: 'trending',
    risk: 'moderate',
    timeframe: 'swing',
    metrics: {
      sharpe: 1.82,
      annualReturn: 24.5,
      maxDrawdown: -8.3,
      winRate: 68
    }
  },
  {
    id: 'mean-reversion',
    title: 'RSI Mean Reversion',
    description: 'Trades oversold bounces and overbought reversals using RSI divergences.',
    color: 'orange',
    tags: ['RSI', 'reversal', 'S&P-500', 'NASDAQ', 'SPY', 'QQQ'],
    creator: 'sarahkim',
    behavior: 'meanrev',
    risk: 'conservative',
    timeframe: 'swing',
    metrics: {
      sharpe: 2.15,
      annualReturn: 31.2,
      maxDrawdown: -6.7,
      winRate: 72
    }
  },
  {
    id: 'momentum',
    title: 'Momentum Breakout',
    description: 'Captures explosive moves after consolidation periods.',
    color: 'green',
    tags: ['breakout', 'volume', 'S&P-500', 'NASDAQ', 'Russell-2000', 'SPY', 'QQQ', 'IWM'],
    creator: 'mikejohnson',
    behavior: 'breakout',
    risk: 'aggressive',
    timeframe: 'intraday',
    metrics: {
      sharpe: 1.45,
      annualReturn: 18.9,
      maxDrawdown: -12.1,
      winRate: 62
    }
  }
];

const statisticalStrategies: Strategy[] = [
  {
    id: 'pairs-trading',
    title: 'Pairs Trading',
    description: 'Market-neutral strategy trading correlated pairs divergence.',
    color: 'purple',
    tags: ['pairs', 'neutral', 'S&P-500', 'NASDAQ', 'sector-ETFs'],
    creator: 'quantdave',
    behavior: 'meanrev',
    risk: 'conservative',
    timeframe: 'position',
    metrics: {
      sharpe: 2.54,
      annualReturn: 28.7,
      maxDrawdown: -4.2,
      winRate: 76
    }
  },
  {
    id: 'volatility-harvest',
    title: 'Vol Harvester',
    description: 'Profits from volatility spikes and VIX contango.',
    color: 'red',
    tags: ['VIX', 'options', 'VXX', 'UVXY', 'volatility-ETFs'],
    creator: 'voltrader',
    behavior: 'volatility',
    risk: 'aggressive',
    timeframe: 'swing',
    metrics: {
      sharpe: 1.95,
      annualReturn: 35.8,
      maxDrawdown: -15.3,
      winRate: 65
    }
  },
  {
    id: 'bollinger-squeeze',
    title: 'Bollinger Squeeze',
    description: 'Trades volatility expansion after consolidation.',
    color: 'teal',
    tags: ['BB', 'squeeze', 'S&P-500', 'NASDAQ', 'SPY', 'QQQ'],
    behavior: 'breakout',
    risk: 'moderate',
    timeframe: 'intraday',
    metrics: {
      sharpe: 1.67,
      annualReturn: 22.4,
      maxDrawdown: -9.8,
      winRate: 69
    }
  }
];

const mlStrategies: Strategy[] = [
  {
    id: 'trend-rider',
    title: 'Trend Rider XL',
    description: 'Multi-timeframe trend following with dynamic position sizing.',
    color: 'indigo',
    tags: ['multi-TF', 'adaptive', 'S&P-500', 'NASDAQ', 'Russell-2000', 'SPY', 'QQQ', 'IWM'],
    creator: 'trendmaster',
    behavior: 'trending',
    risk: 'moderate',
    timeframe: 'position',
    metrics: {
      sharpe: 2.91,
      annualReturn: 42.5,
      maxDrawdown: -9.8,
      winRate: 71
    }
  },
  {
    id: 'gap-fade',
    title: 'Gap Fade Pro',
    description: 'Fades opening gaps with statistical edge.',
    color: 'pink',
    tags: ['gaps', 'open', 'S&P-500', 'NASDAQ', 'SPY', 'QQQ', 'individual-stocks'],
    creator: 'gapfader',
    behavior: 'meanrev',
    risk: 'moderate',
    timeframe: 'intraday',
    metrics: {
      sharpe: 1.93,
      annualReturn: 27.8,
      maxDrawdown: -7.2,
      winRate: 74
    }
  },
  {
    id: 'custom',
    title: 'Strategy Builder',
    description: 'Create your own strategy with visual tools.',
    color: 'cyan',
    tags: ['custom', 'builder', 'any-universe'],
    metrics: {
      sharpe: 0,
      annualReturn: 0,
      maxDrawdown: 0,
      winRate: 0
    }
  }
];

const additionalStrategies: Strategy[] = [
  {
    id: 'macd-cross',
    title: 'MACD Cross Signal',
    description: 'Classic MACD signal line crossover with histogram confirmation.',
    color: 'blue',
    tags: ['trending', 'MACD', 'crossover', 'histogram', 'swing', 'moderate-risk', 'beginner', 'momentum', 'S&P-500', 'NASDAQ', 'SPY', 'QQQ'],
    metrics: {
      sharpe: 1.64,
      annualReturn: 21.3,
      maxDrawdown: -11.2,
      winRate: 59
    }
  },
  {
    id: 'stoch-rsi',
    title: 'Stochastic RSI',
    description: 'Combines Stochastic and RSI for precise overbought/oversold signals.',
    color: 'orange',
    tags: ['mean-reversion', 'stochastic', 'RSI', 'oversold', 'overbought', 'intraday', 'moderate-risk', 'intermediate', 'S&P-500', 'NASDAQ', 'SPY', 'QQQ'],
    metrics: {
      sharpe: 2.08,
      annualReturn: 26.7,
      maxDrawdown: -8.9,
      winRate: 71
    }
  },
  {
    id: 'channel-breakout',
    title: 'Channel Breakout',
    description: 'Trades breakouts from established support and resistance channels.',
    color: 'green',
    tags: ['breakout', 'channels', 'support-resistance', 'swing', 'moderate-risk', 'technical', 'intermediate', 'S&P-500', 'NASDAQ', 'Russell-2000', 'SPY', 'QQQ', 'IWM'],
    metrics: {
      sharpe: 1.77,
      annualReturn: 29.4,
      maxDrawdown: -13.6,
      winRate: 54
    }
  },
  {
    id: 'vwap-reversion',
    title: 'VWAP Reversion',
    description: 'Mean reversion strategy using Volume Weighted Average Price.',
    color: 'purple',
    tags: ['mean-reversion', 'VWAP', 'volume', 'intraday', 'low-risk', 'institutional', 'beginner', 'SPY', 'S&P-500', 'ETF'],
    creator: 'flowtrader',
    metrics: {
      sharpe: 2.31,
      annualReturn: 19.8,
      maxDrawdown: -5.4,
      winRate: 78
    }
  },
  {
    id: 'buy-the-dip',
    title: 'Buy the Dip',
    description: 'Systematic dip buying with risk management and trend filters.',
    color: 'red',
    tags: ['mean-reversion', 'dip-buying', 'swing', 'contrarian', 'beginner', 'systematic', 'bull-market', 'SPY', 'QQQ', 'ETF'],
    creator: 'dipbuyerxyz',
    metrics: {
      sharpe: 1.52,
      annualReturn: 31.7,
      maxDrawdown: -16.8,
      winRate: 64
    }
  },
  {
    id: 'ma-ribbon',
    title: 'MA Ribbon',
    description: 'Multiple moving averages create a trend-following ribbon system.',
    color: 'teal',
    tags: ['trending', 'MA-ribbon', 'multi-timeframe', 'swing', 'intermediate', 'systematic', 'momentum', 'S&P-500', 'NASDAQ', 'SPY', 'QQQ'],
    metrics: {
      sharpe: 1.89,
      annualReturn: 25.1,
      maxDrawdown: -10.3,
      winRate: 62
    }
  },
  {
    id: 'fibonacci-retrace',
    title: 'Fibonacci Retracement',
    description: 'Trades pullbacks to key Fibonacci retracement levels.',
    color: 'indigo',
    tags: ['mean-reversion', 'fibonacci', 'technical', 'swing', 'intermediate', 'pullback', 'support-resistance', 'S&P-500', 'NASDAQ', 'individual-stocks'],
    metrics: {
      sharpe: 1.73,
      annualReturn: 23.9,
      maxDrawdown: -12.1,
      winRate: 67
    }
  },
  {
    id: 'iron-condor',
    title: 'Iron Condor',
    description: 'Options strategy profiting from low volatility and time decay.',
    color: 'pink',
    tags: ['volatility', 'options', 'theta-decay', 'range-bound', 'advanced', 'premium-selling', 'market-neutral', 'SPX', 'RUT', 'NDX', 'index-options'],
    metrics: {
      sharpe: 2.17,
      annualReturn: 18.4,
      maxDrawdown: -7.3,
      winRate: 82
    }
  },
  {
    id: 'news-sentiment',
    title: 'News Sentiment',
    description: 'NLP-driven strategy using real-time news sentiment analysis.',
    color: 'cyan',
    tags: ['sentiment', 'NLP', 'news', 'event-driven', 'intraday', 'alternative-data', 'advanced', 'systematic', 'S&P-500', 'individual-stocks'],
    comingSoon: true
  }
];

const cryptoStrategies: Strategy[] = [
  {
    id: 'crypto-arbitrage',
    title: 'Crypto Arbitrage',
    description: 'Cross-exchange arbitrage capturing price differences between crypto exchanges.',
    color: 'orange',
    tags: ['crypto', 'arbitrage', 'market-neutral', 'systematic', 'high-frequency', 'bitcoin', 'ethereum', 'advanced', 'BTC', 'ETH', 'multi-exchange'],
    creator: 'cryptoarb',
    metrics: {
      sharpe: 3.12,
      annualReturn: 45.8,
      maxDrawdown: -4.2,
      winRate: 89
    }
  },
  {
    id: 'defi-yield-farming',
    title: 'DeFi Yield Farming',
    description: 'Automated yield optimization across DeFi protocols and liquidity pools.',
    color: 'green',
    tags: ['crypto', 'DeFi', 'yield-farming', 'liquidity', 'ethereum', 'position', 'moderate-risk', 'advanced', 'ETH', 'USDC', 'USDT', 'stablecoins'],
    creator: 'defifarmer',
    metrics: {
      sharpe: 2.67,
      annualReturn: 78.4,
      maxDrawdown: -23.1,
      winRate: 76
    }
  },
  {
    id: 'bitcoin-halving',
    title: 'Bitcoin Halving Cycle',
    description: 'Long-term strategy based on Bitcoin halving cycles and market psychology.',
    color: 'indigo',
    tags: ['crypto', 'bitcoin', 'halving', 'cycle', 'position', 'long-term', 'macro', 'beginner', 'BTC', 'bitcoin-only'],
    creator: 'hodlmaster',
    metrics: {
      sharpe: 1.95,
      annualReturn: 127.3,
      maxDrawdown: -45.8,
      winRate: 71
    }
  },
  {
    id: 'altcoin-momentum',
    title: 'Altcoin Momentum',
    description: 'Momentum strategy for high-beta altcoins during bull market phases.',
    color: 'red',
    tags: ['crypto', 'altcoin', 'momentum', 'high-risk', 'bull-market', 'swing', 'aggressive', 'volatile', 'ETH', 'SOL', 'ADA', 'MATIC', 'altcoins'],
    metrics: {
      sharpe: 1.43,
      annualReturn: 89.2,
      maxDrawdown: -67.4,
      winRate: 58
    }
  }
];

const forexStrategies: Strategy[] = [
  {
    id: 'carry-trade',
    title: 'Currency Carry Trade',
    description: 'Profits from interest rate differentials between currency pairs.',
    color: 'purple',
    tags: ['forex', 'carry-trade', 'interest-rates', 'position', 'macro', 'fundamental', 'conservative', 'systematic', 'EUR-USD', 'GBP-USD', 'USD-JPY', 'AUD-USD'],
    metrics: {
      sharpe: 2.08,
      annualReturn: 22.7,
      maxDrawdown: -8.9,
      winRate: 74
    }
  },
  {
    id: 'london-breakout',
    title: 'London Breakout',
    description: 'Trades volatility expansion during London market opening hours.',
    color: 'teal',
    tags: ['forex', 'breakout', 'london-session', 'intraday', 'volatility', 'timezone', 'moderate-risk', 'systematic', 'EUR-USD', 'GBP-USD', 'USD-JPY'],
    metrics: {
      sharpe: 1.89,
      annualReturn: 28.4,
      maxDrawdown: -11.7,
      winRate: 63
    }
  }
];

const commoditiesStrategies: Strategy[] = [
  {
    id: 'gold-volatility',
    title: 'Gold Volatility',
    description: 'Trades gold price volatility during economic uncertainty periods.',
    color: 'orange',
    tags: ['commodities', 'gold', 'volatility', 'safe-haven', 'macro', 'swing', 'moderate-risk', 'hedging', 'GLD', 'GOLD', 'IAU', 'gold-ETFs'],
    metrics: {
      sharpe: 1.76,
      annualReturn: 19.8,
      maxDrawdown: -9.2,
      winRate: 69
    }
  },
  {
    id: 'oil-contango',
    title: 'Oil Contango',
    description: 'Profits from oil futures contango and backwardation patterns.',
    color: 'red',
    tags: ['commodities', 'oil', 'futures', 'contango', 'calendar-spreads', 'position', 'advanced', 'systematic', 'USO', 'OIL', 'UCO', 'oil-ETFs', 'WTI', 'Brent'],
    metrics: {
      sharpe: 2.31,
      annualReturn: 24.6,
      maxDrawdown: -7.8,
      winRate: 78
    }
  }
];

interface TearsheetData {
  strategy: Strategy;
  isOpen: boolean;
}

type SortBy = 'sharpe' | 'returns' | 'name' | 'winrate';

export const ExplorePage: React.FC = () => {
  const navigate = useNavigate();
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedStrategy, setSelectedStrategy] = useState<string | null>(null);
  const [tearsheet, setTearsheet] = useState<TearsheetData>({ strategy: null as any, isOpen: false });
  const [hoveredCard, setHoveredCard] = useState<string | null>(null);
  const [sortBy, setSortBy] = useState<SortBy>('sharpe');
  const [searchTerms, setSearchTerms] = useState<string[]>([]);
  const [displayLimit, setDisplayLimit] = useState(18);

  const handleTagClick = (tag: string) => {
    setSearchTerms(prev => {
      if (prev.includes(tag)) {
        // Remove tag if already selected
        return prev.filter(t => t !== tag);
      } else {
        // Add tag if not already selected
        return [...prev, tag];
      }
    });
  };

  const handleStrategySelect = (strategy: Strategy) => {
    if (!strategy.comingSoon) {
      if (strategy.id === 'custom') {
        // Navigate to Research page with builder open
        navigate('/research', { state: { openBuilder: true } });
      } else {
        setTearsheet({ strategy, isOpen: true });
      }
    }
  };

  const handleNotebookClick = (e: React.MouseEvent, strategy: Strategy) => {
    e.stopPropagation();
    // Store strategy in context or state for research page
    navigate('/research', { state: { strategy } });
  };

  const handleDeployClick = (e: React.MouseEvent, strategy: Strategy) => {
    e.stopPropagation();
    // Store strategy in context or state for monitor page
    navigate('/monitor', { state: { strategy } });
  };

  const handleOpenBuilder = () => {
    // Navigate to Research page with builder open
    navigate('/research', { state: { openBuilder: true } });
  };

  // Combine all strategies
  const allStrategies = [
    ...coreStrategies,
    ...statisticalStrategies,
    ...mlStrategies,
    ...additionalStrategies,
    ...cryptoStrategies,
    ...forexStrategies,
    ...commoditiesStrategies
  ];

  const filterAndSortStrategies = () => {
    let filtered = allStrategies;

    // Multi-tag filter
    const allSearchTerms = [...searchTerms];
    if (searchQuery.trim()) {
      allSearchTerms.push(...searchQuery.toLowerCase().split(' ').filter(term => term.length > 0));
    }

    if (allSearchTerms.length > 0) {
      filtered = filtered.filter(strategy => {
        const searchableText = [
          strategy.title.toLowerCase(),
          strategy.description.toLowerCase(),
          ...strategy.tags.map(tag => tag.toLowerCase())
        ];
        
        // Add creator to searchable text if it exists
        if (strategy.creator) {
          searchableText.push(strategy.creator.toLowerCase());
          searchableText.push(`@${strategy.creator.toLowerCase()}`); // Support @username format
        }
        
        return allSearchTerms.every(term => {
          const cleanTerm = term.startsWith('@') ? term.slice(1) : term; // Remove @ prefix for matching
          return searchableText.some(text => {
            if (term.startsWith('@')) {
              // For @username searches, match against creator field specifically
              return strategy.creator && (strategy.creator.toLowerCase().includes(cleanTerm.toLowerCase()) || text === term);
            } else {
              // Regular search
              return text.includes(cleanTerm.toLowerCase());
            }
          });
        });
      });
    }

    // Sort
    filtered.sort((a, b) => {
      switch (sortBy) {
        case 'sharpe':
          return (b.metrics?.sharpe || 0) - (a.metrics?.sharpe || 0);
        case 'returns':
          return (b.metrics?.annualReturn || 0) - (a.metrics?.annualReturn || 0);
        case 'winrate':
          return (b.metrics?.winRate || 0) - (a.metrics?.winRate || 0);
        case 'name':
          return a.title.localeCompare(b.title);
        default:
          return 0;
      }
    });

    return filtered;
  };

  const renderStrategyCard = (strategy: any) => {
    const cardClasses = `${styles.strategyCard} ${styles[`color${strategy.color.charAt(0).toUpperCase() + strategy.color.slice(1)}`]} ${strategy.comingSoon ? styles.comingSoon : ''}`;
    const isHovered = hoveredCard === strategy.id;
    
    return (
      <div
        key={strategy.id}
        className={cardClasses}
        onClick={() => handleStrategySelect(strategy)}
        onMouseEnter={() => setHoveredCard(strategy.id)}
        onMouseLeave={() => setHoveredCard(null)}
        style={{ cursor: strategy.comingSoon ? 'not-allowed' : 'pointer' }}
      >
        {strategy.comingSoon && (
          <span className={styles.comingSoonBadge}>Soon</span>
        )}
        
        <div className={styles.cardContent}>
          <h3 className={styles.strategyTitle}>{strategy.title}</h3>
          {strategy.creator && (
            <div className={styles.creatorInfo}>
              <span className={styles.creatorLabel}>by</span>
              <button 
                className={styles.creatorName}
                onClick={(e) => {
                  e.stopPropagation();
                  handleTagClick(`@${strategy.creator}`);
                }}
                title={`Search for strategies by @${strategy.creator}`}
              >
                @{strategy.creator}
              </button>
            </div>
          )}
          
          {strategy.metrics && (
            <div className={styles.compactMetrics}>
              <div className={styles.primaryMetric}>
                <span className={styles.primaryValue}>{strategy.metrics.sharpe.toFixed(2)}</span>
                <span className={styles.primaryLabel}>Sharpe</span>
              </div>
              <div className={styles.secondaryMetrics}>
                <span className={styles.secondaryMetric}>{strategy.metrics.annualReturn.toFixed(0)}%</span>
                <span className={styles.secondaryMetric}>{strategy.metrics.winRate}%</span>
              </div>
            </div>
          )}
          
          <div className={styles.cardFooter}>
            {strategy.tags.slice(0, 3).map(tag => (
              <button 
                key={tag} 
                className={`${styles.compactTag} ${searchTerms.includes(tag) ? styles.activeTag : ''}`}
                onClick={(e) => {
                  e.stopPropagation();
                  handleTagClick(tag);
                }}
              >
                {tag}
              </button>
            ))}
          </div>
        </div>
        
        {isHovered && !strategy.comingSoon && (
          <div className={styles.hoverOverlay}>
            <button 
              className={styles.overlayBtn}
              onClick={(e) => handleNotebookClick(e, strategy)}
              title="Research"
            >
              <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" style={{ width: '20px', height: '20px' }}>
                {/* Spiral binding */}
                <circle cx="4" cy="4" r="1.5"></circle>
                <circle cx="4" cy="8" r="1.5"></circle>
                <circle cx="4" cy="12" r="1.5"></circle>
                <circle cx="4" cy="16" r="1.5"></circle>
                <circle cx="4" cy="20" r="1.5"></circle>
                {/* Notebook pages */}
                <rect x="7" y="2" width="14" height="20" rx="1"></rect>
                {/* Lines on pages */}
                <line x1="10" y1="7" x2="18" y2="7"></line>
                <line x1="10" y1="11" x2="18" y2="11"></line>
                <line x1="10" y1="15" x2="16" y2="15"></line>
              </svg>
            </button>
            <button 
              className={styles.overlayBtn}
              onClick={(e) => handleDeployClick(e, strategy)}
              title="Deploy"
            >
              <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" style={{ width: '20px', height: '20px' }}>
                {/* Rocket/Deploy icon */}
                <path d="M4.5 16.5c-1.5 1.26-2 5-2 5s3.74-.5 5-2c.71-.84.7-2.13-.09-2.91a2.18 2.18 0 0 0-2.91-.09z"></path>
                <path d="M12 15l-3-3a22 22 0 0 1 2-3.95A12.88 12.88 0 0 1 22 2c0 2.72-.78 7.5-6 11a22.35 22.35 0 0 1-4 2z"></path>
                <path d="M9 12H4s.55-3.03 2-4c1.62-1.08 5 0 5 0"></path>
                <path d="M12 15v5s3.03-.55 4-2c1.08-1.62 0-5 0-5"></path>
              </svg>
            </button>
          </div>
        )}
      </div>
    );
  };

  return (
    <div className={styles.exploreContainer}>
      <div className={styles.catalogueContainer}>
        <div className={styles.controlsBar}>
          <div className={styles.filterControls}>
            <div className={styles.sortButtons}>
              <button 
                className={`${styles.sortBtn} ${sortBy === 'sharpe' ? styles.active : ''}`}
                onClick={() => setSortBy('sharpe')}
              >
                Sharpe
              </button>
              <button 
                className={`${styles.sortBtn} ${sortBy === 'returns' ? styles.active : ''}`}
                onClick={() => setSortBy('returns')}
              >
                Returns
              </button>
              <button 
                className={`${styles.sortBtn} ${sortBy === 'winrate' ? styles.active : ''}`}
                onClick={() => setSortBy('winrate')}
              >
                Win %
              </button>
              <button 
                className={`${styles.sortBtn} ${sortBy === 'name' ? styles.active : ''}`}
                onClick={() => setSortBy('name')}
              >
                A-Z
              </button>
            </div>
          </div>
          
          <div className={styles.searchWrapper}>
            <div className={styles.searchContainer}>
              <input
                type="text"
                placeholder="search strategies... (e.g., trending swing @alexchen)"
                className={styles.searchInput}
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
              />
              {searchTerms.length > 0 && (
                <div className={styles.activeFilters}>
                  {searchTerms.map(term => (
                    <button
                      key={term}
                      className={styles.filterChip}
                      onClick={() => handleTagClick(term)}
                    >
                      {term} ×
                    </button>
                  ))}
                </div>
              )}
            </div>
            <button
              className={styles.addButton}
              onClick={handleOpenBuilder}
            >
              +
            </button>
          </div>
        </div>

        {/* Results Count */}
        <div className={styles.resultsInfo}>
          <span className={styles.resultsCount}>
            Showing {Math.min(displayLimit, filterAndSortStrategies().length)} of {filterAndSortStrategies().length} strategies
          </span>
          {searchTerms.length > 0 && (
            <span className={styles.filterInfo}>
              • Filtered by: {searchTerms.join(', ')}
            </span>
          )}
        </div>

        {/* Strategy Grid */}
        <div className={styles.strategyGrid}>
          {filterAndSortStrategies().slice(0, displayLimit).map(renderStrategyCard)}
        </div>
        
        {/* Load More / Pagination */}
        {filterAndSortStrategies().length > displayLimit && (
          <div className={styles.loadMoreContainer}>
            <button 
              className={styles.loadMoreBtn}
              onClick={() => setDisplayLimit(prev => prev + 12)}
            >
              Load More ({filterAndSortStrategies().length - displayLimit} remaining)
            </button>
            <button 
              className={styles.showAllBtn}
              onClick={() => setDisplayLimit(filterAndSortStrategies().length)}
            >
              Show All
            </button>
          </div>
        )}
        
        {filterAndSortStrategies().length === 0 && (
          <div className={styles.emptyState}>
            <p>No strategies found</p>
            <p className={styles.emptyHint}>Try different search terms like "crypto", "low-risk", "intraday", or "@username"</p>
          </div>
        )}

        {/* Tearsheet Modal */}
        {tearsheet.isOpen && tearsheet.strategy && (
          <div className={styles.tearsheetModal} onClick={() => setTearsheet({ ...tearsheet, isOpen: false })}>
            <div className={styles.tearsheetContent} onClick={(e) => e.stopPropagation()}>
              <button className={styles.tearsheetClose} onClick={() => setTearsheet({ ...tearsheet, isOpen: false })}>×</button>
              <h2 className={styles.tearsheetTitle}>{tearsheet.strategy.title}</h2>
              
              <div className={styles.tearsheetMetrics}>
                <div className={styles.tearsheetMetric}>
                  <span className={styles.tearsheetMetricValue}>{tearsheet.strategy.metrics?.sharpe.toFixed(2)}</span>
                  <span className={styles.tearsheetMetricLabel}>Sharpe Ratio</span>
                </div>
                <div className={styles.tearsheetMetric}>
                  <span className={styles.tearsheetMetricValue}>{tearsheet.strategy.metrics?.annualReturn.toFixed(1)}%</span>
                  <span className={styles.tearsheetMetricLabel}>Annual Return</span>
                </div>
                <div className={styles.tearsheetMetric}>
                  <span className={styles.tearsheetMetricValue}>{tearsheet.strategy.metrics?.maxDrawdown.toFixed(1)}%</span>
                  <span className={styles.tearsheetMetricLabel}>Max Drawdown</span>
                </div>
                <div className={styles.tearsheetMetric}>
                  <span className={styles.tearsheetMetricValue}>{tearsheet.strategy.metrics?.winRate}%</span>
                  <span className={styles.tearsheetMetricLabel}>Win Rate</span>
                </div>
              </div>
              
              <div className={styles.tearsheetChart}>
                <div className={styles.chartHeader}>
                  <h3 className={styles.chartTitle}>Equity Curve</h3>
                  <div className={styles.chartStats}>
                    <span className={styles.chartStat}>+{tearsheet.strategy.metrics?.annualReturn.toFixed(1)}% Annual</span>
                    <span className={styles.chartStat}>Max DD: {tearsheet.strategy.metrics?.maxDrawdown.toFixed(1)}%</span>
                  </div>
                </div>
                <div className={styles.equityCurveContainer}>
                  <svg className={styles.equityCurve} viewBox="0 0 400 150" preserveAspectRatio="xMidYMid meet">
                    {/* Grid lines */}
                    <defs>
                      <pattern id="grid" width="40" height="30" patternUnits="userSpaceOnUse">
                        <path d="M 40 0 L 0 0 0 30" fill="none" stroke="var(--color-border-primary)" strokeWidth="0.5" opacity="0.3"/>
                      </pattern>
                    </defs>
                    <rect width="100%" height="100%" fill="url(#grid)" />
                    
                    {/* Generate mock equity curve based on strategy metrics */}
                    <path
                      d={(() => {
                        const points = 50;
                        const width = 400;
                        const height = 150;
                        const padding = 20;
                        
                        // Generate realistic equity curve based on strategy metrics
                        const annualReturn = tearsheet.strategy.metrics?.annualReturn || 20;
                        const maxDD = Math.abs(tearsheet.strategy.metrics?.maxDrawdown || -10);
                        const sharpe = tearsheet.strategy.metrics?.sharpe || 1.5;
                        
                        let equity = 100; // Start at 100%
                        let maxEquity = 100;
                        const equityPath: number[] = [];
                        
                        for (let i = 0; i < points; i++) {
                          const t = i / (points - 1);
                          
                          // Base trend (annual return)
                          const trend = 100 + (annualReturn * t);
                          
                          // Add volatility (inverse of Sharpe)
                          const volatility = (annualReturn / Math.max(sharpe, 0.5)) * 0.3;
                          const noise = (Math.sin(i * 0.5) + Math.sin(i * 0.3) * 0.5) * volatility;
                          
                          // Add drawdown periods
                          let ddAdjustment = 0;
                          if (t > 0.3 && t < 0.5) { // Early drawdown
                            ddAdjustment = -maxDD * 0.6 * Math.sin((t - 0.3) * Math.PI / 0.2);
                          }
                          if (t > 0.7 && t < 0.8) { // Later drawdown
                            ddAdjustment = -maxDD * 0.4 * Math.sin((t - 0.7) * Math.PI / 0.1);
                          }
                          
                          equity = trend + noise + ddAdjustment;
                          equityPath.push(equity);
                          maxEquity = Math.max(maxEquity, equity);
                        }
                        
                        // Normalize and scale to chart
                        const minEquity = Math.min(...equityPath);
                        const range = maxEquity - minEquity;
                        
                        return equityPath.map((eq, i) => {
                          const x = padding + (i / (points - 1)) * (width - 2 * padding);
                          const y = padding + (1 - (eq - minEquity) / range) * (height - 2 * padding);
                          return i === 0 ? `M ${x} ${y}` : `L ${x} ${y}`;
                        }).join(' ');
                      })()}
                      fill="none"
                      stroke="var(--color-accent-primary)"
                      strokeWidth="3"
                      strokeLinecap="round"
                      strokeLinejoin="round"
                    />
                    
                    {/* Y-axis labels */}
                    <text x="10" y="25" fill="var(--color-text-secondary)" fontSize="10" fontFamily="var(--font-family-mono)">+{(tearsheet.strategy.metrics?.annualReturn || 20).toFixed(0)}%</text>
                    <text x="10" y="75" fill="var(--color-text-secondary)" fontSize="10" fontFamily="var(--font-family-mono)">0%</text>
                    <text x="10" y="135" fill="var(--color-text-secondary)" fontSize="10" fontFamily="var(--font-family-mono)">{tearsheet.strategy.metrics?.maxDrawdown.toFixed(0)}%</text>
                    
                    {/* X-axis labels */}
                    <text x="30" y="145" fill="var(--color-text-secondary)" fontSize="10" fontFamily="var(--font-family-mono)">Start</text>
                    <text x="180" y="145" fill="var(--color-text-secondary)" fontSize="10" fontFamily="var(--font-family-mono)">1Y</text>
                    <text x="350" y="145" fill="var(--color-text-secondary)" fontSize="10" fontFamily="var(--font-family-mono)">2Y</text>
                  </svg>
                </div>
              </div>
              
              <div className={styles.tearsheetActions}>
                <button 
                  className={styles.tearsheetBtnIcon}
                  onClick={() => {
                    setTearsheet({ ...tearsheet, isOpen: false });
                    navigate('/research', { state: { strategy: tearsheet.strategy } });
                  }}
                  title="Open in Research"
                >
                  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" style={{ width: '20px', height: '20px' }}>
                    {/* Spiral binding */}
                    <circle cx="4" cy="4" r="1.5"></circle>
                    <circle cx="4" cy="8" r="1.5"></circle>
                    <circle cx="4" cy="12" r="1.5"></circle>
                    <circle cx="4" cy="16" r="1.5"></circle>
                    <circle cx="4" cy="20" r="1.5"></circle>
                    {/* Notebook pages */}
                    <rect x="7" y="2" width="14" height="20" rx="1"></rect>
                    {/* Lines on pages */}
                    <line x1="10" y1="7" x2="18" y2="7"></line>
                    <line x1="10" y1="11" x2="18" y2="11"></line>
                    <line x1="10" y1="15" x2="16" y2="15"></line>
                  </svg>
                </button>
                <button 
                  className={styles.tearsheetBtnIcon}
                  onClick={() => {
                    setTearsheet({ ...tearsheet, isOpen: false });
                    navigate('/monitor', { state: { strategy: tearsheet.strategy } });
                  }}
                  title="Deploy Strategy"
                >
                  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" style={{ width: '20px', height: '20px' }}>
                    {/* Rocket/Deploy icon */}
                    <path d="M4.5 16.5c-1.5 1.26-2 5-2 5s3.74-.5 5-2c.71-.84.7-2.13-.09-2.91a2.18 2.18 0 0 0-2.91-.09z"></path>
                    <path d="M12 15l-3-3a22 22 0 0 1 2-3.95A12.88 12.88 0 0 1 22 2c0 2.72-.78 7.5-6 11a22.35 22.35 0 0 1-4 2z"></path>
                    <path d="M9 12H4s.55-3.03 2-4c1.62-1.08 5 0 5 0"></path>
                    <path d="M12 15v5s3.03-.55 4-2c1.08-1.62 0-5 0-5"></path>
                  </svg>
                </button>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};