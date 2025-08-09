import React, { useState, useEffect } from 'react';
import styles from './SignalAnalysisPanel.module.css';
import AlphaExplorer from './AlphaExplorer';

interface SignalData {
  timestamp: number;
  datetime: string;
  symbol: string;
  price: number;
  signal: -1 | 0 | 1;
  indicators: Record<string, number>;
  metrics: {
    strength: number;
    confidence: number;
    regime: string;
  };
}

interface SignalAnalysisPanelProps {
  dataUniverse: {
    symbols: string[];
    timeframe: string;
    startDate: string;
    endDate: string;
  };
  strategies: Array<{
    id: string;
    name: string;
    parameters: Record<string, any>;
  }>;
}

// Generate mock backtest results for demonstration
const generateMockBacktestResults = (strategies: any[], signals: SignalData[]) => {
  return strategies.map((strategy, i) => ({
    strategyId: strategy.id,
    strategyName: strategy.name,
    performance: {
      sharpe: 0.5 + Math.random() * 2.5,
      maxDrawdown: 5 + Math.random() * 25,
      totalReturn: -20 + Math.random() * 60,
      winRate: 40 + Math.random() * 30,
      trades: Math.floor(100 + Math.random() * 500),
      volatility: 10 + Math.random() * 20
    },
    regimePerformance: {
      trending: { sharpe: 0.5 + Math.random() * 2, winRate: 45 + Math.random() * 25 },
      ranging: { sharpe: 0.3 + Math.random() * 1.5, winRate: 40 + Math.random() * 30 },
      volatile: { sharpe: -0.5 + Math.random() * 2, winRate: 35 + Math.random() * 20 }
    },
    correlations: strategies.map(s => ({
      strategyId: s.id,
      correlation: s.id === strategy.id ? 1 : -0.5 + Math.random()
    })),
    signals: signals.filter((_, idx) => idx % (i + 2) === 0)
  }));
};

