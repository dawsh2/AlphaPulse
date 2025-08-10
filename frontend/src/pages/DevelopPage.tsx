import React, { useState, useEffect, useRef } from 'react';
import styles from './DevelopPage.module.css';
import CodeEditor from '../components/CodeEditor/CodeEditor';
import { generateFileContent } from '../services/fileContentGenerator';
import { loadFileStructure } from '../services/fileSystemService';
import { Terminal } from '../components/features/Develop/Terminal';

interface TerminalTab {
  id: string;
  name: string;
  content: string[];
  currentInput: string;
  cwd: string;
}

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
  
  const initializeConsole = () => {
    // This is now handled by the Terminal component
  };

  const addOutput = (text: string, tabId?: string) => {
    const targetTabId = tabId || activeTerminalTab;
    setTerminalTabs(prev => prev.map(tab => 
      tab.id === targetTabId 
        ? { ...tab, content: [...tab.content, text] }
        : tab
    ));
  };

  const loadFiles = async () => {
    const fileStructure = await loadFileStructure();
    setFiles(fileStructure);
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
    await generateFileContent(filePath, fileName, {
      tabs,
      setTabs,
      setActiveTab,
      setEditorHidden
    });
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
      }, 500);
      
      setTimeout(() => {
        const timestamp = new Date().toISOString();
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

        <Terminal
          terminalTabs={terminalTabs}
          activeTerminalTab={activeTerminalTab}
          outputOpen={outputOpen}
          editorHidden={editorHidden}
          splitOrientation={splitOrientation}
          splitSize={splitSize}
          terminalTabCounter={terminalTabCounter}
          setTerminalTabs={setTerminalTabs}
          setActiveTerminalTab={setActiveTerminalTab}
          setTerminalTabCounter={setTerminalTabCounter}
          setSplitOrientation={setSplitOrientation}
          setSplitSize={setSplitSize}
          setOutputOpen={setOutputOpen}
          onSplitDragStart={handleSplitDragStart}
          onInitializeConsole={initializeConsole}
          styles={styles}
        />
      </div>
    </div>
  );
};