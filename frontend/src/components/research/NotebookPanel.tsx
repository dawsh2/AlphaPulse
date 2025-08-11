/**
 * NotebookPanel Component - Interactive notebook interface with code/markdown cells
 * Extracted from ResearchPage for better separation of concerns
 */
import React from 'react';
import Editor from '@monaco-editor/react';
import styles from '../../pages/ResearchPage.module.css';

// Types
interface CodeSnippet {
  id: string;
  name: string;
  code: string;
  description?: string;
}

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
  isExecuting?: boolean;
  showAiAnalysis?: boolean;
  isAiChat?: boolean;
  parentCellId?: string;
  aiMessages?: AiMessage[];
  chatInput?: string;
}

interface NotebookPanelProps {
  notebookCells: NotebookCell[];
  activeCell: string | null;
  theme: string;
  setActiveCell: (cellId: string | null) => void;
  setNotebookCells: React.Dispatch<React.SetStateAction<NotebookCell[]>>;
  executeCell: (cellId: string) => Promise<void>;
  deleteCell: (cellId: string) => void;
  updateCellContent: (cellId: string, content: string) => void;
  addCell: (type: 'code' | 'markdown', afterId?: string) => void;
}

export const NotebookPanel: React.FC<NotebookPanelProps> = ({
  notebookCells,
  activeCell,
  theme,
  setActiveCell,
  setNotebookCells,
  executeCell,
  deleteCell,
  updateCellContent,
  addCell,
}) => {
  return (
    <div className={styles.notebookView}>
      <div className={styles.notebookCells}>
        {notebookCells.map(cell => {
          // Render AI chat cells differently
          if (cell.type === 'ai-chat') {
            return (
              <div 
                key={cell.id}
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
                    placeholder="Ask about your results or request specific analysis..."
                  />
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
              className={`${styles.notebookCell} ${styles[`${cell.type}Cell`]} ${activeCell === cell.id ? styles.active : ''}`}
              onClick={() => setActiveCell(cell.id)}
            >
              <div className={styles.cellHeader}>
                <span className={styles.cellType}>{cell.type}</span>
                <div className={styles.cellActions}>
                  <button 
                    onClick={() => executeCell(cell.id)} 
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
                    <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
                      <path d="M4.646 4.646a.5.5 0 0 1 .708 0L8 7.293l2.646-2.647a.5.5 0 0 1 .708.708L8.707 8l2.647 2.646a.5.5 0 0 1-.708.708L8 8.707l-2.646 2.647a.5.5 0 0 1-.708-.708L7.293 8 4.646 5.354a.5.5 0 0 1 0-.708z"/>
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
                    theme={theme}
                    onMount={(editor, monaco) => {
                      // Define the cream theme with more explicit colors
                      monaco.editor.defineTheme('cream-light', {
                        base: 'vs',
                        inherit: true,
                        rules: [
                          { token: '', foreground: '33332d', background: 'faf7f0' }
                        ],
                        colors: {
                          'editor.background': '#faf7f0',
                          'editor.foreground': '#33332d',
                          'editorLineNumber.foreground': '#8b8680',
                          'editorLineNumber.activeForeground': '#33332d',
                          'editor.selectionBackground': '#e5e0d5',
                          'editor.lineHighlightBackground': '#f5f2ea',
                          'editorCursor.foreground': '#33332d',
                          'editorWidget.background': '#f5f2ea',
                          'editorSuggestWidget.background': '#f5f2ea',
                          'editorHoverWidget.background': '#f5f2ea'
                        }
                      });
                      
                      // Force apply the theme
                      monaco.editor.setTheme(theme);
                    }}
                    options={{
                      fontSize: 13,
                      lineHeight: 1.5,
                      fontFamily: "'IBM Plex Mono', 'SF Mono', Monaco, Consolas, 'Courier New', monospace",
                      minimap: { enabled: false },
                      scrollBeyondLastLine: false,
                      automaticLayout: true,
                      wordWrap: 'on',
                      lineNumbers: 'on',
                      folding: false,
                      selectOnLineNumbers: true,
                      matchBrackets: 'always',
                      autoIndent: 'advanced',
                      formatOnPaste: true,
                      formatOnType: true,
                      tabSize: 4,
                      insertSpaces: true,
                      renderWhitespace: 'boundary',
                      smoothScrolling: true,
                      cursorBlinking: 'smooth',
                      cursorSmoothCaretAnimation: 'on',
                      scrollbar: {
                        vertical: 'visible',
                        horizontal: 'visible',
                        verticalScrollbarSize: 10,
                        horizontalScrollbarSize: 10,
                        alwaysConsumeMouseWheel: false
                      },
                      overviewRulerLanes: 0,
                      fixedOverflowWidgets: true
                    }}
                  />
                </div>
              ) : (
                <textarea
                  className={styles.cellTextarea}
                  value={cell.content}
                  onChange={(e) => updateCellContent(cell.id, e.target.value)}
                  onFocus={() => setActiveCell(cell.id)}
                  placeholder="Enter markdown content..."
                />
              )}
            </div>

            {cell.output && (
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
                <pre>{cell.output}</pre>
              </div>
            )}
          </div>
          );
        })}
        
        {/* Add Cell Button at Bottom */}
        <div className={styles.addCellContainer}>
          <button 
            className={styles.addCellButton}
            onClick={() => addCell('code')}
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