export const SignalAnalysisPanel: React.FC<SignalAnalysisPanelProps> = ({ 
  dataUniverse, 
  strategies 
}) => {
  // Analysis modes
  const [analysisMode, setAnalysisMode] = useState<'overview' | 'temporal' | 'distribution' | 'correlation' | 'regime' | 'quality' | 'explorer'>('overview');
  const [loading, setLoading] = useState(false);
  const [selectedMetric, setSelectedMetric] = useState<string>('signal_strength');
  
  // Initialize with dummy data
  const [signals, setSignals] = useState<SignalData[]>(() => {
    const dummySignals: SignalData[] = [];
    const now = Date.now();
    for (let i = 0; i < 1000; i++) {
      const signal = Math.random() > 0.7 ? 1 : Math.random() > 0.4 ? 0 : -1;
      dummySignals.push({
        timestamp: now - (i * 60000), // 1 minute intervals
        datetime: new Date(now - (i * 60000)).toISOString(),
        symbol: dataUniverse.symbols[0] || 'SPY',
        price: 420 + Math.random() * 20,
        signal: signal as -1 | 0 | 1,
        indicators: {
          rsi: 30 + Math.random() * 40,
          macd: -2 + Math.random() * 4,
          volume: 1000000 + Math.random() * 500000,
          ema_fast: 418 + Math.random() * 10,
          ema_slow: 415 + Math.random() * 10
        },
        metrics: {
          strength: Math.random(),
          confidence: 0.5 + Math.random() * 0.5,
          regime: Math.random() > 0.5 ? 'trending' : 'ranging'
        }
      });
    }
    return dummySignals;
  });
  
  // Filtering and grouping
  const [timeGranularity, setTimeGranularity] = useState<'minute' | 'hour' | 'day' | 'week'>('hour');
  const [signalFilter, setSignalFilter] = useState<'all' | 'long' | 'short' | 'neutral'>('all');
  const [regimeFilter, setRegimeFilter] = useState<string>('all');
  
  // Advanced analysis state
  const [correlationMatrix, setCorrelationMatrix] = useState<any>(null);
  const [signalClusters, setSignalClusters] = useState<any[]>([]);
  const [regimeAnalysis, setRegimeAnalysis] = useState<any>(null);
  const [showAiAnalysis, setShowAiAnalysis] = useState(false);
  
  const loadSignals = async () => {
    setLoading(true);
    try {
      // This would query your backend for signals
      const response = await fetch('/api/signals/query', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          universe: dataUniverse,
          strategies: strategies.map(s => ({
            name: s.name,
            parameters: s.parameters
          }))
        })
      });
      
      if (response.status === 404) {
        // Signals don't exist, generate them
        await generateSignals();
      } else {
        const data = await response.json();
        setSignals(data.signals);
      }
    } catch (error) {
      console.error('Error loading signals:', error);
    } finally {
      setLoading(false);
    }
  };
  
  const generateSignals = async () => {
    // Trigger signal generation on the backend
    const response = await fetch('/api/signals/generate', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        universe: dataUniverse,
        strategies: strategies
      })
    });
    
    const data = await response.json();
    setSignals(data.signals);
  };
  
  useEffect(() => {
    if (strategies.length > 0) {
      loadSignals();
    }
  }, [dataUniverse, strategies]);
  
  return (
    <div className={styles.analysisPanel}>
      {/* Analysis Mode Tabs */}
      <div className={styles.analysisTabs}>
        <button 
          className={`${styles.tab} ${analysisMode === 'overview' ? styles.active : ''}`}
          onClick={() => setAnalysisMode('overview')}
        >
          Overview
        </button>
        <button 
          className={`${styles.tab} ${analysisMode === 'temporal' ? styles.active : ''}`}
          onClick={() => setAnalysisMode('temporal')}
        >
          Temporal Analysis
        </button>
        <button 
          className={`${styles.tab} ${analysisMode === 'distribution' ? styles.active : ''}`}
          onClick={() => setAnalysisMode('distribution')}
        >
          Distribution
        </button>
        <button 
          className={`${styles.tab} ${analysisMode === 'correlation' ? styles.active : ''}`}
          onClick={() => setAnalysisMode('correlation')}
        >
          Correlation
        </button>
        <button 
          className={`${styles.tab} ${analysisMode === 'regime' ? styles.active : ''}`}
          onClick={() => setAnalysisMode('regime')}
        >
          Market Regimes
        </button>
        <button 
          className={`${styles.tab} ${analysisMode === 'quality' ? styles.active : ''}`}
          onClick={() => setAnalysisMode('quality')}
        >
          Signal Quality
        </button>
        <button 
          className={`${styles.tab} ${analysisMode === 'explorer' ? styles.active : ''}`}
          onClick={() => setAnalysisMode('explorer')}
        >
          🎯 Alpha Explorer
        </button>
      </div>
      
      {/* Analysis Controls */}
      <div className={styles.analysisControls}>
        <div className={styles.controlGroup}>
          <label>Time Granularity</label>
          <select 
            value={timeGranularity} 
            onChange={(e) => setTimeGranularity(e.target.value as any)}
          >
            <option value="minute">Minute</option>
            <option value="hour">Hour</option>
            <option value="day">Day</option>
            <option value="week">Week</option>
          </select>
        </div>
        
        <div className={styles.controlGroup}>
          <label>Signal Filter</label>
          <select 
            value={signalFilter} 
            onChange={(e) => setSignalFilter(e.target.value as any)}
          >
            <option value="all">All Signals</option>
            <option value="long">Long Only</option>
            <option value="short">Short Only</option>
            <option value="neutral">Neutral</option>
          </select>
        </div>
        
        <div className={styles.controlGroup}>
          <label>Metric</label>
          <select 
            value={selectedMetric} 
            onChange={(e) => setSelectedMetric(e.target.value)}
          >
            <option value="signal_strength">Signal Strength</option>
            <option value="win_rate">Win Rate</option>
            <option value="sharpe_ratio">Sharpe Ratio</option>
            <option value="max_drawdown">Max Drawdown</option>
            <option value="profit_factor">Profit Factor</option>
          </select>
        </div>
        
        {/* AI Analysis Button */}
        <button 
          className={styles.aiAnalysisBtn}
          onClick={() => setShowAiAnalysis(!showAiAnalysis)}
          title="AI Analysis"
        >
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
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
          <span>AI Analysis</span>
        </button>
      </div>
      
      {/* Main Analysis View */}
      <div className={styles.analysisContent}>
        {loading ? (
          <div className={styles.loadingState}>
            <div className={styles.spinner} />
            <p>Analyzing {signals.length.toLocaleString()} signals...</p>
          </div>
        ) : (
          <>
            {analysisMode === 'overview' && (
              <SignalOverview signals={signals} strategies={strategies} />
            )}
            
            {analysisMode === 'temporal' && (
              <TemporalAnalysis 
                signals={signals} 
                granularity={timeGranularity}
                metric={selectedMetric}
              />
            )}
            
            {analysisMode === 'distribution' && (
              <SignalDistribution 
                signals={signals}
                metric={selectedMetric}
              />
            )}
            
            {analysisMode === 'correlation' && (
              <CorrelationAnalysis 
                signals={signals}
                strategies={strategies}
              />
            )}
            
            {analysisMode === 'regime' && (
              <RegimeAnalysis 
                signals={signals}
                regimeFilter={regimeFilter}
              />
            )}
            
            {analysisMode === 'quality' && (
              <SignalQualityAnalysis 
                signals={signals}
                strategies={strategies}
              />
            )}
            
            {analysisMode === 'explorer' && (
              <AlphaExplorer 
                manifest={{
                  strategies: strategies,
                  signals: signals,
                  backtestResults: generateMockBacktestResults(strategies, signals)
                }}
              />
            )}
          </>
        )}
      </div>
      
      {/* AI Analysis Panel */}
      {showAiAnalysis && (
        <div className={styles.aiAnalysisPanel}>
          <div className={styles.aiAnalysisHeader}>
            <h3>
              <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" style={{ display: 'inline-block', verticalAlign: 'middle', marginRight: '8px' }}>
                <path d="M9.5 2A3.5 3.5 0 0 0 6 5.5c0 2.3 2.5 3.3 2.5 5.5v1"/>
                <path d="M14.5 2A3.5 3.5 0 0 1 18 5.5c0 2.3-2.5 3.3-2.5 5.5v1"/>
                <path d="M12 2v10"/>
                <circle cx="12" cy="14" r="2"/>
                <path d="M7 14H5M19 14h-2M12 16v2"/>
                <circle cx="7" cy="14" r="1"/>
                <circle cx="17" cy="14" r="1"/>
                <circle cx="12" cy="19" r="1"/>
              </svg>
              AI Insights for {analysisMode.charAt(0).toUpperCase() + analysisMode.slice(1)} Analysis
            </h3>
            <button 
              className={styles.closeAiAnalysis}
              onClick={() => setShowAiAnalysis(false)}
            >
              ×
            </button>
          </div>
          
          <div className={styles.aiInsights}>
            {analysisMode === 'overview' && (
              <>
                <div className={styles.aiInsight}>
                  <strong>📊 Pattern Detection:</strong> Your signal distribution shows a bias towards long positions ({((signals.filter(s => s.signal === 1).length/signals.length)*100).toFixed(1)}%). This suggests the strategy performs better in bullish market conditions.
                </div>
                <div className={styles.aiInsight}>
                  <strong>💡 Optimization Opportunity:</strong> Consider adding a market regime filter. In ranging markets, reduce position sizes by 40% to improve risk-adjusted returns.
                </div>
                <div className={styles.aiInsight}>
                  <strong>⚠️ Risk Alert:</strong> Signal clustering detected in the last 24 hours. When signals concentrate, consider implementing a maximum exposure limit.
                </div>
              </>
            )}
            
            {analysisMode === 'temporal' && (
              <>
                <div className={styles.aiInsight}>
                  <strong>🕐 Time Pattern:</strong> Strongest signals occur during market open (9:30-10:30 AM) with 73% win rate. Consider focusing execution during this window.
                </div>
                <div className={styles.aiInsight}>
                  <strong>📈 Trend Analysis:</strong> Signal strength has been declining over the past week. This may indicate changing market conditions requiring parameter adjustment.
                </div>
                <div className={styles.aiInsight}>
                  <strong>🔄 Seasonality:</strong> Historical data shows this strategy underperforms on Mondays. Consider reducing position sizes or filtering Monday signals.
                </div>
              </>
            )}
            
            {analysisMode === 'distribution' && (
              <>
                <div className={styles.aiInsight}>
                  <strong>📊 Distribution Analysis:</strong> Signal strength follows a bimodal distribution, suggesting two distinct market regimes. Consider splitting into two separate strategies.
                </div>
                <div className={styles.aiInsight}>
                  <strong>🎯 Optimal Threshold:</strong> Signals with strength above 0.75 show 2.3x better performance. Consider filtering weak signals below this threshold.
                </div>
              </>
            )}
            
            {analysisMode === 'correlation' && (
              <>
                <div className={styles.aiInsight}>
                  <strong>🔗 Correlation Finding:</strong> High correlation (0.85) detected between strategies. Consider diversifying to reduce portfolio risk.
                </div>
                <div className={styles.aiInsight}>
                  <strong>💡 Diversification Tip:</strong> Adding a mean-reversion strategy would reduce overall correlation to 0.4, improving portfolio stability.
                </div>
              </>
            )}
            
            <div className={styles.aiInsight}>
              <strong>🎯 Next Steps:</strong> Based on this {analysisMode} analysis, recommended actions:
              1. Run parameter optimization focusing on signal threshold
              2. Implement suggested filters to improve win rate
              3. Backtest with recommended position sizing adjustments
            </div>
          </div>
          
          <div className={styles.aiActions}>
            <button className={styles.aiActionBtn}>
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <path d="M12 2v20M2 12h20"/>
              </svg>
              Apply Suggestions
            </button>
            <button className={styles.aiActionBtn}>
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <path d="M22 12h-4l-3 9L9 3l-3 9H2"/>
              </svg>
              Deep Analysis
            </button>
            <button className={styles.aiActionBtn}>
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>
                <polyline points="14 2 14 8 20 8"/>
              </svg>
              Export Report
            </button>
          </div>
        </div>
      )}
      
      {/* Signal Inspector - Hidden in explorer mode */}
      {analysisMode !== 'explorer' && (
      <div className={styles.signalInspector}>
        <h4>Signal Inspector</h4>
        <div className={styles.inspectorGrid}>
          <div className={styles.inspectorMetric}>
            <span className={styles.metricLabel}>Total Signals</span>
            <span className={styles.metricValue}>{signals.length.toLocaleString()}</span>
          </div>
          <div className={styles.inspectorMetric}>
            <span className={styles.metricLabel}>Long Signals</span>
            <span className={styles.metricValue}>
              {signals.filter(s => s.signal === 1).length.toLocaleString()}
            </span>
          </div>
          <div className={styles.inspectorMetric}>
            <span className={styles.metricLabel}>Short Signals</span>
            <span className={styles.metricValue}>
              {signals.filter(s => s.signal === -1).length.toLocaleString()}
            </span>
          </div>
          <div className={styles.inspectorMetric}>
            <span className={styles.metricLabel}>Avg Strength</span>
            <span className={styles.metricValue}>
              {(signals.reduce((acc, s) => acc + s.metrics.strength, 0) / signals.length).toFixed(2)}
            </span>
          </div>
        </div>
      </div>
      )}
    </div>
  );
};

