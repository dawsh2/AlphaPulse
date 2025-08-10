/**
 * Metrics panel for monitoring performance
 */

import React from 'react';
import { formatCurrency, formatPercent, formatNumber } from '../../../utils/format';
import styles from './Monitor.module.css';

interface Metrics {
  totalPnL: number;
  winRate: number;
  sharpeRatio: number;
  maxDrawdown: number;
  totalTrades: number;
  avgTrade: number;
}

interface MetricsPanelProps {
  metrics: Metrics;
}

export const MetricsPanel: React.FC<MetricsPanelProps> = ({ metrics }) => {
  const metricItems = [
    {
      label: 'Total P&L',
      value: formatCurrency(metrics.totalPnL),
      color: metrics.totalPnL >= 0 ? 'green' : 'red',
    },
    {
      label: 'Win Rate',
      value: formatPercent(metrics.winRate / 100, 1),
      color: metrics.winRate >= 50 ? 'green' : 'orange',
    },
    {
      label: 'Sharpe Ratio',
      value: metrics.sharpeRatio.toFixed(2),
      color: metrics.sharpeRatio >= 1 ? 'green' : 'orange',
    },
    {
      label: 'Max Drawdown',
      value: formatPercent(metrics.maxDrawdown / 100, 1),
      color: 'red',
    },
    {
      label: 'Total Trades',
      value: formatNumber(metrics.totalTrades),
      color: 'neutral',
    },
    {
      label: 'Avg Trade',
      value: formatCurrency(metrics.avgTrade),
      color: metrics.avgTrade >= 0 ? 'green' : 'red',
    },
  ];

  return (
    <div className={styles.metricsGrid}>
      {metricItems.map((item, index) => (
        <div key={index} className={styles.metricCard}>
          <div className={styles.metricLabel}>{item.label}</div>
          <div 
            className={`${styles.metricValue} ${styles[`metric${item.color}`]}`}
          >
            {item.value}
          </div>
        </div>
      ))}
    </div>
  );
};