import React, { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
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
  parameters: Record<string, { value: number; min: number; max: number; step: number }>;
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
}

// Strategy templates with pre-built conditions
const strategyTemplates: StrategyTemplate[] = [
  {
    id: 'oversold_bounce',
    name: 'Oversold Bounce',
    description: 'Buy when RSI oversold with volume confirmation',
    category: 'Mean Reversion',
    icon: '🎯',
    entryConditions: ['RSI(14) < 30', 'Volume > SMA(Volume, 20) * 1.5'],
    exitConditions: ['RSI(14) > 70'],
    stopLoss: 'Price < Entry * 0.95',
    parameters: {
      rsi_period: { value: 14, min: 10, max: 21, step: 1 },
      rsi_oversold: { value: 30, min: 20, max: 35, step: 1 },
      volume_multiplier: { value: 1.5, min: 1.2, max: 3.0, step: 0.1 }
    }
  },
  {
    id: 'volume_breakout',
    name: 'Volume Breakout',
    description: 'High volume breakout above resistance',
    category: 'Momentum',
    icon: '💥',
    entryConditions: ['Price > SMA(20)', 'Volume > SMA(Volume, 20) * 2.0', 'Price > High(5) * 1.01'],
    exitConditions: ['Price < SMA(20)'],
    takeProfit: 'Price > Entry * 1.08',
    parameters: {
      sma_period: { value: 20, min: 10, max: 50, step: 1 },
      volume_threshold: { value: 2.0, min: 1.5, max: 4.0, step: 0.1 },
      breakout_pct: { value: 1.01, min: 1.005, max: 1.02, step: 0.001 }
    }
  },
  {
    id: 'bollinger_reversion',
    name: 'Bollinger Mean Reversion',
    description: 'Trade bounces off Bollinger Bands',
    category: 'Mean Reversion',
    icon: '🎪',
    entryConditions: ['Price < BB_Lower(20, 2)', 'RSI(14) < 35'],
    exitConditions: ['Price > BB_Upper(20, 2)'],
    stopLoss: 'Price < BB_Lower(20, 2) * 0.98',
    parameters: {
      bb_period: { value: 20, min: 15, max: 30, step: 1 },
      bb_std: { value: 2, min: 1.5, max: 2.5, step: 0.1 },
      rsi_threshold: { value: 35, min: 25, max: 40, step: 1 }
    }
  },
  {
    id: 'ema_cross',
    name: 'EMA Crossover',
    description: 'Fast EMA crossing above slow EMA with trend confirmation',
    category: 'Trend Following',
    icon: '📈',
    entryConditions: ['EMA(12) > EMA(26)', 'EMA(12)[1] <= EMA(26)[1]', 'Volume > SMA(Volume, 20)'],
    exitConditions: ['EMA(12) < EMA(26)'],
    parameters: {
      fast_ema: { value: 12, min: 8, max: 21, step: 1 },
      slow_ema: { value: 26, min: 21, max: 50, step: 1 }
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
  const navigate = useNavigate();
  
  // Strategy building state
  const [selectedTemplate, setSelectedTemplate] = useState<StrategyTemplate | null>(null);
  const [entryConditions, setEntryConditions] = useState<TradingCondition[]>([]);
  const [exitConditions, setExitConditions] = useState<TradingCondition[]>([]);
  const [stopLoss, setStopLoss] = useState<string>('');
  const [takeProfit, setTakeProfit] = useState<string>('');
  
  // Data universe
  const [dataUniverse, setDataUniverse] = useState<DataUniverse>({
    symbols: ['SPY'],
    timeframe: 'Daily',
    startDate: '2023-01-01',
    endDate: '2024-12-31'
  });
  
  // Backtesting state
  const [isBacktesting, setIsBacktesting] = useState(false);
  const [backtestResults, setBacktestResults] = useState<BacktestResults | null>(null);
  
  // UI state
  const [activeView, setActiveView] = useState<'builder' | 'templates' | 'results'>('templates');
  const [searchQuery, setSearchQuery] = useState('');

  if (!isOpen) return null;

  // Filter templates based on search
  const filteredTemplates = strategyTemplates.filter(template =>
    template.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
    template.description.toLowerCase().includes(searchQuery.toLowerCase()) ||
    template.category.toLowerCase().includes(searchQuery.toLowerCase())
  );

  // Group templates by category
  const templatesByCategory = filteredTemplates.reduce((acc, template) => {
    if (!acc[template.category]) {
      acc[template.category] = [];
    }
    acc[template.category].push(template);
    return acc;
  }, {} as Record<string, StrategyTemplate[]>);

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
    
    setActiveView('builder');
  };

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

  const validateCondition = (condition: string): boolean => {
    // Simple validation - check if condition contains indicator and operator
    const hasIndicator = indicatorSuggestions.some(ind => condition.includes(ind.split('(')[0]));
    const hasOperator = ['>', '<', '>=', '<=', '='].some(op => condition.includes(op));
    return hasIndicator && hasOperator;
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

  return (
    <div className={styles.workbenchContainer}>
      {/* Strategy Templates/Builder Sidebar */}
      <aside className={styles.strategySidebar}>
        <div className={styles.libraryHeader}>
          <h3>Component Library</h3>
          <input
            type="text"
            placeholder="Search indicators..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className={styles.searchInput}
          />
        </div>

        <div className={styles.libraryContent}>
          {Object.entries(indicatorsByCategory).map(([category, indicators]) => (
            <div key={category} className={styles.indicatorCategory}>
              <div className={styles.categoryHeader}>
                <span className={styles.categoryIcon}>
                  {category === 'Momentum' ? '🎯' : 
                   category === 'Trend' ? '📈' : 
                   category === 'Volatility' ? '⚡' : '📦'}
                </span>
                <span className={styles.categoryName}>{category}</span>
              </div>
              
              <div className={styles.indicatorList}>
                {indicators.map(indicator => (
                  <button
                    key={indicator.id}
                    className={`${styles.indicatorBtn} ${activeIndicators.find(ai => ai.id === indicator.id) ? styles.active : ''}`}
                    onClick={() => addIndicator(indicator)}
                    disabled={!!activeIndicators.find(ai => ai.id === indicator.id)}
                  >
                    <span className={styles.indicatorIcon}>{indicator.icon}</span>
                    <div className={styles.indicatorInfo}>
                      <span className={styles.indicatorName}>{indicator.name}</span>
                      <span className={styles.indicatorDesc}>{indicator.description}</span>
                    </div>
                  </button>
                ))}
              </div>
            </div>
          ))}
        </div>

        {/* Active Indicators */}
        <div className={styles.activeIndicators}>
          <h4>Active Components ({activeIndicators.length})</h4>
          {activeIndicators.map(indicator => (
            <div key={indicator.id} className={styles.activeIndicator}>
              <div className={styles.activeIndicatorHeader}>
                <span className={styles.indicatorIcon}>{indicator.icon}</span>
                <span className={styles.indicatorName}>{indicator.name}</span>
                <button 
                  className={styles.removeBtn}
                  onClick={() => removeIndicator(indicator.id)}
                >
                  ×
                </button>
              </div>
              
              {/* Parameter Controls */}
              <div className={styles.parameterControls}>
                {Object.entries(indicator.params).map(([param, value]) => (
                  <div key={param} className={styles.parameterControl}>
                    <label className={styles.parameterLabel}>
                      {param}: {value}
                    </label>
                    <input
                      type="range"
                      min={param.includes('period') ? 5 : param.includes('std') ? 0.5 : 1}
                      max={param.includes('period') ? 50 : param.includes('std') ? 4 : 100}
                      step={param.includes('std') ? 0.1 : 1}
                      value={value}
                      onChange={(e) => updateIndicatorParam(indicator.id, param, parseFloat(e.target.value))}
                      className={styles.parameterSlider}
                    />
                  </div>
                ))}
                
                {/* Weight Control */}
                <div className={styles.parameterControl}>
                  <label className={styles.parameterLabel}>
                    Weight: {indicator.weight.toFixed(1)}
                  </label>
                  <input
                    type="range"
                    min="0.1"
                    max="3.0"
                    step="0.1"
                    value={indicator.weight}
                    onChange={(e) => updateIndicatorWeight(indicator.id, parseFloat(e.target.value))}
                    className={styles.parameterSlider}
                  />
                </div>
              </div>
            </div>
          ))}
        </div>

        {/* Signal Logic */}
        <div className={styles.signalLogic}>
          <h4>Signal Logic</h4>
          <div className={styles.logicOptions}>
            {(['ALL', 'ANY', 'MAJORITY', 'WEIGHTED'] as SignalLogic[]).map(logic => (
              <button
                key={logic}
                className={`${styles.logicBtn} ${signalLogic === logic ? styles.active : ''}`}
                onClick={() => setSignalLogic(logic)}
              >
                {logic}
              </button>
            ))}
          </div>
        </div>
      </aside>

      {/* Main Analysis Dashboard */}
      <main className={styles.analysisDashboard}>
        {/* Header */}
        <div className={styles.dashboardHeader}>
          <div className={styles.headerLeft}>
            <h2>Strategy Research Workbench</h2>
            <span className={styles.activeCount}>{activeIndicators.length} indicators active</span>
          </div>
          <div className={styles.headerRight}>
            <button className={styles.deployBtn} disabled={activeIndicators.length === 0}>
              Deploy Strategy
            </button>
            <button className={styles.closeBtn} onClick={onClose}>×</button>
          </div>
        </div>

        {/* Analysis Grid */}
        <div className={styles.analysisGrid}>
          {/* Ensemble Performance */}
          <div className={styles.analysisPanel}>
            <div className={styles.panelHeader}>
              <h3>📊 Ensemble Performance</h3>
              <span className={styles.updateIndicator}>● Live</span>
            </div>
            <div className={styles.metricsGrid}>
              <div className={styles.metric}>
                <span className={styles.metricValue}>{ensembleMetrics.signalStrength.toFixed(2)}</span>
                <span className={styles.metricLabel}>Signal Strength</span>
              </div>
              <div className={styles.metric}>
                <span className={styles.metricValue}>{ensembleMetrics.winRate.toFixed(0)}%</span>
                <span className={styles.metricLabel}>Win Rate</span>
              </div>
              <div className={styles.metric}>
                <span className={styles.metricValue}>{ensembleMetrics.sharpeRatio.toFixed(2)}</span>
                <span className={styles.metricLabel}>Sharpe Ratio</span>
              </div>
              <div className={styles.metric}>
                <span className={styles.metricValue}>{ensembleMetrics.totalReturn.toFixed(1)}%</span>
                <span className={styles.metricLabel}>Annual Return</span>
              </div>
            </div>
          </div>

          {/* Regime Analysis */}
          <div className={styles.analysisPanel}>
            <div className={styles.panelHeader}>
              <h3>🏛️ Regime Analysis</h3>
            </div>
            <div className={styles.regimeGrid}>
              <div className={styles.regimeItem}>
                <span className={styles.regimeLabel}>Bull Market</span>
                <span className={`${styles.regimeValue} ${regimePerformance.bull > 0 ? styles.positive : styles.negative}`}>
                  {regimePerformance.bull > 0 ? '+' : ''}{regimePerformance.bull.toFixed(1)}%
                </span>
              </div>
              <div className={styles.regimeItem}>
                <span className={styles.regimeLabel}>Bear Market</span>
                <span className={`${styles.regimeValue} ${regimePerformance.bear > 0 ? styles.positive : styles.negative}`}>
                  {regimePerformance.bear > 0 ? '+' : ''}{regimePerformance.bear.toFixed(1)}%
                </span>
              </div>
              <div className={styles.regimeItem}>
                <span className={styles.regimeLabel}>High Volatility</span>
                <span className={`${styles.regimeValue} ${regimePerformance.highVol > 0 ? styles.positive : styles.negative}`}>
                  {regimePerformance.highVol > 0 ? '+' : ''}{regimePerformance.highVol.toFixed(1)}%
                </span>
              </div>
              <div className={styles.regimeItem}>
                <span className={styles.regimeLabel}>Low Volatility</span>
                <span className={`${styles.regimeValue} ${regimePerformance.lowVol > 0 ? styles.positive : styles.negative}`}>
                  {regimePerformance.lowVol > 0 ? '+' : ''}{regimePerformance.lowVol.toFixed(1)}%
                </span>
              </div>
              <div className={styles.regimeItem}>
                <span className={styles.regimeLabel}>Trending</span>
                <span className={`${styles.regimeValue} ${regimePerformance.trending > 0 ? styles.positive : styles.negative}`}>
                  {regimePerformance.trending > 0 ? '+' : ''}{regimePerformance.trending.toFixed(1)}%
                </span>
              </div>
              <div className={styles.regimeItem}>
                <span className={styles.regimeLabel}>Sideways</span>
                <span className={`${styles.regimeValue} ${regimePerformance.sideways > 0 ? styles.positive : styles.negative}`}>
                  {regimePerformance.sideways > 0 ? '+' : ''}{regimePerformance.sideways.toFixed(1)}%
                </span>
              </div>
            </div>
          </div>

          {/* Correlation Matrix */}
          <div className={styles.analysisPanel}>
            <div className={styles.panelHeader}>
              <h3>🔗 Signal Correlation</h3>
            </div>
            {activeIndicators.length > 1 ? (
              <div className={styles.correlationMatrix}>
                <div className={styles.correlationGrid}>
                  {activeIndicators.map(ind1 => (
                    <div key={`row-${ind1.id}`} className={styles.correlationRow}>
                      <span className={styles.correlationLabel}>{ind1.name}</span>
                      {activeIndicators.map(ind2 => (
                        <div
                          key={`cell-${ind1.id}-${ind2.id}`}
                          className={styles.correlationCell}
                          style={{
                            backgroundColor: getCorrelationColor(correlationMatrix[ind1.id]?.[ind2.id] || 0),
                            opacity: correlationMatrix[ind1.id]?.[ind2.id] || 0
                          }}
                          title={`${ind1.name} vs ${ind2.name}: ${(correlationMatrix[ind1.id]?.[ind2.id] || 0).toFixed(2)}`}
                        >
                          {(correlationMatrix[ind1.id]?.[ind2.id] || 0).toFixed(2)}
                        </div>
                      ))}
                    </div>
                  ))}
                </div>
              </div>
            ) : (
              <div className={styles.emptyState}>
                Add 2+ indicators to see correlations
              </div>
            )}
          </div>

          {/* Live Chart Placeholder */}
          <div className={`${styles.analysisPanel} ${styles.chartPanel}`}>
            <div className={styles.panelHeader}>
              <h3>📈 Live Analysis</h3>
            </div>
            <div className={styles.chartContainer}>
              <div className={styles.chartPlaceholder}>
                {activeIndicators.length > 0 ? (
                  <div className={styles.chartInfo}>
                    <span className={styles.chartIcon}>📊</span>
                    <span>Interactive chart with {activeIndicators.length} signal overlays</span>
                    <span className={styles.chartNote}>Real-time signal visualization</span>
                  </div>
                ) : (
                  <div className={styles.chartInfo}>
                    <span className={styles.chartIcon}>📈</span>
                    <span>Add indicators to see live analysis</span>
                    <span className={styles.chartNote}>Charts will update in real-time</span>
                  </div>
                )}
              </div>
            </div>
          </div>
        </div>
      </main>
    </div>
  );
};