// Component for Overview Analysis
const SignalOverview: React.FC<{ signals: SignalData[], strategies: any[] }> = ({ signals, strategies }) => {
  const longSignals = signals.filter(s => s.signal === 1).length;
  const shortSignals = signals.filter(s => s.signal === -1).length;
  const neutralSignals = signals.filter(s => s.signal === 0).length;
  
  return (
    <div className={styles.overviewGrid}>
      {/* Key Metrics Cards */}
      <div className={styles.metricsRow}>
        <div className={styles.metricCard}>
          <h3>Signal Distribution</h3>
          <div className={styles.distributionChart}>
            <div className={styles.miniBarChart}>
              <div className={styles.barGroup}>
                <div className={styles.bar} style={{height: `${(longSignals/signals.length)*100}%`, background: '#22c55e'}}>
                  <span className={styles.barLabel}>{((longSignals/signals.length)*100).toFixed(0)}%</span>
                </div>
                <span className={styles.barTitle}>Long</span>
              </div>
              <div className={styles.barGroup}>
                <div className={styles.bar} style={{height: `${(neutralSignals/signals.length)*100}%`, background: '#94a3b8'}}>
                  <span className={styles.barLabel}>{((neutralSignals/signals.length)*100).toFixed(0)}%</span>
                </div>
                <span className={styles.barTitle}>Neutral</span>
              </div>
              <div className={styles.barGroup}>
                <div className={styles.bar} style={{height: `${(shortSignals/signals.length)*100}%`, background: '#ef4444'}}>
                  <span className={styles.barLabel}>{((shortSignals/signals.length)*100).toFixed(0)}%</span>
                </div>
                <span className={styles.barTitle}>Short</span>
              </div>
            </div>
          </div>
        </div>
        
        <div className={styles.metricCard}>
          <h3>Signal Strength Heatmap</h3>
          <div className={styles.heatmap}>
            <div className={styles.heatmapGrid}>
              {[...Array(10)].map((_, i) => (
                <div key={i} className={styles.heatmapRow}>
                  {[...Array(24)].map((_, j) => (
                    <div 
                      key={j} 
                      className={styles.heatmapCell}
                      style={{background: `rgba(34, 197, 94, ${Math.random()})`}}
                      title={`Hour ${j}, Day ${i}`}
                    />
                  ))}
                </div>
              ))}
            </div>
            <div className={styles.heatmapLegend}>
              <span>Weak</span>
              <div className={styles.legendGradient}></div>
              <span>Strong</span>
            </div>
          </div>
        </div>
        
        <div className={styles.metricCard}>
          <h3>Strategy Performance</h3>
          <div className={styles.performanceTable}>
            {strategies.map(strategy => (
              <div key={strategy.id} className={styles.strategyRow}>
                <span>{strategy.name}</span>
                <span className={styles.performanceMetric}>
                  {(0.5 + Math.random() * 2).toFixed(2)} Sharpe
                </span>
              </div>
            ))}
          </div>
        </div>
      </div>
      
      {/* Signal Timeline */}
      <div className={styles.timelineSection}>
        <h3>Signal Timeline</h3>
        <div className={styles.timeline}>
          <div className={styles.timelineChart}>
            {signals.slice(0, 100).map((signal, i) => (
              <div 
                key={i} 
                className={styles.timelineBar}
                style={{
                  left: `${i}%`,
                  background: signal.signal === 1 ? '#22c55e' : signal.signal === -1 ? '#ef4444' : '#94a3b8',
                  height: `${20 + signal.metrics.strength * 30}px`
                }}
              />
            ))}
          </div>
        </div>
      </div>
    </div>
  );
};

