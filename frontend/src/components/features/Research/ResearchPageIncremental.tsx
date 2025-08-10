import React, { useState, useEffect } from 'react';
import { useLocation, useNavigate } from 'react-router-dom';
import styles from '../../../pages/ResearchPage.module.css';
import exploreStyles from '../../../pages/ExplorePage.module.css';
import { StrategyWorkbench } from '../../StrategyBuilder/StrategyWorkbench';
import Editor from '@monaco-editor/react';
import * as monaco from 'monaco-editor';
import { dataStorage } from '../../../services/data';
import type { DatasetInfo } from '../../../services/data';
import { 
  allStrategies,
  type Strategy 
} from '../../../data/strategies';
import { 
  useStrategyFiltering, 
  getRandomTags,
  type SortBy 
} from '../../../hooks/useStrategyFiltering';
import { MobileOverlay } from '../../common/MobileOverlay';
import { SwipeIndicator } from '../../common/SwipeIndicator';
import { SidebarWrapper } from '../../common/SidebarWrapper';
import { SidebarTabs } from '../../common/SidebarTabs';
import { DataIcon, BuilderIcon, NotebookIcon, ExploreIcon } from '../../common/Icons';
import { TearsheetModal } from '../../common/TearsheetModal';
import { StrategyCard } from '../../common/StrategyCard';
import { ExploreSearchBar } from '../../common/ExploreSearchBar';
import { StrategyGrid } from '../../common/StrategyGrid';
import { StrategyDirectory } from '../../common/StrategyDirectory';
import { DataExplorerSidebar } from '../../common/DataExplorerSidebar';
import { NotebookSidebar } from '../../common/NotebookSidebar';
import { BuilderSidebar } from '../../common/BuilderSidebar';
import { DataViewer } from '../../common/DataViewer';
import { BuilderMainContent } from '../../common/BuilderMainContent';
import { NotebookAddCell } from '../../common/NotebookAddCell';
import { NotebookView } from '../../common/NotebookView';

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


interface TearsheetData {
  strategy: Strategy;
  isOpen: boolean;
}

type SidebarTab = 'builder' | 'notebooks';
type MainView = 'explore' | 'notebook' | 'builder' | 'data';

