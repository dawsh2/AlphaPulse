import React from 'react';
import styles from './MetricCard.module.css';

interface MetricCardProps {
  value: string | number;
  label: string;
  trend?: 'positive' | 'negative' | 'neutral';
  format?: 'percentage' | 'number' | 'currency';
}

export const MetricCard: React.FC<MetricCardProps> = ({ 
  value, 
  label, 
  trend = 'neutral',
  format = 'number' 
}) => {
  const formatValue = () => {
    if (typeof value === 'number') {
      switch (format) {
        case 'percentage':
          return `${value > 0 ? '+' : ''}${value.toFixed(1)}%`;
        case 'currency':
          return `$${value.toLocaleString()}`;
        default:
          return value.toFixed(2);
      }
    }
    return value;
  };

  return (
    <div className={styles.metricCard}>
      <div className={`${styles.metricValue} ${styles[trend]}`}>
        {formatValue()}
      </div>
      <div className={styles.metricLabel}>{label}</div>
    </div>
  );
};