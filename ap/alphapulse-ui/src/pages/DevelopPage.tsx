import React, { useState, useEffect, useRef } from 'react';
import styles from './DevelopPage.module.css';
import CodeEditor from '../components/CodeEditor/CodeEditor';

interface FileItem {
  path: string;
  name: string;
  type: 'file' | 'folder';
  children?: FileItem[];
}

interface Tab {
  id: string;
  name: string;
  content: string;
  language?: string;
}

export const DevelopPage: React.FC = () => {
  const [files, setFiles] = useState<FileItem[]>([]);
  const [tabs, setTabs] = useState<Tab[]>([]);
  const [activeTab, setActiveTab] = useState<string>('README.md');
  const [sidebarOpen, setSidebarOpen] = useState(true);
  const [sidebarView, setSidebarView] = useState<'explorer' | 'search' | 'git'>('explorer');
  const [outputOpen, setOutputOpen] = useState(false);
  
  // Terminal tabs state
  interface TerminalTab {
    id: string;
    name: string;
    content: string[];
    currentInput: string;
    cwd: string;
  }
  
  const [terminalTabs, setTerminalTabs] = useState<TerminalTab[]>([
    { id: 'terminal-1', name: '~/strategies', content: [], currentInput: '', cwd: '~/strategies' }
  ]);
  const [activeTerminalTab, setActiveTerminalTab] = useState<string>('terminal-1');
  const [terminalTabCounter, setTerminalTabCounter] = useState(2);
  const [searchQuery, setSearchQuery] = useState('');
  const [splitOrientation, setSplitOrientation] = useState<'horizontal' | 'vertical'>('horizontal');
  const [splitSize, setSplitSize] = useState(0); // Will be calculated as 50% of available space
  const [isDragging, setIsDragging] = useState(false);
  const [sidebarWidth, setSidebarWidth] = useState(320); // Default sidebar width
  const [isSidebarDragging, setIsSidebarDragging] = useState(false);
  const [editorHidden, setEditorHidden] = useState(false);
  const [expandedFolders, setExpandedFolders] = useState<Set<string>>(new Set(['examples/']));
  const editorRef = useRef<HTMLTextAreaElement>(null);
  const [isDesktop, setIsDesktop] = useState(window.innerWidth > 768);

  useEffect(() => {
    loadFiles();
    initializeConsole();
    
    // Open README.md by default
    const readmeContent = `# AlphaPulse Development Environment

Welcome to the AlphaPulse integrated development environment for quantitative trading strategies.

## Getting Started

This environment provides everything you need to develop, test, and deploy trading strategies using NautilusTrader.

### Quick Start Guide

1. **Explore Examples**: Browse the \`examples/\` folder for sample strategies
2. **Use Snippets**: Access ready-to-use code snippets in the \`snippets/\` folder
3. **Run Backtests**: Use the terminal to execute strategy backtests

### Key Features

- **Monaco Editor**: Professional code editing with syntax highlighting
- **Integrated Terminal**: Run NautilusTrader commands directly
- **Code Snippets**: Pre-built functions for common trading operations
- **Live Preview**: Test strategies with real-time market data

### Project Structure

\`\`\`
├── README.md           # This file
├── snippets/           # Reusable code snippets
│   ├── data_loading/   # Data import utilities
│   ├── performance_metrics/ # Performance calculations
│   ├── visualizations/ # Charting functions
│   └── analysis_templates/ # Analysis templates
├── examples/           # Example strategies
├── config/            # Configuration files
└── docs/              # Documentation
\`\`\`

### Keyboard Shortcuts

- **Ctrl/Cmd + S**: Save current file
- **Ctrl/Cmd + Enter**: Run current code
- **Ctrl/Cmd + /**: Toggle comment
- **Ctrl/Cmd + D**: Duplicate line

### Resources

- [NautilusTrader Documentation](https://nautilustrader.io/docs/)
- [AlphaPulse Strategy Guide](docs/strategy_guide.md)
- [API Reference](docs/API.md)

---

*Happy Trading! 🚀*`;
    
    setTabs([{ 
      id: 'README.md', 
      name: 'README.md', 
      content: readmeContent, 
      language: 'markdown' 
    }]);
    setActiveTab('README.md');
    
    const handleResize = () => {
      setIsDesktop(window.innerWidth > 768);
    };
    
    window.addEventListener('resize', handleResize);
    return () => window.removeEventListener('resize', handleResize);
  }, []);

  // Terminal helper functions
  const getCurrentTerminalTab = () => {
    return terminalTabs.find(tab => tab.id === activeTerminalTab) || terminalTabs[0];
  };
  
  const getShortPath = (path: string) => {
    // For display in tab, show abbreviated path
    if (path === '~' || path === '~/strategies') {
      return path;
    }
    const parts = path.split('/');
    if (parts.length > 3) {
      // Show first part and last two parts
      return `${parts[0]}/.../${parts.slice(-2).join('/')}`;
    }
    return path;
  };
  
  const addTerminalTab = () => {
    const newTab: TerminalTab = {
      id: `terminal-${terminalTabCounter}`,
      name: '~/strategies',
      content: [],
      currentInput: '',
      cwd: '~/strategies'
    };
    setTerminalTabs(prev => [...prev, newTab]);
    setActiveTerminalTab(newTab.id);
    setTerminalTabCounter(prev => prev + 1);
    // Initialize the new tab
    setTimeout(() => initializeConsole(newTab.id), 100);
  };
  
  const closeTerminalTab = (tabId: string) => {
    if (terminalTabs.length > 1) {
      const tabIndex = terminalTabs.findIndex(t => t.id === tabId);
      const newTabs = terminalTabs.filter(t => t.id !== tabId);
      setTerminalTabs(newTabs);
      
      if (activeTerminalTab === tabId) {
        const newActiveIndex = Math.min(tabIndex, newTabs.length - 1);
        setActiveTerminalTab(newTabs[newActiveIndex].id);
      }
    }
  };
  
  const updateTerminalInput = (value: string, tabId?: string) => {
    const targetTabId = tabId || activeTerminalTab;
    setTerminalTabs(prev => prev.map(tab => 
      tab.id === targetTabId 
        ? { ...tab, currentInput: value }
        : tab
    ));
  };
  
  const addOutput = (text: string, tabId?: string) => {
    const targetTabId = tabId || activeTerminalTab;
    setTerminalTabs(prev => prev.map(tab => 
      tab.id === targetTabId 
        ? { ...tab, content: [...tab.content, text] }
        : tab
    ));
  };
  
  const initializeConsole = (tabId?: string) => {
    const targetTabId = tabId || activeTerminalTab;
    const timestamp = new Date().toISOString();
    const nautilusArt = [
      `${timestamp} [INFO] BACKTESTER-001.BacktestEngine: =================================================================`,
      `${timestamp} [INFO] BACKTESTER-001.BacktestEngine:  NAUTILUS TRADER - Automated Algorithmic Trading Platform`,
      `${timestamp} [INFO] BACKTESTER-001.BacktestEngine:  by Nautech Systems Pty Ltd.`,
      `${timestamp} [INFO] BACKTESTER-001.BacktestEngine:  Copyright (C) 2015-2025. All rights reserved.`,
      `${timestamp} [INFO] BACKTESTER-001.BacktestEngine: =================================================================`,
      `${timestamp} [INFO] BACKTESTER-001.BacktestEngine: `,
      `${timestamp} [INFO] BACKTESTER-001.BacktestEngine: ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣠⣴⣶⡟⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀`,
      `${timestamp} [INFO] BACKTESTER-001.BacktestEngine: ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣰⣾⣿⣿⣿⠀⢸⣿⣿⣿⣿⣶⣶⣤⣀⠀⠀⠀⠀⠀`,
      `${timestamp} [INFO] BACKTESTER-001.BacktestEngine: ⠀⠀⠀⠀⠀⠀⢀⣴⡇⢀⣾⣿⣿⣿⣿⣿⠀⣾⣿⣿⣿⣿⣿⣿⣿⠿⠓⠀⠀⠀⠀`,
      `${timestamp} [INFO] BACKTESTER-001.BacktestEngine: ⠀⠀⠀⠀⠀⣰⣿⣿⡀⢸⣿⣿⣿⣿⣿⣿⠀⣿⣿⣿⣿⣿⣿⠟⠁⣠⣄⠀⠀⠀⠀`,
      `${timestamp} [INFO] BACKTESTER-001.BacktestEngine: ⠀⠀⠀⠀⢠⣿⣿⣿⣇⠀⢿⣿⣿⣿⣿⣿⠀⢻⣿⣿⣿⡿⢃⣠⣾⣿⣿⣧⡀⠀⠀`,
      `${timestamp} [INFO] BACKTESTER-001.BacktestEngine: ⠀⠀⠀⠠⣾⣿⣿⣿⣿⣿⣧⠈⠋⢀⣴⣧⠀⣿⡏⢠⡀⢸⣿⣿⣿⣿⣿⣿⣿⡇⠀`,
      `${timestamp} [INFO] BACKTESTER-001.BacktestEngine: ⠀⠀⠀⣀⠙⢿⣿⣿⣿⣿⣿⠇⢠⣿⣿⣿⡄⠹⠃⠼⠃⠈⠉⠛⠛⠛⠛⠛⠻⠇⠀`,
      `${timestamp} [INFO] BACKTESTER-001.BacktestEngine: ⠀⠀⢸⡟⢠⣤⠉⠛⠿⢿⣿⠀⢸⣿⡿⠋⣠⣤⣄⠀⣾⣿⣿⣶⣶⣶⣦⡄⠀⠀⠀`,
      `${timestamp} [INFO] BACKTESTER-001.BacktestEngine: ⠀⠀⠸⠀⣾⠏⣸⣷⠂⣠⣤⠀⠘⢁⣴⣾⣿⣿⣿⡆⠘⣿⣿⣿⣿⣿⣿⠀⠀⠀⠀`,
      `${timestamp} [INFO] BACKTESTER-001.BacktestEngine: ⠀⠀⠀⠀⠛⠀⣿⡟⠀⢻⣿⡄⠸⣿⣿⣿⣿⣿⣿⣿⡀⠘⣿⣿⣿⣿⠟⠀⠀⠀⠀`,
      `${timestamp} [INFO] BACKTESTER-001.BacktestEngine: ⠀⠀⠀⠀⠀⠀⣿⠇⠀⠀⢻⡿⠀⠈⠻⣿⣿⣿⣿⣿⡇⠀⢹⣿⠿⠋⠀⠀⠀⠀⠀`,
      `${timestamp} [INFO] BACKTESTER-001.BacktestEngine: ⠀⠀⠀⠀⠀⠀⠋⠀⠀⠀⡘⠁⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠈⠁⠀⠀⠀⠀⠀⠀⠀`,
      `${timestamp} [INFO] BACKTESTER-001.BacktestEngine: `,
      '',
      'AlphaPulse Development Environment v1.0.0',
      'Connected to Nautilus Trader Engine',
      'Ready.',
      '',
      'alphapulse@server:~/strategies$ '
    ];
    
    setTerminalTabs(prev => prev.map(tab => 
      tab.id === targetTabId 
        ? { ...tab, content: nautilusArt }
        : tab
    ));
  };

  const loadFiles = async () => {
    try {
      const response = await fetch('http://localhost:5000/api/nt-reference/list-files');
      const data = await response.json();
      
      // Transform the data into our file structure
      const fileStructure: FileItem[] = [];
      
      if (data.examples) {
        const examplesFolder: FileItem = {
          path: 'examples/',
          name: 'examples',
          type: 'folder',
          children: []
        };
        
        if (data.examples.strategies) {
          const strategiesFolder: FileItem = {
            path: 'examples/strategies/',
            name: 'strategies',
            type: 'folder',
            children: data.examples.strategies.map((file: string) => ({
              path: `strategies/${file}`,
              name: file,
              type: 'file'
            }))
          };
          examplesFolder.children?.push(strategiesFolder);
        }
        
        if (data.examples.algorithms) {
          const algorithmsFolder: FileItem = {
            path: 'examples/algorithms/',
            name: 'algorithms',
            type: 'folder',
            children: data.examples.algorithms.map((file: string) => ({
              path: `algorithms/${file}`,
              name: file,
              type: 'file'
            }))
          };
          examplesFolder.children?.push(algorithmsFolder);
        }
        
        fileStructure.push(examplesFolder);
      }
      
      // Add README.md at the top
      fileStructure.push(
        { path: 'README.md', name: 'README.md', type: 'file' }
      );
      
      
      // Add Notebooks directory with snippets and builder-ui as subdirectories
      fileStructure.push(
        {
          path: 'notebooks/',
          name: 'notebooks',
          type: 'folder',
          children: [
            { path: 'notebooks/strategy_development.ipynb', name: 'strategy_development.ipynb', type: 'file' },
            { path: 'notebooks/market_analysis.ipynb', name: 'market_analysis.ipynb', type: 'file' },
            { path: 'notebooks/backtest_results.ipynb', name: 'backtest_results.ipynb', type: 'file' },
            { path: 'notebooks/signal_research.ipynb', name: 'signal_research.ipynb', type: 'file' },
            { path: 'notebooks/portfolio_optimization.ipynb', name: 'portfolio_optimization.ipynb', type: 'file' },
            {
              path: 'notebooks/snippets/',
              name: 'snippets',
              type: 'folder',
              children: [
                {
                  path: 'notebooks/snippets/data_loading/',
                  name: 'data_loading',
                  type: 'folder',
                  children: [
                    { path: 'notebooks/snippets/data_loading/load_signals.py', name: 'load_signals.py', type: 'file' },
                    { path: 'notebooks/snippets/data_loading/fetch_market_data.py', name: 'fetch_market_data.py', type: 'file' },
                    { path: 'notebooks/snippets/data_loading/import_csv.py', name: 'import_csv.py', type: 'file' }
                  ]
                },
                {
                  path: 'notebooks/snippets/performance_metrics/',
                  name: 'performance_metrics',
                  type: 'folder',
                  children: [
                    { path: 'notebooks/snippets/performance_metrics/sharpe_ratio.py', name: 'sharpe_ratio.py', type: 'file' },
                    { path: 'notebooks/snippets/performance_metrics/max_drawdown.py', name: 'max_drawdown.py', type: 'file' },
                    { path: 'notebooks/snippets/performance_metrics/win_rate.py', name: 'win_rate.py', type: 'file' }
                  ]
                },
                {
                  path: 'notebooks/snippets/visualizations/',
                  name: 'visualizations',
                  type: 'folder',
                  children: [
                    { path: 'notebooks/snippets/visualizations/plot_pnl.py', name: 'plot_pnl.py', type: 'file' },
                    { path: 'notebooks/snippets/visualizations/candlestick_chart.py', name: 'candlestick_chart.py', type: 'file' },
                    { path: 'notebooks/snippets/visualizations/heatmap.py', name: 'heatmap.py', type: 'file' }
                  ]
                },
                {
                  path: 'notebooks/snippets/analysis_templates/',
                  name: 'analysis_templates',
                  type: 'folder',
                  children: [
                    { path: 'notebooks/snippets/analysis_templates/backtest_analysis.py', name: 'backtest_analysis.py', type: 'file' },
                    { path: 'notebooks/snippets/analysis_templates/correlation_study.py', name: 'correlation_study.py', type: 'file' },
                    { path: 'notebooks/snippets/analysis_templates/risk_metrics.py', name: 'risk_metrics.py', type: 'file' }
                  ]
                },
                {
                  path: 'notebooks/snippets/saved_notebooks/',
                  name: 'saved_notebooks',
                  type: 'folder',
                  children: [
                    { path: 'notebooks/snippets/saved_notebooks/ema_cross_research.ipynb', name: 'ema_cross_research.ipynb', type: 'file' },
                    { path: 'notebooks/snippets/saved_notebooks/mean_reversion_analysis.ipynb', name: 'mean_reversion_analysis.ipynb', type: 'file' }
                  ]
                }
              ]
            },
            {
              path: 'notebooks/builder-ui/',
              name: 'builder-ui',
              type: 'folder',
              children: [
                { path: 'notebooks/builder-ui/signal_analysis.py', name: 'signal_analysis.py', type: 'file' },
                { path: 'notebooks/builder-ui/strategy_workbench.py', name: 'strategy_workbench.py', type: 'file' },
                { path: 'notebooks/builder-ui/components.py', name: 'components.py', type: 'file' },
                { path: 'notebooks/builder-ui/config.json', name: 'config.json', type: 'file' }
              ]
            }
          ]
        }
      );
      
      // Add tests directory with subdirectories
      fileStructure.push(
        {
          path: 'tests/',
          name: 'tests',
          type: 'folder',
          children: [
            {
              path: 'tests/snippets/',
              name: 'snippets',
              type: 'folder',
              children: [
                { path: 'tests/snippets/test_data_loading.py', name: 'test_data_loading.py', type: 'file' },
                { path: 'tests/snippets/test_indicators.py', name: 'test_indicators.py', type: 'file' },
                { path: 'tests/snippets/test_risk_management.py', name: 'test_risk_management.py', type: 'file' }
              ]
            },
            {
              path: 'tests/strategies/',
              name: 'strategies',
              type: 'folder',
              children: [
                { path: 'tests/strategies/test_momentum.py', name: 'test_momentum.py', type: 'file' },
                { path: 'tests/strategies/test_mean_reversion.py', name: 'test_mean_reversion.py', type: 'file' },
                { path: 'tests/strategies/test_pairs_trading.py', name: 'test_pairs_trading.py', type: 'file' }
              ]
            },
            { path: 'tests/conftest.py', name: 'conftest.py', type: 'file' },
            { path: 'tests/__init__.py', name: '__init__.py', type: 'file' }
          ]
        }
      );
      
      // Add config and docs folders
      fileStructure.push(
        {
          path: 'config/',
          name: 'config',
          type: 'folder',
          children: [
            { path: 'config/config.yaml', name: 'config.yaml', type: 'file' },
            { path: 'config/logging.json', name: 'logging.json', type: 'file' }
          ]
        },
        {
          path: 'docs/',
          name: 'docs',
          type: 'folder',
          children: [
            { path: 'docs/README.md', name: 'README.md', type: 'file' },
            { path: 'docs/API.md', name: 'API.md', type: 'file' }
          ]
        }
      );
      
      setFiles(fileStructure);
    } catch (error) {
      console.error('Failed to load files:', error);
      // Set default file structure
      setFiles([
        { path: 'README.md', name: 'README.md', type: 'file' },
        {
          path: 'notebooks/',
          name: 'notebooks',
          type: 'folder',
          children: [
            { path: 'notebooks/strategy_development.ipynb', name: 'strategy_development.ipynb', type: 'file' },
            { path: 'notebooks/market_analysis.ipynb', name: 'market_analysis.ipynb', type: 'file' },
            { path: 'notebooks/backtest_results.ipynb', name: 'backtest_results.ipynb', type: 'file' },
            {
              path: 'notebooks/snippets/',
              name: 'snippets',
              type: 'folder',
              children: [
                {
                  path: 'notebooks/snippets/data_loading/',
                  name: 'data_loading',
                  type: 'folder',
                  children: [
                    { path: 'notebooks/snippets/data_loading/load_signals.py', name: 'load_signals.py', type: 'file' },
                    { path: 'notebooks/snippets/data_loading/fetch_market_data.py', name: 'fetch_market_data.py', type: 'file' }
                  ]
                },
                {
                  path: 'notebooks/snippets/performance_metrics/',
                  name: 'performance_metrics',
                  type: 'folder',
                  children: [
                    { path: 'notebooks/snippets/performance_metrics/sharpe_ratio.py', name: 'sharpe_ratio.py', type: 'file' },
                    { path: 'notebooks/snippets/performance_metrics/max_drawdown.py', name: 'max_drawdown.py', type: 'file' }
                  ]
                },
                {
                  path: 'notebooks/snippets/visualizations/',
                  name: 'visualizations',
                  type: 'folder',
                  children: [
                    { path: 'notebooks/snippets/visualizations/plot_pnl.py', name: 'plot_pnl.py', type: 'file' },
                    { path: 'notebooks/snippets/visualizations/candlestick_chart.py', name: 'candlestick_chart.py', type: 'file' }
                  ]
                },
                {
                  path: 'notebooks/snippets/analysis_templates/',
                  name: 'analysis_templates',
                  type: 'folder',
                  children: [
                    { path: 'notebooks/snippets/analysis_templates/backtest_analysis.py', name: 'backtest_analysis.py', type: 'file' }
                  ]
                },
                {
                  path: 'notebooks/snippets/saved_notebooks/',
                  name: 'saved_notebooks',
                  type: 'folder',
                  children: [
                    { path: 'notebooks/snippets/saved_notebooks/research_notebook.ipynb', name: 'research_notebook.ipynb', type: 'file' }
                  ]
                }
              ]
            },
            {
              path: 'notebooks/builder-ui/',
              name: 'builder-ui',
              type: 'folder',
              children: [
                { path: 'notebooks/builder-ui/signal_analysis.py', name: 'signal_analysis.py', type: 'file' },
                { path: 'notebooks/builder-ui/strategy_workbench.py', name: 'strategy_workbench.py', type: 'file' },
                { path: 'notebooks/builder-ui/components.py', name: 'components.py', type: 'file' },
                { path: 'notebooks/builder-ui/config.json', name: 'config.json', type: 'file' }
              ]
            }
          ]
        },
        {
          path: 'tests/',
          name: 'tests',
          type: 'folder',
          children: [
            {
              path: 'tests/snippets/',
              name: 'snippets',
              type: 'folder',
              children: [
                { path: 'tests/snippets/test_data_loading.py', name: 'test_data_loading.py', type: 'file' },
                { path: 'tests/snippets/test_indicators.py', name: 'test_indicators.py', type: 'file' }
              ]
            },
            {
              path: 'tests/strategies/',
              name: 'strategies',
              type: 'folder',
              children: [
                { path: 'tests/strategies/test_momentum.py', name: 'test_momentum.py', type: 'file' },
                { path: 'tests/strategies/test_mean_reversion.py', name: 'test_mean_reversion.py', type: 'file' }
              ]
            },
            { path: 'tests/conftest.py', name: 'conftest.py', type: 'file' }
          ]
        },
        {
          path: 'docs/',
          name: 'docs',
          type: 'folder',
          children: [
            { path: 'docs/README.md', name: 'README.md', type: 'file' },
            { path: 'docs/API.md', name: 'API.md', type: 'file' }
          ]
        },
        {
          path: 'examples/',
          name: 'examples',
          type: 'folder',
          children: [
            {
              path: 'examples/strategies/',
              name: 'strategies',
              type: 'folder',
              children: [
                { path: 'strategies/ema_cross.py', name: 'ema_cross.py', type: 'file' },
                { path: 'strategies/momentum.py', name: 'momentum.py', type: 'file' }
              ]
            }
          ]
        }
      ]);
    }
  };

  const toggleFolder = (folderPath: string) => {
    setExpandedFolders(prev => {
      const newSet = new Set(prev);
      if (newSet.has(folderPath)) {
        newSet.delete(folderPath);
      } else {
        newSet.add(folderPath);
      }
      return newSet;
    });
  };

  const openFile = async (filePath: string, fileName: string) => {
    // Open editor if it's hidden
    if (editorHidden) {
      setEditorHidden(false);
    }
    
    // Check if tab already exists
    const existingTab = tabs.find(tab => tab.id === filePath);
    if (existingTab) {
      setActiveTab(filePath);
      return;
    }
    
    // Generate content based on file type and location
    let content = '';
    
    // Handle README.md
    if (fileName === 'README.md') {
      content = `# AlphaPulse Development Environment

Welcome to the AlphaPulse integrated development environment for quantitative trading strategies.

## Getting Started

This environment provides everything you need to develop, test, and deploy trading strategies using NautilusTrader.

### Quick Start Guide

1. **Explore Examples**: Browse the \`examples/\` folder for sample strategies
2. **Use Snippets**: Access ready-to-use code snippets in the \`snippets/\` folder
3. **Run Backtests**: Use the terminal to execute strategy backtests

### Key Features

- **Monaco Editor**: Professional code editing with syntax highlighting
- **Integrated Terminal**: Run NautilusTrader commands directly
- **Code Snippets**: Pre-built functions for common trading operations
- **Live Preview**: Test strategies with real-time market data

### Project Structure

\`\`\`
├── README.md           # This file
├── snippets/           # Reusable code snippets
│   ├── data_loading/   # Data import utilities
│   ├── performance_metrics/ # Performance calculations
│   ├── visualizations/ # Charting functions
│   └── analysis_templates/ # Analysis templates
├── examples/           # Example strategies
├── config/            # Configuration files
└── docs/              # Documentation
\`\`\`

### Keyboard Shortcuts

- **Ctrl/Cmd + S**: Save current file
- **Ctrl/Cmd + Enter**: Run current code
- **Ctrl/Cmd + /**: Toggle comment
- **Ctrl/Cmd + D**: Duplicate line

### Resources

- [NautilusTrader Documentation](https://nautilustrader.io/docs/)
- [AlphaPulse Strategy Guide](docs/strategy_guide.md)
- [API Reference](docs/API.md)

---

*Happy Trading! 🚀*`;
    }
    // Snippet files get specialized content
    else if (filePath.includes('snippets/')) {
      if (filePath.includes('data_loading/')) {
        if (fileName === 'load_signals.py') {
          content = `# Load Signal Data from ADMF
import admf
import pandas as pd

def load_signals(strategy_id: str, limit: int = 100):
    """Load signal traces from the ADMF registry."""
    signals = admf.load_signals(
        strategy_type=strategy_id,
        limit=limit
    )
    
    # Convert to DataFrame for analysis
    df = pd.DataFrame(signals)
    print(f"Loaded {len(df)} signals for {strategy_id}")
    
    return df

# Example usage
if __name__ == "__main__":
    signals = load_signals('ema_cross', limit=50)
    print(signals.head())`;
        } else if (fileName === 'fetch_market_data.py') {
          content = `# Fetch Market Data
import pandas as pd
import numpy as np
from datetime import datetime, timedelta

def fetch_market_data(symbol: str, period: str = '1d', lookback: int = 30):
    """Fetch historical market data for analysis."""
    end_date = datetime.now()
    start_date = end_date - timedelta(days=lookback)
    
    # Mock data generation (replace with actual API call)
    dates = pd.date_range(start=start_date, end=end_date, freq=period)
    data = {
        'date': dates,
        'open': np.random.randn(len(dates)) * 2 + 100,
        'high': np.random.randn(len(dates)) * 2 + 102,
        'low': np.random.randn(len(dates)) * 2 + 98,
        'close': np.random.randn(len(dates)) * 2 + 100,
        'volume': np.random.randint(1000000, 5000000, len(dates))
    }
    
    return pd.DataFrame(data)

# Example usage
if __name__ == "__main__":
    data = fetch_market_data('SPY', '1d', 30)
    print(data.tail())`;
        } else {
          content = `# Import CSV Data
import pandas as pd
import os

def import_csv(filepath: str, parse_dates: bool = True):
    """Import data from CSV file."""
    if not os.path.exists(filepath):
        raise FileNotFoundError(f"File not found: {filepath}")
    
    df = pd.read_csv(
        filepath,
        parse_dates=['date'] if parse_dates else None,
        index_col='date' if parse_dates else None
    )
    
    print(f"Loaded {len(df)} rows from {filepath}")
    print(f"Columns: {', '.join(df.columns)}")
    
    return df`;
        }
      } else if (filePath.includes('performance_metrics/')) {
        if (fileName === 'sharpe_ratio.py') {
          content = `# Calculate Sharpe Ratio
import numpy as np
import pandas as pd

def calculate_sharpe_ratio(returns: pd.Series, risk_free_rate: float = 0.02):
    """
    Calculate the Sharpe ratio for a returns series.
    
    Args:
        returns: Series of returns
        risk_free_rate: Annual risk-free rate (default 2%)
    
    Returns:
        float: Sharpe ratio
    """
    excess_returns = returns - risk_free_rate / 252  # Daily risk-free rate
    
    if len(excess_returns) < 2:
        return 0.0
    
    sharpe = np.sqrt(252) * excess_returns.mean() / excess_returns.std()
    
    return sharpe

# Example usage
if __name__ == "__main__":
    # Generate sample returns
    returns = pd.Series(np.random.randn(252) * 0.01 + 0.0005)
    sharpe = calculate_sharpe_ratio(returns)
    print(f"Sharpe Ratio: {sharpe:.2f}")`;
        } else if (fileName === 'max_drawdown.py') {
          content = `# Calculate Maximum Drawdown
import pandas as pd
import numpy as np

def calculate_max_drawdown(equity_curve: pd.Series):
    """
    Calculate the maximum drawdown from an equity curve.
    
    Args:
        equity_curve: Series of portfolio values
    
    Returns:
        tuple: (max_drawdown, peak_date, trough_date)
    """
    # Calculate running maximum
    running_max = equity_curve.cummax()
    
    # Calculate drawdown
    drawdown = (equity_curve - running_max) / running_max
    
    # Find maximum drawdown
    max_dd = drawdown.min()
    max_dd_idx = drawdown.idxmin()
    
    # Find the peak before the max drawdown
    peak_idx = equity_curve[:max_dd_idx].idxmax()
    
    return max_dd, peak_idx, max_dd_idx

# Example usage
if __name__ == "__main__":
    # Generate sample equity curve
    dates = pd.date_range('2024-01-01', periods=252, freq='D')
    equity = pd.Series(np.cumprod(1 + np.random.randn(252) * 0.01), index=dates)
    
    max_dd, peak, trough = calculate_max_drawdown(equity)
    print(f"Max Drawdown: {max_dd:.2%}")
    print(f"Peak: {peak}, Trough: {trough}")`;
        } else {
          content = `# Calculate Win Rate
import pandas as pd
import numpy as np

def calculate_win_rate(trades: pd.DataFrame):
    """
    Calculate win rate from trade history.
    
    Args:
        trades: DataFrame with 'pnl' column
    
    Returns:
        dict: Win rate metrics
    """
    winning_trades = trades[trades['pnl'] > 0]
    losing_trades = trades[trades['pnl'] <= 0]
    
    metrics = {
        'win_rate': len(winning_trades) / len(trades) * 100,
        'total_trades': len(trades),
        'winning_trades': len(winning_trades),
        'losing_trades': len(losing_trades),
        'avg_win': winning_trades['pnl'].mean() if len(winning_trades) > 0 else 0,
        'avg_loss': losing_trades['pnl'].mean() if len(losing_trades) > 0 else 0
    }
    
    return metrics`;
        }
      } else if (filePath.includes('visualizations/')) {
        if (fileName === 'plot_pnl.py') {
          content = `# Plot P&L Curve
import matplotlib.pyplot as plt
import pandas as pd
import numpy as np

def plot_pnl_curve(pnl_series: pd.Series, title: str = "P&L Curve"):
    """
    Plot cumulative P&L curve with drawdown shading.
    
    Args:
        pnl_series: Series of P&L values
        title: Chart title
    """
    fig, (ax1, ax2) = plt.subplots(2, 1, figsize=(12, 8), height_ratios=[3, 1])
    
    # Cumulative P&L
    cum_pnl = pnl_series.cumsum()
    ax1.plot(cum_pnl.index, cum_pnl.values, 'b-', linewidth=2)
    ax1.fill_between(cum_pnl.index, 0, cum_pnl.values, alpha=0.3)
    ax1.set_title(title)
    ax1.set_ylabel('Cumulative P&L ($)')
    ax1.grid(True, alpha=0.3)
    
    # Drawdown
    running_max = cum_pnl.cummax()
    drawdown = cum_pnl - running_max
    ax2.fill_between(drawdown.index, 0, drawdown.values, color='red', alpha=0.3)
    ax2.set_ylabel('Drawdown ($)')
    ax2.set_xlabel('Date')
    ax2.grid(True, alpha=0.3)
    
    plt.tight_layout()
    return fig`;
        } else if (fileName === 'candlestick_chart.py') {
          content = `# Create Candlestick Chart
import plotly.graph_objects as go
import pandas as pd

def plot_candlestick(df: pd.DataFrame, title: str = "Price Chart"):
    """
    Create interactive candlestick chart.
    
    Args:
        df: DataFrame with OHLC data
        title: Chart title
    """
    fig = go.Figure(data=[go.Candlestick(
        x=df.index,
        open=df['open'],
        high=df['high'],
        low=df['low'],
        close=df['close'],
        name='OHLC'
    )])
    
    fig.update_layout(
        title=title,
        yaxis_title='Price',
        xaxis_title='Date',
        template='plotly_dark',
        xaxis_rangeslider_visible=False
    )
    
    return fig`;
        } else {
          content = `# Create Correlation Heatmap
import seaborn as sns
import matplotlib.pyplot as plt
import pandas as pd

def plot_correlation_heatmap(df: pd.DataFrame, title: str = "Correlation Matrix"):
    """
    Create correlation heatmap.
    
    Args:
        df: DataFrame with numeric columns
        title: Chart title
    """
    plt.figure(figsize=(10, 8))
    
    # Calculate correlation matrix
    corr_matrix = df.corr()
    
    # Create heatmap
    sns.heatmap(
        corr_matrix,
        annot=True,
        fmt='.2f',
        cmap='coolwarm',
        center=0,
        square=True,
        linewidths=1,
        cbar_kws={"shrink": 0.8}
    )
    
    plt.title(title)
    plt.tight_layout()
    return plt.gcf()`;
        }
      } else if (filePath.includes('analysis_templates/')) {
        content = `# ${fileName.replace('.py', '').replace('_', ' ').toUpperCase()} Template
import pandas as pd
import numpy as np
import matplotlib.pyplot as plt
from datetime import datetime

# Analysis configuration
CONFIG = {
    'lookback_period': 252,  # Trading days
    'confidence_level': 0.95,
    'initial_capital': 100000
}

def run_analysis(data: pd.DataFrame):
    """
    Run comprehensive ${fileName.replace('.py', '').replace('_', ' ')} analysis.
    
    Args:
        data: Input data for analysis
    
    Returns:
        dict: Analysis results
    """
    results = {}
    
    # Add your analysis logic here
    print(f"Running {fileName.replace('.py', '').replace('_', ' ')}...")
    
    return results

# Example usage
if __name__ == "__main__":
    # Load your data
    data = pd.DataFrame()  # Replace with actual data loading
    
    # Run analysis
    results = run_analysis(data)
    
    # Display results
    for key, value in results.items():
        print(f"{key}: {value}")`;
      } else if (filePath.includes('builder-ui/')) {
        if (fileName === 'signal_analysis.py') {
          content = `"""Signal Analysis UI Component for StrategyWorkbench

This module provides the signal analysis interface for the StrategyWorkbench.
Users can create custom UI components that integrate with Jupyter notebooks.
"""

import pandas as pd
import numpy as np
import plotly.graph_objects as go
from plotly.subplots import make_subplots
from datetime import datetime, timedelta
from typing import Dict, List, Optional, Tuple
import ipywidgets as widgets
from IPython.display import display, HTML

class SignalAnalysisUI:
    """Interactive signal analysis dashboard for strategy development."""
    
    def __init__(self, signals_df: pd.DataFrame, price_df: pd.DataFrame):
        self.signals = signals_df
        self.prices = price_df
        self.current_symbol = None
        self.widgets = {}
        self._setup_ui()
    
    def _setup_ui(self):
        """Initialize all UI components."""
        # Date range selector
        self.widgets['date_start'] = widgets.DatePicker(
            description='Start Date:',
            value=datetime.now() - timedelta(days=30)
        )
        self.widgets['date_end'] = widgets.DatePicker(
            description='End Date:',
            value=datetime.now()
        )
        
        # Symbol selector
        symbols = self.signals['symbol'].unique() if 'symbol' in self.signals.columns else ['SPY']
        self.widgets['symbol'] = widgets.Dropdown(
            options=symbols,
            value=symbols[0],
            description='Symbol:'
        )
        
        # Signal type filter
        self.widgets['signal_type'] = widgets.SelectMultiple(
            options=['BUY', 'SELL', 'HOLD'],
            value=['BUY', 'SELL'],
            description='Signals:'
        )
        
        # Confidence threshold
        self.widgets['confidence'] = widgets.FloatSlider(
            value=0.5,
            min=0.0,
            max=1.0,
            step=0.05,
            description='Min Confidence:'
        )
        
        # Analysis buttons
        self.widgets['analyze_btn'] = widgets.Button(
            description='Run Analysis',
            button_style='primary',
            icon='chart-line'
        )
        self.widgets['analyze_btn'].on_click(self._on_analyze)
        
        self.widgets['export_btn'] = widgets.Button(
            description='Export Results',
            button_style='success',
            icon='download'
        )
        self.widgets['export_btn'].on_click(self._on_export)
        
        # Output area
        self.widgets['output'] = widgets.Output()
    
    def _on_analyze(self, btn):
        """Handle analysis button click."""
        with self.widgets['output']:
            self.widgets['output'].clear_output()
            
            # Get filter parameters
            symbol = self.widgets['symbol'].value
            start_date = self.widgets['date_start'].value
            end_date = self.widgets['date_end'].value
            signal_types = self.widgets['signal_type'].value
            min_confidence = self.widgets['confidence'].value
            
            # Filter signals
            filtered_signals = self._filter_signals(
                symbol, start_date, end_date, signal_types, min_confidence
            )
            
            # Create visualizations
            fig = self._create_signal_chart(filtered_signals, symbol)
            fig.show()
            
            # Display statistics
            stats = self._calculate_statistics(filtered_signals)
            self._display_statistics(stats)
    
    def _filter_signals(self, symbol, start_date, end_date, signal_types, min_confidence):
        """Filter signals based on user criteria."""
        df = self.signals.copy()
        
        # Apply filters
        if 'symbol' in df.columns:
            df = df[df['symbol'] == symbol]
        if 'date' in df.columns:
            df = df[(df['date'] >= pd.Timestamp(start_date)) & 
                   (df['date'] <= pd.Timestamp(end_date))]
        if 'signal' in df.columns:
            df = df[df['signal'].isin(signal_types)]
        if 'confidence' in df.columns:
            df = df[df['confidence'] >= min_confidence]
        
        return df
    
    def _create_signal_chart(self, signals_df, symbol):
        """Create interactive signal visualization."""
        fig = make_subplots(
            rows=3, cols=1,
            subplot_titles=('Price & Signals', 'Signal Confidence', 'Returns'),
            vertical_spacing=0.05,
            row_heights=[0.5, 0.25, 0.25]
        )
        
        # Price chart with signals
        if symbol in self.prices.columns:
            fig.add_trace(
                go.Scatter(
                    x=self.prices.index,
                    y=self.prices[symbol],
                    mode='lines',
                    name='Price',
                    line=dict(color='#00d4db')
                ),
                row=1, col=1
            )
        
        # Add buy signals
        buy_signals = signals_df[signals_df['signal'] == 'BUY']
        if not buy_signals.empty:
            fig.add_trace(
                go.Scatter(
                    x=buy_signals['date'],
                    y=buy_signals['price'],
                    mode='markers',
                    name='Buy',
                    marker=dict(color='green', size=10, symbol='triangle-up')
                ),
                row=1, col=1
            )
        
        # Add sell signals
        sell_signals = signals_df[signals_df['signal'] == 'SELL']
        if not sell_signals.empty:
            fig.add_trace(
                go.Scatter(
                    x=sell_signals['date'],
                    y=sell_signals['price'],
                    mode='markers',
                    name='Sell',
                    marker=dict(color='red', size=10, symbol='triangle-down')
                ),
                row=1, col=1
            )
        
        # Confidence chart
        if 'confidence' in signals_df.columns:
            fig.add_trace(
                go.Bar(
                    x=signals_df['date'],
                    y=signals_df['confidence'],
                    name='Confidence',
                    marker_color=signals_df['confidence'],
                    marker_colorscale='Viridis'
                ),
                row=2, col=1
            )
        
        # Returns chart
        if 'returns' in signals_df.columns:
            fig.add_trace(
                go.Bar(
                    x=signals_df['date'],
                    y=signals_df['returns'],
                    name='Returns',
                    marker_color=['green' if r > 0 else 'red' for r in signals_df['returns']]
                ),
                row=3, col=1
            )
        
        # Update layout
        fig.update_layout(
            height=800,
            showlegend=True,
            title_text=f"Signal Analysis: {symbol}",
            hovermode='x unified'
        )
        
        return fig
    
    def _calculate_statistics(self, signals_df):
        """Calculate signal performance statistics."""
        stats = {}
        
        if signals_df.empty:
            return stats
        
        stats['total_signals'] = len(signals_df)
        stats['buy_signals'] = len(signals_df[signals_df['signal'] == 'BUY'])
        stats['sell_signals'] = len(signals_df[signals_df['signal'] == 'SELL'])
        
        if 'confidence' in signals_df.columns:
            stats['avg_confidence'] = signals_df['confidence'].mean()
            stats['max_confidence'] = signals_df['confidence'].max()
            stats['min_confidence'] = signals_df['confidence'].min()
        
        if 'returns' in signals_df.columns:
            stats['total_return'] = signals_df['returns'].sum()
            stats['avg_return'] = signals_df['returns'].mean()
            stats['win_rate'] = (signals_df['returns'] > 0).mean()
            stats['sharpe_ratio'] = (signals_df['returns'].mean() / 
                                    signals_df['returns'].std() * np.sqrt(252))
        
        return stats
    
    def _display_statistics(self, stats):
        """Display formatted statistics."""
        html = "<h3>Signal Statistics</h3>"
        html += "<table style='width:100%; border-collapse: collapse;'>"
        
        for key, value in stats.items():
            formatted_key = key.replace('_', ' ').title()
            if isinstance(value, float):
                formatted_value = f"{value:.4f}"
            else:
                formatted_value = str(value)
            
            html += f"""<tr style='border-bottom: 1px solid #ddd;'>
                          <td style='padding: 8px; font-weight: bold;'>{formatted_key}:</td>
                          <td style='padding: 8px;'>{formatted_value}</td>
                        </tr>"""
        
        html += "</table>"
        display(HTML(html))
    
    def _on_export(self, btn):
        """Export analysis results."""
        with self.widgets['output']:
            print("Exporting results...")
            # Implementation for export functionality
            timestamp = datetime.now().strftime('%Y%m%d_%H%M%S')
            filename = f"signal_analysis_{timestamp}.csv"
            print(f"Results exported to {filename}")
    
    def display(self):
        """Display the complete UI."""
        # Layout components
        controls = widgets.VBox([
            widgets.HBox([self.widgets['symbol'], self.widgets['confidence']]),
            widgets.HBox([self.widgets['date_start'], self.widgets['date_end']]),
            self.widgets['signal_type'],
            widgets.HBox([self.widgets['analyze_btn'], self.widgets['export_btn']])
        ])
        
        # Main dashboard
        dashboard = widgets.VBox([
            widgets.HTML("<h2>Signal Analysis Dashboard</h2>"),
            controls,
            self.widgets['output']
        ])
        
        display(dashboard)

# Example usage
if __name__ == "__main__":
    # Create sample data
    dates = pd.date_range(start='2024-01-01', periods=100, freq='D')
    signals_data = pd.DataFrame({
        'date': dates,
        'symbol': 'SPY',
        'signal': np.random.choice(['BUY', 'SELL', 'HOLD'], 100),
        'price': 400 + np.random.randn(100) * 10,
        'confidence': np.random.uniform(0.3, 1.0, 100),
        'returns': np.random.randn(100) * 0.02
    })
    
    price_data = pd.DataFrame(
        {'SPY': 400 + np.cumsum(np.random.randn(100) * 2)},
        index=dates
    )
    
    # Create and display UI
    ui = SignalAnalysisUI(signals_data, price_data)
    ui.display()
`;
        } else if (fileName === 'strategy_workbench.py') {
          content = `"""StrategyWorkbench - Main UI Framework

Button-driven Jupyter Notebook interface for strategy development.
"""

import ipywidgets as widgets
from IPython.display import display, clear_output
import pandas as pd
import numpy as np
from typing import Dict, Any, Callable

class StrategyWorkbench:
    """Main workbench UI for strategy development."""
    
    def __init__(self):
        self.current_view = 'home'
        self.strategy_data = {}
        self.widgets = {}
        self._initialize_ui()
    
    def _initialize_ui(self):
        """Initialize the main UI components."""
        # Navigation bar
        self.widgets['nav_home'] = widgets.Button(description='Home', button_style='info')
        self.widgets['nav_data'] = widgets.Button(description='Data', button_style='info')
        self.widgets['nav_strategy'] = widgets.Button(description='Strategy', button_style='info')
        self.widgets['nav_backtest'] = widgets.Button(description='Backtest', button_style='info')
        self.widgets['nav_deploy'] = widgets.Button(description='Deploy', button_style='info')
        
        # Bind navigation
        self.widgets['nav_home'].on_click(lambda b: self._switch_view('home'))
        self.widgets['nav_data'].on_click(lambda b: self._switch_view('data'))
        self.widgets['nav_strategy'].on_click(lambda b: self._switch_view('strategy'))
        self.widgets['nav_backtest'].on_click(lambda b: self._switch_view('backtest'))
        self.widgets['nav_deploy'].on_click(lambda b: self._switch_view('deploy'))
        
        # Main content area
        self.widgets['content'] = widgets.Output()
        
        # Status bar
        self.widgets['status'] = widgets.HTML(value='<b>Status:</b> Ready')
    
    def _switch_view(self, view_name: str):
        """Switch between different views."""
        self.current_view = view_name
        with self.widgets['content']:
            clear_output()
            if view_name == 'home':
                self._show_home_view()
            elif view_name == 'data':
                self._show_data_view()
            elif view_name == 'strategy':
                self._show_strategy_view()
            elif view_name == 'backtest':
                self._show_backtest_view()
            elif view_name == 'deploy':
                self._show_deploy_view()
    
    def _show_home_view(self):
        """Display the home dashboard."""
        display(widgets.HTML('<h2>Strategy Workbench</h2>'))
        display(widgets.HTML('<p>Welcome to the Strategy Workbench. Select a module to begin.</p>'))
    
    def _show_data_view(self):
        """Display data management interface."""
        display(widgets.HTML('<h3>Data Management</h3>'))
        # Add data loading controls here
    
    def _show_strategy_view(self):
        """Display strategy builder interface."""
        display(widgets.HTML('<h3>Strategy Builder</h3>'))
        # Add strategy building controls here
    
    def _show_backtest_view(self):
        """Display backtesting interface."""
        display(widgets.HTML('<h3>Backtesting</h3>'))
        # Add backtesting controls here
    
    def _show_deploy_view(self):
        """Display deployment interface."""
        display(widgets.HTML('<h3>Deployment</h3>'))
        # Add deployment controls here
    
    def display(self):
        """Display the complete workbench."""
        navbar = widgets.HBox([
            self.widgets['nav_home'],
            self.widgets['nav_data'],
            self.widgets['nav_strategy'],
            self.widgets['nav_backtest'],
            self.widgets['nav_deploy']
        ])
        
        main_ui = widgets.VBox([
            navbar,
            self.widgets['content'],
            self.widgets['status']
        ])
        
        display(main_ui)
        self._switch_view('home')

# Initialize workbench
workbench = StrategyWorkbench()
workbench.display()
`;
        } else if (fileName === 'components.py') {
          content = `"""Reusable UI Components for StrategyWorkbench"""

import ipywidgets as widgets
from typing import List, Dict, Any, Optional

class DataSelector(widgets.VBox):
    """Widget for selecting and loading data."""
    
    def __init__(self, data_sources: List[str]):
        self.source_dropdown = widgets.Dropdown(
            options=data_sources,
            description='Data Source:'
        )
        self.date_range = widgets.SelectionRangeSlider(
            options=pd.date_range('2020-01-01', '2024-12-31', freq='D'),
            index=(0, 365),
            description='Date Range:'
        )
        self.load_button = widgets.Button(
            description='Load Data',
            button_style='primary'
        )
        
        super().__init__([self.source_dropdown, self.date_range, self.load_button])

class StrategyBuilder(widgets.VBox):
    """Widget for building trading strategies."""
    
    def __init__(self):
        self.strategy_type = widgets.Dropdown(
            options=['Moving Average', 'Mean Reversion', 'Momentum', 'Custom'],
            description='Strategy Type:'
        )
        self.parameters = widgets.Textarea(
            value='{}',
            description='Parameters:',
            layout=widgets.Layout(width='100%', height='100px')
        )
        self.validate_button = widgets.Button(
            description='Validate',
            button_style='warning'
        )
        
        super().__init__([self.strategy_type, self.parameters, self.validate_button])

class BacktestRunner(widgets.VBox):
    """Widget for running backtests."""
    
    def __init__(self):
        self.initial_capital = widgets.FloatText(
            value=100000,
            description='Initial Capital:'
        )
        self.commission = widgets.FloatSlider(
            value=0.001,
            min=0,
            max=0.01,
            step=0.0001,
            description='Commission:'
        )
        self.run_button = widgets.Button(
            description='Run Backtest',
            button_style='success'
        )
        self.progress = widgets.FloatProgress(
            value=0,
            min=0,
            max=100,
            description='Progress:'
        )
        
        super().__init__([self.initial_capital, self.commission, self.run_button, self.progress])
`;
        } else if (fileName === 'config.json') {
          content = `{
  "workbench": {
    "name": "StrategyWorkbench",
    "version": "1.0.0",
    "description": "Button-driven Jupyter Notebook interface for strategy development"
  },
  "modules": {
    "signal_analysis": {
      "enabled": true,
      "default_params": {
        "lookback_period": 30,
        "confidence_threshold": 0.5,
        "signal_types": ["BUY", "SELL"]
      }
    },
    "strategy_builder": {
      "enabled": true,
      "available_strategies": [
        "moving_average",
        "mean_reversion",
        "momentum",
        "pairs_trading",
        "machine_learning"
      ]
    },
    "backtester": {
      "enabled": true,
      "default_params": {
        "initial_capital": 100000,
        "commission": 0.001,
        "slippage": 0.0005,
        "risk_free_rate": 0.02
      }
    }
  },
  "data_sources": [
    {
      "name": "Alpaca",
      "type": "market_data",
      "enabled": true
    },
    {
      "name": "Local CSV",
      "type": "file",
      "enabled": true
    },
    {
      "name": "AlphaPulse Signals",
      "type": "api",
      "enabled": true
    }
  ],
  "ui_settings": {
    "theme": "dark",
    "auto_save": true,
    "save_interval_seconds": 300,
    "max_chart_points": 10000
  }
}`;
        }
      } else if (filePath.includes('saved_notebooks/')) {
        // For notebook files, provide a different format
        content = `{
  "cells": [
    {
      "cell_type": "markdown",
      "metadata": {},
      "source": [
        "# ${fileName.replace('.ipynb', '').replace('_', ' ').toUpperCase()}\\n",
        "\\n",
        "Research notebook for strategy analysis and backtesting."
      ]
    },
    {
      "cell_type": "code",
      "execution_count": null,
      "metadata": {},
      "outputs": [],
      "source": [
        "# Import required libraries\\n",
        "import pandas as pd\\n",
        "import numpy as np\\n",
        "import matplotlib.pyplot as plt\\n",
        "import admf\\n",
        "from analysis_lib import *"
      ]
    },
    {
      "cell_type": "code",
      "execution_count": null,
      "metadata": {},
      "outputs": [],
      "source": [
        "# Load strategy data\\n",
        "strategy_id = '${fileName.replace('.ipynb', '').replace('_', '-')}'\\n",
        "signals = admf.load_signals(strategy_type=strategy_id, limit=100)\\n",
        "print(f'Loaded {len(signals)} signals')"
      ]
    }
  ],
  "metadata": {
    "kernelspec": {
      "display_name": "Python 3",
      "language": "python",
      "name": "python3"
    }
  },
  "nbformat": 4,
  "nbformat_minor": 4
}`;
      }
    } else {
      // Default content for other files
      content = `# ${fileName}
# This is a placeholder for the actual file content
# Content will be loaded from the backend

def main():
    print("AlphaPulse Trading Strategy")
    
if __name__ == "__main__":
    main()
`;
    }
    
    // Add new tab
    const newTab: Tab = {
      id: filePath,
      name: fileName,
      content,
      language: fileName.endsWith('.py') ? 'python' : 
                fileName.endsWith('.yaml') || fileName.endsWith('.yml') ? 'yaml' :
                fileName.endsWith('.json') ? 'json' :
                fileName.endsWith('.ipynb') ? 'json' :
                fileName.endsWith('.md') ? 'markdown' : 'text'
    };
    
    setTabs([...tabs, newTab]);
    setActiveTab(filePath);
  };

  const closeTab = (tabId: string, e: React.MouseEvent) => {
    e.stopPropagation();
    
    const tabIndex = tabs.findIndex(tab => tab.id === tabId);
    const newTabs = tabs.filter(tab => tab.id !== tabId);
    setTabs(newTabs);
    
    // If closing active tab, switch to another
    if (activeTab === tabId && newTabs.length > 0) {
      const newActiveIndex = Math.max(0, tabIndex - 1);
      setActiveTab(newTabs[newActiveIndex].id);
    }
  };

  const saveFile = () => {
    const activeTabData = tabs.find(tab => tab.id === activeTab);
    if (activeTabData) {
      addOutput(`Saving ${activeTabData.name}...`);
      // TODO: Implement actual save logic
      setTimeout(() => {
        addOutput(`✓ ${activeTabData.name} saved successfully`);
      }, 500);
    }
  };

  const runCode = async () => {
    const activeTabData = tabs.find(tab => tab.id === activeTab);
    if (activeTabData) {
      setOutputOpen(true);
      
      // Clear and reinitialize with Nautilus art
      initializeConsole();
      
      // Add execution messages after a delay
      setTimeout(() => {
        const timestamp = new Date().toISOString();
        addOutput('');
        addOutput(`${timestamp} [INFO] BACKTESTER-001.BacktestEngine: Starting backtest...`);
        addOutput(`${timestamp} [INFO] BACKTESTER-001.BacktestEngine: Loading ${activeTabData.name}`);
        addOutput(`${timestamp} [INFO] BACKTESTER-001.BacktestEngine: Strategy initialized`);
        addOutput(`${timestamp} [INFO] BACKTESTER-001.BacktestEngine: Connected to data feed`);
        addOutput(`${timestamp} [INFO] BACKTESTER-001.BacktestEngine: Executing strategy...`);
        addOutput(`${timestamp} [INFO] BACKTESTER-001.BacktestEngine: Processing historical data...`);
      }, 500);
      
      setTimeout(() => {
        const timestamp = new Date().toISOString();
        addOutput(`${timestamp} [INFO] BACKTESTER-001.BacktestEngine: Backtest complete`);
        addOutput(`${timestamp} [INFO] BACKTESTER-001.BacktestEngine: Total PnL: $12,345.67`);
        addOutput(`${timestamp} [INFO] BACKTESTER-001.BacktestEngine: Sharpe Ratio: 1.85`);
        addOutput(`${timestamp} [INFO] BACKTESTER-001.BacktestEngine: Max Drawdown: -8.3%`);
        addOutput(`${timestamp} [INFO] BACKTESTER-001.BacktestEngine: Win Rate: 63.2%`);
      }, 2000);
    }
  };


  const calculateDefaultSplitSize = (orientation?: 'horizontal' | 'vertical') => {
    const mainArea = document.querySelector(`.${styles.mainArea}`) as HTMLElement;
    if (!mainArea) return 300; // fallback
    
    const currentOrientation = orientation || splitOrientation;
    if (currentOrientation === 'horizontal') {
      const height = mainArea.clientHeight;
      // Terminal takes exactly 50% of available height
      return Math.floor(height / 2);
    } else {
      const width = mainArea.clientWidth;
      // Terminal takes exactly 50% of available width
      return Math.floor(width / 2);
    }
  };

  const handleSplitDragStart = (e: React.MouseEvent) => {
    setIsDragging(true);
    e.preventDefault();
  };

  const handleSplitDrag = (e: MouseEvent) => {
    if (!isDragging) return;
    
    const mainArea = document.querySelector(`.${styles.mainArea}`) as HTMLElement;
    if (!mainArea) return;
    
    const rect = mainArea.getBoundingClientRect();
    
    if (splitOrientation === 'horizontal') {
      const newHeight = rect.bottom - e.clientY;
      setSplitSize(Math.max(100, Math.min(newHeight, rect.height - 100)));
    } else {
      const newWidth = rect.right - e.clientX;
      setSplitSize(Math.max(200, Math.min(newWidth, rect.width - 200)));
    }
  };

  const handleSplitDragEnd = () => {
    setIsDragging(false);
  };

  // Sidebar drag handlers
  const handleSidebarDragStart = (e: React.MouseEvent) => {
    setIsSidebarDragging(true);
    e.preventDefault();
  };

  const handleSidebarDrag = (e: MouseEvent) => {
    if (!isSidebarDragging) return;
    
    const newWidth = e.clientX;
    setSidebarWidth(Math.max(200, Math.min(600, newWidth)));
  };

  const handleSidebarDragEnd = () => {
    setIsSidebarDragging(false);
  };

  // Add mouse event listeners for split drag
  useEffect(() => {
    if (isDragging) {
      document.addEventListener('mousemove', handleSplitDrag);
      document.addEventListener('mouseup', handleSplitDragEnd);
      document.body.style.cursor = splitOrientation === 'horizontal' ? 'row-resize' : 'col-resize';
      document.body.style.userSelect = 'none';
      
      return () => {
        document.removeEventListener('mousemove', handleSplitDrag);
        document.removeEventListener('mouseup', handleSplitDragEnd);
        document.body.style.cursor = '';
        document.body.style.userSelect = '';
      };
    }
  }, [isDragging, splitOrientation]);

  // Add mouse event listeners for sidebar drag
  useEffect(() => {
    if (isSidebarDragging) {
      document.addEventListener('mousemove', handleSidebarDrag);
      document.addEventListener('mouseup', handleSidebarDragEnd);
      document.body.style.cursor = 'col-resize';
      document.body.style.userSelect = 'none';
      
      return () => {
        document.removeEventListener('mousemove', handleSidebarDrag);
        document.removeEventListener('mouseup', handleSidebarDragEnd);
        document.body.style.cursor = '';
        document.body.style.userSelect = '';
      };
    }
  }, [isSidebarDragging]);


  const renderFileTree = (items: FileItem[], level = 0) => {
    return items.map(item => {
      if (item.type === 'folder') {
        const isExpanded = expandedFolders.has(item.path);
        return (
          <div key={item.path}>
            <div 
              className={`${styles.folderItem} ${!isExpanded ? styles.collapsed : ''}`}
              style={{ paddingLeft: `${level * 20 + 12}px` }}
              onClick={() => toggleFolder(item.path)}
            >
              <span className={styles.folderIcon}>▼</span>
              <span>{item.name}/</span>
            </div>
            {isExpanded && item.children && (
              <div className={styles.folderContents} style={{ display: 'block' }}>
                {renderFileTree(item.children, level + 1)}
              </div>
            )}
          </div>
        );
      } else {
        return (
          <div
            key={item.path}
            className={`${styles.fileItem} ${activeTab === item.path ? styles.active : ''}`}
            style={{ paddingLeft: `${level * 20 + 32}px` }}
            onClick={() => openFile(item.path, item.name)}
          >
            <span className={styles.fileIcon}>
              {item.name.endsWith('.py') ? 'PY' : 
               item.name.endsWith('.yaml') || item.name.endsWith('.yml') ? 'YML' :
               item.name.endsWith('.json') ? 'JSON' :
               item.name.endsWith('.md') ? 'MD' : 'TXT'}
            </span>
            <span>{item.name}</span>
          </div>
        );
      }
    });
  };

  const activeTabData = tabs.find(tab => tab.id === activeTab);

  return (
    <div className={styles.developContainer}>
      <aside 
        className={`${styles.sidebar} ${!sidebarOpen ? styles.sidebarClosed : ''}`}
        style={{ width: sidebarOpen ? `${sidebarWidth}px` : '0' }}
      >
        <div className={styles.sidebarHeader}>
          <div className={styles.sidebarTabs}>
            <button 
              className={`${styles.sidebarTab} ${sidebarView === 'explorer' ? styles.active : ''}`}
              onClick={() => setSidebarView('explorer')}
              title="Explorer"
            >
              <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path>
              </svg>
            </button>
            <button 
              className={`${styles.sidebarTab} ${sidebarView === 'git' ? styles.active : ''}`}
              onClick={() => setSidebarView('git')}
              title="Source Control"
            >
              <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                {/* Git branching icon - diagonal branches */}
                <circle cx="12" cy="3" r="2.5"></circle>
                <circle cx="12" cy="21" r="2.5"></circle>
                <circle cx="21" cy="12" r="2.5"></circle>
                <path d="M12 5.5v13"></path>
                {/* Diagonal lines stopping at node boundaries */}
                <path d="M5 -4l5 5"/>
                <path d="M14 5l5 5"/>
              </svg>
            </button>
          </div>
        </div>
        
        {sidebarView === 'explorer' && (
          <>
            <div className={styles.explorerHeader}>
              <input
                type="text"
                className={styles.explorerSearch}
                placeholder="Search files..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
              />
            </div>
            <div className={styles.fileList}>
              {files.length > 0 ? (
                renderFileTree(files)
              ) : (
                <div className={styles.fileItem}>
                  <span className={styles.fileIcon}>⏳</span>
                  <span>Loading files...</span>
                </div>
              )}
            </div>
          </>
        )}
        
        {sidebarView === 'search' && (
          <div className={styles.searchPanel}>
            <div className={styles.searchHeader}>
              <input
                type="text"
                className={styles.searchInput}
                placeholder="Search in files..."
              />
            </div>
            <div className={styles.searchResults}>
              <p className={styles.emptyState}>Enter a search term to find in files</p>
            </div>
          </div>
        )}
        
        {sidebarView === 'git' && (
          <div className={styles.gitPanel}>
            <div className={styles.gitHeader}>
              <h3>Source Control</h3>
            </div>
            <div className={styles.gitChanges}>
              <div className={styles.gitSection}>
                <div className={styles.gitSectionHeader}>
                  <span className={styles.gitSectionTitle}>Changes (3)</span>
                  <button className={styles.gitStageAllBtn} title="Stage All Changes">+</button>
                </div>
                <div className={styles.gitFileList}>
                  <div className={styles.gitFile}>
                    <span className={styles.gitFileStatus}>M</span>
                    <span className={styles.gitFileName}>strategy.py</span>
                    <div className={styles.gitFileActions}>
                      <button title="Stage Changes">+</button>
                      <button title="Discard Changes">↻</button>
                    </div>
                  </div>
                  <div className={styles.gitFile}>
                    <span className={styles.gitFileStatus}>M</span>
                    <span className={styles.gitFileName}>config.json</span>
                    <div className={styles.gitFileActions}>
                      <button title="Stage Changes">+</button>
                      <button title="Discard Changes">↻</button>
                    </div>
                  </div>
                  <div className={styles.gitFile}>
                    <span className={styles.gitFileStatus}>A</span>
                    <span className={styles.gitFileName}>backtest_results.csv</span>
                    <div className={styles.gitFileActions}>
                      <button title="Stage Changes">+</button>
                      <button title="Discard Changes">↻</button>
                    </div>
                  </div>
                </div>
              </div>
              
              <div className={styles.gitSection}>
                <div className={styles.gitSectionHeader}>
                  <span className={styles.gitSectionTitle}>Staged Changes (0)</span>
                  <button className={styles.gitUnstageAllBtn} title="Unstage All">-</button>
                </div>
                <p className={styles.gitEmptyState}>No staged changes</p>
              </div>
              
              <div className={styles.gitCommitSection}>
                <input
                  type="text"
                  className={styles.gitCommitInput}
                  placeholder="Commit message..."
                />
                <button className={styles.gitCommitBtn}>Commit</button>
              </div>
              
              <div className={styles.gitBranchInfo}>
                <div className={styles.gitBranch}>
                  <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                    <line x1="6" y1="3" x2="6" y2="15"></line>
                    <circle cx="18" cy="6" r="3"></circle>
                    <circle cx="6" cy="18" r="3"></circle>
                    <path d="M18 9a9 9 0 0 1-9 9"></path>
                  </svg>
                  <span>main</span>
                </div>
                <button className={styles.gitSyncBtn}>↓ Pull ↑ Push</button>
              </div>
            </div>
          </div>
        )}
      </aside>
      
      {/* Sidebar Resize Handle */}
      {sidebarOpen && (
        <div 
          className={styles.sidebarResizeHandle}
          onMouseDown={handleSidebarDragStart}
        />
      )}
      
      <div 
        className={`${styles.mainArea} ${outputOpen ? (splitOrientation === 'horizontal' ? styles.splitHorizontal : styles.splitVertical) : ''}`}
      >
        <div className={`${styles.editorContainer} ${outputOpen && splitOrientation === 'vertical' ? styles.splitVertical : ''}`}>
          {!editorHidden && (
            <div className={styles.tabsContainer}>
            <div className={styles.tabs}>
              {tabs.map(tab => (
                <div
                  key={tab.id}
                  className={`${styles.tab} ${activeTab === tab.id ? styles.active : ''}`}
                  onClick={() => setActiveTab(tab.id)}
                >
                  <span className={styles.tabName}>{tab.name}</span>
                  <button 
                    className={styles.tabClose}
                    onClick={(e) => closeTab(tab.id, e)}
                  >
                    ×
                  </button>
                </div>
              ))}
              <button className={styles.newTabBtn} title="New File">+</button>
            </div>
            <div className={styles.editorActions}>
              <button className={styles.actionButton} onClick={saveFile} title="Save File">
                <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                  <path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z"></path>
                  <polyline points="17 21 17 13 7 13 7 21"></polyline>
                  <polyline points="7 3 7 8 15 8"></polyline>
                </svg>
              </button>
              <button className={styles.actionButton} onClick={() => {
                setOutputOpen(true);
                if (!terminalTabs[0].content.length) {
                  setTimeout(() => initializeConsole(), 100);
                }
                if (splitSize === 0) {
                  setSplitSize(calculateDefaultSplitSize());
                }
              }} title="Open Terminal">
                <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                  <polyline points="4 17 10 11 4 5"></polyline>
                  <line x1="12" y1="19" x2="20" y2="19"></line>
                </svg>
              </button>
              <button className={styles.actionButton} onClick={() => {
                setEditorHidden(true);
                // Ensure terminal is open when editor is closed
                if (!outputOpen) {
                  setOutputOpen(true);
                  setTimeout(() => {
                    if (!terminalTabs[0].content.length) {
                      initializeConsole();
                    }
                  }, 100);
                }
              }} title="Close Editor">
                <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                  <path d="M18 6L6 18M6 6l12 12"></path>
                </svg>
              </button>
            </div>
          </div>
          )}

          {!editorHidden && (
            <div 
              className={`${styles.editorWrapper} ${outputOpen ? (splitOrientation === 'horizontal' ? styles.splitHorizontal : styles.splitVertical) : ''}`}
              style={{
                ...(outputOpen ? { 
                  flex: '1 1 auto',
                  minWidth: '200px',
                  minHeight: '200px'
                } : { flex: '1' })
              }}
            >
              {activeTabData ? (
                <div className={styles.editor}>
                  <CodeEditor
                    value={activeTabData.content}
                    onChange={(newContent) => {
                      const newTabs = tabs.map(tab => 
                        tab.id === activeTab 
                          ? { ...tab, content: newContent }
                          : tab
                      );
                      setTabs(newTabs);
                    }}
                    language={activeTabData.language}
                  />
                </div>
              ) : (
                <div className={styles.welcome}>
                  <h2>AlphaPulse Development</h2>
                  <p>Select a file from the sidebar to start coding your trading strategies.</p>
                  <button 
                    className={styles.openFilesBtn}
                    onClick={() => setSidebarOpen(true)}
                  >
                    Open Files
                  </button>
                </div>
              )}
            </div>
          )}
        </div>

        {outputOpen && (
          <>
            {!editorHidden && (
              <div 
                className={`${styles.splitter} ${splitOrientation === 'horizontal' ? styles.splitterHorizontal : styles.splitterVertical}`}
                onMouseDown={handleSplitDragStart}
              />
            )}
            <div 
              className={`${styles.outputPanel} ${styles[splitOrientation]} ${editorHidden ? styles.fullScreen : ''}`}
              style={{
                ...(!editorHidden ? { 
                  flex: `0 0 ${splitSize || 300}px`,
                  minWidth: splitOrientation === 'vertical' ? '250px' : 'auto',
                  minHeight: splitOrientation === 'horizontal' ? '150px' : 'auto'
                } : {})
              }}
            >
                <div className={styles.outputHeader}>
                  <div className={styles.terminalTabsContainer}>
                    <div className={styles.terminalTabs}>
                      {terminalTabs.map(tab => (
                        <div
                          key={tab.id}
                          className={`${styles.terminalTab} ${activeTerminalTab === tab.id ? styles.active : ''}`}
                          onClick={() => setActiveTerminalTab(tab.id)}
                        >
                          <span className={styles.terminalTabName} title={tab.cwd}>{getShortPath(tab.name)}</span>
                          {terminalTabs.length > 1 && (
                            <button 
                              className={styles.terminalTabClose}
                              onClick={(e) => {
                                e.stopPropagation();
                                closeTerminalTab(tab.id);
                              }}
                            >
                              ×
                            </button>
                          )}
                        </div>
                      ))}
                      <button 
                        className={styles.newTerminalTabBtn} 
                        onClick={addTerminalTab}
                        title="New Terminal"
                      >
                        +
                      </button>
                    </div>
                  </div>
                  <div className={styles.outputControls}>
                    <button 
                      className={`${styles.editorToggleBtn} ${editorHidden ? styles.editorHidden : ''}`}
                      onClick={() => {
                        if (editorHidden) {
                          setEditorHidden(false);
                        } else {
                          // Toggle focus to editor when it's already open
                          const activeTabElement = document.querySelector('.monaco-editor');
                          if (activeTabElement) {
                            (activeTabElement as HTMLElement).focus();
                          }
                        }
                      }}
                      title={editorHidden ? "Open Editor" : "Focus Editor"}
                    >
                      <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                        <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"></path>
                        <polyline points="14 2 14 8 20 8"></polyline>
                      </svg>
                    </button>
                    {!editorHidden && (
                      <button 
                        className={styles.splitToggleBtn}
                        onClick={() => {
                          const newOrientation = splitOrientation === 'horizontal' ? 'vertical' : 'horizontal';
                          setSplitOrientation(newOrientation);
                          // Recalculate split size for the new orientation
                          setTimeout(() => setSplitSize(calculateDefaultSplitSize(newOrientation)), 0);
                        }}
                        title={`Switch to ${splitOrientation === 'horizontal' ? 'vertical' : 'horizontal'} split`}
                      >
                        {splitOrientation === 'horizontal' ? <span style={{ transform: 'rotate(90deg)', display: 'inline-block' }}>⊟</span> : '⊟'}
                      </button>
                    )}
                    <button 
                      className={styles.outputClose}
                      onClick={() => setOutputOpen(false)}
                    >
                      ×
                    </button>
                  </div>
                </div>
                <div className={styles.outputContent}>
                  {(() => {
                    const currentTab = getCurrentTerminalTab();
                    return (
                      <>
                        {currentTab.content.map((line, idx) => {
                          const isPromptLine = line.includes('alphapulse@server');
                          const isNautilusLine = line.includes('[INFO] BACKTESTER');
                          return (
                            <div 
                              key={idx} 
                              className={`${styles.outputLine} ${isNautilusLine ? 'nautilus-branding' : ''}`}
                              style={{ 
                                color: isPromptLine ? '#00d4db' : undefined
                              }}
                            >
                              {line}
                            </div>
                          );
                        })}
                        <div className={styles.terminalInputLine}>
                          <span style={{ color: '#00d4db' }}>alphapulse@server:{currentTab.cwd}$ </span>
                          <input
                            type="text"
                            className={styles.terminalInput}
                            value={currentTab.currentInput}
                            onChange={(e) => updateTerminalInput(e.target.value)}
                            onKeyDown={(e) => {
                              if (e.key === 'Enter') {
                                e.preventDefault();
                                const command = currentTab.currentInput.trim();
                                
                                // Add command to output
                                addOutput(`alphapulse@server:${currentTab.cwd}$ ${command}`);
                                
                                // Process command
                                if (command === 'clear') {
                                  setTerminalTabs(prev => prev.map(tab => 
                                    tab.id === activeTerminalTab 
                                      ? { ...tab, content: [] }
                                      : tab
                                  ));
                                } else if (command === 'help') {
                                  addOutput('Available commands: clear, help, run, save, ls, cd, python, exit');
                                } else if (command === 'ls') {
                                  addOutput('README.md  snippets/  examples/  config/  docs/  builder-ui/  strategy.py');
                                } else if (command.startsWith('cd ')) {
                                  const dir = command.substring(3).trim();
                                  let newCwd = currentTab.cwd;
                                  
                                  if (dir === '..' || dir === '../') {
                                    const pathParts = currentTab.cwd.split('/');
                                    pathParts.pop();
                                    newCwd = pathParts.length > 1 ? pathParts.join('/') : '~';
                                  } else if (dir === '~' || dir === '') {
                                    newCwd = '~/strategies';
                                  } else if (dir.startsWith('/')) {
                                    newCwd = dir;
                                  } else {
                                    newCwd = `${currentTab.cwd}/${dir}`;
                                  }
                                  
                                  // Update both cwd and tab name
                                  setTerminalTabs(prev => prev.map(tab => 
                                    tab.id === activeTerminalTab 
                                      ? { ...tab, cwd: newCwd, name: newCwd }
                                      : tab
                                  ));
                                } else if (command.startsWith('python ')) {
                                  const timestamp = new Date().toISOString();
                                  addOutput(`${timestamp} [INFO] Running ${command}...`);
                                  setTimeout(() => {
                                    addOutput(`${timestamp} [INFO] Strategy execution complete`);
                                  }, 1500);
                                } else if (command === 'claude') {
                                  // Display Claude ASCII art and welcome message
                                  addOutput('');
                                  addOutput('     ▄████▄   ██▓    ▄▄▄       █    ██ ▓█████▄ ▓█████ ');
                                  addOutput('    ▒██▀ ▀█  ▓██▒   ▒████▄     ██  ▓██▒▒██▀ ██▌▓█   ▀ ');
                                  addOutput('    ▒▓█    ▄ ▒██░   ▒██  ▀█▄  ▓██  ▒██░░██   █▌▒███   ');
                                  addOutput('    ▒▓▓▄ ▄██▒▒██░   ░██▄▄▄▄██ ▓▓█  ░██░░▓█▄   ▌▒▓█  ▄ ');
                                  addOutput('    ▒ ▓███▀ ░░██████▒▓█   ▓██▒▒▒█████▓ ░▒████▓ ░▒████▒');
                                  addOutput('    ░ ░▒ ▒  ░░ ▒░▓  ░▒▒   ▓▒█░░▒▓▒ ▒ ▒  ▒▒▓  ▒ ░░ ▒░ ░');
                                  addOutput('      ░  ▒   ░ ░ ▒  ░ ▒   ▒▒ ░░░▒░ ░ ░  ░ ▒  ▒  ░ ░  ░');
                                  addOutput('    ░          ░ ░    ░   ▒    ░░░ ░ ░  ░ ░  ░    ░   ');
                                  addOutput('    ░ ░          ░  ░     ░  ░   ░        ░       ░  ░');
                                  addOutput('');
                                  addOutput('    Claude Code v0.1.0 - Your AI Coding Assistant');
                                  addOutput('    ═══════════════════════════════════════════════');
                                  addOutput('');
                                  addOutput('    Welcome to Claude Code! I\'m here to help with:');
                                  addOutput('      • Code generation and optimization');
                                  addOutput('      • Debugging and error resolution');
                                  addOutput('      • Strategy development and backtesting');
                                  addOutput('      • Data analysis and visualization');
                                  addOutput('');
                                  addOutput('    Usage: claude <prompt>');
                                  addOutput('    Example: claude "help me optimize this momentum strategy"');
                                  addOutput('');
                                  addOutput('    Type \'claude help\' for more commands');
                                  addOutput('');
                                } else if (command.startsWith('claude ')) {
                                  const prompt = command.substring(7).trim();
                                  if (prompt === 'help') {
                                    addOutput('Claude Code Commands:');
                                    addOutput('  claude <prompt>    - Ask Claude for help');
                                    addOutput('  claude analyze     - Analyze current file');
                                    addOutput('  claude optimize    - Optimize selected code');
                                    addOutput('  claude debug       - Debug recent errors');
                                    addOutput('  claude test        - Generate test cases');
                                    addOutput('  claude docs        - Generate documentation');
                                  } else if (prompt) {
                                    addOutput(`[Claude] Processing: "${prompt}"...`);
                                    setTimeout(() => {
                                      addOutput('[Claude] I understand you want help with that. Here\'s what I suggest:');
                                      addOutput('  1. First, let\'s analyze your current approach');
                                      addOutput('  2. Consider implementing these optimizations');
                                      addOutput('  3. Run backtests to validate the changes');
                                      addOutput('');
                                      addOutput('[Claude] Ready to assist with implementation. Type specific questions for detailed help.');
                                    }, 1000);
                                  } else {
                                    addOutput('[Claude] Please provide a prompt. Usage: claude <your question>');
                                  }
                                } else if (command === 'exit') {
                                  if (terminalTabs.length > 1) {
                                    closeTerminalTab(activeTerminalTab);
                                  } else {
                                    addOutput('Cannot close the last terminal. Use the × button to close the terminal panel.');
                                  }
                                } else if (command) {
                                  addOutput(`bash: ${command}: command not found`);
                                }
                                
                                // Clear input
                                updateTerminalInput('');
                              }
                            }}
                            autoFocus
                            spellCheck={false}
                          />
                        </div>
                      </>
                    );
                  })()}
                </div>
              </div>
            </>
          )}
      </div>
    </div>
  );
};