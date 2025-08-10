import React from 'react';
import styles from './AIInsights.module.css';

export interface Insight {
  id: string;
  icon: string;
  text: string;
  actionLabel: string;
  actionType: 'regime' | 'volatility' | 'stress' | 'factor';
}

interface AIInsightsProps {
  insights: Insight[];
  onActionClick: (actionType: string) => void;
}

export const AIInsights: React.FC<AIInsightsProps> = ({ insights, onActionClick }) => {
  return (
    <div className={styles.aiInsights}>
      <div className={styles.insightsHeader}>
        <span className={styles.headerIcon}>🤖</span>
        <h3 className={styles.insightsTitle}>AI Recommendations</h3>
      </div>
      
      <div className={styles.insightsList}>
        {insights.map(insight => (
          <div key={insight.id} className={styles.insightItem}>
            <div className={styles.insightIcon}>{insight.icon}</div>
            <div className={styles.insightContent}>
              <div className={styles.insightText}>{insight.text}</div>
              <button 
                className={styles.insightAction}
                onClick={() => onActionClick(insight.actionType)}
              >
                {insight.actionLabel}
              </button>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
};