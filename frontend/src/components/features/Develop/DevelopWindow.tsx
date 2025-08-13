/**
 * DevelopWindow Component
 * Unified window that can display either editor or terminal tabs
 */

import React, { useEffect, useRef, useState } from 'react';
import CodeEditor from '../../CodeEditor/CodeEditor';
import styles from '../../../pages/DevelopPage.module.css';
import { terminalService } from '../../../services/terminalService';
import { saveFileContent } from '../../../services/fileSystemService';
import { XTerminal } from './XTerminal';

export interface UnifiedTab {
  id: string;
  name: string;
  type: 'editor' | 'terminal';
  // Editor-specific properties
  content?: string;
  language?: string;
  // Terminal-specific properties
  terminalContent?: string[];
  currentInput?: string;
  cwd?: string;
  useXTerm?: boolean; // Flag to use new XTerminal vs old terminal
}

interface DevelopWindowProps {
  tabs: UnifiedTab[];
  activeTab: string;
  setTabs: (tabs: UnifiedTab[]) => void;
  setActiveTab: (tabId: string) => void;
  onNewTab: () => void;
  onCloseTab: (tabId: string, e: React.MouseEvent) => void;
  onSaveFile?: () => void;
  onSplitWindow?: (orientation: 'horizontal' | 'vertical') => void;
  isSplit?: boolean;
  onCloseWindow?: () => void;
}

