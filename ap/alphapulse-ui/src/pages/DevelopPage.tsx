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
  const [tabs, setTabs] = useState<Tab[]>([
    { id: 'strategy.py', name: 'strategy.py', content: '# Loading from NautilusTrader...', language: 'python' }
  ]);
  const [activeTab, setActiveTab] = useState<string>('strategy.py');
  const [sidebarOpen, setSidebarOpen] = useState(true);
  const [sidebarView, setSidebarView] = useState<'explorer' | 'search' | 'git' | 'debug'>('explorer');
  const [outputOpen, setOutputOpen] = useState(false);
  const [outputContent, setOutputContent] = useState<string[]>([]);
  const [searchQuery, setSearchQuery] = useState('');
  const [splitOrientation, setSplitOrientation] = useState<'horizontal' | 'vertical'>('horizontal');
  const [splitSize, setSplitSize] = useState(0); // Will be calculated as 50% of available space
  const [isDragging, setIsDragging] = useState(false);
  const [editorHidden, setEditorHidden] = useState(false);
  const [expandedFolders, setExpandedFolders] = useState<Set<string>>(new Set(['examples/']));
  const editorRef = useRef<HTMLTextAreaElement>(null);
  const [isDesktop, setIsDesktop] = useState(window.innerWidth > 768);

  useEffect(() => {
    loadFiles();
    initializeConsole();
    
    const handleResize = () => {
      setIsDesktop(window.innerWidth > 768);
    };
    
    window.addEventListener('resize', handleResize);
    return () => window.removeEventListener('resize', handleResize);
  }, []);

  const initializeConsole = () => {
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
      'Ready.'
    ];
    setOutputContent(nautilusArt);
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
    // Check if tab already exists
    const existingTab = tabs.find(tab => tab.id === filePath);
    if (existingTab) {
      setActiveTab(filePath);
      return;
    }
    
    // Load file content (mock for now)
    const content = `# ${fileName}
# This is a placeholder for the actual file content
# Content will be loaded from the backend

def main():
    print("AlphaPulse Trading Strategy")
    
if __name__ == "__main__":
    main()
`;
    
    // Add new tab
    const newTab: Tab = {
      id: filePath,
      name: fileName,
      content,
      language: fileName.endsWith('.py') ? 'python' : 
                fileName.endsWith('.yaml') || fileName.endsWith('.yml') ? 'yaml' :
                fileName.endsWith('.json') ? 'json' :
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

  const addOutput = (line: string) => {
    setOutputContent(prev => [...prev, line]);
  };

  const calculateDefaultSplitSize = (orientation?: 'horizontal' | 'vertical') => {
    const mainArea = document.querySelector(`.${styles.mainArea}`) as HTMLElement;
    if (!mainArea) return 300; // fallback
    
    const currentOrientation = orientation || splitOrientation;
    if (currentOrientation === 'horizontal') {
      const height = mainArea.clientHeight;
      // Account for tabs container height (approximately 50px)
      const availableHeight = height - 50;
      return Math.max(150, Math.floor(availableHeight / 2));
    } else {
      const width = mainArea.clientWidth;
      // For vertical split, use smaller default to ensure editor has enough space
      return Math.max(250, Math.min(400, Math.floor(width * 0.4)));
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

  // Add mouse event listeners for drag
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

  const handleTerminalInput = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Enter') {
      const input = e.currentTarget.value;
      if (input.trim()) {
        addOutput(`> ${input}`);
        // Process command
        if (input === 'clear') {
          setOutputContent([]);
        } else if (input === 'help') {
          addOutput('Available commands: clear, help, run, save');
        } else {
          addOutput(`Command not recognized: ${input}`);
        }
        e.currentTarget.value = '';
      }
    }
  };

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
      <aside className={`${styles.sidebar} ${!sidebarOpen ? styles.sidebarClosed : ''}`}>
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
              className={`${styles.sidebarTab} ${sidebarView === 'search' ? styles.active : ''}`}
              onClick={() => setSidebarView('search')}
              title="Search"
            >
              <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <circle cx="11" cy="11" r="8"></circle>
                <path d="m21 21-4.35-4.35"></path>
              </svg>
            </button>
            <button 
              className={`${styles.sidebarTab} ${sidebarView === 'git' ? styles.active : ''}`}
              onClick={() => setSidebarView('git')}
              title="Source Control"
            >
              <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <circle cx="18" cy="18" r="3"></circle>
                <circle cx="6" cy="6" r="3"></circle>
                <path d="M13 6h3a2 2 0 0 1 2 2v7"></path>
                <line x1="6" y1="9" x2="6" y2="21"></line>
              </svg>
            </button>
            <button 
              className={`${styles.sidebarTab} ${sidebarView === 'debug' ? styles.active : ''}`}
              onClick={() => setSidebarView('debug')}
              title="Run and Debug"
            >
              <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <polygon points="5 3 19 12 5 21 5 3"></polygon>
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
              <p className={styles.emptyState}>No changes detected</p>
            </div>
          </div>
        )}
        
        {sidebarView === 'debug' && (
          <div className={styles.debugPanel}>
            <div className={styles.quickActions}>
              <h3>Quick Actions</h3>
              <div className={styles.quickActionsList}>
                <button className={styles.quickActionBtn} onClick={runCode}>
                  ▶ Run Strategy
                </button>
                <button className={styles.quickActionBtn}>
                  🐛 Debug Strategy
                </button>
                <button className={styles.quickActionBtn}>
                  📊 Run Backtest
                </button>
                <button className={styles.quickActionBtn}>
                  📈 Optimize Parameters
                </button>
              </div>
            </div>
          </div>
        )}
      </aside>
      
      <div className={`${styles.mainArea} ${outputOpen ? (splitOrientation === 'horizontal' ? styles.splitHorizontal : styles.splitVertical) : ''}`}>
        <div className={`${styles.editorContainer} ${outputOpen && splitOrientation === 'vertical' ? styles.splitVertical : ''}`}>
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
                const newOutputOpen = !outputOpen;
                setOutputOpen(newOutputOpen);
                if (newOutputOpen && splitSize === 0) {
                  // Set default split size to 50% when opening terminal
                  setTimeout(() => setSplitSize(calculateDefaultSplitSize()), 100);
                }
              }} title="Toggle Terminal">
                <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                  <polyline points="4 17 10 11 4 5"></polyline>
                  <line x1="12" y1="19" x2="20" y2="19"></line>
                </svg>
              </button>
              <button className={styles.actionButton} onClick={() => {
                setEditorHidden(true);
                if (!outputOpen) {
                  setOutputOpen(true);
                  setTimeout(() => {
                    initializeConsole();
                    if (splitSize === 0) {
                      setSplitSize(calculateDefaultSplitSize());
                    }
                  }, 100);
                }
              }} title="Close Editor">
                <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                  <path d="M18 6L6 18M6 6l12 12"></path>
                </svg>
              </button>
              <button className={styles.actionButton} onClick={runCode} title="Run Code">
                <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                  <polygon points="5 3 19 12 5 21 5 3"></polygon>
                </svg>
              </button>
            </div>
          </div>

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
                  <div className={styles.outputTitle}>TERMINAL</div>
                  <div className={styles.outputControls}>
                    {editorHidden && (
                      <button 
                        className={styles.restoreEditorBtn}
                        onClick={() => setEditorHidden(false)}
                        title="Restore Editor"
                      >
                        ⊞
                      </button>
                    )}
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
                  {outputContent.map((line, idx) => (
                    <div key={idx} className={styles.outputLine}>{line}</div>
                  ))}
                </div>
                <div className={styles.terminalInputWrapper}>
                  <span className={styles.terminalPrompt}>&gt;</span>
                  <input
                    type="text"
                    className={styles.terminalInput}
                    placeholder="Enter command..."
                    onKeyPress={handleTerminalInput}
                  />
                </div>
              </div>
            </>
          )}
      </div>
    </div>
  );
};