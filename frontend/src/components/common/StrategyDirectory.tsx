/**
 * Strategy Directory Component
 * Displays categorized strategies in the sidebar
 * Preserves exact styling from original ResearchPage
 */

import React from 'react';
import type { Strategy } from '../../data/strategies';

interface StrategyDirectoryProps {
  styles: any; // CSS module styles
  strategies: Strategy[];
  collapsedCategories: Set<string>;
  onToggleCategory: (category: string) => void;
  onStrategyClick: (strategy: Strategy) => void;
}

export const StrategyDirectory: React.FC<StrategyDirectoryProps> = ({
  styles,
  strategies,
  collapsedCategories,
  onToggleCategory,
  onStrategyClick
}) => {
  const strategyCategories = {
    'Trending': strategies.filter(s => s.tags.includes('trending')),
    'Mean Reversion': strategies.filter(s => s.tags.includes('mean-reversion')),
    'Momentum': strategies.filter(s => s.tags.includes('momentum')),
    'Machine Learning': strategies.filter(s => s.tags.includes('ml')),
    'High Frequency': strategies.filter(s => s.tags.includes('high-frequency')),
    'Options': strategies.filter(s => s.tags.includes('options')),
    'Crypto': strategies.filter(s => s.tags.includes('crypto')),
    'Forex': strategies.filter(s => s.tags.includes('forex'))
  };

  return (
    <div className={styles.tabContent}>
      {/* Categories with strategies - no header text */}
      {Object.entries(strategyCategories).map(([category, categoryStrategies]) => (
        categoryStrategies.length > 0 && (
          <div key={category} className={styles.strategyCategory}>
            <div 
              className={`${styles.categoryHeader} ${collapsedCategories.has(category) ? styles.collapsed : ''}`}
              onClick={() => onToggleCategory(category)}
            >
              <span className={styles.categoryArrow}>▼</span>
              <span>{category} ({categoryStrategies.length})</span>
            </div>
            {!collapsedCategories.has(category) && (
              <div className={styles.strategyList}>
                {categoryStrategies.slice(0, 5).map(strategy => (
                  <div 
                    key={strategy.id}
                    className={styles.strategyItem}
                    onClick={() => onStrategyClick(strategy)}
                  >
                    <div className={styles.strategyName}>{strategy.title}</div>
                    <div className={styles.strategyDesc}>
                      {strategy.metrics?.sharpe.toFixed(2)} Sharpe • {strategy.metrics?.winRate}% Win
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        )
      ))}
    </div>
  );
};