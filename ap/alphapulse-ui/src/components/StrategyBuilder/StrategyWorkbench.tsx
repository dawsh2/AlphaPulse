import React, { useState, useEffect } from 'react';
import styles from './StrategyWorkbench.module.css';

interface StrategyWorkbenchProps {
  isOpen: boolean;
  onClose: () => void;
  initialTemplate?: string;
}

// Strategy condition types
interface TradingCondition {
  id: string;
  type: 'entry' | 'exit' | 'stop';
  condition: string;
  isValid: boolean;
}

interface ParameterRange {
  min: number;
  max: number;
  step: number;
  default?: number;
  values?: number[]; // For discrete values
}

interface StrategyTemplate {
  id: string;
  name: string;
  description: string;
  category: string;
  icon: string;
  entryConditions: string[];
  exitConditions: string[];
  stopLoss?: string;
  takeProfit?: string;
  parameters: Record<string, ParameterRange>;
}

interface BacktestResults {
  totalTrades: number;
  winRate: number;
  avgReturn: number;
  sharpeRatio: number;
  maxDrawdown: number;
  profitFactor: number;
  trades: Trade[];
}

interface Trade {
  id: string;
  timestamp: string;
  type: 'BUY' | 'SELL';
  price: number;
  quantity: number;
  pnl?: number;
}

interface DataUniverse {
  symbols: string[];
  timeframe: string;
  startDate: string;
  endDate: string;
  parameterSpace: Record<string, ParameterRange>;
}




interface OptimizationResult {
  parameters: Record<string, number>;
  metrics: {
    sharpe: number;
    calmar: number;
    maxDrawdown: number;
    totalReturn: number;
    winRate: number;
  };
  trades: number;
  rank: number;
}

// Strategy templates with parameter ranges as the default
const strategyTemplates: StrategyTemplate[] = [
  {
    id: 'oversold_bounce',
    name: 'Oversold Bounce',
    description: 'Buy when RSI oversold with volume confirmation',
    category: 'Mean Reversion',
    icon: '',
    entryConditions: ['RSI({rsi_period}) < {rsi_oversold}', 'Volume > SMA(Volume, 20) * {volume_multiplier}'],
    exitConditions: ['RSI({rsi_period}) > {rsi_overbought}'],
    stopLoss: 'Price < Entry * 0.95',
    parameters: {
      rsi_period: { min: 10, max: 21, step: 1, default: 14, values: [10, 14, 21] },
      rsi_oversold: { min: 20, max: 35, step: 5, default: 30 },
      rsi_overbought: { min: 65, max: 80, step: 5, default: 70 },
      volume_multiplier: { min: 1.2, max: 3.0, step: 0.2, default: 1.5 }
    }
  },
  {
    id: 'volume_breakout',
    name: 'Volume Breakout',
    description: 'High volume breakout above resistance',
    category: 'Momentum',
    icon: '💥',
    entryConditions: ['Price > SMA({sma_period})', 'Volume > SMA(Volume, 20) * {volume_threshold}', 'Price > High(5) * {breakout_pct}'],
    exitConditions: ['Price < SMA({sma_period})'],
    takeProfit: 'Price > Entry * 1.08',
    parameters: {
      sma_period: { min: 10, max: 50, step: 5, default: 20, values: [10, 20, 50] },
      volume_threshold: { min: 1.5, max: 4.0, step: 0.5, default: 2.0 },
      breakout_pct: { min: 1.005, max: 1.02, step: 0.005, default: 1.01 }
    }
  },
  {
    id: 'bollinger_reversion',
    name: 'Bollinger Mean Reversion',
    description: 'Trade bounces off Bollinger Bands',
    category: 'Mean Reversion',
    icon: '🎪',
    entryConditions: ['Price < BB_Lower({bb_period}, {bb_std})', 'RSI(14) < {rsi_threshold}'],
    exitConditions: ['Price > BB_Upper({bb_period}, {bb_std})'],
    stopLoss: 'Price < BB_Lower({bb_period}, {bb_std}) * 0.98',
    parameters: {
      bb_period: { min: 15, max: 30, step: 5, default: 20, values: [15, 20, 25, 30] },
      bb_std: { min: 1.5, max: 2.5, step: 0.5, default: 2.0 },
      rsi_threshold: { min: 25, max: 40, step: 5, default: 35 }
    }
  },
  {
    id: 'ema_cross',
    name: 'EMA Crossover',
    description: 'Fast EMA crossing above slow EMA with trend confirmation',
    category: 'Trend Following',
    icon: '',
    entryConditions: ['EMA({fast_ema}) > EMA({slow_ema})', 'EMA({fast_ema})[1] <= EMA({slow_ema})[1]', 'Volume > SMA(Volume, 20)'],
    exitConditions: ['EMA({fast_ema}) < EMA({slow_ema})'],
    parameters: {
      fast_ema: { min: 8, max: 21, step: 1, default: 12, values: [8, 12, 21] },
      slow_ema: { min: 21, max: 50, step: 1, default: 26, values: [21, 26, 50] }
    }
  }
];

