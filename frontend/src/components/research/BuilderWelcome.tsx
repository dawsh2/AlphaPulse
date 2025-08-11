/**
 * BuilderWelcome Component - Welcome screen for strategy builder when no template is selected
 * Extracted from ResearchPage builder welcome section
 */
import React from 'react';
import styles from '../../pages/ResearchPage.module.css';

export const BuilderWelcome: React.FC = () => {
  return (
    <div className={styles.builderWelcome}>
      <h2>Strategy Builder</h2>
      <p>Build and backtest custom trading strategies using our visual interface.</p>
      <div className={styles.builderFeatures}>
        <div className={styles.featureItem}>
          <span className={styles.featureIcon}>
            <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <circle cx="12" cy="12" r="10"></circle>
              <circle cx="12" cy="12" r="6"></circle>
              <circle cx="12" cy="12" r="2"></circle>
            </svg>
          </span>
          <span>Visual strategy construction</span>
        </div>
        <div className={styles.featureItem}>
          <span className={styles.featureIcon}>
            <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <line x1="18" y1="20" x2="18" y2="10"></line>
              <line x1="12" y1="20" x2="12" y2="4"></line>
              <line x1="6" y1="20" x2="6" y2="14"></line>
            </svg>
          </span>
          <span>Real-time backtesting</span>
        </div>
        <div className={styles.featureItem}>
          <span className={styles.featureIcon}>
            <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"></polygon>
            </svg>
          </span>
          <span>Parameter optimization</span>
        </div>
      </div>
    </div>
  );
};