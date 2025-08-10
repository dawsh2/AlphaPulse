/**
 * Terminal Component for Development Page
 * Extracted from DevelopPage.tsx - includes terminal tabs, output, and command execution
 * NO FALLBACK CODE - Clean extraction only
 */

import React from 'react';
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

  const addOutput = (message: string, tabId?: string) => {
    const targetTabId = tabId || activeTerminalTab;
    setTerminalTabs(prev => prev.map(tab => 
      tab.id === targetTabId 
        ? { ...tab, content: [...tab.content, message] }
        : tab
    ));
  };

  const initializeConsole = () => {
    const currentTab = getCurrentTerminalTab();
    if (currentTab && !currentTab.content.length) {
      addOutput('');
      addOutput(' ███▄▄▄▄      ▄████████ ███    █▄      ███      ▄█   ▄█        ███    █▄     ▄████████ ');
      addOutput(' ███▀▀▀██▄   ███    ███ ███    ███ ▀█████████▄ ███  ███        ███    ███   ███    ███ ');
      addOutput(' ███   ███   ███    ███ ███    ███    ▀███▀▀██ ███▌ ███        ███    ███   ███    █▀  ');
      addOutput(' ███   ███   ███    ███ ███    ███     ███   ▀ ███▌ ███        ███    ███   ███        ');
      addOutput(' ███   ███ ▀███████████ ███    ███     ███     ███▌ ███      ▀▄███▄▄▄███  ▀███████████ ');
      addOutput(' ███   ███   ███    ███ ███    ███     ███     ███  ███       ▀▀███▀▀▀███           ███ ');
      addOutput(' ███   ███   ███    ███ ███    ███     ███     ███  ███▌    ▄  ███    ███     ▄█    ███ ');
      addOutput('  ▀█   █▀    ███    █▀  ████████▀     ▄████▀   █▀   █████▄▄██  ███    ███   ▄████████▀  ');
      addOutput('                                                    ▀         ███    ███                ');
      addOutput('');
      addOutput('Welcome to NautilusTrader Development Environment v2.0');
      addOutput('═══════════════════════════════════════════════════════');
      addOutput('');
      addOutput('🚀 Ready for quantitative trading strategy development');
      addOutput('📊 Integrated with market data, backtesting, and live trading');
      addOutput('');
      addOutput('Quick Start:');
      addOutput('  • Type "help" to see available commands');
      addOutput('  • Type "examples" to explore sample strategies');
      addOutput('  • Type "claude" for AI assistance');
      addOutput('');
      addOutput('Happy coding! 📈');
      addOutput('');
    }
  };

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
                </div>
                <div className={styles.inputLine}>
                  <span className={styles.prompt}>
                    {currentTab.cwd}$
                  </span>
                  <input
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
    </>
  );
};