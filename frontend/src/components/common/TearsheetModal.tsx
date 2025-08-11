/**
 * Tearsheet Modal Component
 * Displays strategy performance metrics in a modal
 * Preserves exact styling from original ResearchPage
 */

import React from 'react';
import type { Strategy } from '../../data/strategies';

interface TearsheetModalProps {
  isOpen: boolean;
  strategy: Strategy | null;
  styles: any; // CSS module styles from ExplorePage
  onClose: () => void;
  onNotebookClick: (strategy: Strategy) => void;
  onDeployClick: (strategy: Strategy) => void;
}

export const TearsheetModal: React.FC<TearsheetModalProps> = ({
  isOpen,
  strategy,
  styles,
  onClose,
  onNotebookClick,
  onDeployClick
}) => {
  if (!isOpen || !strategy) return null;

  return (
    <div className={styles.tearsheetModal} onClick={onClose}>
      <div className={styles.tearsheetContent} onClick={(e) => e.stopPropagation()}>
        <button className={styles.tearsheetClose} onClick={onClose}>×</button>
        <h2 className={styles.tearsheetTitle}>{strategy.title}</h2>
        
        <div className={styles.tearsheetMetrics}>
          <div className={styles.tearsheetMetric}>
            <span className={styles.tearsheetMetricValue}>{strategy.metrics?.sharpe.toFixed(2)}</span>
            <span className={styles.tearsheetMetricLabel}>Sharpe Ratio</span>
          </div>
          <div className={styles.tearsheetMetric}>
            <span className={styles.tearsheetMetricValue}>{strategy.metrics?.annualReturn.toFixed(1)}%</span>
            <span className={styles.tearsheetMetricLabel}>Annual Return</span>
          </div>
          <div className={styles.tearsheetMetric}>
            <span className={styles.tearsheetMetricValue}>{strategy.metrics?.maxDrawdown.toFixed(1)}%</span>
            <span className={styles.tearsheetMetricLabel}>Max Drawdown</span>
          </div>
          <div className={styles.tearsheetMetric}>
            <span className={styles.tearsheetMetricValue}>{strategy.metrics?.winRate}%</span>
            <span className={styles.tearsheetMetricLabel}>Win Rate</span>
          </div>
        </div>
        
        <div className={styles.tearsheetActions}>
          <button 
            className={styles.tearsheetIconBtn}
            onClick={() => {
              onNotebookClick(strategy);
              onClose();
            }}
            title="Open in Notebook"
          >
            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" style={{ width: '24px', height: '24px' }}>
              {/* Spiral binding */}
              <circle cx="4" cy="4" r="1.5"></circle>
              <circle cx="4" cy="8" r="1.5"></circle>
              <circle cx="4" cy="12" r="1.5"></circle>
              <circle cx="4" cy="16" r="1.5"></circle>
              <circle cx="4" cy="20" r="1.5"></circle>
              {/* Notebook pages */}
              <rect x="7" y="2" width="14" height="20" rx="1"></rect>
              <line x1="11" y1="6" x2="17" y2="6"></line>
              <line x1="11" y1="10" x2="17" y2="10"></line>
              <line x1="11" y1="14" x2="17" y2="14"></line>
              <line x1="11" y1="18" x2="17" y2="18"></line>
            </svg>
            <span>Research</span>
          </button>
          <button 
            className={styles.tearsheetIconBtn}
            onClick={() => onDeployClick(strategy)}
            title="Deploy Strategy"
          >
            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" style={{ width: '24px', height: '24px' }}>
              {/* Rocket/Deploy icon */}
              <path d="M4.5 16.5c-1.5 1.26-2 5-2 5s3.74-.5 5-2c.71-.84.7-2.13-.09-2.91a2.18 2.18 0 0 0-2.91-.09z"></path>
              <path d="M12 15l-3-3a22 22 0 0 1 2-3.95A12.88 12.88 0 0 1 22 2c0 2.72-.78 7.5-6 11a22.35 22.35 0 0 1-4 2z"></path>
              <path d="M9 12H4s.55-3.03 2-4c1.62-1.08 5 0 5 0"></path>
              <path d="M12 15v5s3.03-.55 4-2c1.08-1.62 0-5 0-5"></path>
            </svg>
            <span>Deploy</span>
          </button>
        </div>
      </div>
    </div>
  );
};