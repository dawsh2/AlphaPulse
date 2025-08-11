/**
 * BuilderPanel Component - Strategy builder interface
 * Extracted from ResearchPage for better separation of concerns
 */
import React from 'react';
import { StrategyWorkbench } from '../StrategyBuilder/StrategyWorkbench';
import styles from '../../pages/ResearchPage.module.css';

interface BuilderPanelProps {
  selectedTemplate: string | null;
  setSelectedTemplate: (template: string | null) => void;
  setActiveTab: (tab: string | null) => void;
  setMainView: (view: string) => void;
}

export const BuilderPanel: React.FC<BuilderPanelProps> = ({
  selectedTemplate,
  setSelectedTemplate,
  setActiveTab,
  setMainView,
}) => {
  return (
    <div className={styles.builderView}>
      <div className={styles.builderMainContent}>
        {selectedTemplate ? (
          <StrategyWorkbench 
            isOpen={true}
            onClose={() => {
              setSelectedTemplate(null);
              setActiveTab(null);
              setMainView('explore');
            }}
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