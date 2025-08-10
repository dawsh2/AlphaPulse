/**
 * Main Development Page Container
 */

import React, { useState, useCallback, useEffect } from 'react';
import { FileExplorer, type FileItem } from './FileExplorer';
import { CodeEditor } from './CodeEditor';
import { TerminalEmulator } from './TerminalEmulator';
import { GitPanel } from './GitPanel';
import { generateId } from '../../../utils/hash';
import styles from './Develop.module.css';

interface Tab {
  id: string;
  title: string;
  content: string;
  language: string;
  path: string;
  isDirty: boolean;
}

export const DevelopContainer: React.FC = () => {
  // File system state
  const [files, setFiles] = useState<FileItem[]>([
    {
      path: 'strategies/',
      name: 'strategies',
      type: 'folder',
      children: [
        {
          path: 'strategies/mean_reversion.py',
          name: 'mean_reversion.py',
          type: 'file',
          content: `"""Mean Reversion Strategy"""
import pandas as pd
import numpy as np
from alphapulse import Strategy, Signal

class MeanReversionStrategy(Strategy):
    def __init__(self, window=20, z_threshold=2):
        super().__init__()
        self.window = window
        self.z_threshold = z_threshold
    
    def generate_signals(self, data):
        # Calculate rolling mean and std
        rolling_mean = data['close'].rolling(self.window).mean()
        rolling_std = data['close'].rolling(self.window).std()
        
        # Calculate z-score
        z_score = (data['close'] - rolling_mean) / rolling_std
        
        # Generate signals
        signals = pd.Series(index=data.index, data=Signal.HOLD)
        signals[z_score < -self.z_threshold] = Signal.BUY
        signals[z_score > self.z_threshold] = Signal.SELL
        
        return signals`,
          language: 'python',
        },
        {
          path: 'strategies/momentum.py',
          name: 'momentum.py',
          type: 'file',
          content: `"""Momentum Strategy"""
from alphapulse import Strategy

class MomentumStrategy(Strategy):
    pass`,
          language: 'python',
        },
      ],
    },
    {
      path: 'indicators/',
      name: 'indicators',
      type: 'folder',
      children: [
        {
          path: 'indicators/rsi.py',
          name: 'rsi.py',
          type: 'file',
          content: `"""RSI Indicator"""
def calculate_rsi(data, period=14):
    pass`,
          language: 'python',
        },
      ],
    },
    {
      path: 'config.yaml',
      name: 'config.yaml',
      type: 'file',
      content: `# AlphaPulse Configuration
api:
  endpoint: "https://api.alphapulse.io"
  key: "your-api-key"

backtest:
  start_date: "2023-01-01"
  end_date: "2023-12-31"
  initial_capital: 100000`,
      language: 'yaml',
    },
  ]);

  // Editor state
  const [tabs, setTabs] = useState<Tab[]>([]);
  const [activeTabId, setActiveTabId] = useState<string | null>(null);
  const [selectedFile, setSelectedFile] = useState<string | null>(null);

  // Layout state
  const [leftPanelWidth, setLeftPanelWidth] = useState(250);
  const [rightPanelWidth, setRightPanelWidth] = useState(300);
  const [bottomPanelHeight, setBottomPanelHeight] = useState(200);
  const [showTerminal, setShowTerminal] = useState(true);
  const [showGit, setShowGit] = useState(true);

  // Git state
  const [gitFiles, setGitFiles] = useState([
    { path: 'strategies/mean_reversion.py', status: 'modified' as const, staged: false },
    { path: 'config.yaml', status: 'modified' as const, staged: true },
  ]);
  const [gitCommits] = useState([
    {
      hash: 'a1b2c3d4e5f6g7h8',
      author: 'AlphaPulse Dev',
      date: new Date(Date.now() - 3600000),
      message: 'Add mean reversion strategy',
    },
    {
      hash: 'b2c3d4e5f6g7h8i9',
      author: 'AlphaPulse Dev',
      date: new Date(Date.now() - 7200000),
      message: 'Initial commit',
    },
  ]);

  const activeTab = tabs.find(t => t.id === activeTabId);

  // File operations
  const handleFileSelect = useCallback((file: FileItem) => {
    if (file.type !== 'file') return;

    setSelectedFile(file.path);

    // Check if already open
    const existingTab = tabs.find(t => t.path === file.path);
    if (existingTab) {
      setActiveTabId(existingTab.id);
      return;
    }

    // Create new tab
    const newTab: Tab = {
      id: generateId('tab'),
      title: file.name,
      content: file.content || '',
      language: file.language || 'text',
      path: file.path,
      isDirty: false,
    };

    setTabs(prev => [...prev, newTab]);
    setActiveTabId(newTab.id);
  }, [tabs]);

  const handleFileCreate = useCallback((parentPath: string, type: 'file' | 'folder') => {
    const name = prompt(`Enter ${type} name:`);
    if (!name) return;

    const newPath = parentPath === '/' ? name : `${parentPath}/${name}`;
    const newItem: FileItem = {
      path: newPath,
      name,
      type,
      content: type === 'file' ? '' : undefined,
      children: type === 'folder' ? [] : undefined,
    };

    // Update file tree
    setFiles(prev => {
      const addToTree = (items: FileItem[]): FileItem[] => {
        if (parentPath === '/') {
          return [...items, newItem];
        }
        
        return items.map(item => {
          if (item.path === parentPath && item.type === 'folder') {
            return {
              ...item,
              children: [...(item.children || []), newItem],
            };
          }
          if (item.children) {
            return {
              ...item,
              children: addToTree(item.children),
            };
          }
          return item;
        });
      };
      
      return addToTree(prev);
    });
  }, []);

  const handleFileDelete = useCallback((path: string) => {
    if (!confirm(`Delete ${path}?`)) return;

    // Remove from files
    setFiles(prev => {
      const removeFromTree = (items: FileItem[]): FileItem[] => {
        return items
          .filter(item => item.path !== path)
          .map(item => ({
            ...item,
            children: item.children ? removeFromTree(item.children) : undefined,
          }));
      };
      return removeFromTree(prev);
    });

    // Close tab if open
    setTabs(prev => prev.filter(t => t.path !== path));
  }, []);

  const handleFileRename = useCallback((oldPath: string, newPath: string) => {
    // Update file tree
    setFiles(prev => {
      const updateTree = (items: FileItem[]): FileItem[] => {
        return items.map(item => {
          if (item.path === oldPath) {
            return {
              ...item,
              path: newPath,
              name: newPath.split('/').pop() || item.name,
            };
          }
          if (item.children) {
            return {
              ...item,
              children: updateTree(item.children),
            };
          }
          return item;
        });
      };
      return updateTree(prev);
    });

    // Update tab if open
    setTabs(prev => prev.map(tab =>
      tab.path === oldPath
        ? { ...tab, path: newPath, title: newPath.split('/').pop() || tab.title }
        : tab
    ));
  }, []);

  // Tab operations
  const handleTabClose = useCallback((tabId: string) => {
    const tab = tabs.find(t => t.id === tabId);
    if (tab?.isDirty) {
      if (!confirm(`Discard changes to ${tab.title}?`)) return;
    }

    setTabs(prev => prev.filter(t => t.id !== tabId));
    if (activeTabId === tabId) {
      const remainingTabs = tabs.filter(t => t.id !== tabId);
      setActiveTabId(remainingTabs[remainingTabs.length - 1]?.id || null);
    }
  }, [tabs, activeTabId]);

  const handleCodeChange = useCallback((value: string) => {
    if (!activeTabId) return;

    setTabs(prev => prev.map(tab =>
      tab.id === activeTabId
        ? { ...tab, content: value, isDirty: true }
        : tab
    ));
  }, [activeTabId]);

  const handleCodeSave = useCallback((value: string) => {
    if (!activeTab) return;

    // Update file content
    setFiles(prev => {
      const updateContent = (items: FileItem[]): FileItem[] => {
        return items.map(item => {
          if (item.path === activeTab.path) {
            return { ...item, content: value };
          }
          if (item.children) {
            return {
              ...item,
              children: updateContent(item.children),
            };
          }
          return item;
        });
      };
      return updateContent(prev);
    });

    // Mark tab as saved
    setTabs(prev => prev.map(tab =>
      tab.id === activeTabId
        ? { ...tab, isDirty: false }
        : tab
    ));

    console.log(`Saved ${activeTab.path}`);
  }, [activeTab, activeTabId]);

  // Terminal command execution
  const handleCommandExecute = async (command: string): Promise<string> => {
    // Simulate command execution
    if (command.startsWith('run-backtest')) {
      return 'Running backtest...\nBacktest complete: Sharpe Ratio: 1.5, Max Drawdown: -15%';
    }
    if (command.startsWith('analyze')) {
      return 'Analyzing symbol...\nAnalysis complete: Bullish signal detected';
    }
    return `Executing: ${command}`;
  };

  // Git operations
  const handleGitStage = useCallback((path: string) => {
    setGitFiles(prev => prev.map(file =>
      file.path === path ? { ...file, staged: true } : file
    ));
  }, []);

  const handleGitUnstage = useCallback((path: string) => {
    setGitFiles(prev => prev.map(file =>
      file.path === path ? { ...file, staged: false } : file
    ));
  }, []);

  const handleGitCommit = useCallback((message: string) => {
    console.log('Committing:', message);
    // Reset staged files
    setGitFiles(prev => prev.filter(f => !f.staged));
  }, []);

  // Keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Save: Cmd/Ctrl + S
      if ((e.metaKey || e.ctrlKey) && e.key === 's') {
        e.preventDefault();
        if (activeTab) {
          handleCodeSave(activeTab.content);
        }
      }
      // Close tab: Cmd/Ctrl + W
      if ((e.metaKey || e.ctrlKey) && e.key === 'w') {
        e.preventDefault();
        if (activeTabId) {
          handleTabClose(activeTabId);
        }
      }
      // Toggle terminal: Cmd/Ctrl + `
      if ((e.metaKey || e.ctrlKey) && e.key === '`') {
        e.preventDefault();
        setShowTerminal(prev => !prev);
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [activeTab, activeTabId, handleCodeSave, handleTabClose]);

  return (
    <div className={styles.developContainer}>
      {/* Left Panel - File Explorer */}
      <div className={styles.leftPanel} style={{ width: leftPanelWidth }}>
        <FileExplorer
          files={files}
          selectedFile={selectedFile}
          onFileSelect={handleFileSelect}
          onFileCreate={handleFileCreate}
          onFileDelete={handleFileDelete}
          onFileRename={handleFileRename}
        />
      </div>

      {/* Center Panel - Editor */}
      <div className={styles.centerPanel}>
        <div className={styles.editorArea}>
          {tabs.length > 0 && (
            <div className={styles.tabBar}>
              {tabs.map(tab => (
                <div
                  key={tab.id}
                  className={`${styles.tab} ${
                    tab.id === activeTabId ? styles.active : ''
                  }`}
                  onClick={() => setActiveTabId(tab.id)}
                >
                  <span className={styles.tabTitle}>
                    {tab.isDirty && '• '}
                    {tab.title}
                  </span>
                  <button
                    className={styles.tabClose}
                    onClick={(e) => {
                      e.stopPropagation();
                      handleTabClose(tab.id);
                    }}
                  >
                    ×
                  </button>
                </div>
              ))}
            </div>
          )}
          
          {activeTab ? (
            <CodeEditor
              value={activeTab.content}
              language={activeTab.language}
              onChange={handleCodeChange}
              onSave={handleCodeSave}
              height="calc(100% - 40px)"
            />
          ) : (
            <div className={styles.welcomeScreen}>
              <h2>AlphaPulse Development Environment</h2>
              <p>Select a file from the explorer to start editing</p>
              <div className={styles.shortcuts}>
                <h3>Keyboard Shortcuts</h3>
                <ul>
                  <li><kbd>Cmd+S</kbd> Save file</li>
                  <li><kbd>Cmd+W</kbd> Close tab</li>
                  <li><kbd>Cmd+`</kbd> Toggle terminal</li>
                </ul>
              </div>
            </div>
          )}
        </div>

        {/* Bottom Panel - Terminal */}
        {showTerminal && (
          <div className={styles.bottomPanel} style={{ height: bottomPanelHeight }}>
            <TerminalEmulator
              onCommandExecute={handleCommandExecute}
              workingDirectory="~/alphapulse"
            />
          </div>
        )}
      </div>

      {/* Right Panel - Git */}
      {showGit && (
        <div className={styles.rightPanel} style={{ width: rightPanelWidth }}>
          <GitPanel
            files={gitFiles}
            commits={gitCommits}
            currentBranch="main"
            onStageFile={handleGitStage}
            onUnstageFile={handleGitUnstage}
            onCommit={handleGitCommit}
          />
        </div>
      )}
    </div>
  );
};