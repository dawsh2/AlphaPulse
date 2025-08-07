import React, { useState, useEffect, useRef } from 'react';
import styles from './DevelopPage.module.css';

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
  const [outputOpen, setOutputOpen] = useState(false);
  const [outputContent, setOutputContent] = useState<string[]>([
    'AlphaPulse Development Environment v1.0.0',
    'Ready.'
  ]);
  const [searchQuery, setSearchQuery] = useState('');
  const [expandedFolders, setExpandedFolders] = useState<Set<string>>(new Set(['examples/']));
  const editorRef = useRef<HTMLTextAreaElement>(null);
  const [isDesktop, setIsDesktop] = useState(window.innerWidth > 768);

  useEffect(() => {
    loadFiles();
    
    const handleResize = () => {
      setIsDesktop(window.innerWidth > 768);
    };
    
    window.addEventListener('resize', handleResize);
    return () => window.removeEventListener('resize', handleResize);
  }, []);

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
      addOutput(`Running ${activeTabData.name}...`);
      
      // TODO: Implement actual code execution
      setTimeout(() => {
        addOutput('Python 3.11.5');
        addOutput('Executing strategy...');
        addOutput('[INFO] Strategy initialized');
        addOutput('[INFO] Connected to data feed');
        addOutput('[INFO] Strategy running...');
      }, 500);
    }
  };

  const addOutput = (line: string) => {
    setOutputContent(prev => [...prev, line]);
  };

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
      <div className={styles.mainArea}>
        <aside className={`${styles.sidebar} ${!sidebarOpen ? styles.sidebarClosed : ''}`}>
          <div className={styles.sidebarHeader}>
            <input
              type="text"
              className={styles.explorerSearch}
              placeholder="EXPLORER"
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
        </aside>

        <div className={styles.editorContainer}>
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
              <button className={styles.actionButton} onClick={() => setOutputOpen(!outputOpen)} title="Toggle Terminal">
                <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                  <polyline points="4 17 10 11 4 5"></polyline>
                  <line x1="12" y1="19" x2="20" y2="19"></line>
                </svg>
              </button>
              <button className={styles.actionButton} onClick={runCode} title="Run Code">
                <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                  <polygon points="5 3 19 12 5 21 5 3"></polygon>
                </svg>
              </button>
            </div>
          </div>

          <div className={styles.editorWrapper}>
            {activeTabData ? (
              <div className={styles.editor}>
                <textarea
                  ref={editorRef}
                  className={styles.editorTextarea}
                  value={activeTabData.content}
                  onChange={(e) => {
                    const newTabs = tabs.map(tab => 
                      tab.id === activeTab 
                        ? { ...tab, content: e.target.value }
                        : tab
                    );
                    setTabs(newTabs);
                  }}
                  spellCheck={false}
                  autoComplete="off"
                  autoCorrect="off"
                  autoCapitalize="off"
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

          {outputOpen && (
            <div className={styles.outputPanel}>
              <div className={styles.outputHeader}>
                <div className={styles.outputTitle}>TERMINAL</div>
                <button 
                  className={styles.outputClose}
                  onClick={() => setOutputOpen(false)}
                >
                  ×
                </button>
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
          )}
        </div>
      </div>
    </div>
  );
};