export const DevelopWindow: React.FC<DevelopWindowProps> = ({
  tabs,
  activeTab,
  setTabs,
  setActiveTab,
  onNewTab,
  onCloseTab,
  onSaveFile,
  onSplitWindow,
  isSplit = false,
  onCloseWindow
}) => {
  const outputEndRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const initializedTerminals = useRef(new Set<string>());
  const [isTerminalFocused, setIsTerminalFocused] = useState(false);

  const activeTabData = tabs.find(tab => tab.id === activeTab);

  // Initialize terminal with Nautilus ASCII art
  useEffect(() => {
    if (activeTabData?.type === 'terminal' && !initializedTerminals.current.has(activeTabData.id)) {
      if (!activeTabData.terminalContent || activeTabData.terminalContent.length === 0) {
        const timestamp = new Date().toISOString();
        const initialContent = [
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
          `${timestamp} [INFO] BACKTESTER-001.BacktestEngine: ⠀⠀⠀⠀⠀⠀⣿⠀⠀⠀⠀⠙⠀⠀⠀⠈⠛⠿⠿⠛⠁⠀⠈⠁⠀⠀⠀⠀⠀⠀⠀`,
          `${timestamp} [INFO] BACKTESTER-001.BacktestEngine: ⠀⠀⠀⠀⠀⠀⣿⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀`,
          `${timestamp} [INFO] BACKTESTER-001.BacktestEngine: ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀`,
          '',
          `${timestamp} [INFO] BACKTESTER-001.BacktestEngine: =================================================================`,
          `${timestamp} [INFO] BACKTESTER-001.BacktestEngine: Component initialized.`,
          `${timestamp} [INFO] BACKTESTER-001.BacktestEngine: ================================================================= `,
          '',
          'Type "help" for available commands or "examples" to see strategy samples.',
          ''
        ];
        
        setTabs(tabs.map(tab => 
          tab.id === activeTabData.id 
            ? { ...tab, terminalContent: initialContent }
            : tab
        ));
        
        initializedTerminals.current.add(activeTabData.id);
      }
    }
  }, [activeTabData, tabs, setTabs]);

  // Auto-scroll terminal to bottom
  useEffect(() => {
    if (activeTabData?.type === 'terminal' && outputEndRef.current) {
      outputEndRef.current.scrollIntoView({ behavior: 'smooth' });
    }
  }, [activeTabData?.terminalContent]);

  const handleTerminalInput = (tabId: string, value: string) => {
    setTabs(tabs.map(tab => 
      tab.id === tabId 
        ? { ...tab, currentInput: value }
        : tab
    ));
  };


  const handleTerminalCommand = async (tabId: string) => {
    const tab = tabs.find(t => t.id === tabId);
    if (!tab || tab.type !== 'terminal') return;

    const command = tab.currentInput?.trim() || '';
    const newContent = [...(tab.terminalContent || [])];
    
    // Add command to output
    newContent.push(`${tab.cwd || '/'}$ ${command}`);
    
    // Update tab to show command was sent
    setTabs(tabs.map(t => 
      t.id === tabId 
        ? { ...t, terminalContent: newContent, currentInput: '' }
        : t
    ));
    
    // Special handling for clear command
    if (command === 'clear') {
      setTabs(tabs.map(t => 
        t.id === tabId 
          ? { ...t, terminalContent: [], currentInput: '' }
          : t
      ));
      return;
    }
    
    // Execute command via backend
    try {
      const result = await terminalService.execute(command);
      
      // Add output to terminal
      const outputLines = result.output.split('\n');
      const updatedContent = [...newContent, ...outputLines];
      
      // Update working directory if changed
      const newCwd = result.cwd || tab.cwd || '/';
      
      setTabs(tabs.map(t => 
        t.id === tabId 
          ? { 
              ...t, 
              terminalContent: updatedContent, 
              cwd: newCwd,
              name: newCwd === '/' ? '~' : newCwd
            }
          : t
      ));
    } catch (error) {
      // Add error to output
      const errorContent = [...newContent, `Error: ${error}`];
      setTabs(tabs.map(t => 
        t.id === tabId 
          ? { ...t, terminalContent: errorContent }
          : t
      ));
    }
  };

  const renderTabContent = () => {
    if (!activeTabData) {
      return (
        <div className={styles.welcome}>
          <h2>AlphaPulse Development</h2>
          <p>Create a new tab or select an existing one to get started.</p>
        </div>
      );
    }

    if (activeTabData.type === 'editor') {
      return (
        <div className={styles.editor}>
          <CodeEditor
            value={activeTabData.content || ''}
            onChange={(newContent) => {
              setTabs(tabs.map(tab => 
                tab.id === activeTab 
                  ? { ...tab, content: newContent }
                  : tab
              ));
            }}
            language={activeTabData.language}
          />
        </div>
      );
    } else {
      // Terminal tab - choose between XTerminal (new) or legacy terminal
      if (activeTabData.useXTerm) {
        return (
          <XTerminal 
            className={styles.outputContent}
            onCommand={(command) => {
              console.log('Terminal command:', command);
            }}
          />
        );
      } else {
        // Legacy terminal implementation
        return (
          <div 
            className={styles.outputContent}
            style={{
              borderBottom: '3px solid var(--color-border-primary)',
              borderRight: '3px solid var(--color-border-primary)'
            }}
          >
            <div className={styles.outputLines}>
              {(activeTabData.terminalContent || []).map((line, index) => (
                <div key={index} className={styles.outputLine}>
                  {line}
                </div>
              ))}
              <div ref={outputEndRef} />
            </div>
            <div className={styles.terminalInputLine}>
              <span className={styles.prompt}>
                {activeTabData.cwd || '~/strategies'}$ 
              </span>
              <div style={{ position: 'relative', flex: 1, height: '1.4em' }}>
                {/* Visible text display - starts with a space after prompt */}
                <span 
                  style={{
                    color: '#00d4ff',
                    fontFamily: 'var(--font-family-mono)',
                    fontSize: '13px',
                    fontWeight: '500',
                    textShadow: '0 0 5px #00d4ff, 0 0 10px #00d4ff',
                    position: 'absolute',
                    top: '0',
                    left: '0.6em', // Space after prompt
                    whiteSpace: 'pre',
                    pointerEvents: 'none',
                    zIndex: 15
                  }}
                >
                  {activeTabData.currentInput || ''}
                </span>
                {/* Hidden input for capturing keystrokes */}
                <input
                  ref={inputRef}
                  type="text"
                  className={styles.terminalInput}
                  value={activeTabData.currentInput || ''}
                  onChange={(e) => handleTerminalInput(activeTabData.id, e.target.value)}
                  onFocus={() => setIsTerminalFocused(true)}
                  onBlur={() => setIsTerminalFocused(false)}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') {
                      handleTerminalCommand(activeTabData.id);
                    }
                  }}
                  style={{ 
                    color: 'transparent', // Make input text invisible
                    paddingLeft: '0.6em' // Align input with visible text
                  }}
                  autoFocus
                  spellCheck={false}
                />
                {/* Fat cursor positioned exactly after the visible text with space */}
                <div 
                  className={`${styles.fatCursor} ${isTerminalFocused ? styles.focused : ''}`}
                  style={{ 
                    left: `${0.6 + (activeTabData.currentInput || '').length * 0.6}em`
                  }}
                />
              </div>
            </div>
          </div>
        );
      }
    }
  };

  return (
    <div className={styles.editorContainer}>
      <div className={styles.tabsContainer}>
        <div className={styles.tabs}>
          {tabs.map(tab => (
            <div
              key={tab.id}
              className={`${styles.tab} ${activeTab === tab.id ? styles.active : ''}`}
              onClick={() => setActiveTab(tab.id)}
            >
              <span className={styles.tabName}>
                {tab.name}
              </span>
              <button 
                className={styles.tabClose}
                onClick={(e) => onCloseTab(tab.id, e)}
              >
                ×
              </button>
            </div>
          ))}
          <button 
            className={styles.newTabBtn} 
            title="New Terminal"
            onClick={onNewTab}
          >
            +
          </button>
        </div>
        <div className={styles.editorActions}>
          {/* Run button for Python files */}
          {activeTabData?.type === 'editor' && activeTabData.name.endsWith('.py') && (
            <button 
              className={styles.actionButton} 
              onClick={async () => {
                // Save file first
                if (activeTabData.content) {
                  await saveFileContent(activeTabData.id, activeTabData.content);
                }
                
                // Create or switch to terminal tab
                const terminalTab = tabs.find(t => t.type === 'terminal');
                if (terminalTab) {
                  setActiveTab(terminalTab.id);
                } else {
                  // Create new terminal tab
                  const newTab: UnifiedTab = {
                    id: `terminal-${Date.now()}`,
                    name: '~',
                    type: 'terminal',
                    terminalContent: [],
                    currentInput: '',
                    cwd: '/'
                  };
                  setTabs([...tabs, newTab]);
                  setActiveTab(newTab.id);
                }
                
                // Execute the Python file
                setTimeout(async () => {
                  const result = await terminalService.execute(`python ${activeTabData.id}`);
                  const terminalTab = tabs.find(t => t.type === 'terminal');
                  if (terminalTab) {
                    const outputLines = result.output.split('\n');
                    setTabs(tabs.map(t => 
                      t.id === terminalTab.id 
                        ? { 
                            ...t, 
                            terminalContent: [
                              ...(t.terminalContent || []),
                              `$ python ${activeTabData.id}`,
                              ...outputLines
                            ]
                          }
                        : t
                    ));
                  }
                }, 100);
              }}
              title="Run Python File"
            >
              <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <polygon points="5 3 19 12 5 21 5 3"></polygon>
              </svg>
            </button>
          )}
          {/* Save button for editor files */}
          {activeTabData?.type === 'editor' && onSaveFile && (
            <button 
              className={styles.actionButton} 
              onClick={async () => {
                if (activeTabData.content) {
                  await saveFileContent(activeTabData.id, activeTabData.content);
                }
              }}
              title="Save File (Cmd/Ctrl+S)"
            >
              <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z"></path>
                <polyline points="17 21 17 13 7 13 7 21"></polyline>
                <polyline points="7 3 7 8 15 8"></polyline>
              </svg>
            </button>
          )}
          {/* Split horizontally button */}
          {onSplitWindow && (
            <button 
              className={styles.actionButton} 
              onClick={() => onSplitWindow('horizontal')}
              title="Split Horizontally"
            >
              <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect>
                <line x1="3" y1="12" x2="21" y2="12"></line>
              </svg>
            </button>
          )}
          {/* Split vertically button */}
          {onSplitWindow && (
            <button 
              className={styles.actionButton} 
              onClick={() => onSplitWindow('vertical')}
              title="Split Vertically"
            >
              <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect>
                <line x1="12" y1="3" x2="12" y2="21"></line>
              </svg>
            </button>
          )}
          {/* Close window button - all windows have this */}
          {onCloseWindow && (
            <button 
              className={styles.actionButton} 
              onClick={onCloseWindow}
              title="Close Window"
            >
              <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <line x1="18" y1="6" x2="6" y2="18"></line>
                <line x1="6" y1="6" x2="18" y2="18"></line>
              </svg>
            </button>
          )}
        </div>
      </div>
      
      <div className={styles.editorWrapper}>
        {renderTabContent()}
      </div>
    </div>
  );
};