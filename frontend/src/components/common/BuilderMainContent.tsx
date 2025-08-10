/**
 * Builder Main Content Component
 * Displays strategy builder interface or welcome screen
 * Preserves exact styling from original ResearchPage
 */

import React from 'react';
import { StrategyWorkbench } from '../StrategyBuilder/StrategyWorkbench';

interface BuilderMainContentProps {
  styles: any; // CSS module styles
  selectedTemplate: string | null;
  onTemplateClose: () => void;
}

export const BuilderMainContent: React.FC<BuilderMainContentProps> = ({
  styles,
  selectedTemplate,
  onTemplateClose
}) => {
  return (
    <div className={styles.builderView}>
      <div className={styles.builderMainContent}>
        {selectedTemplate ? (
          <StrategyWorkbench 
            isOpen={true}
            onClose={onTemplateClose}
            initialTemplate={selectedTemplate}
          />
        ) : (
          <div className={styles.builderWelcome}>
            <h2>Strategy Builder</h2>
            <p>Build and backtest custom trading strategies using our visual interface.</p>
            <div className={styles.builderFeatures}>
              <div className={styles.featureItem}>
                <span className={styles.featureIcon}>🎯</span>
                <span>Visual strategy construction</span>
              </div>
              <div className={styles.featureItem}>
                <span className={styles.featureIcon}>📊</span>
                <span>Real-time backtesting</span>
              </div>
              <div className={styles.featureItem}>
                <span className={styles.featureIcon}>⚡</span>
                <span>Parameter optimization</span>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};