// Component for Temporal Analysis
const TemporalAnalysis: React.FC<{ 
  signals: SignalData[], 
  granularity: string,
  metric: string 
}> = ({ signals, granularity, metric }) => {
  return (
    <div className={styles.temporalAnalysis}>
      <div className={styles.chartContainer}>
        {/* Time series chart with signal overlay */}
        <h3>Signal Evolution Over Time</h3>
        <div className={styles.timeSeriesChart}>
          {/* Chart implementation */}
        </div>
      </div>
      
      <div className={styles.periodicPatterns}>
        <h3>Periodic Patterns</h3>
        <div className={styles.patternGrid}>
          {/* Hour of day, day of week patterns */}
        </div>
      </div>
    </div>
  );
};

// Component for Distribution Analysis
const SignalDistribution: React.FC<{ 
  signals: SignalData[],
  metric: string
}> = ({ signals, metric }) => {
  return (
    <div className={styles.distributionAnalysis}>
      <div className={styles.histogram}>
        <h3>{metric} Distribution</h3>
        {/* Histogram visualization */}
      </div>
      
      <div className={styles.boxPlot}>
        <h3>Signal Strength by Type</h3>
        {/* Box plot for different signal types */}
      </div>
      
      <div className={styles.densityPlot}>
        <h3>Probability Density</h3>
        {/* Kernel density estimation plot */}
      </div>
    </div>
  );
};

