import React from 'react';
import styles from './AnalysisChart.module.css';

interface AnalysisChartProps {
  type: 'equity' | 'regime' | 'drawdown' | 'heatmap' | 'factor';
  title: string;
  description: string;
}

export const AnalysisChart: React.FC<AnalysisChartProps> = ({ type, title, description }) => {
  const getChartIcon = () => {
    switch (type) {
      case 'equity': return '📈';
      case 'regime': return '📊';
      case 'drawdown': return '📉';
      case 'heatmap': return '🔥';
      case 'factor': return '🧬';
      default: return '📊';
    }
  };

  // Mock data for equity curve visualization
  const renderEquityCurve = () => {
    if (type === 'equity') {
      return (
        <div className={styles.equityChart}>
          <svg viewBox="0 0 400 180" className={styles.chartSvg}>
            {/* Gradient definitions */}
            <defs>
              <linearGradient id="greenGradient" x1="0%" y1="0%" x2="0%" y2="100%">
                <stop offset="0%" stopColor="#4CAF50" stopOpacity="0.6" />
                <stop offset="100%" stopColor="#4CAF50" stopOpacity="0.1" />
              </linearGradient>
            </defs>
            
            {/* Fill area under curve */}
            <path
              d="M 10 160 L 50 150 L 90 140 L 130 120 L 170 110 L 210 90 L 250 80 L 290 60 L 330 40 L 370 20 L 370 160 L 10 160 Z"
              fill="url(#greenGradient)"
            />
            
            {/* Main equity curve */}
            <path
              d="M 10 160 L 50 150 L 90 140 L 130 120 L 170 110 L 210 90 L 250 80 L 290 60 L 330 40 L 370 20"
              stroke="#4CAF50"
              strokeWidth="3"
              fill="none"
              strokeLinecap="round"
              filter="drop-shadow(0 2px 4px rgba(76, 175, 80, 0.3))"
            />
            
            {/* Benchmark line */}
            <path
              d="M 10 160 L 50 155 L 90 150 L 130 145 L 170 140 L 210 135 L 250 130 L 290 125 L 330 120 L 370 115"
              stroke="#89CDF1"
              strokeWidth="2"
              fill="none"
              strokeDasharray="5,5"
              opacity="0.7"
            />
            
            {/* Grid lines */}
            <line x1="10" y1="160" x2="390" y2="160" stroke="var(--color-border-primary)" strokeWidth="1" opacity="0.3" />
            <line x1="10" y1="20" x2="10" y2="160" stroke="var(--color-border-primary)" strokeWidth="1" opacity="0.3" />
          </svg>
          <div className={styles.chartLegend}>
            <span className={styles.legendItem}>
              <span className={styles.legendColor} style={{ background: '#4CAF50' }}></span>
              Strategy
            </span>
            <span className={styles.legendItem}>
              <span className={styles.legendColor} style={{ background: '#89CDF1' }}></span>
              Benchmark
            </span>
          </div>
        </div>
      );
    }
    return null;
  };

  return (
    <div className={styles.chartContainer}>
      {type === 'equity' ? (
        renderEquityCurve()
      ) : (
        <div className={styles.chartPlaceholder}>
          <div className={styles.chartIcon}>{getChartIcon()}</div>
          <div className={styles.chartTitle}>{title}</div>
          <div className={styles.chartDescription}>{description}</div>
        </div>
      )}
    </div>
  );
};