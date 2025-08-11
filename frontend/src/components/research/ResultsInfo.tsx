/**
 * ResultsInfo Component - Displays result count and active filters
 * Extracted from ResearchPage explore view results info section
 */
import React from 'react';
import exploreStyles from '../../pages/ExplorePage.module.css';

interface ResultsInfoProps {
  displayLimit: number;
  totalResults: number;
  searchTerms: string[];
}

export const ResultsInfo: React.FC<ResultsInfoProps> = ({
  displayLimit,
  totalResults,
  searchTerms,
}) => {
  return (
    <div className={exploreStyles.resultsInfo}>
      <span className={exploreStyles.resultsCount}>
        Showing {Math.min(displayLimit, totalResults)} of {totalResults} strategies
      </span>
      {searchTerms.length > 0 && (
        <span className={exploreStyles.filterInfo}>
          • Filtered by: {searchTerms.join(', ')}
        </span>
      )}
    </div>
  );
};