/**
 * NotebookView Component - Main content for the notebook view
 * Extracted from ResearchPage renderMainContent notebook section
 */
import React, { useState } from 'react';
import Editor from '@monaco-editor/react';
import * as monaco from 'monaco-editor';
import styles from '../../pages/ResearchPage.module.css';

interface AiMessage {
  role: 'assistant' | 'user';
  content: string;
  timestamp?: string;
}

interface NotebookCell {
  id: string;
  type: 'code' | 'markdown' | 'ai-chat';
  content: string;
  output?: string;
  images?: string[] | null;
  isExecuting?: boolean;
  showAiAnalysis?: boolean;
  isAiChat?: boolean;
  parentCellId?: string;
  aiMessages?: AiMessage[];
  chatInput?: string;
}

interface NotebookViewProps {
  notebookCells: NotebookCell[];
  setNotebookCells: React.Dispatch<React.SetStateAction<NotebookCell[]>>;
  activeCell: string | null;
  setActiveCell: (id: string | null) => void;
  deleteCell: (id: string) => void;
  executeCell: (id: string) => void;
  updateCellContent: (id: string, content: string) => void;
  addCellAfter: (id: string, type: 'code' | 'markdown') => void;
  toggleAiAnalysis: (id: string) => void;
  editorTheme: string;
  addCell: (type: 'code' | 'markdown') => void;
  notebookName?: string;
  setNotebookName?: (name: string) => void;
  onSaveNotebook?: () => void;
}

