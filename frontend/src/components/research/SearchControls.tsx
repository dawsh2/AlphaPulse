/**
 * SearchControls Component - Search bar, sorting, and filter controls for strategy/data exploration
 * Extracted from ResearchPage explore view controls bar
 */
import React from 'react';
import exploreStyles from '../../pages/ExplorePage.module.css';

type SortBy = 'new' | 'sharpe' | 'returns' | 'name' | 'winrate' | 'size' | 'updated' | 'frequency';

interface SearchControlsProps {
  exploreSearchQuery: string;
  setExploreSearchQuery: (query: string) => void;
  sortBy: SortBy;
  setSortBy: (sort: SortBy) => void;
  sortDropdownOpen: boolean;
  setSortDropdownOpen: (open: boolean) => void;
  searchTerms: string[];
  onTagClick: (tag: string) => void;
  onNewStrategy: () => void;
  viewType?: 'strategies' | 'data';
}

export const SearchControls: React.FC<SearchControlsProps> = ({
  exploreSearchQuery,
  setExploreSearchQuery,
  sortBy,
  setSortBy,
  sortDropdownOpen,
  setSortDropdownOpen,
  searchTerms,
  onTagClick,
  onNewStrategy,
  viewType = 'strategies',
}) => {
  const isDataView = viewType === 'data';
  
  const getSortLabel = () => {
    if (isDataView) {
      return sortBy === 'new' ? 'Recent' : 
             sortBy === 'size' ? 'Size' : 
             sortBy === 'updated' ? 'Updated' : 
             sortBy === 'frequency' ? 'Frequency' : 
             sortBy === 'name' ? 'A-Z' : 'Recent';
    } else {
      return sortBy === 'new' ? 'New' : 
             sortBy === 'sharpe' ? 'Sharpe' : 
             sortBy === 'returns' ? 'Returns' : 
             sortBy === 'winrate' ? 'Win %' : 
             sortBy === 'name' ? 'A-Z' : 'New';
    }
  };
  
  return (
    <div className={exploreStyles.controlsBar}>
      <div className={exploreStyles.searchWrapper}>
        <div className={exploreStyles.searchSortGroup}>
          <input
            type="text"
            placeholder={isDataView ? "search datasets... (e.g., crypto, stocks, S&P 500)" : "search strategies... (e.g., trending swing @alexchen)"}
            className={exploreStyles.searchInput}
            value={exploreSearchQuery}
            onChange={(e) => setExploreSearchQuery(e.target.value)}
          />
          
          <div 
            className={exploreStyles.sortDropdown}
            onMouseEnter={() => setSortDropdownOpen(true)}
            onMouseLeave={() => setSortDropdownOpen(false)}
          >
            <button 
              className={exploreStyles.sortButton}
              onClick={() => setSortDropdownOpen(!sortDropdownOpen)}
            >
              Sort: {getSortLabel()}
              <span style={{ marginLeft: '8px' }}>▼</span>
            </button>
          {sortDropdownOpen && (
            <div 
              className={exploreStyles.sortMenu}
              onMouseEnter={() => setSortDropdownOpen(true)}
              onMouseLeave={() => setSortDropdownOpen(false)}
            >
              {isDataView ? (
                <>
                  <button 
                    className={`${exploreStyles.sortOption} ${sortBy === 'new' ? exploreStyles.active : ''}`}
                    onClick={() => { setSortBy('new'); setSortDropdownOpen(false); }}
                  >
                    Recent
                  </button>
                  <button 
                    className={`${exploreStyles.sortOption} ${sortBy === 'size' ? exploreStyles.active : ''}`}
                    onClick={() => { setSortBy('size'); setSortDropdownOpen(false); }}
                  >
                    Size
                  </button>
                  <button 
                    className={`${exploreStyles.sortOption} ${sortBy === 'updated' ? exploreStyles.active : ''}`}
                    onClick={() => { setSortBy('updated'); setSortDropdownOpen(false); }}
                  >
                    Updated
                  </button>
                  <button 
                    className={`${exploreStyles.sortOption} ${sortBy === 'frequency' ? exploreStyles.active : ''}`}
                    onClick={() => { setSortBy('frequency'); setSortDropdownOpen(false); }}
                  >
                    Frequency
                  </button>
                  <button 
                    className={`${exploreStyles.sortOption} ${sortBy === 'name' ? exploreStyles.active : ''}`}
                    onClick={() => { setSortBy('name'); setSortDropdownOpen(false); }}
                  >
                    A-Z
                  </button>
                </>
              ) : (
                <>
                  <button 
                    className={`${exploreStyles.sortOption} ${sortBy === 'new' ? exploreStyles.active : ''}`}
                    onClick={() => { setSortBy('new'); setSortDropdownOpen(false); }}
                  >
                    New
                  </button>
                  <button 
                    className={`${exploreStyles.sortOption} ${sortBy === 'sharpe' ? exploreStyles.active : ''}`}
                    onClick={() => { setSortBy('sharpe'); setSortDropdownOpen(false); }}
                  >
                    Sharpe
                  </button>
                  <button 
                    className={`${exploreStyles.sortOption} ${sortBy === 'returns' ? exploreStyles.active : ''}`}
                    onClick={() => { setSortBy('returns'); setSortDropdownOpen(false); }}
                  >
                    Returns
                  </button>
                  <button 
                    className={`${exploreStyles.sortOption} ${sortBy === 'winrate' ? exploreStyles.active : ''}`}
                    onClick={() => { setSortBy('winrate'); setSortDropdownOpen(false); }}
                  >
                    Win %
                  </button>
                  <button 
                    className={`${exploreStyles.sortOption} ${sortBy === 'name' ? exploreStyles.active : ''}`}
                    onClick={() => { setSortBy('name'); setSortDropdownOpen(false); }}
                  >
                    A-Z
                  </button>
                </>
              )}
            </div>
          )}
          </div>
        </div>
        
        {/* Plus button for new strategy */}
        <button 
          className={exploreStyles.newStrategyBtn}
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
        <div className={exploreStyles.activeFilters}>
          {searchTerms.map(term => (
            <button
              key={term}
              className={exploreStyles.filterChip}
              onClick={() => onTagClick(term)}
            >
              {term} ×
            </button>
          ))}
        </div>
      )}
    </div>
  );
};