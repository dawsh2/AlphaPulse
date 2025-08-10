import React, { useState, useEffect } from 'react';
import { useLocation, useNavigate } from 'react-router-dom';
import styles from '../../../pages/ResearchPage.module.css';
import exploreStyles from '../../../pages/ExplorePage.module.css';
import { StrategyWorkbench } from '../../StrategyBuilder/StrategyWorkbench';
import Editor from '@monaco-editor/react';
import * as monaco from 'monaco-editor';
import { dataStorage } from '../../../services/data';
import type { DatasetInfo } from '../../../services/data';

// Types
interface CodeSnippet {
  id: string;
  name: string;
  code: string;
  description?: string;
}

interface NotebookTemplate {
  id: string;
  title: string;
  description: string;
  cells: NotebookCell[];
}

interface AiMessage {
  role: 'assistant' | 'user';
  content: string;
  timestamp?: string;
}

interface NotebookCell {
  id: string;
  type: 'code' | 'markdown' | 'ai-chat';
  content: string;
  output?: string;
  isExecuting?: boolean;
  showAiAnalysis?: boolean;
  isAiChat?: boolean;
  parentCellId?: string;
  aiMessages?: AiMessage[];
  chatInput?: string;
}

interface SavedNotebook {
  id: string;
  name: string;
  lastModified: string;
  cells: NotebookCell[];
}