export const NotebookView: React.FC<NotebookViewProps> = ({
  notebookCells,
  setNotebookCells,
  activeCell,
  setActiveCell,
  deleteCell,
  executeCell,
  updateCellContent,
  addCellAfter,
  toggleAiAnalysis,
  editorTheme,
  addCell,
  notebookName = 'Untitled Notebook',
  setNotebookName,
  onSaveNotebook
}) => {
  const [searchQuery, setSearchQuery] = useState('');
  const [shiftHeld, setShiftHeld] = useState(false);
  const notebookRef = React.useRef<HTMLDivElement>(null);
  const cellsContainerRef = React.useRef<HTMLDivElement>(null);
  
  // Track shift key state
  React.useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Shift' && !e.repeat) {
        setShiftHeld(true);
      }
    };
    
    const handleKeyUp = (e: KeyboardEvent) => {
      if (e.key === 'Shift') {
        setShiftHeld(false);
      }
    };
    
    // Reset on blur to handle alt-tab etc
    const handleBlur = () => {
      setShiftHeld(false);
    };
    
    window.addEventListener('keydown', handleKeyDown);
    window.addEventListener('keyup', handleKeyUp);
    window.addEventListener('blur', handleBlur);
    
    return () => {
      window.removeEventListener('keydown', handleKeyDown);
      window.removeEventListener('keyup', handleKeyUp);
      window.removeEventListener('blur', handleBlur);
    };
  }, []);
  
  // Handle wheel events when shift is held
  React.useEffect(() => {
    const handleWheel = (e: WheelEvent) => {
      if (e.shiftKey) {
        const target = e.target as HTMLElement;
        console.log('Shift+scroll detected on:', target.className, target);
        
        // Check if we're anywhere in the notebook view
        const isInNotebook = target.closest(`.${styles.notebookView}`);
        if (isInNotebook) {
          console.log('Inside notebook view, preventing default');
          e.preventDefault();
          e.stopPropagation();
          
          // Scroll the notebook container instead
          if (cellsContainerRef.current) {
            console.log('Scrolling container by', e.deltaY);
            cellsContainerRef.current.scrollTop += e.deltaY;
          } else {
            console.log('No container ref!');
          }
          return false;
        }
      }
    };
    
    // Add to window with capture to intercept before Monaco
    window.addEventListener('wheel', handleWheel, { passive: false, capture: true });
    
    return () => {
      window.removeEventListener('wheel', handleWheel, { capture: true });
    };
  }, []);
  
  // Filter cells based on search query
  const filteredCells = searchQuery.trim() 
    ? notebookCells.filter(cell => {
        const searchLower = searchQuery.toLowerCase();
        const contentMatch = cell.content?.toLowerCase().includes(searchLower);
        const outputMatch = cell.output?.toLowerCase().includes(searchLower);
        const aiMessagesMatch = cell.aiMessages?.some(msg => 
          msg.content.toLowerCase().includes(searchLower)
        );
        return contentMatch || outputMatch || aiMessagesMatch;
      })
    : notebookCells;
  
  // Debug: Log cells to see if they have IDs
  React.useEffect(() => {
    console.log('Current notebook cells:', notebookCells.map(c => ({ id: c.id, type: c.type })));
  }, [notebookCells]);
  
  return (
    <div 
      ref={notebookRef}
      className={`${styles.notebookView} ${shiftHeld ? styles.shiftScrollMode : ''}`}
    >
      {/* Shift+Scroll indicator */}
      {shiftHeld && (
        <div className={styles.scrollModeIndicator}>
          Shift+Scroll: Page scrolling enabled
        </div>
      )}
      
      {/* Header with notebook name and search */}
      <div className={styles.notebookHeader}>
        <div className={styles.notebookTitle}>
          {setNotebookName ? (
            <>
              <input
                type="text"
                value={notebookName}
                onChange={(e) => setNotebookName(e.target.value)}
                className={styles.notebookNameInput}
                placeholder="Notebook name"
              />
              <button
                className={styles.saveNotebookButton}
                onClick={() => {
                  if (onSaveNotebook) {
                    onSaveNotebook();
                  } else {
                    // Fallback save logic
                    const savedNotebook = {
                      id: `notebook-${Date.now()}`,
                      name: notebookName,
                      lastModified: new Date().toISOString().split('T')[0],
                      cells: notebookCells
                    };
                    
                    // Save to localStorage
                    const existingNotebooks = JSON.parse(localStorage.getItem('savedNotebooks') || '[]');
                    existingNotebooks.push(savedNotebook);
                    localStorage.setItem('savedNotebooks', JSON.stringify(existingNotebooks));
                    
                    console.log('Notebook saved:', savedNotebook);
                  }
                }}
                title="Save notebook"
              >
                <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                  <path d="M19 21H5a2 2 0 01-2-2V5a2 2 0 012-2h11l5 5v11a2 2 0 01-2 2z"/>
                  <polyline points="17 21 17 13 7 13 7 21"/>
                  <polyline points="7 3 7 8 15 8"/>
                </svg>
              </button>
              <button
                className={styles.cleanupKernelButton}
                onClick={async () => {
                  try {
                    const response = await fetch('http://localhost:5002/api/notebook/cleanup', {
                      method: 'POST',
                      headers: { 'Content-Type': 'application/json' }
                    });
                    const data = await response.json();
                    console.log('Kernel cleanup:', data);
                    // Show a toast or notification
                    alert(data.message || 'Kernel cleaned up');
                  } catch (error) {
                    console.error('Failed to cleanup kernel:', error);
                    alert('Failed to cleanup kernel');
                  }
                }}
                title="Cleanup kernel (releases DuckDB locks)"
              >
                <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                  <path d="M3 6h18"/>
                  <path d="M8 6V4c0-1.1.9-2 2-2h4c1.1 0 2 .9 2 2v2"/>
                  <path d="M19 6v14c0 1.1-.9 2-2 2H7c-1.1 0-2-.9-2-2V6"/>
                  <line x1="10" y1="11" x2="10" y2="17"/>
                  <line x1="14" y1="11" x2="14" y2="17"/>
                </svg>
              </button>
            </>
          ) : (
            <span className={styles.notebookNameDisplay}>{notebookName}</span>
          )}
        </div>
        <input
          type="text"
          placeholder="Search cells..."
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          className={styles.notebookSearchInput}
        />
        {searchQuery && (
          <button 
            onClick={() => setSearchQuery('')}
            className={styles.clearSearchBtn}
            title="Clear search"
          >
            ×
          </button>
        )}
        {searchQuery && (
          <span className={styles.searchResultsCount}>
            {filteredCells.length} of {notebookCells.length} cells
          </span>
        )}
      </div>
      
      <div 
        ref={cellsContainerRef} 
        className={styles.notebookCells}
      >
        {filteredCells.map(cell => {
          // Render AI chat cells differently
          if (cell.type === 'ai-chat') {
            return (
              <div 
                key={cell.id}
                id={`cell-${cell.id}`}
                className={`${styles.aiChatCell} ${activeCell === cell.id ? styles.active : ''}`}
                onClick={() => setActiveCell(cell.id)}
              >
                <div className={styles.aiChatHeader}>
                  <div className={styles.aiChatTitle}>
                    <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                      <path d="M9.5 2A3.5 3.5 0 0 0 6 5.5c0 2.3 2.5 3.3 2.5 5.5v1"/>
                      <path d="M14.5 2A3.5 3.5 0 0 1 18 5.5c0 2.3-2.5 3.3-2.5 5.5v1"/>
                      <path d="M12 2v10"/>
                      <circle cx="12" cy="14" r="2"/>
                      <path d="M7 14H5M19 14h-2M12 16v2"/>
                      <circle cx="7" cy="14" r="1"/>
                      <circle cx="17" cy="14" r="1"/>
                      <circle cx="12" cy="19" r="1"/>
                    </svg>
                    <span>AI Analysis Assistant</span>
                  </div>
                  <button 
                    onClick={() => deleteCell(cell.id)}
                    className={styles.closeAiChat}
                    title="Close AI chat"
                  >
                    ×
                  </button>
                </div>
                
                <div className={styles.aiChatMessages}>
                  {cell.aiMessages?.map((msg, idx) => (
                    <div key={idx} className={`${styles.aiMessage} ${styles[msg.role]}`}>
                      <span className={styles.messageRole}>
                        {msg.role === 'assistant' ? '🤖' : '👤'}
                      </span>
                      <div className={styles.messageContent}>{msg.content}</div>
                    </div>
                  ))}
                </div>
                
                <div className={styles.aiChatInput}>
                  <input
                    type="text"
                    value={cell.chatInput || ''}
                    onChange={(e) => {
                      setNotebookCells(prev =>
                        prev.map(c =>
                          c.id === cell.id
                            ? { ...c, chatInput: e.target.value }
                            : c
                        )
                      );
                    }}
                    onKeyPress={(e) => {
                      if (e.key === 'Enter' && cell.chatInput?.trim()) {
                        // Add user message and generate AI response
                        const userMessage: AiMessage = {
                          role: 'user',
                          content: cell.chatInput
                        };
                        
                        // Generate AI response based on user input
                        let aiResponse = '';
                        const input = cell.chatInput.toLowerCase();
                        
                        if (input.includes('volatility') || input.includes('vol')) {
                          aiResponse = `Good choice focusing on volatility. I recommend using these snippets:
1. \`snippets.risk.volatility_decomp(returns, window=30)\` - Separates market vs idiosyncratic volatility
2. \`snippets.risk.rolling_correlation(returns, benchmark='SPY')\` - Shows when correlations spike

This will help identify if the volatility is systematic or strategy-specific. Ready to generate the analysis cell?`;
                        } else if (input.includes('drawdown') || input.includes('risk')) {
                          aiResponse = `For drawdown analysis, let's use:
1. \`snippets.risk.drawdown_clusters(returns, min_duration=5)\` - Identifies drawdown patterns
2. \`snippets.risk.max_drawdown_duration(returns)\` - Time to recovery analysis
3. \`snippets.risk.conditional_drawdown(returns, confidence=0.95)\` - Expected shortfall

These will give you a complete risk picture. Generate the cell?`;
                        } else if (input.includes('performance') || input.includes('returns')) {
                          aiResponse = `To analyze performance, I suggest:
1. \`snippets.performance.rolling_sharpe(returns, window=60)\` - Time-varying risk-adjusted returns
2. \`snippets.performance.factor_attribution(returns, factors=['MKT', 'SMB', 'HML'])\` - Factor decomposition
3. \`snippets.performance.regime_analysis(returns, vix_threshold=20)\` - Performance by market regime

This will show where your returns are coming from. Ready to build the cell?`;
                        } else {
                          aiResponse = `Based on your question, I can help with:
• Volatility analysis - decompose market vs strategy risk
• Drawdown patterns - understand your risk profile
• Performance attribution - see what drives returns
• Signal quality - evaluate entry/exit timing

Which area would you like to explore first?`;
                        }
                        
                        const aiMessage: AiMessage = {
                          role: 'assistant',
                          content: aiResponse
                        };
                        
                        setNotebookCells(prev =>
                          prev.map(c =>
                            c.id === cell.id
                              ? {
                                  ...c,
                                  aiMessages: [...(c.aiMessages || []), userMessage, aiMessage],
                                  chatInput: ''
                                }
                              : c
                          )
                        );
                      }
                    }}
                    placeholder="Ask about your strategy analysis..."
                    className={styles.chatInputField}
                  />
                  <button 
                    onClick={() => {
                      if (cell.chatInput?.trim()) {
                        // Same logic as Enter key
                        const userMessage: AiMessage = {
                          role: 'user',
                          content: cell.chatInput
                        };
                        
                        let aiResponse = 'I can help you analyze that. Let me suggest the best approach...';
                        
                        const aiMessage: AiMessage = {
                          role: 'assistant',
                          content: aiResponse
                        };
                        
                        setNotebookCells(prev =>
                          prev.map(c =>
                            c.id === cell.id
                              ? {
                                  ...c,
                                  aiMessages: [...(c.aiMessages || []), userMessage, aiMessage],
                                  chatInput: ''
                                }
                              : c
                          )
                        );
                      }
                    }}
                    className={styles.sendButton}
                  >
                    Send
                  </button>
                  <button 
                    className={styles.generateCellBtn}
                    onClick={() => {
                      // Generate a new code cell with recommended snippets
                      const codeTemplate = `# AI-recommended analysis based on your discussion
import admf
from snippets import risk, performance, signals

# Volatility decomposition
vol_decomp = risk.volatility_decomp(returns, window=30)
print("Market vs Idiosyncratic Volatility:")
print(vol_decomp)

# Rolling correlation analysis
correlations = risk.rolling_correlation(returns, benchmark='SPY')
correlations.plot(title='Rolling Correlation with Market')

# Performance attribution
attribution = performance.factor_attribution(returns, factors=['MKT', 'SMB', 'HML'])
print("\\nFactor Attribution:")
print(attribution)`;
                      
                      const newCell: NotebookCell = {
                        id: `cell-${Date.now()}`,
                        type: 'code',
                        content: codeTemplate
                      };
                      
                      setNotebookCells(prev => {
                        const index = prev.findIndex(c => c.id === cell.id);
                        const newCells = [...prev];
                        newCells.splice(index + 1, 0, newCell);
                        return newCells;
                      });
                      
                      // Auto-scroll to the new cell
                      setTimeout(() => {
                        const element = document.getElementById(`cell-${newCell.id}`);
                        if (element) {
                          element.scrollIntoView({ behavior: 'smooth', block: 'center' });
                        }
                      }, 100);
                    }}
                    title="Generate analysis cell from recommendations"
                  >
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                      <path d="M12 2v20M2 12h20"/>
                    </svg>
                    Generate Cell
                  </button>
                </div>
              </div>
            );
          }
          
          // Regular cell rendering
          return (
            <div 
              key={cell.id}
              id={`cell-${cell.id}`}
              className={`${styles.notebookCell} ${styles[`${cell.type}Cell`]} ${activeCell === cell.id ? styles.active : ''}`}
              onClick={() => setActiveCell(cell.id)}
            >
              <div className={styles.cellHeader}>
                <span className={styles.cellType}>{cell.type}</span>
                <div className={styles.cellActions}>
                  <button 
                    onClick={(e) => {
                      e.stopPropagation();
                      console.log('Execute button clicked for cell:', cell.id);
                      executeCell(cell.id);
                    }} 
                    disabled={cell.isExecuting}
                    className={styles.cellActionBtn}
                    title="Run cell"
                  >
                    {cell.isExecuting ? (
                      <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
                        <circle cx="8" cy="8" r="6" stroke="currentColor" strokeWidth="2" strokeDasharray="4 2">
                          <animateTransform attributeName="transform" type="rotate" from="0 8 8" to="360 8 8" dur="1s" repeatCount="indefinite"/>
                        </circle>
                      </svg>
                    ) : (
                      <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
                        <path d="M5 3.5v9l7-4.5z"/>
                      </svg>
                    )}
                  </button>
                  <button 
                    onClick={() => deleteCell(cell.id)}
                    className={styles.cellActionBtn}
                    title="Delete cell"
                  >
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                      <line x1="18" y1="6" x2="6" y2="18"></line>
                      <line x1="6" y1="6" x2="18" y2="18"></line>
                    </svg>
                  </button>
                </div>
              </div>
              
              <div className={styles.cellContent}>
                {cell.type === 'code' ? (
                  <div className={styles.codeEditor}>
                    <Editor
                      height="300px"
                      language="python"
                      value={cell.content}
                      onChange={(value) => updateCellContent(cell.id, value || '')}
                      theme={editorTheme}
                      options={{
                        minimap: { enabled: false },
                        fontSize: 13,
                        lineNumbers: 'on',
                        folding: true,
                        padding: { top: 10, bottom: 10 },
                        scrollBeyondLastLine: false,
                        scrollbar: {
                          alwaysConsumeMouseWheel: false,
                          vertical: 'auto',
                          horizontal: 'auto'
                        }
                      }}
                    />
                  </div>
                ) : (
                  <div className={styles.markdownEditor}>
                    <Editor
                      height="150px"
                      language="markdown"
                      value={cell.content}
                      onChange={(value) => updateCellContent(cell.id, value || '')}
                      theme={editorTheme}
                      options={{
                        minimap: { enabled: false },
                        scrollBeyondLastLine: false,
                        scrollbar: {
                          alwaysConsumeMouseWheel: false,
                          vertical: 'auto',
                          horizontal: 'auto'
                        },
                        lineNumbers: 'off',
                        folding: false,
                        fontSize: 14,
                        wordWrap: 'on',
                        padding: { top: 10, bottom: 10 }
                      }}
                    />
                  </div>
                )}
              </div>
              
              {(cell.output || cell.images) && (
                <div className={styles.cellOutput}>
                  <div className={styles.outputHeader}>
                    <span className={styles.outputLabel}>Output</span>
                    <button
                      className={styles.aiAnalyzeBtn}
                      onClick={(e) => {
                        e.stopPropagation();
                        // Create AI chat cell after this cell
                        const newAiCell: NotebookCell = {
                          id: `ai-chat-${Date.now()}`,
                          type: 'ai-chat',
                          content: '',
                          output: cell.output,
                          isAiChat: true,
                          parentCellId: cell.id,
                          aiMessages: [
                            {
                              role: 'assistant',
                              content: `I've analyzed your results. ${
                                cell.output?.includes('Overview') 
                                  ? "Your strategies show interesting patterns. The Sharpe ratio of 1.87 is solid, but I notice the max drawdown of -12.4%. What's your main concern - risk management or performance optimization?"
                                  : cell.output?.includes('Backtest')
                                  ? "Your backtest shows 142 trades with a 62.7% win rate. The expectancy of $1,247 per trade is promising. Would you like to explore position sizing optimization or signal filtering?"
                                  : "I can see several areas for improvement in your analysis. What aspect would you like to investigate first - volatility patterns, correlation analysis, or performance attribution?"
                              }`
                            }
                          ],
                          chatInput: ''
                        };
                        
                        setNotebookCells(prev => {
                          const index = prev.findIndex(c => c.id === cell.id);
                          const newCells = [...prev];
                          // Check if AI chat already exists for this cell
                          const existingAiChat = prev.find(c => c.parentCellId === cell.id);
                          if (!existingAiChat) {
                            newCells.splice(index + 1, 0, newAiCell);
                          }
                          return newCells;
                        });
                        setActiveCell(newAiCell.id);
                        
                        // Auto-scroll to the new AI cell
                        setTimeout(() => {
                          const element = document.getElementById(`cell-${newAiCell.id}`);
                          if (element) {
                            element.scrollIntoView({ behavior: 'smooth', block: 'center' });
                          }
                        }, 100);
                      }}
                      title="AI Analysis"
                    >
                      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                        {/* Computerized brain icon - circuit-like design */}
                        <path d="M9.5 2A3.5 3.5 0 0 0 6 5.5c0 2.3 2.5 3.3 2.5 5.5v1"/>
                        <path d="M14.5 2A3.5 3.5 0 0 1 18 5.5c0 2.3-2.5 3.3-2.5 5.5v1"/>
                        <path d="M12 2v10"/>
                        <circle cx="12" cy="14" r="2"/>
                        <path d="M7 14H5M19 14h-2M12 16v2"/>
                        <circle cx="7" cy="14" r="1"/>
                        <circle cx="17" cy="14" r="1"/>
                        <circle cx="12" cy="19" r="1"/>
                        <path d="M9 19H7v2M15 19h2v2M5 14v-2M19 14v-2"/>
                        <circle cx="5" cy="11" r="0.5"/>
                        <circle cx="19" cy="11" r="0.5"/>
                        <circle cx="7" cy="21" r="0.5"/>
                        <circle cx="17" cy="21" r="0.5"/>
                      </svg>
                      <span>AI Analysis</span>
                    </button>
                  </div>
                  {cell.output && <pre>{cell.output}</pre>}
                  {cell.images && cell.images.map((imageData, idx) => (
                    <img 
                      key={idx}
                      src={`data:image/png;base64,${imageData}`}
                      alt={`Plot ${idx + 1}`}
                      style={{ maxWidth: '100%', marginTop: '10px' }}
                    />
                  ))}
                </div>
              )}
            </div>
          );
        })}
        
        {/* Add Cell Button at Bottom */}
        <div className={styles.addCellContainer}>
          <button 
            className={styles.addCellButton}
            onClick={() => {
              addCell('code');
              // Auto-scroll to bottom after adding cell
              setTimeout(() => {
                const cells = document.querySelectorAll('[id^="cell-"]');
                if (cells.length > 0) {
                  const lastCell = cells[cells.length - 1];
                  lastCell.scrollIntoView({ behavior: 'smooth', block: 'center' });
                }
              }, 100);
            }}
            title="Add new cell"
          >
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <line x1="12" y1="5" x2="12" y2="19"></line>
              <line x1="5" y1="12" x2="19" y2="12"></line>
            </svg>
            <span>Add Cell</span>
          </button>
        </div>
      </div>
    </div>
  );
};