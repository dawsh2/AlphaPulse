/**
 * Terminal Emulator component for development environment
 */

import React, { useState, useRef, useEffect, useCallback } from 'react';
import styles from './Develop.module.css';

interface Command {
  id: string;
  input: string;
  output: string;
  timestamp: Date;
  status: 'success' | 'error' | 'running';
}

interface TerminalEmulatorProps {
  initialCommands?: Command[];
  onCommandExecute?: (command: string) => Promise<string>;
  workingDirectory?: string;
}

export const TerminalEmulator: React.FC<TerminalEmulatorProps> = ({
  initialCommands = [],
  onCommandExecute,
  workingDirectory = '~/alphapulse',
}) => {
  const [commands, setCommands] = useState<Command[]>(initialCommands);
  const [currentInput, setCurrentInput] = useState('');
  const [history, setHistory] = useState<string[]>([]);
  const [historyIndex, setHistoryIndex] = useState(-1);
  const [isExecuting, setIsExecuting] = useState(false);
  const terminalRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  // Auto-scroll to bottom
  useEffect(() => {
    if (terminalRef.current) {
      terminalRef.current.scrollTop = terminalRef.current.scrollHeight;
    }
  }, [commands]);

  // Built-in commands
  const executeBuiltInCommand = async (command: string): Promise<string> => {
    const [cmd, ...args] = command.trim().split(' ');
    
    switch (cmd) {
      case 'clear':
        setCommands([]);
        return '';
      
      case 'help':
        return `Available commands:
  clear    - Clear terminal
  help     - Show this help message
  history  - Show command history
  pwd      - Print working directory
  ls       - List files
  cd       - Change directory
  echo     - Print text
  
Custom strategy commands:
  run-backtest <strategy> - Run backtest for strategy
  analyze <symbol> - Analyze symbol
  optimize <params> - Optimize parameters`;
      
      case 'history':
        return history.map((cmd, i) => `${i + 1}  ${cmd}`).join('\n');
      
      case 'pwd':
        return workingDirectory;
      
      case 'echo':
        return args.join(' ');
      
      case 'ls':
        return `strategies/
indicators/
backtests/
data/
config.yaml
requirements.txt`;
      
      default:
        if (onCommandExecute) {
          return await onCommandExecute(command);
        }
        return `Command not found: ${cmd}. Type 'help' for available commands.`;
    }
  };

  const handleCommand = useCallback(async (input: string) => {
    if (!input.trim()) return;

    const newCommand: Command = {
      id: Date.now().toString(),
      input,
      output: '',
      timestamp: new Date(),
      status: 'running',
    };

    setCommands(prev => [...prev, newCommand]);
    setHistory(prev => [...prev, input]);
    setHistoryIndex(-1);
    setCurrentInput('');
    setIsExecuting(true);

    try {
      const output = await executeBuiltInCommand(input);
      setCommands(prev =>
        prev.map(cmd =>
          cmd.id === newCommand.id
            ? { ...cmd, output, status: 'success' }
            : cmd
        )
      );
    } catch (error) {
      setCommands(prev =>
        prev.map(cmd =>
          cmd.id === newCommand.id
            ? {
                ...cmd,
                output: `Error: ${error instanceof Error ? error.message : 'Unknown error'}`,
                status: 'error',
              }
            : cmd
        )
      );
    } finally {
      setIsExecuting(false);
    }
  }, [history, onCommandExecute, workingDirectory]);

  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Enter') {
      handleCommand(currentInput);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      if (history.length > 0 && historyIndex < history.length - 1) {
        const newIndex = historyIndex + 1;
        setHistoryIndex(newIndex);
        setCurrentInput(history[history.length - 1 - newIndex]);
      }
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      if (historyIndex > 0) {
        const newIndex = historyIndex - 1;
        setHistoryIndex(newIndex);
        setCurrentInput(history[history.length - 1 - newIndex]);
      } else if (historyIndex === 0) {
        setHistoryIndex(-1);
        setCurrentInput('');
      }
    } else if (e.ctrlKey && e.key === 'l') {
      e.preventDefault();
      setCommands([]);
    }
  };

  return (
    <div className={styles.terminal}>
      <div className={styles.terminalHeader}>
        <div className={styles.terminalTitle}>Terminal</div>
        <div className={styles.terminalActions}>
          <button
            className={styles.terminalButton}
            onClick={() => setCommands([])}
            title="Clear Terminal"
          >
            🗑️
          </button>
        </div>
      </div>
      
      <div className={styles.terminalBody} ref={terminalRef}>
        {commands.map(cmd => (
          <div key={cmd.id} className={styles.commandBlock}>
            <div className={styles.commandInput}>
              <span className={styles.prompt}>
                {workingDirectory} $
              </span>
              <span className={styles.command}>{cmd.input}</span>
            </div>
            {cmd.output && (
              <div
                className={`${styles.commandOutput} ${
                  cmd.status === 'error' ? styles.error : ''
                }`}
              >
                {cmd.status === 'running' ? (
                  <span className={styles.running}>Running...</span>
                ) : (
                  <pre>{cmd.output}</pre>
                )}
              </div>
            )}
          </div>
        ))}
        
        <div className={styles.commandBlock}>
          <div className={styles.commandInput}>
            <span className={styles.prompt}>
              {workingDirectory} $
            </span>
            <input
              ref={inputRef}
              type="text"
              className={styles.terminalInput}
              value={currentInput}
              onChange={(e) => setCurrentInput(e.target.value)}
              onKeyDown={handleKeyDown}
              disabled={isExecuting}
              placeholder={isExecuting ? 'Executing...' : 'Type a command...'}
              autoFocus
            />
          </div>
        </div>
      </div>
    </div>
  );
};