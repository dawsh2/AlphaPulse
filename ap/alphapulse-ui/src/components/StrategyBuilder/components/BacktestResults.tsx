import React from 'react';
import styles from './BacktestResults.module.css';
import { MetricCard } from './MetricCard';
import { AnalysisChart } from './AnalysisChart';

interface BacktestResultsProps {
  metrics: {
    sharpeRatio: number;
    annualReturn: number;
    maxDrawdown: number;
    winRate: number;
    totalTrades: number;
    profitFactor: number;
  };
  isOptimization?: boolean;
  optimalParams?: Record<string, number>;
  testedCombinations?: number;
  onDeploy: () => void;
  onClose: () => void;
  onOpenNotebook: () => void;
}

export const BacktestResults: React.FC<BacktestResultsProps> = ({
  metrics,
  isOptimization = false,
  optimalParams = {},
  testedCombinations = 0,
  onDeploy,
  onClose,
  onOpenNotebook
}) => {

  return (
    <div className={styles.resultsContainer}>

      {/* Main Content Grid */}
      <div className={styles.mainContent}>
        {/* Left Panel: Results & Controls */}
        <div className={styles.resultsPanel}>
          {/* Performance Metrics - Only 4 key metrics */}
          <div className={styles.metricsGrid}>
            <MetricCard 
              value={metrics.sharpeRatio} 
              label="Sharpe" 
              trend="positive"
            />
            <MetricCard 
              value={metrics.annualReturn} 
              label="Annual %" 
              format="percentage"
              trend={metrics.annualReturn > 0 ? 'positive' : 'negative'}
            />
            <MetricCard 
              value={metrics.maxDrawdown} 
              label="Max DD" 
              format="percentage"
              trend="negative"
            />
            <MetricCard 
              value={metrics.winRate} 
              label="Win %" 
              format="percentage"
              trend={metrics.winRate > 50 ? 'positive' : 'negative'}
            />
          </div>

          {/* Optimization Results or AI Insights */}
          {isOptimization ? (
            <div className={styles.optimizationResults}>
              <h3 className={styles.optimizationTitle}>🎯 Optimal Parameters Found</h3>
              <div className={styles.optimalParamsList}>
                <div className={styles.optimalParam}>
                  <span className={styles.paramName}>RSI Period:</span>
                  <span className={styles.paramValue}>14</span>
                  <span className={styles.paramRange}>(tested 10-20)</span>
                </div>
                <div className={styles.optimalParam}>
                  <span className={styles.paramName}>Oversold:</span>
                  <span className={styles.paramValue}>30</span>
                  <span className={styles.paramRange}>(tested 25-35)</span>
                </div>
              </div>
              <div className={styles.testedInfo}>
                <span className={styles.testedIcon}>🔬</span>
                <span className={styles.testedText}>Best of {testedCombinations || 120} combinations tested</span>
              </div>
            </div>
          ) : (
            <div className={styles.compactInsights}>
              <h3 className={styles.insightsTitle}>🤖 Key Insights</h3>
              <div className={styles.insightsList}>
                <div className={styles.insightItem}>
                  <span className={styles.insightIcon}>📈</span>
                  <span className={styles.insightText}>Strong regime performance</span>
                </div>
                <div className={styles.insightItem}>
                  <span className={styles.insightIcon}>🌊</span>
                  <span className={styles.insightText}>VIX correlation: 0.73</span>
                </div>
              </div>
            </div>
          )}
        </div>

        {/* Right Panel: Heatmap or Equity Curve */}
        <div className={styles.analysisPanel}>
          <div className={styles.analysisView}>
            {isOptimization ? (
              <>
                <h3 className={styles.heatmapTitle}>Parameter Performance Heatmap</h3>
                <div className={styles.heatmapContainer}>
                  <div className={styles.heatmapGrid}>
                    {/* Mock heatmap visualization */}
                    {[...Array(11)].map((_, row) => (
                      <div key={row} className={styles.heatmapRow}>
                        {[...Array(11)].map((_, col) => {
                          const intensity = Math.abs(5 - row) + Math.abs(5 - col);
                          const performance = 2.5 - (intensity * 0.15);
                          const color = performance > 2 ? '#4CAF50' : 
                                       performance > 1.5 ? '#FFC107' : 
                                       performance > 1 ? '#FF9800' : '#F44336';
                          return (
                            <div 
                              key={col} 
                              className={styles.heatmapCell}
                              style={{ background: color, opacity: 0.7 + (performance * 0.1) }}
                              title={`RSI: ${10 + row}, Oversold: ${25 + col} - Sharpe: ${performance.toFixed(2)}`}
                            />
                          );
                        })}
                      </div>
                    ))}
                  </div>
                  <div className={styles.heatmapAxes}>
                    <span className={styles.xAxis}>RSI Period →</span>
                    <span className={styles.yAxis}>Oversold Level →</span>
                  </div>
                  <div className={styles.heatmapLegend}>
                    <span>Poor</span>
                    <div className={styles.legendGradient}></div>
                    <span>Optimal</span>
                  </div>
                </div>
              </>
            ) : (
              <AnalysisChart 
                type="equity"
                title="Equity Curve"
                description="Cumulative returns over time"
              />
            )}
          </div>
        </div>
      </div>

      {/* Action Buttons - Icon only, bottom right */}
      <div className={styles.actionsBar}>
        <div className={styles.leftActions}>
          {/* Empty left side */}
        </div>
        <div className={styles.rightActions}>
          <button className={styles.btnIcon} onClick={onOpenNotebook} title="Open in Research Tab">
            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" style={{ width: '20px', height: '20px' }}>
              {/* Spiral binding */}
              <circle cx="4" cy="4" r="1.5"></circle>
              <circle cx="4" cy="8" r="1.5"></circle>
              <circle cx="4" cy="12" r="1.5"></circle>
              <circle cx="4" cy="16" r="1.5"></circle>
              <circle cx="4" cy="20" r="1.5"></circle>
              {/* Notebook pages */}
              <rect x="7" y="2" width="14" height="20" rx="1"></rect>
              {/* Lines on pages */}
              <line x1="10" y1="7" x2="18" y2="7"></line>
              <line x1="10" y1="11" x2="18" y2="11"></line>
              <line x1="10" y1="15" x2="16" y2="15"></line>
            </svg>
          </button>
          <button className={styles.btnIcon} onClick={onDeploy} title="Deploy Strategy">
            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" style={{ width: '20px', height: '20px' }}>
              {/* Rocket/Deploy icon */}
              <path d="M4.5 16.5c-1.5 1.26-2 5-2 5s3.74-.5 5-2c.71-.84.7-2.13-.09-2.91a2.18 2.18 0 0 0-2.91-.09z"></path>
              <path d="M12 15l-3-3a22 22 0 0 1 2-3.95A12.88 12.88 0 0 1 22 2c0 2.72-.78 7.5-6 11a22.35 22.35 0 0 1-4 2z"></path>
              <path d="M9 12H4s.55-3.03 2-4c1.62-1.08 5 0 5 0"></path>
              <path d="M12 15v5s3.03-.55 4-2c1.08-1.62 0-5 0-5"></path>
            </svg>
          </button>
        </div>
      </div>
    </div>
  );
};