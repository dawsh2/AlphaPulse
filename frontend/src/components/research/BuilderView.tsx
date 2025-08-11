/**
 * BuilderView Component - Main content for the builder view
 * Extracted from ResearchPage renderMainContent builder section
 */
import React from 'react';
import styles from '../../pages/ResearchPage.module.css';
import { StrategyWorkbench } from '../StrategyBuilder/StrategyWorkbench';
import { BuilderWelcome } from './BuilderWelcome';

interface BuilderViewProps {
  selectedTemplate: string | null;
  setSelectedTemplate: (template: string | null) => void;
  setActiveTab: (tab: string | null) => void;
  setMainView: (view: string) => void;
}

export const BuilderView: React.FC<BuilderViewProps> = ({
  selectedTemplate,
  setSelectedTemplate,
  setActiveTab,
  setMainView
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
          <BuilderWelcome />
        )}
      </div>
    </div>
  );
};