// Strategy data imported from data/strategies.ts

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

  // AI Chat handlers
  const handleCreateAiChat = (cell: NotebookCellData) => {
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
  };

  const handleSendAiMessage = (cellId: string, message: string) => {
    const userMessage = { role: 'user', content: message };
    
    // Generate AI response based on user input
    let aiResponse = '';
    const input = message.toLowerCase();
    
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
    
    const aiMessage = { role: 'assistant', content: aiResponse };
    
    setNotebookCells(prev =>
      prev.map(c =>
        c.id === cellId
          ? { 
              ...c, 
              aiMessages: [...(c.aiMessages || []), userMessage, aiMessage],
              chatInput: ''
            }
          : c
      )
    );
  };

  const handleAiInputChange = (cellId: string, input: string) => {
    setNotebookCells(prev =>
      prev.map(c =>
        c.id === cellId ? { ...c, chatInput: input } : c
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

  // Use the custom hook for filtering and sorting
  const filteredStrategies = useStrategyFiltering({
    strategies: allStrategies,
    searchQuery: exploreSearchQuery,
    searchTerms,
    sortBy
  });

  const renderStrategyCard = (strategy: Strategy) => {
    const displayTags = getRandomTags(strategy.tags, strategy.id);
    
    return (
      <StrategyCard
        key={strategy.id}
        strategy={strategy}
        styles={exploreStyles}
        isHovered={hoveredCard === strategy.id}
        searchTerms={searchTerms}
        displayTags={displayTags}
        onSelect={handleStrategySelect}
        onHoverEnter={setHoveredCard}
        onHoverLeave={() => setHoveredCard(null)}
        onTagClick={handleTagClick}
        onNotebookClick={handleNotebookClick}
        onDeployClick={handleDeployClick}
      />
    );
  };


  const renderSidebarContent = () => {
    // When in explore view, show strategy directory
    if (mainView === 'explore') {
      return (
        <StrategyDirectory
          styles={styles}
          strategies={filteredStrategies}
          collapsedCategories={collapsedCategories}
          onToggleCategory={toggleCategory}
          onStrategyClick={(strategy) => setTearsheet({ strategy, isOpen: true })}
        />
      );
    }
    
    // Data Explorer view
    if (mainView === 'data') {
      return (
        <DataExplorerSidebar
          styles={styles}
          datasets={datasets}
          loadingDatasets={loadingDatasets}
          collapsedCategories={collapsedCategories}
          onToggleCategory={toggleCategory}
          onExportDataset={(dataset) => {
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
          }}
        />
      );
    }
    
    switch (activeTab) {
      case 'notebooks':
        return (
          <NotebookSidebar
            styles={styles}
            codeSnippets={codeSnippets}
            notebookTemplates={notebookTemplates}
            savedNotebooks={savedNotebooks}
            searchQuery={searchQuery}
            collapsedCategories={collapsedCategories}
            onToggleCategory={toggleCategory}
            onInsertSnippet={insertSnippet}
            onLoadTemplate={loadTemplate}
          />
        );

      case 'builder':
        return (
          <BuilderSidebar
            styles={styles}
            collapsedCategories={collapsedCategories}
            onToggleCategory={toggleCategory}
            onSelectTemplate={(template) => {
              setSelectedTemplate(template);
              setMainView('builder');
            }}
            onSelectStrategy={(strategyType) => {
              setSelectedTemplate(strategyType);
              setMainView('builder');
            }}
          />
        );

      default:
        return null;
    }
  };

  const renderMainContent = () => {
    if (mainView === 'explore') {
      return (
        <div className={exploreStyles.catalogueContainer}>
          <ExploreSearchBar
            styles={exploreStyles}
            searchQuery={exploreSearchQuery}
            sortBy={sortBy}
            sortDropdownOpen={sortDropdownOpen}
            searchTerms={searchTerms}
            displayLimit={displayLimit}
            totalResults={filteredStrategies.length}
            filteredCount={filteredStrategies.length}
            onSearchChange={setExploreSearchQuery}
            onSortChange={setSortBy}
            onSortDropdownToggle={setSortDropdownOpen}
            onTagClick={handleTagClick}
            onNewStrategy={() => {
              // Navigate to builder tab for new strategy
              setActiveTab('builder');
              setMainView('builder');
              // Clear any existing builder state and start fresh
              // TODO: Add state management for builder
              console.log('Opening new strategy builder');
            }}
          />

          <StrategyGrid
            styles={exploreStyles}
            strategies={filteredStrategies}
            displayLimit={displayLimit}
            totalCount={filteredStrategies.length}
            renderCard={renderStrategyCard}
            onLoadMore={() => setDisplayLimit(prev => prev + 12)}
            onShowAll={() => setDisplayLimit(filteredStrategies.length)}
          />

          {/* Tearsheet Modal */}
          <TearsheetModal
            isOpen={tearsheet.isOpen}
            strategy={tearsheet.strategy}
            styles={exploreStyles}
            onClose={() => setTearsheet({ ...tearsheet, isOpen: false })}
            onNotebookClick={(strategy) => {
              handleNotebookClick(new MouseEvent('click') as any, strategy);
            }}
            onDeployClick={(strategy) => {
              navigate('/monitor', { state: { strategy } });
            }}
          />
        </div>
      );
    }
    
    if (mainView === 'builder') {
      return (
        <BuilderMainContent
          styles={styles}
          selectedTemplate={selectedTemplate}
          onTemplateClose={() => {
            setSelectedTemplate(null);
            setActiveTab(null);
            setMainView('explore');
          }}
        />
      );
    }
    
    if (mainView === 'data') {
      // Data viewer main content
      return <DataViewer styles={styles} />;
    }

    return (
      <NotebookView
        styles={styles}
        notebookCells={notebookCells}
        activeCell={activeCell}
        theme={theme}
        onSetActiveCell={setActiveCell}
        onSetNotebookCells={setNotebookCells}
        onUpdateCellContent={updateCellContent}
        onExecuteCell={executeCell}
        onDeleteCell={deleteCell}
        onAddCell={addCell}
      />
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
      <MobileOverlay 
        isVisible={isMobile && sidebarOpen}
        onClick={() => setSidebarOpen(false)}
      />
      
      {/* Swipe Indicator for Mobile */}
      <SwipeIndicator 
        isVisible={isMobile && !sidebarOpen}
        text="Swipe up for sidebar"
      />
      
      {/* Sidebar */}
      <aside className={`${styles.snippetsSidebar} ${sidebarOpen ? styles.open : ''}`}>
        <SidebarTabs 
          styles={styles}
          tabs={[
            {
              id: 'data',
              title: 'Data Explorer',
              isActive: mainView === 'data',
              onClick: () => setMainView('data'),
              icon: <DataIcon />
            },
            {
              id: 'builder',
              title: 'StrategyWorkbench',
              isActive: mainView === 'builder',
              onClick: () => handleTabSwitch('builder'),
              icon: <BuilderIcon />
            },
            {
              id: 'notebook',
              title: 'Notebooks',
              isActive: mainView === 'notebook',
              onClick: () => handleTabSwitch('notebooks'),
              icon: <NotebookIcon />
            },
            {
              id: 'explore',
              title: 'Explore',
              isActive: mainView === 'explore',
              onClick: () => setMainView('explore'),
              icon: <ExploreIcon />
            }
          ]}
        />
        
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
