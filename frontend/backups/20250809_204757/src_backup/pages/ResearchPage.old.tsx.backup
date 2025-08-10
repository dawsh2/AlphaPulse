import React, { useState, useEffect } from 'react';
import { useLocation } from 'react-router-dom';
import styles from './ResearchPage.module.css';
import exploreStyles from './ExplorePage.module.css';
import { StrategyWorkbench } from '../components/StrategyBuilder/StrategyWorkbench';

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


type SidebarTab = 'builder' | 'notebooks';
type MainView = 'explore' | 'notebook' | 'builder';

const ResearchPage: React.FC = () => {
  const location = useLocation();
  
  // State management
  const [activeTab, setActiveTab] = useState<SidebarTab | null>(null);
  const [mainView, setMainView] = useState<MainView>('explore');
  const [isBuilderOpen, setIsBuilderOpen] = useState(false);
  const [selectedTemplate, setSelectedTemplate] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState('');
  const [collapsedCategories, setCollapsedCategories] = useState<Set<string>>(new Set());
  const [notebookCells, setNotebookCells] = useState<NotebookCell[]>([]);
  const [activeCell, setActiveCell] = useState<string | null>(null);

  // Mock data
  const codeSnippets: Record<string, CodeSnippet[]> = {
    'Data Loading': [
      {
        id: 'load_signals',
        name: 'Load Signals',
        code: `import admf

# Load signals with filtering
signals = admf.load_signals(
    strategy_type='bollinger_bands',
    min_sharpe=1.0,
    symbols=['AAPL', 'MSFT']
)
print(f"Loaded {len(signals)} signal traces")`,
        description: 'Load strategy signals from ADMF registry'
      },
      {
        id: 'load_executions',
        name: 'Load Executions',
        code: `# Load execution data
executions = admf.load_executions(
    signal_hash='sig_a7f8d9e6',
    include_trades=True
)
print(f"Found {len(executions)} execution records")`,
        description: 'Load execution data for analysis'
      }
    ],
    'Performance Metrics': [
      {
        id: 'performance_table',
        name: 'Performance Table',
        code: `from analysis_lib import performance_table

# Generate comprehensive performance metrics
metrics = performance_table(signals)
metrics.sort_values('sharpe_ratio', ascending=False).head(10)`,
        description: 'Calculate key performance metrics'
      },
      {
        id: 'sharpe_calculation',
        name: 'Sharpe Ratio',
        code: `# Calculate Sharpe ratio
def calculate_sharpe_ratio(returns, risk_free_rate=0.02):
    excess_returns = returns - risk_free_rate / 252
    return excess_returns.mean() / excess_returns.std() * np.sqrt(252)

sharpe = calculate_sharpe_ratio(strategy_returns)
print(f"Sharpe Ratio: {sharpe:.2f}")`,
        description: 'Calculate annualized Sharpe ratio'
      }
    ],
    'Visualizations': [
      {
        id: 'equity_curves',
        name: 'Equity Curves',
        code: `import matplotlib.pyplot as plt
from analysis_lib import plot_equity_curves

# Plot multiple strategy equity curves
fig = plot_equity_curves(
    signals,
    benchmark='SPY',
    title='Strategy Performance Comparison'
)
fig.show()`,
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
          content: `import admf
from analysis_lib import *

# Load strategies to compare
strategies = admf.load_signals(['momentum', 'mean_reversion'], min_sharpe=1.0)
print(f"Loaded {len(strategies)} strategies for comparison")`
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
        content: `import admf
import pandas as pd
import numpy as np
from analysis_lib import *

# Load sample data
signals = admf.load_signals(strategy_type='ema_cross', limit=5)
print(f"Loaded {len(signals)} signal traces for analysis")`
      }
    ]);
  }, []);

  // Check if opened from Explore page with strategy data or builder request
  useEffect(() => {
    if (location.state?.strategy) {
      // If opened with a strategy, load it into the notebook
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
      // If opened to use the builder, open it directly
      setActiveTab('builder');
      setMainView('builder');
    }
  }, [location.state]);

  // Event handlers
  const handleTabSwitch = (tab: SidebarTab) => {
    if (activeTab === tab) {
      // Clicking active tab returns to explore
      setActiveTab(null);
      setMainView('explore');
    } else {
      setActiveTab(tab);
      if (tab === 'builder') {
        setMainView('builder');
      } else {
        setMainView('notebook');
      }
    }
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
                  <div 
                    className={styles.strategyItem}
                    onClick={() => {
                      setSelectedTemplate('volume_breakout');
                      setMainView('builder');
                    }}
                  >
                    <div className={styles.strategyName}>Volume Breakout</div>
                    <div className={styles.strategyDesc}>High volume momentum</div>
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
                  <div 
                    className={styles.templateItem}
                    onClick={() => {
                      setSelectedTemplate('trend_following');
                      setMainView('builder');
                    }}
                  >
                    <div className={styles.templateName}>Trend Following</div>
                    <div className={styles.templateDesc}>EMA crossover strategies</div>
                  </div>
                  <div 
                    className={styles.templateItem}
                    onClick={() => {
                      setSelectedTemplate('mean_reversion');
                      setMainView('builder');
                    }}
                  >
                    <div className={styles.templateName}>Mean Reversion</div>
                    <div className={styles.templateDesc}>Bollinger band strategies</div>
                  </div>
                  <div 
                    className={styles.templateItem}
                    onClick={() => {
                      setSelectedTemplate('breakout');
                      setMainView('builder');
                    }}
                  >
                    <div className={styles.templateName}>Breakout</div>
                    <div className={styles.templateDesc}>Price channel breakouts</div>
                  </div>
                  <div 
                    className={styles.templateItem}
                    onClick={() => {
                      setSelectedTemplate('optimizer');
                      setMainView('builder');
                    }}
                  >
                    <div className={styles.templateName}>Parameter Optimizer</div>
                    <div className={styles.templateDesc}>Sweep parameter ranges</div>
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
      // Import ExplorePage content here
      return null; // Will be replaced with ExplorePage content
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
                  setActiveTab('notebooks');
                  setMainView('notebook');
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
        <div className={styles.notebookHeader}>
          <div className={styles.notebookTitle}>
            <h2>Research Notebook</h2>
          </div>
          <div className={styles.notebookControls}>
            <button 
              className={styles.addCellBtn}
              onClick={() => addCell('code')}
            >
              + Code
            </button>
            <button 
              className={styles.addCellBtn}
              onClick={() => addCell('markdown')}
            >
              + Markdown
            </button>
          </div>
        </div>
        
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
                  <button onClick={() => executeCell(cell.id)} disabled={cell.isExecuting}>
                    {cell.isExecuting ? '⏳' : '▶️'}
                  </button>
                  <button onClick={() => deleteCell(cell.id)}>🗑️</button>
                </div>
              </div>
              
              <div className={styles.cellContent}>
                <textarea
                  className={styles.cellTextarea}
                  value={cell.content}
                  onChange={(e) => updateCellContent(cell.id, e.target.value)}
                  onFocus={() => setActiveCell(cell.id)}
                />
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
      {/* Sidebar */}
      <aside className={styles.snippetsSidebar}>
        <div className={styles.sidebarHeader}>
          <div className={styles.sidebarTabs}>
            <button 
              className={`${styles.sidebarTab} ${activeTab === 'builder' ? styles.active : ''}`}
              onClick={() => handleTabSwitch('builder')}
            >
              Builder
            </button>
            <button 
              className={`${styles.sidebarTab} ${activeTab === 'notebooks' ? styles.active : ''}`}
              onClick={() => handleTabSwitch('notebooks')}
            >
              Notebooks
            </button>
          </div>
          <input 
            type="text"
            className={styles.snippetSearch}
            placeholder="Search..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
          />
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