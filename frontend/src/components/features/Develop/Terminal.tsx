/**
 * Terminal Component for Development Page
 * Extracted from DevelopPage.tsx - includes terminal tabs, output, and command execution
 * NO FALLBACK CODE - Clean extraction only
 */

import React, { useEffect, useRef } from 'react';
import { formatShortPath } from '../../../utils/format';

export interface TerminalTab {
  id: string;
  name: string;
  content: string[];
  currentInput: string;
  cwd: string;
}

interface TerminalProps {
  terminalTabs: TerminalTab[];
  activeTerminalTab: string;
  outputOpen: boolean;
  editorHidden: boolean;
  splitOrientation: 'horizontal' | 'vertical';
  splitSize: number;
  terminalTabCounter: number;
  setTerminalTabs: (tabs: TerminalTab[]) => void;
  setActiveTerminalTab: (tabId: string) => void;
  setTerminalTabCounter: (counter: number) => void;
  setSplitOrientation: (orientation: 'horizontal' | 'vertical') => void;
  setSplitSize: (size: number) => void;
  setOutputOpen: (open: boolean) => void;
  onSplitDragStart: (e: React.MouseEvent) => void;
  onInitializeConsole: () => void;
  styles: Record<string, string>;
}

export const Terminal: React.FC<TerminalProps> = ({
  terminalTabs,
  activeTerminalTab,
  outputOpen,
  editorHidden,
  splitOrientation,
  splitSize,
  terminalTabCounter,
  setTerminalTabs,
  setActiveTerminalTab,
  setTerminalTabCounter,
  setSplitOrientation,
  setSplitSize,
  setOutputOpen,
  onSplitDragStart,
  onInitializeConsole,
  styles
}) => {
  const getCurrentTerminalTab = () => {
    return terminalTabs.find(tab => tab.id === activeTerminalTab) || terminalTabs[0];
  };

  const addTerminalTab = () => {
    const newTab: TerminalTab = {
      id: `terminal-${terminalTabCounter}`,
      name: '~/strategies',
      content: [],
      currentInput: '',
      cwd: '~/strategies'
    };
    setTerminalTabs([...terminalTabs, newTab]);
    setActiveTerminalTab(newTab.id);
    setTerminalTabCounter(terminalTabCounter + 1);
    setTimeout(() => onInitializeConsole(), 100);
  };

  const closeTerminalTab = (tabId: string) => {
    if (terminalTabs.length <= 1) return;
    
    const tabIndex = terminalTabs.findIndex(t => t.id === tabId);
    const newTabs = terminalTabs.filter(t => t.id !== tabId);
    setTerminalTabs(newTabs);
    
    if (activeTerminalTab === tabId) {
      const newActiveIndex = Math.min(tabIndex, newTabs.length - 1);
      setActiveTerminalTab(newTabs[newActiveIndex].id);
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

  const outputEndRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  
  const addOutput = (message: string, tabId?: string) => {
    const targetTabId = tabId || activeTerminalTab;
    setTerminalTabs(prev => prev.map(tab => 
      tab.id === targetTabId 
        ? { ...tab, content: [...tab.content, message] }
        : tab
    ));
  };
  
  // Auto-scroll to bottom when new output is added
  useEffect(() => {
    if (outputEndRef.current) {
      outputEndRef.current.scrollIntoView({ behavior: 'smooth' });
    }
  }, [terminalTabs]);
  
  // Refocus input after orientation change
  useEffect(() => {
    if (inputRef.current && outputOpen) {
      setTimeout(() => {
        inputRef.current?.focus();
      }, 100);
    }
  }, [splitOrientation, outputOpen]);

  const initializeConsole = () => {
    const currentTab = getCurrentTerminalTab();
    if (currentTab && currentTab.content.length === 0) {
      const timestamp = new Date().toISOString();
      addOutput(`${timestamp} [INFO] BACKTESTER-001.BacktestEngine: =================================================================`);
      addOutput(`${timestamp} [INFO] BACKTESTER-001.BacktestEngine:  NAUTILUS TRADER - Automated Algorithmic Trading Platform`);
      addOutput(`${timestamp} [INFO] BACKTESTER-001.BacktestEngine:  by Nautech Systems Pty Ltd.`);
      addOutput(`${timestamp} [INFO] BACKTESTER-001.BacktestEngine:  Copyright (C) 2015-2025. All rights reserved.`);
      addOutput(`${timestamp} [INFO] BACKTESTER-001.BacktestEngine: =================================================================`);
      addOutput(`${timestamp} [INFO] BACKTESTER-001.BacktestEngine: `);
      addOutput(`${timestamp} [INFO] BACKTESTER-001.BacktestEngine: ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣠⣴⣶⡟⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀`);
      addOutput(`${timestamp} [INFO] BACKTESTER-001.BacktestEngine: ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣰⣾⣿⣿⣿⠀⢸⣿⣿⣿⣿⣶⣶⣤⣀⠀⠀⠀⠀⠀`);
      addOutput(`${timestamp} [INFO] BACKTESTER-001.BacktestEngine: ⠀⠀⠀⠀⠀⠀⢀⣴⡇⢀⣾⣿⣿⣿⣿⣿⠀⣾⣿⣿⣿⣿⣿⣿⣿⠿⠓⠀⠀⠀⠀`);
      addOutput(`${timestamp} [INFO] BACKTESTER-001.BacktestEngine: ⠀⠀⠀⠀⠀⣰⣿⣿⡀⢸⣿⣿⣿⣿⣿⣿⠀⣿⣿⣿⣿⣿⣿⠟⠁⣠⣄⠀⠀⠀⠀`);
      addOutput(`${timestamp} [INFO] BACKTESTER-001.BacktestEngine: ⠀⠀⠀⠀⢠⣿⣿⣿⣇⠀⢿⣿⣿⣿⣿⣿⠀⢻⣿⣿⣿⡿⢃⣠⣾⣿⣿⣧⡀⠀⠀`);
      addOutput(`${timestamp} [INFO] BACKTESTER-001.BacktestEngine: ⠀⠀⠀⠠⣾⣿⣿⣿⣿⣿⣧⠈⠋⢀⣴⣧⠀⣿⡏⢠⡀⢸⣿⣿⣿⣿⣿⣿⣿⡇⠀`);
      addOutput(`${timestamp} [INFO] BACKTESTER-001.BacktestEngine: ⠀⠀⠀⣀⠙⢿⣿⣿⣿⣿⣿⠇⢠⣿⣿⣿⡄⠹⠃⠼⠃⠈⠉⠛⠛⠛⠛⠛⠻⠇⠀`);
      addOutput(`${timestamp} [INFO] BACKTESTER-001.BacktestEngine: ⠀⠀⢸⡟⢠⣤⠉⠛⠿⢿⣿⠀⢸⣿⡿⠋⣠⣤⣄⠀⣾⣿⣿⣶⣶⣶⣦⡄⠀⠀⠀`);
      addOutput(`${timestamp} [INFO] BACKTESTER-001.BacktestEngine: ⠀⠀⠸⠀⣾⠏⣸⣷⠂⣠⣤⠀⠘⢁⣴⣾⣿⣿⣿⡆⠘⣿⣿⣿⣿⣿⣿⠀⠀⠀⠀`);
      addOutput(`${timestamp} [INFO] BACKTESTER-001.BacktestEngine: ⠀⠀⠀⠀⠛⠀⣿⡟⠀⢻⣿⡄⠸⣿⣿⣿⣿⣿⣿⣿⡀⠘⣿⣿⣿⣿⠟⠀⠀⠀⠀`);
      addOutput(`${timestamp} [INFO] BACKTESTER-001.BacktestEngine: ⠀⠀⠀⠀⠀⠀⣿⠇⠀⠀⢻⡿⠀⠈⠻⣿⣿⣿⣿⣿⡇⠀⢹⣿⠿⠋⠀⠀⠀⠀⠀`);
      addOutput(`${timestamp} [INFO] BACKTESTER-001.BacktestEngine: ⠀⠀⠀⠀⠀⠀⣿⠀⠀⠀⠀⠙⠀⠀⠀⠈⠛⠿⠿⠛⠁⠀⠈⠁⠀⠀⠀⠀⠀⠀⠀`);
      addOutput(`${timestamp} [INFO] BACKTESTER-001.BacktestEngine: ⠀⠀⠀⠀⠀⠀⣿⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀`);
      addOutput(`${timestamp} [INFO] BACKTESTER-001.BacktestEngine: ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀`);
      addOutput('');
      addOutput(`${timestamp} [INFO] BACKTESTER-001.BacktestEngine: =================================================================`);
      addOutput(`${timestamp} [INFO] BACKTESTER-001.BacktestEngine: Component initialized.`);
      addOutput(`${timestamp} [INFO] BACKTESTER-001.BacktestEngine: ================================================================= `);
      addOutput('');
      addOutput('Type "help" for available commands or "examples" to see strategy samples.');
      addOutput('');
    }
  };

  // Initialize console when terminal opens
  useEffect(() => {
    if (outputOpen && terminalTabs.length > 0) {
      const currentTab = getCurrentTerminalTab();
      if (currentTab && currentTab.content.length === 0) {
        setTimeout(() => initializeConsole(), 100);
      }
    }
  }, [outputOpen]);

  if (!outputOpen) return null;

  return (
    <>
      {!editorHidden && (
        <div 
          className={`${styles.splitter} ${splitOrientation === 'horizontal' ? styles.splitterHorizontal : styles.splitterVertical}`}
          onMouseDown={onSplitDragStart}
        />
      )}
      <div
        style={{
          flex: editorHidden ? '1' : `0 0 ${splitSize || 300}px`,
          minWidth: splitOrientation === 'vertical' && !editorHidden ? '250px' : 'auto',
          minHeight: splitOrientation === 'horizontal' && !editorHidden ? '150px' : 'auto',
          borderBottom: '3px solid var(--color-border-primary)',
          borderRight: splitOrientation === 'vertical' ? '3px solid var(--color-border-primary)' : 'none',
          display: 'flex',
          flexDirection: 'column',
          position: 'relative'
        }}
      >
      <div 
        className={`${styles.outputPanel} ${styles[splitOrientation]} ${editorHidden ? styles.fullScreen : ''}`}
        style={{
          border: 'none',
          height: '100%',
          flex: '1',
          display: 'flex',
          flexDirection: 'column'
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
                  <span className={styles.terminalTabName} title={tab.cwd}>{formatShortPath(tab.name)}</span>
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
          <div style={{ display: 'flex', gap: '8px', alignItems: 'center' }}>
            <button 
              className={styles.outputClose}
              onClick={() => {
                const newOrientation = splitOrientation === 'horizontal' ? 'vertical' : 'horizontal';
                setSplitOrientation(newOrientation);
                // Recalculate split size for new orientation
                const mainArea = document.querySelector(`.${styles.mainArea}`) as HTMLElement;
                if (mainArea) {
                  if (newOrientation === 'horizontal') {
                    setSplitSize(Math.floor(mainArea.clientHeight / 2));
                  } else {
                    setSplitSize(Math.floor(mainArea.clientWidth / 2));
                  }
                }
              }}
              title={`Switch to ${splitOrientation === 'horizontal' ? 'vertical' : 'horizontal'} split`}
            >
              {splitOrientation === 'horizontal' ? <span style={{ transform: 'rotate(90deg)', display: 'inline-block' }}>⊟</span> : '⊟'}
            </button>
            <button 
              className={styles.outputClose}
              onClick={() => setOutputOpen(false)}
              title="Close Terminal"
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
                <div className={styles.outputLines}>
                  {currentTab.content.map((line, index) => (
                    <div key={index} className={styles.outputLine}>
                      {line}
                    </div>
                  ))}
                  <div ref={outputEndRef} />
                </div>
                <div className={styles.inputLine}>
                  <span className={styles.prompt}>
                    {currentTab.cwd}$
                  </span>
                  <input
                    ref={inputRef}
                    type="text"
                    className={styles.terminalInput}
                    value={currentTab.currentInput}
                    onChange={(e) => updateTerminalInput(e.target.value)}
                    onKeyDown={(e) => {
                      if (e.key === 'Enter') {
                        const command = currentTab.currentInput.trim();
                        const timestamp = new Date().toLocaleTimeString();
                        
                        // Add command to output
                        addOutput(`${currentTab.cwd}$ ${command}`);
                        
                        // Execute command
                        if (command === 'help') {
                          addOutput('');
                          addOutput('Available Commands:');
                          addOutput('══════════════════');
                          addOutput('  help           - Show this help message');
                          addOutput('  clear          - Clear terminal output');
                          addOutput('  ls             - List directory contents');
                          addOutput('  cd <dir>       - Change directory');
                          addOutput('  pwd            - Show current directory');
                          addOutput('  examples       - Show example strategies');
                          addOutput('  python <file>  - Run Python strategy');
                          addOutput('  claude [prompt]- Get AI assistance');
                          addOutput('  exit           - Close terminal tab');
                          addOutput('');
                        } else if (command === 'clear') {
                          setTerminalTabs(prev => prev.map(tab => 
                            tab.id === activeTerminalTab 
                              ? { ...tab, content: [] }
                              : tab
                          ));
                        } else if (command === 'pwd') {
                          addOutput(currentTab.cwd);
                        } else if (command === 'ls') {
                          addOutput('examples/          strategies/        indicators/');
                          addOutput('config/           docs/             tests/');
                          addOutput('README.md         requirements.txt  setup.py');
                        } else if (command.startsWith('cd ')) {
                          const dir = command.substring(3).trim();
                          let newCwd: string;
                          if (dir === '..') {
                            const parts = currentTab.cwd.split('/');
                            parts.pop();
                            newCwd = parts.length > 1 ? parts.join('/') : '~';
                          } else if (dir.startsWith('/')) {
                            newCwd = dir;
                          } else {
                            newCwd = `${currentTab.cwd}/${dir}`;
                          }
                          
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
                          setTimeout(() => {
                            addOutput('[Claude] Hello! I\'m here to help with your NautilusTrader development.');
                            addOutput('');
                            addOutput('I can assist with:');
                            addOutput('  • Strategy implementation and optimization');
                            addOutput('  • Backtesting and performance analysis');
                            addOutput('  • Technical indicator development');
                            addOutput('  • Risk management techniques');
                            addOutput('  • Market data handling and processing');
                            addOutput('');
                            addOutput('For specific help, try:');
                            addOutput('  1. Ask about strategy patterns: "claude How do I implement a mean reversion strategy?"');
                            addOutput('  2. Debug issues: "claude My backtest is showing negative returns, what should I check?"');
                            addOutput('  3. Learn best practices: "claude What are common mistakes in algorithmic trading?"');
                            addOutput('');
                            addOutput('Ready to build some winning strategies? 🚀📈');
                          }, 500);
                        } else if (command.startsWith('claude ')) {
                          const prompt = command.substring(7).trim();
                          if (prompt) {
                            addOutput(`[Claude] Analyzing: "${prompt}"`);
                            setTimeout(() => {
                              addOutput('');
                              addOutput('[Claude] 🤔 Thinking...');
                              addOutput('');
                              addOutput('[Claude] Based on your question, here are some key considerations:');
                              addOutput('');
                              addOutput('  1. Review the strategy logic and ensure proper risk management');
                              addOutput('  2. Check data quality and handle edge cases appropriately');
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
      </div>
    </>
  );
};