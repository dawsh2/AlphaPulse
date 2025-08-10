import React, { useState, useEffect } from 'react';
import styles from './StrategyWorkbench.module.css';
import SignalAnalysisPanel from './SignalAnalysisPanel';

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
  const [currentStrategyParams, setCurrentStrategyParams] = useState<Record<string, any>>({});
  const [addedStrategies, setAddedStrategies] = useState<Array<{
    id: string;
    name: string;
    type: string;
    parameters: Record<string, any>;
  }>>([]);
  
  // Backtesting state
  const [isBacktesting, setIsBacktesting] = useState(false);
  const [backtestResults, setBacktestResults] = useState<BacktestResults | null>(null);
  
  // UI state
  const [activeView, setActiveView] = useState<'builder' | 'results'>('builder');
  
  const [optimizationResults, setOptimizationResults] = useState<OptimizationResult[]>([]);
  const [isOptimizing, setIsOptimizing] = useState(false);
  const [showAiAnalysis, setShowAiAnalysis] = useState(false);

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
        {/* Main Content Area */}
        <div className={styles.builderContent}>
          {/* Signal Analysis Interface */}
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
                  <div className={styles.universeLeft}>
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
                  </div>
                  
                  {/* Date Range on Right Side */}
                  <div className={styles.universeRight}>
                    <div className={styles.dateRangeSection}>
                      <label className={styles.sectionLabel}>Date Range</label>
                      <div className={styles.dateInputsVertical}>
                        <div className={styles.dateRow}>
                          <label className={styles.dateLabel}>Begin</label>
                          <input
                            type="date"
                            className={styles.dateInput}
                            value={dataUniverse.startDate}
                            onChange={(e) => setDataUniverse({...dataUniverse, startDate: e.target.value})}
                          />
                        </div>
                        <div className={styles.dateRow}>
                          <label className={styles.dateLabel}>End</label>
                          <input
                            type="date"
                            className={styles.dateInput}
                            value={dataUniverse.endDate}
                            onChange={(e) => setDataUniverse({...dataUniverse, endDate: e.target.value})}
                          />
                        </div>
                      </div>
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
                  {/* Strategies List on Left Side */}
                  <div className={styles.strategyLeft}>
                    <div className={styles.strategySection}>
                      <label className={styles.sectionLabel}>Strategies</label>
                      <input
                        type="text"
                        className={styles.strategySearch}
                        placeholder="Search strategies..."
                        value={strategySearch}
                        onChange={(e) => setStrategySearch(e.target.value)}
                      />
                      <div className={styles.strategyListSplit}>
                        {/* Available Strategies - Split into two columns */}
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
                                let params = {};
                                
                                switch(strategy.name) {
                                  case 'RSI Mean Reversion':
                                    params = {
                                      rsi_period: { min: 10, max: 21, step: 1 },
                                      rsi_oversold: { min: 20, max: 35, step: 5 },
                                      rsi_overbought: { min: 65, max: 80, step: 5 }
                                    };
                                    break;
                                  case 'MACD Crossover':
                                    params = {
                                      fast_period: { min: 8, max: 15, step: 1 },
                                      slow_period: { min: 20, max: 30, step: 2 },
                                      signal_period: { min: 7, max: 12, step: 1 }
                                    };
                                    break;
                                  case 'Bollinger Breakout':
                                    params = {
                                      bb_period: { min: 15, max: 25, step: 5 },
                                      bb_std: { min: 1.5, max: 3.0, step: 0.5 },
                                      breakout_threshold: { min: 0.5, max: 2.0, step: 0.25 }
                                    };
                                    break;
                                  case 'Volume Profile':
                                    params = {
                                      volume_ma_period: { min: 10, max: 30, step: 5 },
                                      volume_multiplier: { min: 1.5, max: 3.0, step: 0.5 },
                                      price_range: { min: 0.01, max: 0.05, step: 0.01 }
                                    };
                                    break;
                                  case 'EMA Cross':
                                    params = {
                                      fast_ema: { min: 5, max: 20, step: 1 },
                                      slow_ema: { min: 20, max: 50, step: 5 },
                                      confirmation_bars: { min: 1, max: 3, step: 1 }
                                    };
                                    break;
                                  case 'Support Resistance':
                                    params = {
                                      lookback_period: { min: 10, max: 50, step: 10 },
                                      touch_threshold: { min: 0.001, max: 0.01, step: 0.001 },
                                      min_touches: { min: 2, max: 5, step: 1 }
                                    };
                                    break;
                                  case 'Fibonacci Retracement':
                                    params = {
                                      swing_period: { min: 10, max: 30, step: 5 },
                                      fib_level_1: { min: 0.236, max: 0.382, step: 0.146 },
                                      fib_level_2: { min: 0.5, max: 0.618, step: 0.118 }
                                    };
                                    break;
                                  case 'Stochastic Oscillator':
                                    params = {
                                      k_period: { min: 10, max: 20, step: 2 },
                                      d_period: { min: 3, max: 5, step: 1 },
                                      oversold_level: { min: 15, max: 30, step: 5 },
                                      overbought_level: { min: 70, max: 85, step: 5 }
                                    };
                                    break;
                                  default:
                                    params = {
                                      custom_param_1: { min: 1, max: 100, step: 1 },
                                      custom_param_2: { min: 0, max: 1, step: 0.1 }
                                    };
                                }
                                
                                setDataUniverse({
                                  ...dataUniverse,
                                  parameterSpace: params
                                });
                                setCurrentStrategyParams(params);
                              }}
                            >
                              <span className={styles.strategyName}>{strategy.name}</span>
                            </div>
                          ))}
                      </div>
                    </div>
                  </div>
                  
                  {/* Parameters on Right Side */}
                  <div className={styles.strategyRight}>
                    {selectedStrategy && Object.keys(dataUniverse.parameterSpace).length > 0 ? (
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
                                    setCurrentStrategyParams(newSpace);
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
                                    setCurrentStrategyParams(newSpace);
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
                                    setCurrentStrategyParams(newSpace);
                                  }}
                                />
                              </div>
                            </div>
                          ))}
                        </div>
                        <button
                          className={styles.addStrategyBtn}
                          onClick={() => {
                            const strategyType = [
                              { name: 'RSI Mean Reversion', type: 'Technical' },
                              { name: 'MACD Crossover', type: 'Momentum' },
                              { name: 'Bollinger Breakout', type: 'Volatility' },
                              { name: 'Volume Profile', type: 'Volume' },
                              { name: 'EMA Cross', type: 'Trend' },
                              { name: 'Support Resistance', type: 'Price Action' },
                              { name: 'Fibonacci Retracement', type: 'Technical' },
                              { name: 'Stochastic Oscillator', type: 'Momentum' }
                            ].find(s => s.name === selectedStrategy)?.type || 'Custom';
                            
                            setAddedStrategies([...addedStrategies, {
                              id: `${selectedStrategy}_${Date.now()}`,
                              name: selectedStrategy || '',
                              type: strategyType,
                              parameters: currentStrategyParams
                            }]);
                            setSelectedStrategy(null);
                            setCurrentStrategyParams({});
                            setDataUniverse({ ...dataUniverse, parameterSpace: {} });
                          }}
                        >
                          Add Strategy
                        </button>
                      </div>
                    ) : (
                      <div className={styles.parametersPlaceholder}>
                        <p>Select a strategy to configure parameters</p>
                      </div>
                    )}
                  </div>
                </div>
                
                {/* Added Strategies List */}
                {addedStrategies.length > 0 && (
                  <div className={styles.addedStrategiesSection}>
                    <div className={styles.addedStrategiesHeader}>
                      <h4>Active Strategies ({addedStrategies.length})</h4>
                    </div>
                    <div className={styles.addedStrategiesList}>
                      {addedStrategies.map((strategy) => (
                        <div key={strategy.id} className={styles.addedStrategyItem}>
                          <div className={styles.addedStrategyInfo}>
                            <span className={styles.addedStrategyName}>{strategy.name}</span>
                          </div>
                          <div className={styles.addedStrategyParams}>
                            {Object.entries(strategy.parameters).map(([key, value]: [string, any]) => (
                              <span key={key} className={styles.paramChip}>
                                {key}: {value.min}-{value.max}
                              </span>
                            ))}
                          </div>
                          <button
                            className={styles.removeStrategyBtn}
                            onClick={() => {
                              setAddedStrategies(addedStrategies.filter(s => s.id !== strategy.id));
                            }}
                          >
                            ×
                          </button>
                        </div>
                      ))}
                    </div>
                  </div>
                )}
              </div>
              
              {/* Signal Analysis Panel - Shows when strategies are added */}
              {addedStrategies.length > 0 && (
                <div className={styles.signalAnalysisSection}>
                  <SignalAnalysisPanel 
                    dataUniverse={dataUniverse}
                    strategies={addedStrategies}
                  />
                </div>
              )}

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
              
              {/* AI Analysis Section - shown when backtest results are available */}
              {backtestResults && (
                <div className={styles.conditionSection}>
                  <div className={styles.sectionHeader}>
                    <h3>
                      <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" style={{ display: 'inline-block', verticalAlign: 'middle', marginRight: '8px' }}>
                        {/* Computerized brain icon */}
                        <path d="M9.5 2A3.5 3.5 0 0 0 6 5.5c0 2.3 2.5 3.3 2.5 5.5v1"/>
                        <path d="M14.5 2A3.5 3.5 0 0 1 18 5.5c0 2.3-2.5 3.3-2.5 5.5v1"/>
                        <path d="M12 2v10"/>
                        <circle cx="12" cy="14" r="2"/>
                        <path d="M7 14H5M19 14h-2M12 16v2"/>
                        <circle cx="7" cy="14" r="1"/>
                        <circle cx="17" cy="14" r="1"/>
                        <circle cx="12" cy="19" r="1"/>
                      </svg>
                      AI Analysis
                    </h3>
                    <button
                      className={styles.toggleAnalysisBtn}
                      onClick={() => setShowAiAnalysis(!showAiAnalysis)}
                    >
                      {showAiAnalysis ? 'Hide' : 'Show'} Analysis
                    </button>
                  </div>
                  
                  {showAiAnalysis && (
                    <div className={styles.aiAnalysisContent}>
                      <div className={styles.backtestMetrics}>
                        <div className={styles.metricItem}>
                          <span className={styles.metricLabel}>Total Trades</span>
                          <span className={styles.metricValue}>{backtestResults.totalTrades}</span>
                        </div>
                        <div className={styles.metricItem}>
                          <span className={styles.metricLabel}>Win Rate</span>
                          <span className={styles.metricValue}>{backtestResults.winRate.toFixed(1)}%</span>
                        </div>
                        <div className={styles.metricItem}>
                          <span className={styles.metricLabel}>Avg Return</span>
                          <span className={styles.metricValue}>{backtestResults.avgReturn.toFixed(2)}%</span>
                        </div>
                        <div className={styles.metricItem}>
                          <span className={styles.metricLabel}>Sharpe Ratio</span>
                          <span className={styles.metricValue}>{backtestResults.sharpeRatio.toFixed(2)}</span>
                        </div>
                        <div className={styles.metricItem}>
                          <span className={styles.metricLabel}>Max Drawdown</span>
                          <span className={styles.metricValue}>{backtestResults.maxDrawdown.toFixed(1)}%</span>
                        </div>
                      </div>
                      
                      <div className={styles.aiInsights}>
                        <div className={styles.insightItem}>
                          <strong>📊 Pattern Analysis:</strong> Your strategy shows strong performance during trending markets. Consider adding a trend filter to improve results.
                        </div>
                        <div className={styles.insightItem}>
                          <strong>💡 Optimization Suggestion:</strong> The win rate of {backtestResults.winRate.toFixed(1)}% could be improved by tightening entry conditions. Expected improvement: +5-10%.
                        </div>
                        <div className={styles.insightItem}>
                          <strong>⚠️ Risk Alert:</strong> Maximum drawdown of {backtestResults.maxDrawdown.toFixed(1)}% exceeds typical risk tolerance. Consider implementing dynamic position sizing.
                        </div>
                        <div className={styles.insightItem}>
                          <strong>🎯 Next Steps:</strong> Run parameter optimization on RSI period (test 10-20) and stop loss percentage (test 2-5%).
                        </div>
                      </div>
                      
                      <div className={styles.aiActions}>
                        <button className={styles.aiActionBtn}>
                          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                            <path d="M12 2v20M2 12h20"/>
                          </svg>
                          Generate Improved Strategy
                        </button>
                        <button className={styles.aiActionBtn}>
                          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                            <path d="M22 12h-4l-3 9L9 3l-3 9H2"/>
                          </svg>
                          Deep Dive Analysis
                        </button>
                        <button className={styles.aiActionBtn}>
                          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                            <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>
                            <polyline points="14 2 14 8 20 8"/>
                            <line x1="16" y1="13" x2="8" y2="13"/>
                            <line x1="16" y1="17" x2="8" y2="17"/>
                            <polyline points="10 9 9 9 8 9"/>
                          </svg>
                          Export Report
                        </button>
                      </div>
                    </div>
                  )}
                </div>
              )}
            </div>
        </div>
      </main>
    </div>
  );
};
