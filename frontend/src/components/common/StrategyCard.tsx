/**
 * Strategy Card Component
 * Displays a strategy with metrics and actions
 * Preserves exact styling from original ResearchPage
 */

import React from 'react';
import type { Strategy } from '../../data/strategies';

interface StrategyCardProps {
  strategy: Strategy;
  styles: any; // CSS module styles from ExplorePage
  isHovered: boolean;
  searchTerms: string[];
  displayTags: string[];
  onSelect: (strategy: Strategy) => void;
  onHoverEnter: (strategyId: string) => void;
  onHoverLeave: () => void;
  onTagClick: (tag: string) => void;
  onNotebookClick: (e: React.MouseEvent, strategy: Strategy) => void;
  onDeployClick: (e: React.MouseEvent, strategy: Strategy) => void;
}

export const StrategyCard: React.FC<StrategyCardProps> = ({
  strategy,
  styles,
  isHovered,
  searchTerms,
  displayTags,
  onSelect,
  onHoverEnter,
  onHoverLeave,
  onTagClick,
  onNotebookClick,
  onDeployClick
}) => {
  const seed = strategy.id.split('').reduce((acc, char) => acc + char.charCodeAt(0), 0);

  return (
    <div
      key={strategy.id}
      className={`${styles.strategyCard} ${styles[strategy.color]}`}
      onClick={() => onSelect(strategy)}
      onMouseEnter={() => onHoverEnter(strategy.id)}
      onMouseLeave={onHoverLeave}
      style={{ cursor: strategy.comingSoon ? 'not-allowed' : 'pointer' }}
    >
      {strategy.comingSoon && (
        <span className={styles.comingSoonBadge}>Soon</span>
      )}
      
      <div className={styles.cardContent}>
        <h3 className={styles.strategyTitle}>{strategy.title}</h3>
        {strategy.creator && (
          <div className={styles.creatorInfo}>
            <span className={styles.creatorLabel}>by</span>
            <button 
              className={styles.creatorName}
              onClick={(e) => {
                e.stopPropagation();
                onTagClick(`@${strategy.creator}`);
              }}
              title={`Search for strategies by @${strategy.creator}`}
            >
              @{strategy.creator}
            </button>
          </div>
        )}
        
        {strategy.metrics && (
          <div className={styles.compactMetrics}>
            <div className={styles.primaryMetric}>
              <span className={styles.primaryValue}>{strategy.metrics.sharpe.toFixed(2)}</span>
              <span className={styles.primaryLabel}>Sharpe</span>
            </div>
            <div className={styles.secondaryMetrics}>
              <span className={styles.secondaryMetric}>{strategy.metrics.annualReturn.toFixed(0)}%</span>
              <span className={styles.secondaryMetric}>{strategy.metrics.winRate}%</span>
            </div>
          </div>
        )}
        
        <div className={styles.cardFooter}>
          {displayTags.map((tag, index) => (
            <button 
              key={tag} 
              className={`${styles.compactTag} ${styles[`tagColor${(index + seed) % 8}`]} ${searchTerms.includes(tag) ? styles.activeTag : ''}`}
              onClick={(e) => {
                e.stopPropagation();
                onTagClick(tag);
              }}
            >
              {tag}
            </button>
          ))}
        </div>
      </div>
      
      {isHovered && !strategy.comingSoon && (
        <div className={styles.hoverOverlay}>
          <button 
            className={styles.overlayBtn}
            onClick={(e) => onNotebookClick(e, strategy)}
            title="Research"
          >
            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" style={{ width: '20px', height: '20px' }}>
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
          </button>
          <button 
            className={styles.overlayBtn}
            onClick={(e) => onDeployClick(e, strategy)}
            title="Deploy"
          >
            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" style={{ width: '20px', height: '20px' }}>
              {/* Rocket/Deploy icon */}
              <path d="M4.5 16.5c-1.5 1.26-2 5-2 5s3.74-.5 5-2c.71-.84.7-2.13-.09-2.91a2.18 2.18 0 0 0-2.91-.09z"></path>
              <path d="M12 15l-3-3a22 22 0 0 1 2-3.95A12.88 12.88 0 0 1 22 2c0 2.72-.78 7.5-6 11a22.35 22.35 0 0 1-4 2z"></path>
              <path d="M9 12H4s.55-3.03 2-4c1.62-1.08 5 0 5 0"></path>
              <path d="M12 15v5s3.03-.55 4-2c1.08-1.62 0-5 0-5"></path>
            </svg>
          </button>
        </div>
      )}
    </div>
  );
};