interface Strategy {
  id: string;
  title: string;
  description: string;
  color: string;
  tags: string[];
  creator?: string;
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

interface TearsheetData {
  strategy: Strategy;
  isOpen: boolean;
}

type SidebarTab = 'builder' | 'notebooks';
type MainView = 'explore' | 'notebook' | 'builder' | 'data';
type SortBy = 'new' | 'sharpe' | 'returns' | 'name' | 'winrate';

// Strategy data - matching ExplorePage
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
      sharpe: 1.58,
      annualReturn: 28.9,
      maxDrawdown: -14.2,
      winRate: 61
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

const ResearchPage: React.FC = () => {
  const location = useLocation();
  const navigate = useNavigate();
  
  // State management
  const [activeTab, setActiveTab] = useState<SidebarTab>('builder');
  const [mainView, setMainView] = useState<MainView>('explore');
  const [isBuilderOpen, setIsBuilderOpen] = useState(false);
  const [selectedTemplate, setSelectedTemplate] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState('');
  const [collapsedCategories, setCollapsedCategories] = useState<Set<string>>(new Set());
  const [notebookCells, setNotebookCells] = useState<NotebookCell[]>([]);
  const [activeCell, setActiveCell] = useState<string | null>(null);
  const [sidebarOpen, setSidebarOpen] = useState(false); // Mobile sidebar state
  const [isMobile, setIsMobile] = useState(window.innerWidth <= 768);
  const [touchStart, setTouchStart] = useState<number | null>(null);
  const [touchEnd, setTouchEnd] = useState<number | null>(null);
  const [datasets, setDatasets] = useState<DatasetInfo[]>([]);
  const [loadingDatasets, setLoadingDatasets] = useState(false);
  // Initialize with correct theme detection
  const [theme, setTheme] = useState(() => {
    const isDark = document.documentElement.getAttribute('data-theme') === 'dark' ||
                   (!document.documentElement.getAttribute('data-theme') && 
                    window.matchMedia('(prefers-color-scheme: dark)').matches);
    return isDark ? 'vs-dark' : 'cream-light';
  });
  
  // Explore page state
  const [exploreSearchQuery, setExploreSearchQuery] = useState('');
  const [selectedStrategy, setSelectedStrategy] = useState<string | null>(null);
  const [tearsheet, setTearsheet] = useState<TearsheetData>({ strategy: null as any, isOpen: false });
  const [hoveredCard, setHoveredCard] = useState<string | null>(null);
  const [sortBy, setSortBy] = useState<SortBy>('sharpe');
  const [searchTerms, setSearchTerms] = useState<string[]>([]);
  const [displayLimit, setDisplayLimit] = useState(18);
  const [sortDropdownOpen, setSortDropdownOpen] = useState(false);

  // Mock data for notebooks
  const codeSnippets: Record<string, CodeSnippet[]> = {
    'Data Loading': [
      {
        id: 'load_signals',
        name: 'Load Signals',
        code: `import admf\n\n# Load signals with filtering\nsignals = admf.load_signals(\n    strategy_type='bollinger_bands',\n    min_sharpe=1.0,\n    symbols=['AAPL', 'MSFT']\n)\nprint(f"Loaded {len(signals)} signal traces")`,
        description: 'Load strategy signals from ADMF registry'
      },
      {
        id: 'load_executions',
        name: 'Load Executions',
        code: `# Load execution data\nexecutions = admf.load_executions(\n    signal_hash='sig_a7f8d9e6',\n    include_trades=True\n)\nprint(f"Found {len(executions)} execution records")`,
        description: 'Load execution data for analysis'
      }
    ],
    'Performance Metrics': [
      {
        id: 'performance_table',
        name: 'Performance Table',
        code: `from analysis_lib import performance_table\n\n# Generate comprehensive performance metrics\nmetrics = performance_table(signals)\nmetrics.sort_values('sharpe_ratio', ascending=False).head(10)`,
        description: 'Calculate key performance metrics'
      },
      {
        id: 'sharpe_calculation',
        name: 'Sharpe Ratio',
        code: `# Calculate Sharpe ratio\ndef calculate_sharpe_ratio(returns, risk_free_rate=0.02):\n    excess_returns = returns - risk_free_rate / 252\n    return excess_returns.mean() / excess_returns.std() * np.sqrt(252)\n\nsharpe = calculate_sharpe_ratio(strategy_returns)\nprint(f"Sharpe Ratio: {sharpe:.2f}")`,
        description: 'Calculate annualized Sharpe ratio'
      }
    ],
    'Visualizations': [
      {
        id: 'equity_curves',
        name: 'Equity Curves',
        code: `import matplotlib.pyplot as plt\nfrom analysis_lib import plot_equity_curves\n\n# Plot multiple strategy equity curves\nfig = plot_equity_curves(\n    signals,\n    benchmark='SPY',\n    title='Strategy Performance Comparison'\n)\nfig.show()`,
        description: 'Plot strategy equity curves with benchmark'
      }
    ]
  };

  const notebookTemplates: NotebookTemplate[] = [
    {
      id: 'strategy_comparison',
      title: 'Strategy Comparison Analysis',
      description: 'Compare multiple strategies across key performance metrics',
      cells: [
        {
          id: 'cell-1',
          type: 'markdown',
          content: '# Strategy Comparison Analysis\n\nComparing multiple strategies across key performance metrics and risk characteristics.'
        },
        {
          id: 'cell-2',
          type: 'code',
          content: `import admf\nfrom analysis_lib import *\n\n# Load strategies to compare\nstrategies = admf.load_signals(['momentum', 'mean_reversion'], min_sharpe=1.0)\nprint(f"Loaded {len(strategies)} strategies for comparison")`
        }
      ]
    },
    {
      id: 'performance_summary',
      title: 'Complete Performance Analysis',
      description: 'Comprehensive analysis of strategy performance',
      cells: [
        {
          id: 'cell-1',
          type: 'markdown',
          content: '# Performance Summary Report\n\nComprehensive analysis of strategy performance including returns, risk metrics, and trade statistics.'
        }
      ]
    }
  ];

  const savedNotebooks: SavedNotebook[] = [
    {
      id: 'notebook-1',
      name: 'NVDA Momentum Analysis',
      lastModified: '2025-01-15',
      cells: []
    },
    {
      id: 'notebook-2',
      name: 'Portfolio Optimization',
      lastModified: '2025-01-14',
      cells: []
    }
  ];

  // Theme detection
  useEffect(() => {
    // Check if monaco is available before defining theme
    if (typeof monaco !== 'undefined' && monaco.editor) {
      // Define the cream theme once
      monaco.editor.defineTheme('cream-light', {
        base: 'vs',
        inherit: true,
        rules: [],
        colors: {
          'editor.background': '#faf7f0', // Cream/eggshell color
          'editor.foreground': '#33332d',
          'editor.lineHighlightBackground': '#f5f2ea',
          'editor.selectionBackground': '#e5e0d5',
          'editorCursor.foreground': '#33332d',
          'editorLineNumber.foreground': '#8b8680',
          'editorLineNumber.activeForeground': '#33332d'
        }
      });
    }
    
    // Detect current theme
    const updateTheme = () => {
      const isDark = document.documentElement.getAttribute('data-theme') === 'dark' ||
                     (!document.documentElement.getAttribute('data-theme') && 
                      window.matchMedia('(prefers-color-scheme: dark)').matches);
      
      setTheme(isDark ? 'vs-dark' : 'cream-light');
    };
    
    updateTheme();
    
    // Listen for theme changes
    const observer = new MutationObserver(updateTheme);
    observer.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ['data-theme']
    });
    
    return () => observer.disconnect();
  }, []);

  // Initialize with default notebook cells
  useEffect(() => {
    setNotebookCells([
      {
        id: 'cell-1',
        type: 'markdown',
        content: '# Research Notebook\n\nWelcome to the AlphaPulse research environment. Use the sidebar to access code snippets, templates, and saved notebooks.'
      },
      {
        id: 'cell-2',
        type: 'code',
        content: `import admf\nimport pandas as pd\nimport numpy as np\nfrom analysis_lib import *\n\n# Load sample data\nsignals = admf.load_signals(strategy_type='ema_cross', limit=5)\nprint(f"Loaded {len(signals)} signal traces for analysis")`
      }
    ]);
  }, []);

  // Handle window resize for mobile detection
  // Load datasets when data tab is active
  useEffect(() => {
    if (mainView === 'data' && datasets.length === 0) {
      setLoadingDatasets(true);
      dataStorage.getDatasets()
        .then(data => {
          setDatasets(data);
          setLoadingDatasets(false);
        })
        .catch(error => {
          console.error('Failed to load datasets:', error);
          setLoadingDatasets(false);
        });
    }
  }, [mainView]);

  useEffect(() => {
    const handleResize = () => {
      setIsMobile(window.innerWidth <= 768);
    };

    window.addEventListener('resize', handleResize);
    return () => window.removeEventListener('resize', handleResize);
  }, []);

  // Touch event handlers for swipe gestures
  const handleTouchStart = (e: React.TouchEvent) => {
    setTouchEnd(null);
    setTouchStart(e.targetTouches[0].clientY);
  };

  const handleTouchMove = (e: React.TouchEvent) => {
    setTouchEnd(e.targetTouches[0].clientY);
  };

  const handleTouchEnd = () => {
    if (!touchStart || !touchEnd) return;
    
    const distance = touchStart - touchEnd;
    const isSwipeUp = distance > 50;
    const isSwipeDown = distance < -50;
    
    if (isSwipeUp && !sidebarOpen && isMobile) {
      // Swipe up to open sidebar
      setSidebarOpen(true);
    } else if (isSwipeDown && sidebarOpen && isMobile) {
      // Swipe down to close sidebar
      setSidebarOpen(false);
    }
  };

  // Check if opened from Explore page with strategy data or builder request
  useEffect(() => {
    if (location.state?.strategy) {
      const strategy = location.state.strategy;
      const analysisCell: NotebookCell = {
        id: `cell-${Date.now()}`,
        type: 'markdown',
        content: `# ${strategy.title} Analysis\n\nAnalyzing strategy: **${strategy.title}**\n\n**Description:** ${strategy.description}\n\n**Creator:** ${strategy.creator ? `@${strategy.creator}` : 'Unknown'}\n\n**Tags:** ${strategy.tags.join(', ')}`
      };
      
      const codeCell: NotebookCell = {
        id: `cell-${Date.now() + 1}`,
        type: 'code',
        content: `# Load strategy data for analysis\nimport admf\n\n# Load ${strategy.title} strategy data\nsignals = admf.load_signals(strategy_id='${strategy.id}')\nprint(f"Loaded strategy: ${strategy.title}")\nprint(f"Expected Sharpe: ${strategy.metrics?.sharpe || 'N/A'}")\nprint(f"Expected Annual Return: ${strategy.metrics?.annualReturn || 'N/A'}%")`
      };
      
      setNotebookCells([analysisCell, codeCell]);
      setActiveTab('notebooks');
      setMainView('notebook');
    } else if (location.state?.openBuilder) {
      setActiveTab('builder');
      setMainView('builder');
    }
  }, [location.state]);

  // Event handlers
  const handleTabSwitch = (tab: SidebarTab) => {
    setActiveTab(tab);
    
    // When builder button is clicked, open default template
    if (tab === 'builder') {
      setMainView('builder');
      // Set a default 'New Strategy' template
      setSelectedTemplate('new_strategy');
    } 
    // When notebook button is clicked, open default notebook template
    else if (tab === 'notebooks') {
      setMainView('notebook');
      // Load a default notebook template
      if (notebookTemplates.length > 0) {
        loadTemplate(notebookTemplates[0]); // Load first template as default
      }
    }
  };
  
  const handleOpenBuilder = () => {
    setActiveTab('builder');
    setMainView('builder');
    setSelectedTemplate('new_strategy');
  };

  const toggleCategory = (category: string) => {
    setCollapsedCategories(prev => {
      const newSet = new Set(prev);
      if (newSet.has(category)) {
        newSet.delete(category);
      } else {
        newSet.add(category);
      }
      return newSet;
    });
  };

  const insertSnippet = (snippet: CodeSnippet) => {
    const newCell: NotebookCell = {
      id: `cell-${Date.now()}`,
      type: 'code',
      content: snippet.code
    };
    setNotebookCells(prev => [...prev, newCell]);
    setMainView('notebook');
  };

  const loadTemplate = (template: NotebookTemplate) => {
    setNotebookCells(template.cells);
    setMainView('notebook');
  };

  const addCell = (type: 'code' | 'markdown', afterId?: string) => {
    const newCell: NotebookCell = {
      id: `cell-${Date.now()}`,
      type,
      content: type === 'markdown' ? '# New Section' : '# Add your code here'
    };

    if (afterId) {
      setNotebookCells(prev => {
        const index = prev.findIndex(cell => cell.id === afterId);
        const newCells = [...prev];
        newCells.splice(index + 1, 0, newCell);
        return newCells;
      });
    } else {
      setNotebookCells(prev => [...prev, newCell]);
    }
  };

  const deleteCell = (cellId: string) => {
    setNotebookCells(prev => prev.filter(cell => cell.id !== cellId));
  };

  const updateCellContent = (cellId: string, content: string) => {
    setNotebookCells(prev => 
      prev.map(cell => 
        cell.id === cellId ? { ...cell, content } : cell
      )
    );
  };

  const executeCell = async (cellId: string) => {
    setNotebookCells(prev => 
      prev.map(cell => 
        cell.id === cellId ? { ...cell, isExecuting: true } : cell
      )
    );

    // Simulate code execution
    await new Promise(resolve => setTimeout(resolve, 1000));

    // Generate different outputs based on cell content
    const cell = notebookCells.find(c => c.id === cellId);
    let output = 'Execution completed successfully.\nOutput: Sample analysis results...';
    
    if (cell?.content.includes('admf.load_signals')) {
      output = `=== Overview ===
Total Strategies Loaded: 3
Time Range: 2023-01-01 to 2025-01-15
Universe: US Equities (S&P 500)

=== Temporal Analysis ===
Best Performing Period: Q2 2024 (+18.5%)
Worst Performing Period: Q3 2023 (-7.2%)
Average Monthly Return: 2.3%
Volatility: 15.8%

=== Performance Metrics ===
Sharpe Ratio: 1.87
Max Drawdown: -12.4%
Win Rate: 62.3%
Profit Factor: 1.92`;
    } else if (cell?.content.includes('plot') || cell?.content.includes('chart')) {
      output = `[Chart Output]
📊 Strategy Performance Chart Generated
- Equity curve plotted with confidence bands
- Drawdown periods highlighted in red
- Key statistics overlaid`;
    } else if (cell?.content.includes('backtest')) {
      output = `=== Backtest Results ===
Total Trades: 142
Winning Trades: 89 (62.7%)
Losing Trades: 53 (37.3%)
Average Win: +3.2%
Average Loss: -1.8%
Expectancy: $1,247 per trade`;
    }

    setNotebookCells(prev => 
      prev.map(cell => 
        cell.id === cellId 
          ? { 
              ...cell, 
              isExecuting: false, 
              output
            } 
          : cell
      )
    );
  };

  // Explore page handlers
  const handleTagClick = (tag: string) => {
    setSearchTerms(prev => {
      if (prev.includes(tag)) {
        return prev.filter(t => t !== tag);
      } else {
        return [...prev, tag];
      }
    });
  };

  const handleStrategySelect = (strategy: Strategy) => {
    if (!strategy.comingSoon) {
      if (strategy.id === 'custom') {
        setActiveTab('builder');
        setMainView('builder');
        setSelectedTemplate('custom');
      } else {
        setTearsheet({ strategy, isOpen: true });
      }
    }
  };

  const handleNotebookClick = (e: React.MouseEvent, strategy: Strategy) => {
    e.stopPropagation();
    
    // On mobile, open the builder view instead of notebook
    if (isMobile) {
      setActiveTab('builder');
      setMainView('builder');
      setSelectedTemplate(strategy.id);
      setSidebarOpen(false); // Close sidebar on mobile after selection
      return;
    }
    
    // Desktop behavior - open notebook
    const analysisCell: NotebookCell = {
      id: `cell-${Date.now()}`,
      type: 'markdown',
      content: `# ${strategy.title} Analysis\n\nAnalyzing strategy: **${strategy.title}**\n\n**Description:** ${strategy.description}\n\n**Creator:** ${strategy.creator ? `@${strategy.creator}` : 'Unknown'}\n\n**Tags:** ${strategy.tags.join(', ')}`
    };
    
    const codeCell: NotebookCell = {
      id: `cell-${Date.now() + 1}`,
      type: 'code',
      content: `# Load strategy data for analysis\nimport admf\n\n# Load ${strategy.title} strategy data\nsignals = admf.load_signals(strategy_id='${strategy.id}')\nprint(f"Loaded strategy: ${strategy.title}")\nprint(f"Expected Sharpe: ${strategy.metrics?.sharpe || 'N/A'}")\nprint(f"Expected Annual Return: ${strategy.metrics?.annualReturn || 'N/A'}%")`
    };
    
    setNotebookCells([analysisCell, codeCell]);
    setActiveTab('notebooks');
    setMainView('notebook');
  };

  const handleDeployClick = (e: React.MouseEvent, strategy: Strategy) => {
    e.stopPropagation();
    navigate('/monitor', { state: { strategy } });
  };

  const filterAndSortStrategies = () => {
    let filtered = allStrategies;

    // Multi-tag filter
    const allSearchTerms = [...searchTerms];
    if (exploreSearchQuery.trim()) {
      allSearchTerms.push(...exploreSearchQuery.toLowerCase().split(' ').filter(term => term.length > 0));
    }

    if (allSearchTerms.length > 0) {
      filtered = filtered.filter(strategy => {
        const searchableText = [
          strategy.title.toLowerCase(),
          strategy.description.toLowerCase(),
          ...strategy.tags.map(tag => tag.toLowerCase())
        ];
        
        if (strategy.creator) {
          searchableText.push(strategy.creator.toLowerCase());
          searchableText.push(`@${strategy.creator.toLowerCase()}`);
        }
        
        return allSearchTerms.every(term => 
          searchableText.some(text => text.includes(term))
        );
      });
    }

    // Sort
    return filtered.sort((a, b) => {
      if (!a.metrics || !b.metrics) return 0;
      
      switch (sortBy) {
        case 'new':
          // Reverse order to show newest first (higher indices first)
          return allStrategies.indexOf(b) - allStrategies.indexOf(a);
        case 'sharpe':
          return b.metrics.sharpe - a.metrics.sharpe;
        case 'returns':
          return b.metrics.annualReturn - a.metrics.annualReturn;
        case 'winrate':
          return b.metrics.winRate - a.metrics.winRate;
        case 'name':
          return a.title.localeCompare(b.title);
        default:
          return 0;
      }
    });
  };

  // Helper function to get random subset of tags and shuffle them
  const getRandomTags = (tags: string[], strategyId: string) => {
    // Use strategy ID as seed for consistent randomization per strategy
    const seed = strategyId.split('').reduce((acc, char) => acc + char.charCodeAt(0), 0);
    const shuffled = [...tags].sort(() => {
      const random = Math.sin(seed) * 10000;
      return random - Math.floor(random) < 0.5 ? -1 : 1;
    });
    
    // Random number of tags between 2 and 4
    const numTags = 2 + (seed % 3);
    return shuffled.slice(0, Math.min(numTags, tags.length));
  };

  const renderStrategyCard = (strategy: Strategy) => {
    const isHovered = hoveredCard === strategy.id;
    const displayTags = getRandomTags(strategy.tags, strategy.id);
    const seed = strategy.id.split('').reduce((acc, char) => acc + char.charCodeAt(0), 0);
    
    return (
      <div
        key={strategy.id}
        className={`${exploreStyles.strategyCard} ${exploreStyles[strategy.color]}`}
        onClick={() => handleStrategySelect(strategy)}
        onMouseEnter={() => setHoveredCard(strategy.id)}
        onMouseLeave={() => setHoveredCard(null)}
        style={{ cursor: strategy.comingSoon ? 'not-allowed' : 'pointer' }}
      >
        {strategy.comingSoon && (
          <span className={exploreStyles.comingSoonBadge}>Soon</span>
        )}
        
        <div className={exploreStyles.cardContent}>
          <h3 className={exploreStyles.strategyTitle}>{strategy.title}</h3>
          {strategy.creator && (
            <div className={exploreStyles.creatorInfo}>
              <span className={exploreStyles.creatorLabel}>by</span>
              <button 
                className={exploreStyles.creatorName}
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
            <div className={exploreStyles.compactMetrics}>
              <div className={exploreStyles.primaryMetric}>
                <span className={exploreStyles.primaryValue}>{strategy.metrics.sharpe.toFixed(2)}</span>
                <span className={exploreStyles.primaryLabel}>Sharpe</span>
              </div>
              <div className={exploreStyles.secondaryMetrics}>
                <span className={exploreStyles.secondaryMetric}>{strategy.metrics.annualReturn.toFixed(0)}%</span>
                <span className={exploreStyles.secondaryMetric}>{strategy.metrics.winRate}%</span>
              </div>
            </div>
          )}
          
          <div className={exploreStyles.cardFooter}>
            {displayTags.map((tag, index) => (
              <button 
                key={tag} 
                className={`${exploreStyles.compactTag} ${exploreStyles[`tagColor${(index + seed) % 8}`]} ${searchTerms.includes(tag) ? exploreStyles.activeTag : ''}`}
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
          <div className={exploreStyles.hoverOverlay}>
            <button 
              className={exploreStyles.overlayBtn}
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
                <line x1="11" y1="6" x2="17" y2="6"></line>
                <line x1="11" y1="10" x2="17" y2="10"></line>
                <line x1="11" y1="14" x2="17" y2="14"></line>
                <line x1="11" y1="18" x2="17" y2="18"></line>
              </svg>
            </button>
            <button 
              className={exploreStyles.overlayBtn}
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

  const renderSidebarContent = () => {
    // When in explore view, show strategy directory
    if (mainView === 'explore') {
      const strategies = filterAndSortStrategies();
      const strategyCategories = {
        'Trending': strategies.filter(s => s.tags.includes('trending')),
        'Mean Reversion': strategies.filter(s => s.tags.includes('mean-reversion')),
        'Momentum': strategies.filter(s => s.tags.includes('momentum')),
        'Machine Learning': strategies.filter(s => s.tags.includes('ml')),
        'High Frequency': strategies.filter(s => s.tags.includes('high-frequency')),
        'Options': strategies.filter(s => s.tags.includes('options')),
        'Crypto': strategies.filter(s => s.tags.includes('crypto')),
        'Forex': strategies.filter(s => s.tags.includes('forex'))
      };

      return (
        <div className={styles.tabContent}>
          {/* Categories with strategies - no header text */}
          {Object.entries(strategyCategories).map(([category, categoryStrategies]) => (
            categoryStrategies.length > 0 && (
              <div key={category} className={styles.strategyCategory}>
                <div 
                  className={`${styles.categoryHeader} ${collapsedCategories.has(category) ? styles.collapsed : ''}`}
                  onClick={() => toggleCategory(category)}
                >
                  <span className={styles.categoryArrow}>▼</span>
                  <span>{category} ({categoryStrategies.length})</span>
                </div>
                {!collapsedCategories.has(category) && (
                  <div className={styles.strategyList}>
                    {categoryStrategies.slice(0, 5).map(strategy => (
                      <div 
                        key={strategy.id}
                        className={styles.strategyItem}
                        onClick={() => {
                          setTearsheet({ strategy, isOpen: true });
                        }}
                      >
                        <div className={styles.strategyName}>{strategy.title}</div>
                        <div className={styles.strategyDesc}>
                          {strategy.metrics.sharpe.toFixed(2)} Sharpe • {strategy.metrics.winRate}% Win
                        </div>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            )
          ))}
        </div>
      );
    }
    
    // Data Explorer view
    if (mainView === 'data') {
      return (
        <div className={styles.tabContent}>
          {/* Cached Datasets */}
          <div className={styles.dataCategory}>
            <div 
              className={`${styles.categoryHeader} ${collapsedCategories.has('Cached Data') ? styles.collapsed : ''}`}
              onClick={() => toggleCategory('Cached Data')}
            >
              <span className={styles.categoryArrow}>▼</span>
              <span>Cached Market Data (IndexedDB)</span>
            </div>
            {!collapsedCategories.has('Cached Data') && (
              <div className={styles.datasetList}>
                {loadingDatasets ? (
                  <div className={styles.datasetItem}>
                    <div className={styles.datasetName}>Loading datasets...</div>
                  </div>
                ) : datasets.length === 0 ? (
                  <div className={styles.datasetItem}>
                    <div className={styles.datasetName}>No cached data yet</div>
                    <div className={styles.datasetInfo}>Open the Monitor page to fetch and cache market data</div>
                  </div>
                ) : (
                  datasets.map((dataset, index) => {
                    const startDate = new Date(dataset.startTime * 1000).toLocaleDateString();
                    const endDate = new Date(dataset.endTime * 1000).toLocaleDateString();
                    const duration = Math.round((dataset.endTime - dataset.startTime) / (60 * 60 * 24));
                    
                    return (
                      <div key={index} className={styles.datasetItem} onClick={() => {
                        // Export dataset as JSON
                        dataStorage.exportToJSON({
                          symbol: dataset.symbol,
                          exchange: dataset.exchange,
                          interval: dataset.interval
                        }).then(json => {
                          const blob = new Blob([json], { type: 'application/json' });
                          const url = URL.createObjectURL(blob);
                          const a = document.createElement('a');
                          a.href = url;
                          a.download = `${dataset.symbol}_${dataset.exchange}_${dataset.interval}.json`;
                          a.click();
                          URL.revokeObjectURL(url);
                        });
                      }}>
                        <div className={styles.datasetName}>
                          {dataset.symbol} • {dataset.exchange.toUpperCase()} • {dataset.interval}
                        </div>
                        <div className={styles.datasetInfo}>
                          {dataset.candleCount.toLocaleString()} candles • {duration} days • {startDate} to {endDate}
                        </div>
                      </div>
                    );
                  })
                )}
              </div>
            )}
          </div>
          
          {/* Parquet Files (Backend) */}
          <div className={styles.dataCategory}>
            <div 
              className={`${styles.categoryHeader} ${collapsedCategories.has('Parquet Files') ? styles.collapsed : ''}`}
              onClick={() => toggleCategory('Parquet Files')}
            >
              <span className={styles.categoryArrow}>▼</span>
              <span>Parquet Files (Backend Catalog)</span>
            </div>
            {!collapsedCategories.has('Parquet Files') && (
              <div className={styles.datasetList}>
                <div className={styles.datasetItem} onClick={() => {}}>
                  <div className={styles.datasetName}>NVDA.ALPACA-1-MINUTE</div>
                  <div className={styles.datasetInfo}>catalog/data/bar/ • OHLCV</div>
                </div>
              </div>
            )}
          </div>
          
          <div className={styles.dataCategory}>
            <div 
              className={`${styles.categoryHeader} ${collapsedCategories.has('Signals') ? styles.collapsed : ''}`}
              onClick={() => toggleCategory('Signals')}
            >
              <span className={styles.categoryArrow}>▼</span>
              <span>Signals & Features</span>
            </div>
            {!collapsedCategories.has('Signals') && (
              <div className={styles.datasetList}>
                <div className={styles.datasetItem} onClick={() => {}}>
                  <div className={styles.datasetName}>momentum_signals.parquet</div>
                  <div className={styles.datasetInfo}>500K rows • 120MB • Features</div>
                </div>
                <div className={styles.datasetItem} onClick={() => {}}>
                  <div className={styles.datasetName}>ml_features_v2.parquet</div>
                  <div className={styles.datasetInfo}>2M rows • 380MB • ML features</div>
                </div>
              </div>
            )}
          </div>
          
          <div className={styles.dataCategory}>
            <div 
              className={`${styles.categoryHeader} ${collapsedCategories.has('Backtests') ? styles.collapsed : ''}`}
              onClick={() => toggleCategory('Backtests')}
            >
              <span className={styles.categoryArrow}>▼</span>
              <span>Backtest Results</span>
            </div>
            {!collapsedCategories.has('Backtests') && (
              <div className={styles.datasetList}>
                <div className={styles.datasetItem} onClick={() => {}}>
                  <div className={styles.datasetName}>ema_cross_results.parquet</div>
                  <div className={styles.datasetInfo}>10K rows • 5MB • Performance</div>
                </div>
              </div>
            )}
          </div>
          
          {/* Quick Actions */}
          <div className={styles.dataActions}>
            <button className={styles.dataActionBtn}>
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <path d="M12 5v14M5 12h14"></path>
              </svg>
              Upload Dataset
            </button>
            <button className={styles.dataActionBtn}>
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect>
                <line x1="9" y1="9" x2="15" y2="9"></line>
                <line x1="9" y1="15" x2="15" y2="15"></line>
              </svg>
              SQL Query
            </button>
          </div>
        </div>
      );
    }
    
    switch (activeTab) {
      case 'notebooks':
        return (
          <div className={styles.tabContent}>
            {/* Code Snippets Section */}
            {Object.entries(codeSnippets).map(([category, snippets]) => (
              <div key={category} className={styles.snippetCategory}>
                <div 
                  className={`${styles.categoryHeader} ${collapsedCategories.has(category) ? styles.collapsed : ''}`}
                  onClick={() => toggleCategory(category)}
                >
                  <span className={styles.categoryArrow}>▼</span>
                  <span>{category}</span>
                </div>
                {!collapsedCategories.has(category) && (
                  <div className={styles.snippetList}>
                    {snippets
                      .filter(snippet => 
                        snippet.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
                        snippet.code.toLowerCase().includes(searchQuery.toLowerCase())
                      )
                      .map(snippet => (
                        <div 
                          key={snippet.id} 
                          className={styles.snippetItem}
                          onClick={() => insertSnippet(snippet)}
                        >
                          <div>
                            <div className={styles.snippetName}>{snippet.name}</div>
                            {snippet.description && (
                              <div className={styles.snippetDesc}>{snippet.description}</div>
                            )}
                          </div>
                          <span className={styles.insertIcon}>+</span>
                        </div>
                      ))}
                  </div>
                )}
              </div>
            ))}
            
            {/* Templates Section */}
            <div className={styles.templateCategory}>
              <div className={`${styles.categoryHeader} ${collapsedCategories.has('Templates') ? styles.collapsed : ''}`} onClick={() => toggleCategory('Templates')}>
                <span className={styles.categoryArrow}>▼</span>
                <span>Analysis Templates</span>
              </div>
              {!collapsedCategories.has('Templates') && (
                <div className={styles.templateList}>
                  {notebookTemplates.map(template => (
                    <div 
                      key={template.id} 
                      className={styles.templateItem}
                      onClick={() => loadTemplate(template)}
                    >
                      <div className={styles.templateName}>{template.title}</div>
                      <div className={styles.templateDesc}>{template.description}</div>
                    </div>
                  ))}
                </div>
              )}
            </div>
            
            {/* Saved Notebooks Section */}
            <div className={styles.notebookBrowser}>
              <div className={styles.notebookCategory}>
                <div className={`${styles.categoryHeader} ${collapsedCategories.has('Saved Notebooks') ? styles.collapsed : ''}`} onClick={() => toggleCategory('Saved Notebooks')}>
                  <span className={styles.categoryArrow}>▼</span>
                  <span>Saved Notebooks</span>
                </div>
                {!collapsedCategories.has('Saved Notebooks') && (
                  <div className={styles.notebookList}>
                    {savedNotebooks.map(notebook => (
                      <div key={notebook.id} className={styles.notebookItem}>
                        <div className={styles.notebookName}>{notebook.name}</div>
                        <div className={styles.notebookDate}>Modified: {notebook.lastModified}</div>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            </div>
          </div>
        );

      case 'builder':
        return (
          <div className={styles.tabContent}>
            {/* Strategies Section */}
            <div className={styles.strategyCategory}>
              <div 
                className={`${styles.categoryHeader} ${collapsedCategories.has('Strategies') ? styles.collapsed : ''}`}
                onClick={() => toggleCategory('Strategies')}
              >
                <span className={styles.categoryArrow}>▼</span>
                <span>Strategies</span>
              </div>
              {!collapsedCategories.has('Strategies') && (
                <div className={styles.strategyList}>
                  <div 
                    className={styles.strategyItem}
                    onClick={() => {
                      setSelectedTemplate('custom');
                      setMainView('builder');
                    }}
                  >
                    <div className={styles.strategyName}>New Strategy</div>
                    <div className={styles.strategyDesc}>Create from scratch</div>
                  </div>
                  <div 
                    className={styles.strategyItem}
                    onClick={() => {
                      setSelectedTemplate('oversold_bounce');
                      setMainView('builder');
                    }}
                  >
                    <div className={styles.strategyName}>Oversold Bounce</div>
                    <div className={styles.strategyDesc}>RSI mean reversion</div>
                  </div>
                </div>
              )}
            </div>
            
            {/* Templates Section */}
            <div className={styles.templateCategory}>
              <div 
                className={`${styles.categoryHeader} ${collapsedCategories.has('Templates') ? styles.collapsed : ''}`}
                onClick={() => toggleCategory('Templates')}
              >
                <span className={styles.categoryArrow}>▼</span>
                <span>Templates</span>
              </div>
              {!collapsedCategories.has('Templates') && (
                <div className={styles.templateList}>
                  <div 
                    className={styles.templateItem}
                    onClick={() => {
                      setSelectedTemplate('signal_analysis');
                      setMainView('builder');
                    }}
                  >
                    <div className={styles.templateName}>Signal Analysis</div>
                    <div className={styles.templateDesc}>Analyze signals across search space</div>
                  </div>
                </div>
              )}
            </div>
          </div>
        );

      default:
        return null;
    }
  };

  const renderMainContent = () => {
    if (mainView === 'explore') {
      return (
        <div className={exploreStyles.catalogueContainer}>
          <div className={exploreStyles.controlsBar}>
            <div className={exploreStyles.searchWrapper}>
              <div className={exploreStyles.searchSortGroup}>
                <input
                  type="text"
                  placeholder="search strategies... (e.g., trending swing @alexchen)"
                  className={exploreStyles.searchInput}
                  value={exploreSearchQuery}
                  onChange={(e) => setExploreSearchQuery(e.target.value)}
                />
                
                <div 
                  className={exploreStyles.sortDropdown}
                  onMouseEnter={() => setSortDropdownOpen(true)}
                  onMouseLeave={() => setSortDropdownOpen(false)}
                >
                  <button 
                    className={exploreStyles.sortButton}
                    onClick={() => setSortDropdownOpen(!sortDropdownOpen)}
                  >
                    Sort: {sortBy === 'new' ? 'New' : sortBy === 'sharpe' ? 'Sharpe' : sortBy === 'returns' ? 'Returns' : sortBy === 'winrate' ? 'Win %' : 'A-Z'}
                    <span style={{ marginLeft: '8px' }}>▼</span>
                  </button>
                {sortDropdownOpen && (
                  <div 
                    className={exploreStyles.sortMenu}
                    onMouseEnter={() => setSortDropdownOpen(true)}
                    onMouseLeave={() => setSortDropdownOpen(false)}
                  >
                    <button 
                      className={`${exploreStyles.sortOption} ${sortBy === 'new' ? exploreStyles.active : ''}`}
                      onClick={() => { setSortBy('new'); setSortDropdownOpen(false); }}
                    >
                      New
                    </button>
                    <button 
                      className={`${exploreStyles.sortOption} ${sortBy === 'sharpe' ? exploreStyles.active : ''}`}
                      onClick={() => { setSortBy('sharpe'); setSortDropdownOpen(false); }}
                    >
                      Sharpe
                    </button>
                    <button 
                      className={`${exploreStyles.sortOption} ${sortBy === 'returns' ? exploreStyles.active : ''}`}
                      onClick={() => { setSortBy('returns'); setSortDropdownOpen(false); }}
                    >
                      Returns
                    </button>
                    <button 
                      className={`${exploreStyles.sortOption} ${sortBy === 'winrate' ? exploreStyles.active : ''}`}
                      onClick={() => { setSortBy('winrate'); setSortDropdownOpen(false); }}
                    >
                      Win %
                    </button>
                    <button 
                      className={`${exploreStyles.sortOption} ${sortBy === 'name' ? exploreStyles.active : ''}`}
                      onClick={() => { setSortBy('name'); setSortDropdownOpen(false); }}
                    >
                      A-Z
                    </button>
                  </div>
                )}
                </div>
              </div>
              
              {/* Plus button for new strategy */}
              <button 
                className={exploreStyles.newStrategyBtn}
                onClick={() => {
                  // Navigate to builder tab for new strategy
                  setActiveTab('builder');
                  setMainView('builder');
                  // Clear any existing builder state and start fresh
                  // TODO: Add state management for builder
                  console.log('Opening new strategy builder');
                }}
                title="Create New Strategy"
              >
                <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round">
                  <line x1="12" y1="5" x2="12" y2="19"></line>
                  <line x1="5" y1="12" x2="19" y2="12"></line>
                </svg>
              </button>
            </div>
            {searchTerms.length > 0 && (
              <div className={exploreStyles.activeFilters}>
                {searchTerms.map(term => (
                  <button
                    key={term}
                    className={exploreStyles.filterChip}
                    onClick={() => handleTagClick(term)}
                  >
                    {term} ×
                  </button>
                ))}
              </div>
            )}
          </div>

          <div className={exploreStyles.resultsInfo}>
            <span className={exploreStyles.resultsCount}>
              Showing {Math.min(displayLimit, filterAndSortStrategies().length)} of {filterAndSortStrategies().length} strategies
            </span>
            {searchTerms.length > 0 && (
              <span className={exploreStyles.filterInfo}>
                • Filtered by: {searchTerms.join(', ')}
              </span>
            )}
          </div>

          <div className={exploreStyles.strategyGrid}>
            {filterAndSortStrategies().slice(0, displayLimit).map(renderStrategyCard)}
          </div>
          
          {filterAndSortStrategies().length > displayLimit && (
            <div className={exploreStyles.loadMoreContainer}>
              <button 
                className={exploreStyles.loadMoreBtn}
                onClick={() => setDisplayLimit(prev => prev + 12)}
              >
                Load More ({filterAndSortStrategies().length - displayLimit} remaining)
              </button>
              <button 
                className={exploreStyles.showAllBtn}
                onClick={() => setDisplayLimit(filterAndSortStrategies().length)}
              >
                Show All
              </button>
            </div>
          )}
          
          {filterAndSortStrategies().length === 0 && (
            <div className={exploreStyles.emptyState}>
              <p>No strategies found</p>
              <p className={exploreStyles.emptyHint}>Try different search terms like "trending", "low-risk", "intraday", or "@username"</p>
            </div>
          )}

          {/* Tearsheet Modal */}
          {tearsheet.isOpen && tearsheet.strategy && (
            <div className={exploreStyles.tearsheetModal} onClick={() => setTearsheet({ ...tearsheet, isOpen: false })}>
              <div className={exploreStyles.tearsheetContent} onClick={(e) => e.stopPropagation()}>
                <button className={exploreStyles.tearsheetClose} onClick={() => setTearsheet({ ...tearsheet, isOpen: false })}>×</button>
                <h2 className={exploreStyles.tearsheetTitle}>{tearsheet.strategy.title}</h2>
                
                <div className={exploreStyles.tearsheetMetrics}>
                  <div className={exploreStyles.tearsheetMetric}>
                    <span className={exploreStyles.tearsheetMetricValue}>{tearsheet.strategy.metrics?.sharpe.toFixed(2)}</span>
                    <span className={exploreStyles.tearsheetMetricLabel}>Sharpe Ratio</span>
                  </div>
                  <div className={exploreStyles.tearsheetMetric}>
                    <span className={exploreStyles.tearsheetMetricValue}>{tearsheet.strategy.metrics?.annualReturn.toFixed(1)}%</span>
                    <span className={exploreStyles.tearsheetMetricLabel}>Annual Return</span>
                  </div>
                  <div className={exploreStyles.tearsheetMetric}>
                    <span className={exploreStyles.tearsheetMetricValue}>{tearsheet.strategy.metrics?.maxDrawdown.toFixed(1)}%</span>
                    <span className={exploreStyles.tearsheetMetricLabel}>Max Drawdown</span>
                  </div>
                  <div className={exploreStyles.tearsheetMetric}>
                    <span className={exploreStyles.tearsheetMetricValue}>{tearsheet.strategy.metrics?.winRate}%</span>
                    <span className={exploreStyles.tearsheetMetricLabel}>Win Rate</span>
                  </div>
                </div>
                
                <div className={exploreStyles.tearsheetActions}>
                  <button 
                    className={exploreStyles.tearsheetIconBtn}
                    onClick={() => {
                      handleNotebookClick(new MouseEvent('click') as any, tearsheet.strategy);
                      setTearsheet({ ...tearsheet, isOpen: false });
                    }}
                    title="Open in Notebook"
                  >
                    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" style={{ width: '24px', height: '24px' }}>
                      {/* Spiral binding */}
                      <circle cx="4" cy="4" r="1.5"></circle>
                      <circle cx="4" cy="8" r="1.5"></circle>
                      <circle cx="4" cy="12" r="1.5"></circle>
                      <circle cx="4" cy="16" r="1.5"></circle>
                      <circle cx="4" cy="20" r="1.5"></circle>
                      {/* Notebook pages */}
                      <rect x="7" y="2" width="14" height="20" rx="1"></rect>
                      <line x1="11" y1="6" x2="17" y2="6"></line>
                      <line x1="11" y1="10" x2="17" y2="10"></line>
                      <line x1="11" y1="14" x2="17" y2="14"></line>
                      <line x1="11" y1="18" x2="17" y2="18"></line>
                    </svg>
                    <span>Research</span>
                  </button>
                  <button 
                    className={exploreStyles.tearsheetIconBtn}
                    onClick={() => navigate('/monitor', { state: { strategy: tearsheet.strategy } })}
                    title="Deploy Strategy"
                  >
                    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" style={{ width: '24px', height: '24px' }}>
                      {/* Rocket/Deploy icon */}
                      <path d="M4.5 16.5c-1.5 1.26-2 5-2 5s3.74-.5 5-2c.71-.84.7-2.13-.09-2.91a2.18 2.18 0 0 0-2.91-.09z"></path>
                      <path d="M12 15l-3-3a22 22 0 0 1 2-3.95A12.88 12.88 0 0 1 22 2c0 2.72-.78 7.5-6 11a22.35 22.35 0 0 1-4 2z"></path>
                      <path d="M9 12H4s.55-3.03 2-4c1.62-1.08 5 0 5 0"></path>
                      <path d="M12 15v5s3.03-.55 4-2c1.08-1.62 0-5 0-5"></path>
                    </svg>
                    <span>Deploy</span>
                  </button>
                </div>
              </div>
            </div>
          )}
        </div>
      );
    }
    
    if (mainView === 'builder') {
      return (
        <div className={styles.builderView}>
          <div className={styles.builderMainContent}>
            {selectedTemplate ? (
              <StrategyWorkbench 
                isOpen={true}
                onClose={() => {
                  setSelectedTemplate(null);
                  setActiveTab(null);
                  setMainView('explore');
                }}
                initialTemplate={selectedTemplate}
              />
            ) : (
              <div className={styles.builderWelcome}>
                <h2>Strategy Builder</h2>
                <p>Build and backtest custom trading strategies using our visual interface.</p>
                <div className={styles.builderFeatures}>
                  <div className={styles.featureItem}>
                    <span className={styles.featureIcon}>🎯</span>
                    <span>Visual strategy construction</span>
                  </div>
                  <div className={styles.featureItem}>
                    <span className={styles.featureIcon}>📊</span>
                    <span>Real-time backtesting</span>
                  </div>
                  <div className={styles.featureItem}>
                    <span className={styles.featureIcon}>⚡</span>
                    <span>Parameter optimization</span>
                  </div>
                </div>
              </div>
            )}
          </div>
        </div>
      );
    }
    
    if (mainView === 'data') {
      // Data viewer main content
      return (
        <div className={styles.dataViewerContainer}>
          <div className={styles.dataTableContainer}>
            <div className={styles.dataTableHeader}>
              <div className={styles.tableInfo}>
                <span className={styles.tableName}>Select a dataset from the sidebar to view</span>
                <span className={styles.tableStats}></span>
              </div>
            </div>
            
            <div className={styles.sqlEditor}>
              <textarea
                className={styles.sqlInput}
                placeholder="-- Enter SQL query here (e.g., SELECT * FROM read_parquet('data.parquet') LIMIT 100)"
                rows={3}
              />
              <button className={styles.runQueryBtn}>
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                  <polygon points="5 3 19 12 5 21 5 3"></polygon>
                </svg>
                Run Query
              </button>
            </div>
            
            <div className={styles.dataTableWrapper}>
              <div className={styles.emptyDataState}>
                <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" opacity="0.3">
                  <rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect>
                  <line x1="3" y1="9" x2="21" y2="9"></line>
                  <line x1="9" y1="21" x2="9" y2="9"></line>
                  <line x1="15" y1="21" x2="15" y2="9"></line>
                </svg>
                <p>No data loaded</p>
                <p className={styles.dataHint}>Select a dataset or run a SQL query to view data</p>
              </div>
            </div>
          </div>
        </div>
      );
    }

    return (
      <div className={styles.notebookView}>
        <div className={styles.notebookCells}>
          {notebookCells.map(cell => {
            // Render AI chat cells differently
            if (cell.type === 'ai-chat') {
              return (
                <div 
                  key={cell.id}
                  className={`${styles.aiChatCell} ${activeCell === cell.id ? styles.active : ''}`}
                  onClick={() => setActiveCell(cell.id)}
                >
                  <div className={styles.aiChatHeader}>
                    <div className={styles.aiChatTitle}>
                      <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                        <path d="M9.5 2A3.5 3.5 0 0 0 6 5.5c0 2.3 2.5 3.3 2.5 5.5v1"/>
                        <path d="M14.5 2A3.5 3.5 0 0 1 18 5.5c0 2.3-2.5 3.3-2.5 5.5v1"/>
                        <path d="M12 2v10"/>
                        <circle cx="12" cy="14" r="2"/>
                        <path d="M7 14H5M19 14h-2M12 16v2"/>
                        <circle cx="7" cy="14" r="1"/>
                        <circle cx="17" cy="14" r="1"/>
                        <circle cx="12" cy="19" r="1"/>
                      </svg>
                      <span>AI Analysis Assistant</span>
                    </div>
                    <button 
                      onClick={() => deleteCell(cell.id)}
                      className={styles.closeAiChat}
                      title="Close AI chat"
                    >
                      ×
                    </button>
                  </div>
                  
                  <div className={styles.aiChatMessages}>
                    {cell.aiMessages?.map((msg, idx) => (
                      <div key={idx} className={`${styles.aiMessage} ${styles[msg.role]}`}>
                        <span className={styles.messageRole}>
                          {msg.role === 'assistant' ? '🤖' : '👤'}
                        </span>
                        <div className={styles.messageContent}>{msg.content}</div>
                      </div>
                    ))}
                  </div>
                  
                  <div className={styles.aiChatInput}>
                    <input
                      type="text"
                      value={cell.chatInput || ''}
                      onChange={(e) => {
                        setNotebookCells(prev =>
                          prev.map(c =>
                            c.id === cell.id
                              ? { ...c, chatInput: e.target.value }
                              : c
                          )
                        );
                      }}
                      onKeyPress={(e) => {
                        if (e.key === 'Enter' && cell.chatInput?.trim()) {
                          // Add user message and generate AI response
                          const userMessage: AiMessage = {
                            role: 'user',
                            content: cell.chatInput
                          };
                          
                          // Generate AI response based on user input
                          let aiResponse = '';
                          const input = cell.chatInput.toLowerCase();
                          
                          if (input.includes('volatility') || input.includes('vol')) {
                            aiResponse = `Good choice focusing on volatility. I recommend using these snippets:
1. \`snippets.risk.volatility_decomp(returns, window=30)\` - Separates market vs idiosyncratic volatility
2. \`snippets.risk.rolling_correlation(returns, benchmark='SPY')\` - Shows when correlations spike

This will help identify if the volatility is systematic or strategy-specific. Ready to generate the analysis cell?`;
                          } else if (input.includes('drawdown') || input.includes('risk')) {
                            aiResponse = `For drawdown analysis, let's use:
1. \`snippets.risk.drawdown_clusters(returns, min_duration=5)\` - Identifies drawdown patterns
2. \`snippets.risk.max_drawdown_duration(returns)\` - Time to recovery analysis
3. \`snippets.risk.conditional_drawdown(returns, confidence=0.95)\` - Expected shortfall

These will give you a complete risk picture. Generate the cell?`;
                          } else if (input.includes('performance') || input.includes('returns')) {
                            aiResponse = `To analyze performance, I suggest:
1. \`snippets.performance.rolling_sharpe(returns, window=60)\` - Time-varying risk-adjusted returns
2. \`snippets.performance.factor_attribution(returns, factors=['MKT', 'SMB', 'HML'])\` - Factor decomposition
3. \`snippets.performance.regime_analysis(returns, vix_threshold=20)\` - Performance by market regime

This will show where your returns are coming from. Ready to build the cell?`;
                          } else {
                            aiResponse = `Based on your question, I can help with:
• Volatility analysis - decompose market vs strategy risk
• Drawdown patterns - understand your risk profile
• Performance attribution - see what drives returns
• Signal quality - evaluate entry/exit timing

Which area would you like to explore first?`;
                          }
                          
                          const aiMessage: AiMessage = {
                            role: 'assistant',
                            content: aiResponse
                          };
                          
                          setNotebookCells(prev =>
                            prev.map(c =>
                              c.id === cell.id
                                ? { 
                                    ...c, 
                                    aiMessages: [...(c.aiMessages || []), userMessage, aiMessage],
                                    chatInput: ''
                                  }
                                : c
                            )
                          );
                        }
                      }}
                      placeholder="Ask about your results or request specific analysis..."
                    />
                    <button 
                      className={styles.generateCellBtn}
                      onClick={() => {
                        // Generate a new code cell with recommended snippets
                        const codeTemplate = `# AI-recommended analysis based on your discussion
import admf
from snippets import risk, performance, signals

# Volatility decomposition
vol_decomp = risk.volatility_decomp(returns, window=30)
print("Market vs Idiosyncratic Volatility:")
print(vol_decomp)

# Rolling correlation analysis
correlations = risk.rolling_correlation(returns, benchmark='SPY')
correlations.plot(title='Rolling Correlation with Market')

# Performance attribution
attribution = performance.factor_attribution(returns, factors=['MKT', 'SMB', 'HML'])
print("\\nFactor Attribution:")
print(attribution)`;
                        
                        const newCell: NotebookCell = {
                          id: `cell-${Date.now()}`,
                          type: 'code',
                          content: codeTemplate
                        };
                        
                        setNotebookCells(prev => {
                          const index = prev.findIndex(c => c.id === cell.id);
                          const newCells = [...prev];
                          newCells.splice(index + 1, 0, newCell);
                          return newCells;
                        });
                      }}
                      title="Generate analysis cell from recommendations"
                    >
                      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                        <path d="M12 2v20M2 12h20"/>
                      </svg>
                      Generate Cell
                    </button>
                  </div>
                </div>
              );
            }
            
            // Regular cell rendering
            return (
              <div 
                key={cell.id} 
                className={`${styles.notebookCell} ${styles[`${cell.type}Cell`]} ${activeCell === cell.id ? styles.active : ''}`}
                onClick={() => setActiveCell(cell.id)}
              >
                <div className={styles.cellHeader}>
                  <span className={styles.cellType}>{cell.type}</span>
                  <div className={styles.cellActions}>
                    <button 
                      onClick={() => executeCell(cell.id)} 
                      disabled={cell.isExecuting}
                      className={styles.cellActionBtn}
                      title="Run cell"
                    >
                      {cell.isExecuting ? (
                        <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
                          <circle cx="8" cy="8" r="6" stroke="currentColor" strokeWidth="2" strokeDasharray="4 2">
                            <animateTransform attributeName="transform" type="rotate" from="0 8 8" to="360 8 8" dur="1s" repeatCount="indefinite"/>
                          </circle>
                        </svg>
                      ) : (
                        <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
                          <path d="M5 3.5v9l7-4.5z"/>
                        </svg>
                      )}
                    </button>
                    <button 
                      onClick={() => deleteCell(cell.id)}
                      className={styles.cellActionBtn}
                      title="Delete cell"
                    >
                      <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
                        <path d="M4.646 4.646a.5.5 0 0 1 .708 0L8 7.293l2.646-2.647a.5.5 0 0 1 .708.708L8.707 8l2.647 2.646a.5.5 0 0 1-.708.708L8 8.707l-2.646 2.647a.5.5 0 0 1-.708-.708L7.293 8 4.646 5.354a.5.5 0 0 1 0-.708z"/>
                      </svg>
                    </button>
                  </div>
                </div>
                
                <div className={styles.cellContent}>
                  {cell.type === 'code' ? (
                  <div className={styles.codeEditor}>
                    <Editor
                      height="300px"
                      language="python"
                      value={cell.content}
                      onChange={(value) => updateCellContent(cell.id, value || '')}
                      theme={theme}
                      onMount={(editor, monaco) => {
                        // Define the cream theme with more explicit colors
                        monaco.editor.defineTheme('cream-light', {
                          base: 'vs',
                          inherit: true,
                          rules: [
                            { token: '', foreground: '33332d', background: 'faf7f0' }
                          ],
                          colors: {
                            'editor.background': '#faf7f0',
                            'editor.foreground': '#33332d',
                            'editorLineNumber.foreground': '#8b8680',
                            'editorLineNumber.activeForeground': '#33332d',
                            'editor.selectionBackground': '#e5e0d5',
                            'editor.lineHighlightBackground': '#f5f2ea',
                            'editorCursor.foreground': '#33332d',
                            'editorWidget.background': '#f5f2ea',
                            'editorSuggestWidget.background': '#f5f2ea',
                            'editorHoverWidget.background': '#f5f2ea'
                          }
                        });
                        
                        // Force apply the theme
                        monaco.editor.setTheme(theme);
                      }}
                      options={{
                        fontSize: 13,
                        lineHeight: 1.5,
                        fontFamily: "'IBM Plex Mono', 'SF Mono', Monaco, Consolas, 'Courier New', monospace",
                        minimap: { enabled: false },
                        scrollBeyondLastLine: false,
                        automaticLayout: true,
                        wordWrap: 'on',
                        lineNumbers: 'on',
                        folding: false,
                        selectOnLineNumbers: true,
                        matchBrackets: 'always',
                        autoIndent: 'advanced',
                        formatOnPaste: true,
                        formatOnType: true,
                        tabSize: 4,
                        insertSpaces: true,
                        renderWhitespace: 'boundary',
                        smoothScrolling: true,
                        cursorBlinking: 'smooth',
                        cursorSmoothCaretAnimation: 'on',
                        scrollbar: {
                          vertical: 'visible',
                          horizontal: 'visible',
                          verticalScrollbarSize: 10,
                          horizontalScrollbarSize: 10,
                          alwaysConsumeMouseWheel: false
                        },
                        overviewRulerLanes: 0,
                        fixedOverflowWidgets: true
                      }}
                    />
                  </div>
                ) : (
                  <textarea
                    className={styles.cellTextarea}
                    value={cell.content}
                    onChange={(e) => updateCellContent(cell.id, e.target.value)}
                    onFocus={() => setActiveCell(cell.id)}
                    placeholder="Enter markdown content..."
                  />
                )}
              </div>

              {cell.output && (
                <div className={styles.cellOutput}>
                  <div className={styles.outputHeader}>
                    <span className={styles.outputLabel}>Output</span>
                    <button
                      className={styles.aiAnalyzeBtn}
                      onClick={(e) => {
                        e.stopPropagation();
                        // Create AI chat cell after this cell
                        const newAiCell: NotebookCell = {
                          id: `ai-chat-${Date.now()}`,
                          type: 'ai-chat',
                          content: '',
                          output: cell.output,
                          isAiChat: true,
                          parentCellId: cell.id,
                          aiMessages: [
                            {
                              role: 'assistant',
                              content: `I've analyzed your results. ${
                                cell.output?.includes('Overview') 
                                  ? "Your strategies show interesting patterns. The Sharpe ratio of 1.87 is solid, but I notice the max drawdown of -12.4%. What's your main concern - risk management or performance optimization?"
                                  : cell.output?.includes('Backtest')
                                  ? "Your backtest shows 142 trades with a 62.7% win rate. The expectancy of $1,247 per trade is promising. Would you like to explore position sizing optimization or signal filtering?"
                                  : "I can see several areas for improvement in your analysis. What aspect would you like to investigate first - volatility patterns, correlation analysis, or performance attribution?"
                              }`
                            }
                          ],
                          chatInput: ''
                        };
                        
                        setNotebookCells(prev => {
                          const index = prev.findIndex(c => c.id === cell.id);
                          const newCells = [...prev];
                          // Check if AI chat already exists for this cell
                          const existingAiChat = prev.find(c => c.parentCellId === cell.id);
                          if (!existingAiChat) {
                            newCells.splice(index + 1, 0, newAiCell);
                          }
                          return newCells;
                        });
                        setActiveCell(newAiCell.id);
                      }}
                      title="AI Analysis"
                    >
                      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                        {/* Computerized brain icon - circuit-like design */}
                        <path d="M9.5 2A3.5 3.5 0 0 0 6 5.5c0 2.3 2.5 3.3 2.5 5.5v1"/>
                        <path d="M14.5 2A3.5 3.5 0 0 1 18 5.5c0 2.3-2.5 3.3-2.5 5.5v1"/>
                        <path d="M12 2v10"/>
                        <circle cx="12" cy="14" r="2"/>
                        <path d="M7 14H5M19 14h-2M12 16v2"/>
                        <circle cx="7" cy="14" r="1"/>
                        <circle cx="17" cy="14" r="1"/>
                        <circle cx="12" cy="19" r="1"/>
                        <path d="M9 19H7v2M15 19h2v2M5 14v-2M19 14v-2"/>
                        <circle cx="5" cy="11" r="0.5"/>
                        <circle cx="19" cy="11" r="0.5"/>
                        <circle cx="7" cy="21" r="0.5"/>
                        <circle cx="17" cy="21" r="0.5"/>
                      </svg>
                      <span>AI Analysis</span>
                    </button>
                  </div>
                  <pre>{cell.output}</pre>
                </div>
              )}
            </div>
            );
          })}
          
          {/* Add Cell Button at Bottom */}
          <div className={styles.addCellContainer}>
            <button 
              className={styles.addCellButton}
              onClick={() => addCell('code')}
              title="Add new cell"
            >
              <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <line x1="12" y1="5" x2="12" y2="19"></line>
                <line x1="5" y1="12" x2="19" y2="12"></line>
              </svg>
              <span>Add Cell</span>
            </button>
          </div>
        </div>
      </div>
    );
  };

  return (
    <div 
      className={styles.researchContainer}
      onTouchStart={handleTouchStart}
      onTouchMove={handleTouchMove}
      onTouchEnd={handleTouchEnd}
    >
      {/* Overlay for Mobile */}
      {isMobile && sidebarOpen && (
        <div
          style={{
            position: 'fixed',
            top: 0,
            left: 0,
            right: 0,
            bottom: 0,
            background: 'rgba(0, 0, 0, 0.5)',
            zIndex: 199,
            backdropFilter: 'blur(2px)'
          }}
          onClick={() => setSidebarOpen(false)}
        />
      )}
      
      {/* Swipe Indicator for Mobile */}
      {isMobile && !sidebarOpen && (
        <div
          style={{
            position: 'fixed',
            bottom: '20px',
            left: '50%',
            transform: 'translateX(-50%)',
            zIndex: 100,
            background: 'var(--color-bg-secondary)',
            border: '2px solid var(--color-text-primary)',
            borderRadius: 'var(--radius-lg)',
            padding: '8px 16px',
            display: 'flex',
            alignItems: 'center',
            gap: '8px',
            opacity: 0.9,
            pointerEvents: 'none',
            animation: 'pulse 2s infinite'
          }}
        >
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <path d="M12 19V6M5 12l7-7 7 7"/>
          </svg>
          <span style={{ fontSize: '12px', fontWeight: 500 }}>Swipe up for sidebar</span>
        </div>
      )}
      
      {/* Sidebar */}
      <aside className={`${styles.snippetsSidebar} ${sidebarOpen ? styles.open : ''}`}>
        <div className={styles.sidebarHeader}>
          <div className={styles.sidebarTabs}>
            <button 
              className={`${styles.sidebarTab} ${mainView === 'data' ? styles.active : ''}`}
              onClick={() => setMainView('data')}
              title="Data Explorer"
            >
              <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" style={{ width: '24px', height: '24px' }}>
                {/* Database/cylinder icon */}
                <ellipse cx="12" cy="5" rx="9" ry="3"></ellipse>
                <path d="M21 12c0 1.66-4 3-9 3s-9-1.34-9-3"></path>
                <path d="M3 5v14c0 1.66 4 3 9 3s9-1.34 9-3V5"></path>
              </svg>
            </button>
            <button 
              className={`${styles.sidebarTab} ${mainView === 'builder' ? styles.active : ''}`}
              onClick={() => handleTabSwitch('builder')}
              title="StrategyWorkbench"
            >
              <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" style={{ width: '24px', height: '24px' }}>
                {/* Wrench */}
                <path d="M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z"></path>
                {/* Hammer crossing the wrench */}
                <path d="M5 12L3 10l9-9 3 3-1 1" opacity="0.8"></path>
                <rect x="2" y="9" width="4" height="4" rx="1" transform="rotate(-45 4 11)"></rect>
              </svg>
            </button>
            <button 
              className={`${styles.sidebarTab} ${mainView === 'notebook' ? styles.active : ''}`}
              onClick={() => handleTabSwitch('notebooks')}
              title="Notebooks"
            >
              <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" style={{ width: '24px', height: '24px' }}>
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
              className={`${styles.sidebarTab} ${mainView === 'explore' ? styles.active : ''}`}
              onClick={() => setMainView('explore')}
              title="Explore"
            >
              <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" style={{ width: '24px', height: '24px' }}>
                {/* Magnifying glass */}
                <circle cx="11" cy="11" r="8"></circle>
                <path d="m21 21-4.35-4.35"></path>
              </svg>
            </button>
          </div>
        </div>
        
        <div className={styles.sidebarContent}>
          {renderSidebarContent()}
        </div>
      </aside>

      {/* Main Content */}
      <main className={styles.mainArea}>
        {renderMainContent()}
      </main>
    </div>
  );
};

export default ResearchPage;