/**
 * Builder Sidebar Component
 * Displays strategies and templates for the strategy builder
 * Preserves exact styling from original ResearchPage
 */

import React from 'react';

interface BuilderSidebarProps {
  styles: any; // CSS module styles
  collapsedCategories: Set<string>;
  onToggleCategory: (category: string) => void;
  onSelectTemplate: (template: string) => void;
  onSelectStrategy: (strategyType: string) => void;
}

export const BuilderSidebar: React.FC<BuilderSidebarProps> = ({
  styles,
  collapsedCategories,
  onToggleCategory,
  onSelectTemplate,
  onSelectStrategy
}) => {
  return (
    <div className={styles.tabContent}>
      {/* Strategies Section */}
      <div className={styles.strategyCategory}>
        <div 
          className={`${styles.categoryHeader} ${collapsedCategories.has('Strategies') ? styles.collapsed : ''}`}
          onClick={() => onToggleCategory('Strategies')}
        >
          <span className={styles.categoryArrow}>▼</span>
          <span>Strategies</span>
        </div>
        {!collapsedCategories.has('Strategies') && (
          <div className={styles.strategyList}>
            <div 
              className={styles.strategyItem}
              onClick={() => onSelectStrategy('custom')}
            >
              <div className={styles.strategyName}>New Strategy</div>
              <div className={styles.strategyDesc}>Create from scratch</div>
            </div>
            <div 
              className={styles.strategyItem}
              onClick={() => onSelectStrategy('oversold_bounce')}
            >
              <div className={styles.strategyName}>Oversold Bounce</div>
              <div className={styles.strategyDesc}>RSI mean reversion</div>
            </div>
          </div>
        )}
      </div>
      
      {/* Templates Section */}
      <div className={styles.templateCategory}>
        <div 
          className={`${styles.categoryHeader} ${collapsedCategories.has('Templates') ? styles.collapsed : ''}`}
          onClick={() => onToggleCategory('Templates')}
        >
          <span className={styles.categoryArrow}>▼</span>
          <span>Templates</span>
        </div>
        {!collapsedCategories.has('Templates') && (
          <div className={styles.templateList}>
            <div 
              className={styles.templateItem}
              onClick={() => onSelectTemplate('signal_analysis')}
            >
              <div className={styles.templateName}>Signal Analysis</div>
              <div className={styles.templateDesc}>Analyze signals across search space</div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};