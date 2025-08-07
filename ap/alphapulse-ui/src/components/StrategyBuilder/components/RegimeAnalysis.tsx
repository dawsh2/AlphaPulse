import React from 'react';
import styles from './RegimeAnalysis.module.css';
import { AnalysisChart } from './AnalysisChart';

interface RegimeData {
  type: 'bull' | 'bear' | 'sideways';
  performance: number;
  periods: number;
  description: string;
}

export const RegimeAnalysis: React.FC = () => {
  const regimes: RegimeData[] = [
    {
      type: 'bull',
      performance: 24.3,
      periods: 156,
      description: 'Strong uptrend periods'
    },
    {
      type: 'bear',
      performance: -2.1,
      periods: 89,
      description: 'Downtrend & corrections'
    },
    {
      type: 'sideways',
      performance: 8.7,
      periods: 245,
      description: 'Range-bound markets'
    }
  ];

  return (
    <div className={styles.analysisView}>
      <h3 className={styles.panelTitle}>Market Regime Performance</h3>
      
      <div className={styles.regimeSummary}>
        {regimes.map(regime => (
          <div key={regime.type} className={`${styles.regimeCard} ${styles[regime.type]}`}>
            <div className={styles.regimeTitle}>
              {regime.type === 'bull' ? '🐂' : regime.type === 'bear' ? '🐻' : '↔️'} {' '}
              {regime.type.charAt(0).toUpperCase() + regime.type.slice(1)} Market
            </div>
            <div className={styles.regimePerformance}>
              {regime.performance > 0 ? '+' : ''}{regime.performance}%
            </div>
            <div className={styles.regimeDescription}>
              {regime.periods} periods • {regime.description}
            </div>
          </div>
        ))}
      </div>
      
      <AnalysisChart 
        type="regime"
        title="Regime Performance Chart"
        description="Strategy returns vs S&P 500 across different market conditions"
      />
    </div>
  );
};