// Available indicators for autocomplete
const indicatorSuggestions = [
  'RSI(14)', 'RSI(21)', 'MACD', 'MACD_Signal', 'MACD_Histogram',
  'SMA(20)', 'SMA(50)', 'EMA(12)', 'EMA(26)', 'EMA(50)',
  'BB_Upper(20,2)', 'BB_Lower(20,2)', 'BB_Mid(20,2)',
  'Volume', 'Volume_SMA(20)', 'VWAP', 'ATR(14)',
  'Price', 'Open', 'High', 'Low', 'Close',
  'High(5)', 'Low(5)', 'High(20)', 'Low(20)'
];

export const StrategyWorkbench: React.FC<StrategyWorkbenchProps> = ({ isOpen, onClose, initialTemplate }) => {
  
  // Strategy building state
  const [selectedTemplate, setSelectedTemplate] = useState<StrategyTemplate | null>(null);
  const [entryConditions, setEntryConditions] = useState<TradingCondition[]>([]);
  const [exitConditions, setExitConditions] = useState<TradingCondition[]>([]);
  const [stopLoss, setStopLoss] = useState<string>('');
  const [takeProfit, setTakeProfit] = useState<string>('');
  const [trailingStopLoss, setTrailingStopLoss] = useState(false);
  const [trailingTakeProfit, setTrailingTakeProfit] = useState(false);
  // Removed optimization metric - will be determined at analysis step
  
  // Data universe
  const [dataUniverse, setDataUniverse] = useState<DataUniverse>({
    symbols: ['SPY'],
    timeframe: 'Daily',
    startDate: '2023-01-01',
    endDate: '2024-12-31',
    parameterSpace: {}
  });
  
  // Strategy search and selection
  const [strategySearch, setStrategySearch] = useState('');
  const [selectedStrategy, setSelectedStrategy] = useState<string | null>(null);
  
  // Backtesting state
  const [isBacktesting, setIsBacktesting] = useState(false);
  const [backtestResults, setBacktestResults] = useState<BacktestResults | null>(null);
  
  // UI state
  const [activeView, setActiveView] = useState<'builder' | 'results'>('builder');
  
  const [optimizationResults, setOptimizationResults] = useState<OptimizationResult[]>([]);
  const [isOptimizing, setIsOptimizing] = useState(false);

  const loadTemplate = (template: StrategyTemplate) => {
    setSelectedTemplate(template);
    
    // Load entry conditions
    const entryConditions = template.entryConditions.map((condition, index) => ({
      id: `entry_${index}`,
      type: 'entry' as const,
      condition,
      isValid: true
    }));
    setEntryConditions(entryConditions);
    
    // Load exit conditions
    const exitConditions = template.exitConditions.map((condition, index) => ({
      id: `exit_${index}`,
      type: 'exit' as const,
      condition,
      isValid: true
    }));
    setExitConditions(exitConditions);
    
    // Load stop/take profit
    setStopLoss(template.stopLoss || '');
    setTakeProfit(template.takeProfit || '');
    
    // Load parameter space into data universe
    setDataUniverse(prev => ({
      ...prev,
      parameterSpace: template.parameters
    }));
    
    setActiveView('builder');
  };

  // Load initial template when prop changes
  useEffect(() => {
    if (initialTemplate && isOpen) {
      const template = strategyTemplates.find(t => t.id === initialTemplate);
      if (template) {
        loadTemplate(template);
      }
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [initialTemplate, isOpen]);

  const addCondition = (type: 'entry' | 'exit') => {
    const newCondition: TradingCondition = {
      id: `${type}_${Date.now()}`,
      type,
      condition: '',
      isValid: false
    };
    
    if (type === 'entry') {
      setEntryConditions([...entryConditions, newCondition]);
    } else {
      setExitConditions([...exitConditions, newCondition]);
    }
  };

  const updateCondition = (id: string, condition: string) => {
    const isValid = condition.length > 0; // Simple validation for now
    
    setEntryConditions(prev => prev.map(cond => 
      cond.id === id ? { ...cond, condition, isValid } : cond
    ));
    setExitConditions(prev => prev.map(cond => 
      cond.id === id ? { ...cond, condition, isValid } : cond
    ));
  };

  const removeCondition = (id: string) => {
    setEntryConditions(prev => prev.filter(cond => cond.id !== id));
    setExitConditions(prev => prev.filter(cond => cond.id !== id));
  };

  const runBacktest = async () => {
    if (entryConditions.length === 0) return;
    
    setIsBacktesting(true);
    setActiveView('results');
    
    // Simulate backtesting with realistic delay
    await new Promise(resolve => setTimeout(resolve, 2500));
    
    // Mock backtesting results
    const mockResults: BacktestResults = {
      totalTrades: Math.floor(Math.random() * 150) + 25,
      winRate: Math.random() * 30 + 50, // 50-80%
      avgReturn: (Math.random() - 0.2) * 3, // -0.6% to 2.4%
      sharpeRatio: Math.random() * 2 + 0.5, // 0.5 to 2.5
      maxDrawdown: -(Math.random() * 15 + 5), // -5% to -20%
      profitFactor: Math.random() * 2 + 0.8, // 0.8 to 2.8
      trades: [] // Mock trade data
    };
    
    // Generate some mock trades
    for (let i = 0; i < Math.min(mockResults.totalTrades, 10); i++) {
      mockResults.trades.push({
        id: `trade_${i}`,
        timestamp: new Date(Date.now() - Math.random() * 365 * 24 * 60 * 60 * 1000).toISOString(),
        type: i % 2 === 0 ? 'BUY' : 'SELL',
        price: Math.random() * 400 + 100,
        quantity: Math.floor(Math.random() * 100) + 10,
        pnl: (Math.random() - 0.4) * 500
      });
    }
    
    setBacktestResults(mockResults);
    setIsBacktesting(false);
  };


  const getMetricColor = (metric: string, value: number) => {
    switch (metric) {
      case 'winRate':
        return value > 60 ? '#10b981' : value > 45 ? '#f59e0b' : '#ef4444';
      case 'sharpeRatio':
        return value > 1.5 ? '#10b981' : value > 1.0 ? '#f59e0b' : '#ef4444';
      case 'maxDrawdown':
        return value > -10 ? '#10b981' : value > -20 ? '#f59e0b' : '#ef4444';
      default:
        return '#6b7280';
    }
  };

  const calculateTotalCombinations = (parameterSpace: Record<string, ParameterRange>): number => {
    return Object.values(parameterSpace).reduce((total, param) => {
      if (param.values) {
        return total * param.values.length;
      } else {
        const count = Math.floor((param.max - param.min) / param.step) + 1;
        return total * count;
      }
    }, 1);
  };



  const runParameterOptimization = async () => {
    if (Object.keys(dataUniverse.parameterSpace).length === 0) return;
    
    setIsOptimizing(true);
    setActiveView('results');
    
    // Simulate parameter optimization with realistic delay
    await new Promise(resolve => setTimeout(resolve, 4000));
    
    // Generate mock optimization results
    const totalCombinations = calculateTotalCombinations(dataUniverse.parameterSpace);
    
    const results: OptimizationResult[] = [];
    for (let i = 0; i < Math.min(totalCombinations, 50); i++) {
      results.push({
        parameters: Object.entries(dataUniverse.parameterSpace).reduce((acc, [key, param]) => {
          if (param.values) {
            acc[key] = param.values[i % param.values.length];
          } else {
            const steps = Math.floor((param.max - param.min) / param.step) + 1;
            const value = param.min + (i % steps) * param.step;
            acc[key] = value;
          }
          return acc;
        }, {} as Record<string, number>),
        metrics: {
          sharpe: Math.random() * 3 + 0.2,
          calmar: Math.random() * 4 + 0.5,
          maxDrawdown: -(Math.random() * 20 + 2),
          totalReturn: (Math.random() - 0.1) * 50,
          winRate: Math.random() * 40 + 40
        },
        trades: Math.floor(Math.random() * 200) + 20,
        rank: i + 1
      });
    }
    
    // Sort by Sharpe ratio
    results.sort((a, b) => b.metrics.sharpe - a.metrics.sharpe);
    results.forEach((result, index) => result.rank = index + 1);
    
    setOptimizationResults(results);
    setIsOptimizing(false);
  };

  return (
    <div className={styles.workbenchContainer}>
      {/* Main Signal Analysis */}
      <main className={styles.strategyBuilder}>
        {/* Header */}
        <div className={styles.builderHeader}>
          <div className={styles.headerLeft}>
            <h2>Signal Analysis</h2>
            {selectedTemplate && (
              <span className={styles.templateBadge}>
                {selectedTemplate.icon} {selectedTemplate.name}
              </span>
            )}
          </div>
          <div className={styles.headerRight}>
            <div className={styles.viewToggle}>
              <button 
                className={`${styles.toggleBtn} ${activeView === 'builder' ? styles.active : ''}`}
                onClick={() => setActiveView('builder')}
              >
                Builder
              </button>
              <button 
                className={`${styles.toggleBtn} ${activeView === 'results' ? styles.active : ''}`}
                onClick={() => setActiveView('results')}
                disabled={!backtestResults}
              >
                Results
              </button>
            </div>
            <button className={styles.closeBtn} onClick={onClose}>×</button>
          </div>
        </div>

        {/* Main Content Area */}
        <div className={styles.builderContent}>
          {activeView === 'builder' ? (
            // Signal Analysis Interface
            <div className={styles.conditionBuilder}>
              {/* Data Universe - Elegant Search Context */}
              <div className={styles.dataUniverse}>
                <div className={styles.universeHeader}>
                  <div className={styles.universeTitle}>
                    <h3>Data Universe</h3>
                    <span className={styles.signalSpace}>
                      Exploring <strong>{(dataUniverse.symbols.length * calculateTotalCombinations(dataUniverse.parameterSpace)).toLocaleString()}</strong> signal combinations
                    </span>
                  </div>
                  <button 
                    className={styles.scanButton}
                    onClick={runParameterOptimization}
                    disabled={isOptimizing}
                  >
                    {isOptimizing ? (
                      <>
                        <svg className={styles.scanIcon} width="16" height="16" viewBox="0 0 16 16">
                          <circle cx="8" cy="8" r="6" stroke="currentColor" strokeWidth="2" fill="none" strokeDasharray="4 2">
                            <animateTransform attributeName="transform" type="rotate" from="0 8 8" to="360 8 8" dur="1s" repeatCount="indefinite"/>
                          </circle>
                        </svg>
                        Scanning...
                      </>
                    ) : (
                      <>
                        <svg className={styles.scanIcon} width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
                          <path d="M11 6a3 3 0 1 1-6 0 3 3 0 0 1 6 0z"/>
                          <path d="M0 8a8 8 0 1 1 16 0A8 8 0 0 1 0 8zm8-7a7 7 0 0 0-5.468 11.37C3.242 11.226 4.805 10 8 10s4.757 1.225 5.468 2.37A7 7 0 0 0 8 1z"/>
                        </svg>
                        Scan Universe
                      </>
                    )}
                  </button>
                </div>
                
                <div className={styles.universeCompact}>
                  {/* Assets Selection */}
                  <div className={styles.universeSection}>
                    <label className={styles.sectionLabel}>Assets</label>
                    <div className={styles.assetPills}>
                      {['SPY', 'QQQ', 'IWM'].map(ticker => (
                        <button
                          key={ticker}
                          className={`${styles.assetPill} ${dataUniverse.symbols.includes(ticker) ? styles.active : ''}`}
                          onClick={() => {
                            const newSymbols = dataUniverse.symbols.includes(ticker)
                              ? dataUniverse.symbols.filter(s => s !== ticker)
                              : [...dataUniverse.symbols, ticker];
                            setDataUniverse({...dataUniverse, symbols: newSymbols});
                          }}
                        >
                          {ticker}
                        </button>
                      ))}
                      <button className={styles.assetPillAdd}>
                        Custom
                      </button>
                    </div>
                  </div>
                  
                  {/* Timeframe Selection */}
                  <div className={styles.universeSection}>
                    <label className={styles.sectionLabel}>Timeframe</label>
                    <div className={styles.timeframeButtons}>
                      {[
                        { value: '1m', label: '1min' },
                        { value: '5m', label: '5min' },
                        { value: '15m', label: '15min' },
                        { value: '1h', label: '1hr' },
                        { value: 'Daily', label: 'Daily' }
                      ].map(tf => (
                        <button
                          key={tf.value}
                          className={`${styles.timeframeBtn} ${dataUniverse.timeframe === tf.value ? styles.active : ''}`}
                          onClick={() => setDataUniverse({...dataUniverse, timeframe: tf.value})}
                        >
                          {tf.label}
                        </button>
                      ))}
                    </div>
                  </div>
                  
                  {/* Date Range Selection */}
                  <div className={styles.universeSection}>
                    <label className={styles.sectionLabel}>Date Range</label>
                    <div className={styles.dateInputs}>
                      <input
                        type="date"
                        className={styles.dateInput}
                        value={dataUniverse.startDate}
                        onChange={(e) => setDataUniverse({...dataUniverse, startDate: e.target.value})}
                        placeholder="Start Date"
                      />
                      <span className={styles.dateSeparator}>to</span>
                      <input
                        type="date"
                        className={styles.dateInput}
                        value={dataUniverse.endDate}
                        onChange={(e) => setDataUniverse({...dataUniverse, endDate: e.target.value})}
                        placeholder="End Date"
                      />
                    </div>
                  </div>
                </div>
              </div>

              {/* Strategy/Logic/Signal Configuration - Separate Cell */}
              <div className={styles.strategyCell}>
                <div className={styles.strategyCellHeader}>
                  <div className={styles.strategyCellTitle}>
                    <h3>Strategy & Signals</h3>
                    <span className={styles.strategyDescription}>
                      Define your trading logic and parameter search space
                    </span>
                  </div>
                </div>
                
                <div className={styles.strategyLayout}>
                  <div className={styles.strategyLeft}>
                  </div>
                  
                  {/* Strategies & Parameters on Right Side */}
                  <div className={styles.universeRight}>
                    <div className={styles.strategySection}>
                      <label className={styles.sectionLabel}>Strategies</label>
                      <input
                        type="text"
                        className={styles.strategySearch}
                        placeholder="Search strategies..."
                        value={strategySearch}
                        onChange={(e) => setStrategySearch(e.target.value)}
                      />
                      <div className={styles.strategyList}>
                        {/* Available Strategies */}
                        {[
                          { name: 'RSI Mean Reversion', type: 'Technical' },
                          { name: 'MACD Crossover', type: 'Momentum' },
                          { name: 'Bollinger Breakout', type: 'Volatility' },
                          { name: 'Volume Profile', type: 'Volume' },
                          { name: 'EMA Cross', type: 'Trend' },
                          { name: 'Support Resistance', type: 'Price Action' },
                          { name: 'Fibonacci Retracement', type: 'Technical' },
                          { name: 'Stochastic Oscillator', type: 'Momentum' }
                        ]
                          .filter(s => s.name.toLowerCase().includes(strategySearch.toLowerCase()))
                          .map(strategy => (
                            <div 
                              key={strategy.name}
                              className={`${styles.strategyItem} ${selectedStrategy === strategy.name ? styles.active : ''}`}
                              onClick={() => {
                                setSelectedStrategy(strategy.name);
                                // Set default parameters for the strategy
                                if (strategy.name === 'RSI Mean Reversion') {
                                  setDataUniverse({
                                    ...dataUniverse,
                                    parameterSpace: {
                                      rsi_period: { min: 10, max: 21, step: 1 },
                                      rsi_oversold: { min: 20, max: 35, step: 5 },
                                      rsi_overbought: { min: 65, max: 80, step: 5 }
                                    }
                                  });
                                } else if (strategy.name === 'EMA Cross') {
                                  setDataUniverse({
                                    ...dataUniverse,
                                    parameterSpace: {
                                      fast_period: { min: 5, max: 20, step: 1 },
                                      slow_period: { min: 20, max: 50, step: 5 }
                                    }
                                  });
                                }
                              }}
                            >
                              <span className={styles.strategyName}>{strategy.name}</span>
                              <span className={styles.strategyType}>{strategy.type}</span>
                            </div>
                          ))}
                      </div>
                    </div>
                    
                    {/* Parameters Input */}
                    {selectedStrategy && Object.keys(dataUniverse.parameterSpace).length > 0 && (
                      <div className={styles.parametersSection}>
                        <label className={styles.sectionLabel}>Parameter Ranges</label>
                        <div className={styles.parameterInputs}>
                          {Object.entries(dataUniverse.parameterSpace).map(([key, param]) => (
                            <div key={key} className={styles.parameterInputRow}>
                              <label className={styles.paramLabel}>
                                {key.replace(/_/g, ' ').replace(/\b\w/g, c => c.toUpperCase())}
                              </label>
                              <div className={styles.paramInputs}>
                                <input
                                  type="number"
                                  className={styles.paramInput}
                                  placeholder="Min"
                                  value={param.min}
                                  onChange={(e) => {
                                    const newSpace = { ...dataUniverse.parameterSpace };
                                    newSpace[key] = { ...param, min: parseFloat(e.target.value) || 0 };
                                    setDataUniverse({ ...dataUniverse, parameterSpace: newSpace });
                                  }}
                                />
                                <span className={styles.paramSep}>-</span>
                                <input
                                  type="number"
                                  className={styles.paramInput}
                                  placeholder="Max"
                                  value={param.max}
                                  onChange={(e) => {
                                    const newSpace = { ...dataUniverse.parameterSpace };
                                    newSpace[key] = { ...param, max: parseFloat(e.target.value) || 0 };
                                    setDataUniverse({ ...dataUniverse, parameterSpace: newSpace });
                                  }}
                                />
                                <span className={styles.paramSep}>step</span>
                                <input
                                  type="number"
                                  className={styles.paramInput}
                                  placeholder="Step"
                                  value={param.step}
                                  onChange={(e) => {
                                    const newSpace = { ...dataUniverse.parameterSpace };
                                    newSpace[key] = { ...param, step: parseFloat(e.target.value) || 1 };
                                    setDataUniverse({ ...dataUniverse, parameterSpace: newSpace });
                                  }}
                                />
                              </div>
                            </div>
                          ))}
                        </div>
                      </div>
                    )}
                  </div>
                </div>
              </div>

              {/* Entry Conditions */}
              <div className={styles.conditionSection}>
                <div className={styles.sectionHeader}>
                  <h3>Entry Conditions</h3>
                  <button 
                    className={styles.addConditionBtn}
                    onClick={() => addCondition('entry')}
                  >
                    + Add Condition
                  </button>
                </div>
                
                <div className={styles.conditionList}>
                  {entryConditions.length === 0 ? (
                    <div className={styles.emptyState}>
                      <span>No entry conditions defined</span>
                      <span className={styles.hint}>Click a template or add conditions manually</span>
                    </div>
                  ) : (
                    entryConditions.map(condition => (
                      <div key={condition.id} className={styles.conditionItem}>
                        <div className={styles.conditionInput}>
                          <span className={styles.conditionPrefix}>WHEN</span>
                          <input
                            type="text"
                            value={condition.condition}
                            onChange={(e) => updateCondition(condition.id, e.target.value)}
                            placeholder="RSI(14) < 30"
                            className={`${styles.conditionField} ${condition.isValid ? styles.valid : styles.invalid}`}
                          />
                          <button 
                            className={styles.removeConditionBtn}
                            onClick={() => removeCondition(condition.id)}
                          >
                            ×
                          </button>
                        </div>
                      </div>
                    ))
                  )}
                </div>
              </div>
              
              {/* Exit Conditions */}
              <div className={styles.conditionSection}>
                <div className={styles.sectionHeader}>
                  <h3>Exit Conditions</h3>
                  <button 
                    className={styles.addConditionBtn}
                    onClick={() => addCondition('exit')}
                  >
                    + Add Condition
                  </button>
                </div>
                
                <div className={styles.conditionList}>
                  {exitConditions.length === 0 ? (
                    <div className={styles.emptyState}>
                      <span>No exit conditions defined</span>
                      <span className={styles.hint}>Define when to close positions</span>
                    </div>
                  ) : (
                    exitConditions.map(condition => (
                      <div key={condition.id} className={styles.conditionItem}>
                        <div className={styles.conditionInput}>
                          <span className={styles.conditionPrefix}>WHEN</span>
                          <input
                            type="text"
                            value={condition.condition}
                            onChange={(e) => updateCondition(condition.id, e.target.value)}
                            placeholder="RSI(14) > 70"
                            className={`${styles.conditionField} ${condition.isValid ? styles.valid : styles.invalid}`}
                          />
                          <button 
                            className={styles.removeConditionBtn}
                            onClick={() => removeCondition(condition.id)}
                          >
                            ×
                          </button>
                        </div>
                      </div>
                    ))
                  )}
                </div>
              </div>
              
              {/* Risk Management */}
              <div className={styles.conditionSection}>
                <div className={styles.sectionHeader}>
                  <h3>⚠️ Risk Management</h3>
                </div>
                
                <div className={styles.riskControls}>
                  <div className={styles.riskItem}>
                    <div className={styles.riskHeader}>
                      <label>Stop Loss</label>
                      <div className={styles.riskToggle}>
                        <button 
                          className={`${styles.riskTypeBtn} ${!trailingStopLoss ? styles.active : ''}`}
                          onClick={() => setTrailingStopLoss(false)}
                        >
                          Fixed
                        </button>
                        <button 
                          className={`${styles.riskTypeBtn} ${trailingStopLoss ? styles.active : ''}`}
                          onClick={() => setTrailingStopLoss(true)}
                        >
                          Trailing
                        </button>
                      </div>
                    </div>
                    <input
                      type="text"
                      value={stopLoss}
                      onChange={(e) => setStopLoss(e.target.value)}
                      placeholder={trailingStopLoss ? "ATR(14) * 2" : "Price < Entry * 0.95"}
                      className={styles.riskField}
                    />
                  </div>
                  <div className={styles.riskItem}>
                    <div className={styles.riskHeader}>
                      <label>Take Profit</label>
                      <div className={styles.riskToggle}>
                        <button 
                          className={`${styles.riskTypeBtn} ${!trailingTakeProfit ? styles.active : ''}`}
                          onClick={() => setTrailingTakeProfit(false)}
                        >
                          Fixed
                        </button>
                        <button 
                          className={`${styles.riskTypeBtn} ${trailingTakeProfit ? styles.active : ''}`}
                          onClick={() => setTrailingTakeProfit(true)}
                        >
                          Trailing
                        </button>
                      </div>
                    </div>
                    <input
                      type="text"
                      value={takeProfit}
                      onChange={(e) => setTakeProfit(e.target.value)}
                      placeholder={trailingTakeProfit ? "Price > High(20)" : "Price > Entry * 1.10"}
                      className={styles.riskField}
                    />
                  </div>
                </div>
              </div>
            </div>
          ) : activeView === 'results' && (backtestResults || optimizationResults.length > 0) ? (
            // Results View
            <div className={styles.resultsView}>
              {isBacktesting || isOptimizing ? (
                <div className={styles.loadingState}>
                  <div className={styles.spinner}></div>
                  <span>{isOptimizing ? 'Analyzing signals across parameter space...' : 'Running backtest...'}</span>
                  {isOptimizing && (
                    <span className={styles.loadingHint}>
                      Testing {calculateTotalCombinations(dataUniverse.parameterSpace)} parameter combinations
                    </span>
                  )}
                </div>
              ) : optimizationResults.length > 0 ? (
                // Optimization Results
                <div className={styles.optimizationResults}>
                  <div className={styles.resultsHeader}>
                    <h3>🔬 Parameter Optimization Results</h3>
                    <div className={styles.resultsActions}>
                      <button 
                        className={styles.exportBtn}
                        onClick={() => console.log('Export results')}
                      >
                        📊 Export
                      </button>
                      <button 
                        className={styles.advancedAnalysisBtn}
                        onClick={() => console.log('Research Lab - Coming Soon')}
                      >
                        🧪 Research Lab
                      </button>
                    </div>
                  </div>

                  <div className={styles.optimizationSummary}>
                    <div className={styles.summaryItem}>
                      <span className={styles.summaryLabel}>Combinations Tested:</span>
                      <span className={styles.summaryValue}>{optimizationResults.length}</span>
                    </div>
                    <div className={styles.summaryItem}>
                      <span className={styles.summaryLabel}>Best Sharpe Ratio:</span>
                      <span className={styles.summaryValue} style={{ color: getMetricColor('sharpeRatio', optimizationResults[0]?.metrics.sharpe || 0) }}>
                        {optimizationResults[0]?.metrics.sharpe.toFixed(2) || 'N/A'}
                      </span>
                    </div>
                    <div className={styles.summaryItem}>
                      <span className={styles.summaryLabel}>Profitable Strategies:</span>
                      <span className={styles.summaryValue}>
                        {optimizationResults.filter(r => r.metrics.totalReturn > 0).length} / {optimizationResults.length}
                      </span>
                    </div>
                  </div>

                  <div className={styles.optimizationTable}>
                    <div className={styles.tableHeader}>
                      <div className={styles.headerCell}>Rank</div>
                      <div className={styles.headerCell}>Parameters</div>
                      <div className={styles.headerCell}>Sharpe</div>
                      <div className={styles.headerCell}>Return</div>
                      <div className={styles.headerCell}>Max DD</div>
                      <div className={styles.headerCell}>Win Rate</div>
                      <div className={styles.headerCell}>Trades</div>
                      <div className={styles.headerCell}>Actions</div>
                    </div>
                    
                    <div className={styles.tableBody}>
                      {optimizationResults.slice(0, 20).map((result, index) => (
                        <div key={index} className={`${styles.tableRow} ${index === 0 ? styles.bestResult : ''}`}>
                          <div className={styles.tableCell}>
                            {index === 0 && <span className={styles.crownIcon} style={{color: '#FFD700', fontWeight: 'bold'}}>#1</span>}
                            #{result.rank}
                          </div>
                          <div className={styles.tableCell}>
                            <div className={styles.parametersList}>
                              {Object.entries(result.parameters).map(([param, value]) => (
                                <span key={param} className={styles.paramTag}>
                                  {param}: {value}
                                </span>
                              ))}
                            </div>
                          </div>
                          <div className={styles.tableCell}>
                            <span style={{ color: getMetricColor('sharpeRatio', result.metrics.sharpe) }}>
                              {result.metrics.sharpe.toFixed(2)}
                            </span>
                          </div>
                          <div className={styles.tableCell}>
                            <span className={result.metrics.totalReturn > 0 ? styles.positive : styles.negative}>
                              {result.metrics.totalReturn > 0 ? '+' : ''}{result.metrics.totalReturn.toFixed(1)}%
                            </span>
                          </div>
                          <div className={styles.tableCell}>
                            <span style={{ color: getMetricColor('maxDrawdown', result.metrics.maxDrawdown) }}>
                              {result.metrics.maxDrawdown.toFixed(1)}%
                            </span>
                          </div>
                          <div className={styles.tableCell}>
                            <span style={{ color: getMetricColor('winRate', result.metrics.winRate) }}>
                              {result.metrics.winRate.toFixed(0)}%
                            </span>
                          </div>
                          <div className={styles.tableCell}>{result.trades}</div>
                          <div className={styles.tableCell}>
                            <button 
                              className={styles.selectBtn}
                              onClick={() => {
                                // Load this parameter set back into the data universe
                                const newParameterSpace = { ...dataUniverse.parameterSpace };
                                Object.entries(result.parameters).forEach(([param, value]) => {
                                  if (newParameterSpace[param]) {
                                    // Set single value as default
                                    newParameterSpace[param] = { 
                                      ...newParameterSpace[param],
                                      default: value,
                                      min: value,
                                      max: value,
                                      step: 0.1
                                    };
                                  }
                                });
                                setDataUniverse({ ...dataUniverse, parameterSpace: newParameterSpace });
                              }}
                            >
                              Use
                            </button>
                          </div>
                        </div>
                      ))}
                    </div>
                  </div>
                </div>
              ) : backtestResults ? (
                // Single Backtest Results
                <div className={styles.singleBacktestResults}>
                  <div className={styles.resultsGrid}>
                    {/* Performance Metrics */}
                    <div className={styles.metricsPanel}>
                    <div className={styles.panelHeader}>
                      <h3>Performance Metrics</h3>
                    </div>
                    <div className={styles.metricsGrid}>
                      <div className={styles.metric}>
                        <span 
                          className={styles.metricValue}
                          style={{ color: getMetricColor('winRate', backtestResults.winRate) }}
                        >
                          {backtestResults.winRate.toFixed(1)}%
                        </span>
                        <span className={styles.metricLabel}>Win Rate</span>
                      </div>
                      <div className={styles.metric}>
                        <span 
                          className={styles.metricValue}
                          style={{ color: getMetricColor('sharpeRatio', backtestResults.sharpeRatio) }}
                        >
                          {backtestResults.sharpeRatio.toFixed(2)}
                        </span>
                        <span className={styles.metricLabel}>Sharpe Ratio</span>
                      </div>
                      <div className={styles.metric}>
                        <span className={styles.metricValue}>
                          {backtestResults.avgReturn > 0 ? '+' : ''}{backtestResults.avgReturn.toFixed(2)}%
                        </span>
                        <span className={styles.metricLabel}>Avg Return</span>
                      </div>
                      <div className={styles.metric}>
                        <span 
                          className={styles.metricValue}
                          style={{ color: getMetricColor('maxDrawdown', backtestResults.maxDrawdown) }}
                        >
                          {backtestResults.maxDrawdown.toFixed(1)}%
                        </span>
                        <span className={styles.metricLabel}>Max Drawdown</span>
                      </div>
                      <div className={styles.metric}>
                        <span className={styles.metricValue}>{backtestResults.totalTrades}</span>
                        <span className={styles.metricLabel}>Total Trades</span>
                      </div>
                      <div className={styles.metric}>
                        <span className={styles.metricValue}>{backtestResults.profitFactor.toFixed(2)}</span>
                        <span className={styles.metricLabel}>Profit Factor</span>
                      </div>
                    </div>
                  </div>
                  
                  {/* Recent Trades */}
                  <div className={styles.tradesPanel}>
                    <div className={styles.panelHeader}>
                      <h3>Recent Trades</h3>
                    </div>
                    <div className={styles.tradesList}>
                      {backtestResults.trades.slice(0, 5).map(trade => (
                        <div key={trade.id} className={styles.tradeItem}>
                          <div className={styles.tradeInfo}>
                            <span className={`${styles.tradeType} ${styles[trade.type.toLowerCase()]}`}>
                              {trade.type}
                            </span>
                            <span className={styles.tradePrice}>${trade.price.toFixed(2)}</span>
                            <span className={styles.tradeDate}>
                              {new Date(trade.timestamp).toLocaleDateString()}
                            </span>
                          </div>
                          {trade.pnl !== undefined && (
                            <span className={`${styles.tradePnl} ${trade.pnl > 0 ? styles.profit : styles.loss}`}>
                              {trade.pnl > 0 ? '+' : ''}${trade.pnl.toFixed(2)}
                            </span>
                          )}
                        </div>
                      ))}
                    </div>
                  </div>
                  </div>
                </div>
              ) : null}
            </div>
          ) : (
            // Welcome State
            <div className={styles.welcomeState}>
              <div className={styles.welcomeContent}>
                <h2>Welcome to Signal Analysis</h2>
                <p>Define your search context and analyze signals across multiple dimensions.</p>
                
                <div className={styles.quickStart}>
                  <h3>Quick Start</h3>
                  <div className={styles.quickStartOptions}>
                    <button 
                      className={styles.quickStartBtn}
                      onClick={() => setActiveView('templates')}
                    >
                      Choose Template
                    </button>
                    <button 
                      className={styles.quickStartBtn}
                      onClick={() => {
                        setActiveView('builder');
                        addCondition('entry');
                      }}
                    >
                      Build from Scratch
                    </button>
                  </div>
                </div>
              </div>
            </div>
          )}
        </div>
      </main>
    </div>
  );
};