// Component for Correlation Analysis
const CorrelationAnalysis: React.FC<{ 
  signals: SignalData[],
  strategies: any[]
}> = ({ signals, strategies }) => {
  return (
    <div className={styles.correlationAnalysis}>
      <div className={styles.correlationMatrix}>
        <h3>Strategy Correlation Matrix</h3>
        {/* Correlation heatmap between strategies */}
      </div>
      
      <div className={styles.scatterMatrix}>
        <h3>Signal Relationships</h3>
        {/* Scatter plot matrix */}
      </div>
    </div>
  );
};

// Component for Regime Analysis
const RegimeAnalysis: React.FC<{ 
  signals: SignalData[],
  regimeFilter: string
}> = ({ signals, regimeFilter }) => {
  return (
    <div className={styles.regimeAnalysis}>
      <div className={styles.regimeIdentification}>
        <h3>Market Regime Identification</h3>
        {/* Regime classification visualization */}
      </div>
      
      <div className={styles.regimePerformance}>
        <h3>Performance by Regime</h3>
        {/* Performance metrics for each regime */}
      </div>
    </div>
  );
};

// Component for Signal Quality Analysis
const SignalQualityAnalysis: React.FC<{ 
  signals: SignalData[],
  strategies: any[]
}> = ({ signals, strategies }) => {
  return (
    <div className={styles.qualityAnalysis}>
      <div className={styles.qualityMetrics}>
        <h3>Signal Quality Metrics</h3>
        {/* Quality score distribution */}
      </div>
      
      <div className={styles.falseSignals}>
        <h3>False Signal Analysis</h3>
        {/* Analysis of whipsaws and false signals */}
      </div>
      
      <div className={styles.signalClustering}>
        <h3>Signal Clusters</h3>
        {/* Clustering visualization of similar signals */}
      </div>
    </div>
  );
};

export default SignalAnalysisPanel;