/**
 * LoadMoreButtons Component - Load more and show all buttons for strategy pagination
 * Extracted from ResearchPage explore view load more section
 */
import React from 'react';
import exploreStyles from '../../pages/ExplorePage.module.css';

interface LoadMoreButtonsProps {
  totalResults: number;
  displayLimit: number;
  onLoadMore: () => void;
  onShowAll: () => void;
}

export const LoadMoreButtons: React.FC<LoadMoreButtonsProps> = ({
  totalResults,
  displayLimit,
  onLoadMore,
  onShowAll,
}) => {
  // Only show if there are more results to display
  if (totalResults <= displayLimit) {
    return null;
  }

  return (
    <div className={exploreStyles.loadMoreContainer}>
      <button 
        className={exploreStyles.loadMoreBtn}
        onClick={onLoadMore}
      >
        Load More ({totalResults - displayLimit} remaining)
      </button>
      <button 
        className={exploreStyles.showAllBtn}
        onClick={onShowAll}
      >
        Show All
      </button>
    </div>
  );
};