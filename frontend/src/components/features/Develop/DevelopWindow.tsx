/**
 * DevelopWindow Component
 * Unified window that can display either editor or terminal tabs
 */

import React, { useEffect, useRef } from 'react';
import CodeEditor from '../../CodeEditor/CodeEditor';
import styles from '../../../pages/DevelopPage.module.css';

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

  const handleTerminalCommand = (tabId: string) => {
    const tab = tabs.find(t => t.id === tabId);
    if (!tab || tab.type !== 'terminal') return;

    const command = tab.currentInput?.trim() || '';
    const newContent = [...(tab.terminalContent || [])];
    
    // Add command to output
    newContent.push(`${tab.cwd}$ ${command}`);
    
    // Simple command processing
    if (command === 'clear') {
      setTabs(tabs.map(t => 
        t.id === tabId 
          ? { ...t, terminalContent: [], currentInput: '' }
          : t
      ));
      return;
    } else if (command === 'pwd') {
      newContent.push(tab.cwd || '~/strategies');
    } else if (command === 'ls') {
      newContent.push('examples/          strategies/        indicators/');
      newContent.push('config/           docs/             tests/');
    } else if (command.startsWith('cd ')) {
      const dir = command.substring(3).trim();
      let newCwd = tab.cwd || '~/strategies';
      if (dir === '..') {
        const parts = newCwd.split('/');
        parts.pop();
        newCwd = parts.length > 1 ? parts.join('/') : '~';
      } else if (dir.startsWith('/')) {
        newCwd = dir;
      } else {
        newCwd = `${newCwd}/${dir}`;
      }
      setTabs(tabs.map(t => 
        t.id === tabId 
          ? { ...t, cwd: newCwd, name: newCwd, terminalContent: newContent, currentInput: '' }
          : t
      ));
      return;
    } else if (command === 'help') {
      newContent.push('Available Commands:');
      newContent.push('  help    - Show this help');
      newContent.push('  clear   - Clear terminal');
      newContent.push('  ls      - List files');
      newContent.push('  cd      - Change directory');
      newContent.push('  pwd     - Show current directory');
    } else if (command) {
      newContent.push(`bash: ${command}: command not found`);
    }
    
    setTabs(tabs.map(t => 
      t.id === tabId 
        ? { ...t, terminalContent: newContent, currentInput: '' }
        : t
    ));
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
      // Terminal tab
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
            <input
              ref={inputRef}
              type="text"
              className={styles.terminalInput}
              value={activeTabData.currentInput || ''}
              onChange={(e) => handleTerminalInput(activeTabData.id, e.target.value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter') {
                  handleTerminalCommand(activeTabData.id);
                }
              }}
              autoFocus
              spellCheck={false}
            />
          </div>
        </div>
      );
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