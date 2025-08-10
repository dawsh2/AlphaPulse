/**
 * Strategy Grid Component
 * Displays grid of strategy cards with load more and empty states
 * Preserves exact styling from original ResearchPage
 */

import React from 'react';

interface StrategyGridProps {
  styles: any; // CSS module styles from ExplorePage
  strategies: any[]; // Strategy items to display
  displayLimit: number;
  totalCount: number;
  renderCard: (strategy: any) => React.ReactNode;
  onLoadMore: () => void;
  onShowAll: () => void;
}

export const StrategyGrid: React.FC<StrategyGridProps> = ({
  styles,
  strategies,
  displayLimit,
  totalCount,
  renderCard,
  onLoadMore,
  onShowAll
}) => {
  const hasMore = totalCount > displayLimit;
  const isEmpty = strategies.length === 0;

  return (
    <>
      <div className={styles.strategyGrid}>
        {strategies.slice(0, displayLimit).map(renderCard)}
      </div>

      {hasMore && (
        <div className={styles.loadMoreContainer}>
          <button 
            className={styles.loadMoreBtn}
            onClick={onLoadMore}
          >
            Load More ({totalCount - displayLimit} remaining)
          </button>
          <button 
            className={styles.showAllBtn}
            onClick={onShowAll}
          >
            Show All
          </button>
        </div>
      )}

      {isEmpty && (
        <div className={styles.emptyState}>
          <p>No strategies found</p>
          <p className={styles.emptyHint}>Try different search terms like "trending", "low-risk", "intraday", or "@username"</p>
        </div>
      )}
    </>
  );
};