import React, { useState, useEffect } from 'react';
import { useLocation, useNavigate } from 'react-router-dom';
import styles from './ResearchPage.module.css';
import exploreStyles from './ExplorePage.module.css';
import { StrategyWorkbench } from '../components/StrategyBuilder/StrategyWorkbench';
import Editor from '@monaco-editor/react';
import * as monaco from 'monaco-editor';

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

interface NotebookCell {
  id: string;
  type: 'code' | 'markdown';
  content: string;
  output?: string;
  isExecuting?: boolean;
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
type MainView = 'explore' | 'notebook' | 'builder';
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
  useEffect(() => {
    const handleResize = () => {
      setIsMobile(window.innerWidth <= 768);
    };

    window.addEventListener('resize', handleResize);
    return () => window.removeEventListener('resize', handleResize);
  }, []);

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
    // Keep explore view as main content unless explicitly changing to builder/notebook
    // This way the catalogue stays visible
  };
  
  const handleOpenBuilder = () => {
    setActiveTab('builder');
    setMainView('builder');
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

    setNotebookCells(prev => 
      prev.map(cell => 
        cell.id === cellId 
          ? { 
              ...cell, 
              isExecuting: false, 
              output: 'Execution completed successfully.\nOutput: Sample analysis results...' 
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

    return (
      <div className={styles.notebookView}>
        <div className={styles.notebookCells}>
          {notebookCells.map(cell => (
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
                      height="200px"
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
                        
                        const updateHeight = () => {
                          const contentHeight = Math.min(1000, Math.max(100, editor.getContentHeight()));
                          editor.getContainerDomNode().style.height = `${contentHeight}px`;
                          editor.layout();
                        };
                        editor.onDidContentSizeChange(updateHeight);
                        updateHeight();
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
                          vertical: 'auto',
                          horizontal: 'auto',
                          verticalScrollbarSize: 10,
                          horizontalScrollbarSize: 10
                        }
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
                  <pre>{cell.output}</pre>
                </div>
              )}
            </div>
          ))}
        </div>
      </div>
    );
  };

  return (
    <div className={styles.researchContainer}>
      {/* Mobile Menu Button */}
      {isMobile && (
        <button
          className={styles.mobileMenuButton}
          onClick={() => setSidebarOpen(!sidebarOpen)}
          style={{
            position: 'fixed',
            top: 'calc(var(--mobile-header-height) + 10px)',
            left: '10px',
            zIndex: 250,
            padding: '8px',
            background: 'var(--color-bg-primary)',
            border: '2px solid var(--color-text-primary)',
            borderRadius: 'var(--radius-sm)',
            cursor: 'pointer',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center'
          }}
        >
          <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <line x1="3" y1="6" x2="21" y2="6"></line>
            <line x1="3" y1="12" x2="21" y2="12"></line>
            <line x1="3" y1="18" x2="21" y2="18"></line>
          </svg>
        </button>
      )}
      
      {/* Sidebar */}
      <aside className={`${styles.snippetsSidebar} ${sidebarOpen ? styles.open : ''}`}>
        <div className={styles.sidebarHeader}>
          <div className={styles.sidebarTabs}>
            <button 
              className={`${styles.sidebarTab} ${activeTab === 'builder' ? styles.active : ''}`}
              onClick={() => handleTabSwitch('builder')}
              title="Builder"
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
              className={`${styles.sidebarTab} ${activeTab === 'notebooks' ? styles.active : ''}`}
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