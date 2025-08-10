/**
 * Explore Search Bar Component
 * Search input, sort dropdown, filters, and new strategy button
 * Preserves exact styling from original ResearchPage
 */

import React from 'react';

interface ExploreSearchBarProps {
  styles: any; // CSS module styles from ExplorePage
  searchQuery: string;
  sortBy: string;
  sortDropdownOpen: boolean;
  searchTerms: string[];
  displayLimit: number;
  totalResults: number;
  filteredCount: number;
  onSearchChange: (value: string) => void;
  onSortChange: (sortBy: string) => void;
  onSortDropdownToggle: (open: boolean) => void;
  onTagClick: (tag: string) => void;
  onNewStrategy: () => void;
}

export const ExploreSearchBar: React.FC<ExploreSearchBarProps> = ({
  styles,
  searchQuery,
  sortBy,
  sortDropdownOpen,
  searchTerms,
  displayLimit,
  totalResults,
  filteredCount,
  onSearchChange,
  onSortChange,
  onSortDropdownToggle,
  onTagClick,
  onNewStrategy
}) => {
  return (
    <>
      <div className={styles.controlsBar}>
        <div className={styles.searchWrapper}>
          <div className={styles.searchSortGroup}>
            <input
              type="text"
              placeholder="search strategies... (e.g., trending swing @alexchen)"
              className={styles.searchInput}
              value={searchQuery}
              onChange={(e) => onSearchChange(e.target.value)}
            />
            
            <div 
              className={styles.sortDropdown}
              onMouseEnter={() => onSortDropdownToggle(true)}
              onMouseLeave={() => onSortDropdownToggle(false)}
            >
              <button 
                className={styles.sortButton}
                onClick={() => onSortDropdownToggle(!sortDropdownOpen)}
              >
                Sort: {sortBy === 'new' ? 'New' : sortBy === 'sharpe' ? 'Sharpe' : sortBy === 'returns' ? 'Returns' : sortBy === 'winrate' ? 'Win %' : 'A-Z'}
                <span style={{ marginLeft: '8px' }}>▼</span>
              </button>
            {sortDropdownOpen && (
              <div 
                className={styles.sortMenu}
                onMouseEnter={() => onSortDropdownToggle(true)}
                onMouseLeave={() => onSortDropdownToggle(false)}
              >
                <button 
                  className={`${styles.sortOption} ${sortBy === 'new' ? styles.active : ''}`}
                  onClick={() => { onSortChange('new'); onSortDropdownToggle(false); }}
                >
                  New
                </button>
                <button 
                  className={`${styles.sortOption} ${sortBy === 'sharpe' ? styles.active : ''}`}
                  onClick={() => { onSortChange('sharpe'); onSortDropdownToggle(false); }}
                >
                  Sharpe
                </button>
                <button 
                  className={`${styles.sortOption} ${sortBy === 'returns' ? styles.active : ''}`}
                  onClick={() => { onSortChange('returns'); onSortDropdownToggle(false); }}
                >
                  Returns
                </button>
                <button 
                  className={`${styles.sortOption} ${sortBy === 'winrate' ? styles.active : ''}`}
                  onClick={() => { onSortChange('winrate'); onSortDropdownToggle(false); }}
                >
                  Win %
                </button>
                <button 
                  className={`${styles.sortOption} ${sortBy === 'name' ? styles.active : ''}`}
                  onClick={() => { onSortChange('name'); onSortDropdownToggle(false); }}
                >
                  A-Z
                </button>
              </div>
            )}
            </div>
          </div>
          
          {/* Plus button for new strategy */}
          <button 
            className={styles.newStrategyBtn}
            onClick={onNewStrategy}
            title="Create New Strategy"
          >
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round">
              <line x1="12" y1="5" x2="12" y2="19"></line>
              <line x1="5" y1="12" x2="19" y2="12"></line>
            </svg>
          </button>
        </div>
        {searchTerms.length > 0 && (
          <div className={styles.activeFilters}>
            {searchTerms.map(term => (
              <button
                key={term}
                className={styles.filterChip}
                onClick={() => onTagClick(term)}
              >
                {term} ×
              </button>
            ))}
          </div>
        )}
      </div>

      <div className={styles.resultsInfo}>
        <span className={styles.resultsCount}>
          Showing {Math.min(displayLimit, filteredCount)} of {filteredCount} strategies
        </span>
        {searchTerms.length > 0 && (
          <span className={styles.filterInfo}>
            • Filtered by: {searchTerms.join(', ')}
          </span>
        )}
      </div>
    </>
  );
};