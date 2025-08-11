/**
 * ExplorePanel Component - Strategy exploration and search interface
 * Extracted from ResearchPage for better separation of concerns
 */
import React from 'react';
import { useNavigate } from 'react-router-dom';
import exploreStyles from '../../pages/ExplorePage.module.css';

interface Strategy {
  id: string;
  title: string;
  description: string;
  color: string;
  tags: string[];
  creator?: string;
  comingSoon?: boolean;
  metrics?: {
    sharpe: number;
    annualReturn: number;
    maxDrawdown: number;
    winRate: number;
  };
  behavior?: 'trending' | 'meanrev' | 'breakout' | 'volatility';
  risk?: 'conservative' | 'moderate' | 'aggressive';
  timeframe?: 'intraday' | 'swing' | 'position';
}

interface TearsheetData {
  strategy: Strategy;
  isOpen: boolean;
}

type SortBy = 'new' | 'sharpe' | 'returns' | 'name' | 'winrate';

interface ExplorePanelProps {
  allStrategies: Strategy[];
  exploreSearchQuery: string;
  setExploreSearchQuery: (query: string) => void;
  searchTerms: string[];
  setSearchTerms: React.Dispatch<React.SetStateAction<string[]>>;
  sortBy: SortBy;
  setSortBy: (sort: SortBy) => void;
  sortDropdownOpen: boolean;
  setSortDropdownOpen: (open: boolean) => void;
  displayLimit: number;
  setDisplayLimit: React.Dispatch<React.SetStateAction<number>>;
  hoveredCard: string | null;
  setHoveredCard: (id: string | null) => void;
  tearsheet: TearsheetData;
  setTearsheet: (data: TearsheetData) => void;
  setActiveTab: (tab: string | null) => void;
  setMainView: (view: string) => void;
  handleNotebookClick: (e: React.MouseEvent, strategy: Strategy) => void;
}

export const ExplorePanel: React.FC<ExplorePanelProps> = ({
  allStrategies,
  exploreSearchQuery,
  setExploreSearchQuery,
  searchTerms,
  setSearchTerms,
  sortBy,
  setSortBy,
  sortDropdownOpen,
  setSortDropdownOpen,
  displayLimit,
  setDisplayLimit,
  hoveredCard,
  setHoveredCard,
  tearsheet,
  setTearsheet,
  setActiveTab,
  setMainView,
  handleNotebookClick,
}) => {
  const navigate = useNavigate();

  // Helper functions
  const handleTagClick = (tag: string) => {
    setSearchTerms(prev => {
      if (prev.includes(tag)) {
        return prev.filter(t => t !== tag);
      } else {
        return [...prev, tag];
      }
    });
  };

  const handleStrategySelect = (strategy: Strategy) => {
    if (!strategy.comingSoon) {
      if (strategy.id === 'custom') {
        setActiveTab('builder');
        setMainView('builder');
      } else {
        setTearsheet({ strategy, isOpen: true });
      }
    }
  };

  const handleDeployClick = (e: React.MouseEvent, strategy: Strategy) => {
    e.stopPropagation();
    navigate('/monitor', { state: { strategy } });
  };

  const filterAndSortStrategies = () => {
    let filtered = allStrategies;

    // Multi-tag filter
    const allSearchTerms = [...searchTerms];
    if (exploreSearchQuery.trim()) {
      allSearchTerms.push(...exploreSearchQuery.toLowerCase().split(' ').filter(term => term.length > 0));
    }

    if (allSearchTerms.length > 0) {
      filtered = filtered.filter(strategy => {
        const searchableText = [
          strategy.title.toLowerCase(),
          strategy.description.toLowerCase(),
          ...strategy.tags.map(tag => tag.toLowerCase())
        ];
        
        if (strategy.creator) {
          searchableText.push(strategy.creator.toLowerCase());
          searchableText.push(`@${strategy.creator.toLowerCase()}`);
        }
        
        return allSearchTerms.every(term => 
          searchableText.some(text => text.includes(term))
        );
      });
    }

    // Sort
    return filtered.sort((a, b) => {
      if (!a.metrics || !b.metrics) return 0;
      
      switch (sortBy) {
        case 'new':
          // Reverse order to show newest first (higher indices first)
          return allStrategies.indexOf(b) - allStrategies.indexOf(a);
        case 'sharpe':
          return b.metrics.sharpe - a.metrics.sharpe;
        case 'returns':
          return b.metrics.annualReturn - a.metrics.annualReturn;
        case 'winrate':
          return b.metrics.winRate - a.metrics.winRate;
        case 'name':
          return a.title.localeCompare(b.title);
        default:
          return 0;
      }
    });
  };

  // Helper function to get random subset of tags and shuffle them
  const getRandomTags = (tags: string[], strategyId: string) => {
    // Use strategy ID as seed for consistent randomization per strategy
    const seed = strategyId.split('').reduce((acc, char) => acc + char.charCodeAt(0), 0);
    const shuffled = [...tags].sort(() => {
      const random = Math.sin(seed) * 10000;
      return random - Math.floor(random) < 0.5 ? -1 : 1;
    });
    return shuffled.slice(0, Math.min(4, shuffled.length));
  };

  const renderStrategyCard = (strategy: Strategy) => {
    const isHovered = hoveredCard === strategy.id;
    const displayTags = getRandomTags(strategy.tags, strategy.id);
    const seed = strategy.id.split('').reduce((acc, char) => acc + char.charCodeAt(0), 0);
    
    return (
      <div
        key={strategy.id}
        className={`${exploreStyles.strategyCard} ${exploreStyles[strategy.color]}`}
        onClick={() => handleStrategySelect(strategy)}
        onMouseEnter={() => setHoveredCard(strategy.id)}
        onMouseLeave={() => setHoveredCard(null)}
        style={{ cursor: strategy.comingSoon ? 'not-allowed' : 'pointer' }}
      >
        {strategy.comingSoon && (
          <span className={exploreStyles.comingSoonBadge}>Soon</span>
        )}
        
        <div className={exploreStyles.cardContent}>
          <h3 className={exploreStyles.strategyTitle}>{strategy.title}</h3>
          {strategy.creator && (
            <div className={exploreStyles.creatorInfo}>
              <span className={exploreStyles.creatorLabel}>by</span>
              <button 
                className={exploreStyles.creatorName}
                onClick={(e) => {
                  e.stopPropagation();
                  handleTagClick(`@${strategy.creator}`);
                }}
                title={`Search for strategies by @${strategy.creator}`}
              >
                @{strategy.creator}
              </button>
            </div>
          )}
          
          {strategy.metrics && (
            <div className={exploreStyles.compactMetrics}>
              <div className={exploreStyles.primaryMetric}>
                <span className={exploreStyles.primaryValue}>{strategy.metrics.sharpe.toFixed(2)}</span>
                <span className={exploreStyles.primaryLabel}>Sharpe</span>
              </div>
              <div className={exploreStyles.secondaryMetrics}>
                <span className={exploreStyles.secondaryMetric}>{strategy.metrics.annualReturn.toFixed(0)}%</span>
                <span className={exploreStyles.secondaryMetric}>{strategy.metrics.winRate}%</span>
              </div>
            </div>
          )}
          
          <div className={exploreStyles.cardFooter}>
            {displayTags.map((tag, index) => (
              <button 
                key={tag} 
                className={`${exploreStyles.compactTag} ${exploreStyles[`tagColor${(index + seed) % 8}`]} ${searchTerms.includes(tag) ? exploreStyles.activeTag : ''}`}
                onClick={(e) => {
                  e.stopPropagation();
                  handleTagClick(tag);
                }}
              >
                {tag}
              </button>
            ))}
          </div>
        </div>
        
        {isHovered && !strategy.comingSoon && (
          <div className={exploreStyles.hoverOverlay}>
            <button 
              className={exploreStyles.overlayBtn}
              onClick={(e) => handleNotebookClick(e, strategy)}
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
              className={exploreStyles.overlayBtn}
              onClick={(e) => handleDeployClick(e, strategy)}
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

  return (
    <div className={exploreStyles.catalogueContainer}>
      <div className={exploreStyles.controlsBar}>
        <div className={exploreStyles.searchWrapper}>
          <div className={exploreStyles.searchSortGroup}>
            <input
              type="text"
              placeholder="search strategies... (e.g., trending swing @alexchen)"
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
                Sort: {sortBy === 'new' ? 'New' : sortBy === 'sharpe' ? 'Sharpe' : sortBy === 'returns' ? 'Returns' : sortBy === 'winrate' ? 'Win %' : 'A-Z'}
                <span style={{ marginLeft: '8px' }}>▼</span>
              </button>
            {sortDropdownOpen && (
              <div 
                className={exploreStyles.sortMenu}
                onMouseEnter={() => setSortDropdownOpen(true)}
                onMouseLeave={() => setSortDropdownOpen(false)}
              >
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
              </div>
            )}
            </div>
          </div>
          
          {/* Plus button for new strategy */}
          <button 
            className={exploreStyles.newStrategyBtn}
            onClick={() => {
              // Navigate to builder tab for new strategy
              setActiveTab('builder');
              setMainView('builder');
              console.log('Opening new strategy builder');
            }}
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
                onClick={() => handleTagClick(term)}
              >
                {term} ×
              </button>
            ))}
          </div>
        )}
      </div>

      <div className={exploreStyles.resultsInfo}>
        <span className={exploreStyles.resultsCount}>
          Showing {Math.min(displayLimit, filterAndSortStrategies().length)} of {filterAndSortStrategies().length} strategies
        </span>
        {searchTerms.length > 0 && (
          <span className={exploreStyles.filterInfo}>
            • Filtered by: {searchTerms.join(', ')}
          </span>
        )}
      </div>

      <div className={exploreStyles.strategyGrid}>
        {filterAndSortStrategies().slice(0, displayLimit).map(renderStrategyCard)}
      </div>
      
      {filterAndSortStrategies().length > displayLimit && (
        <div className={exploreStyles.loadMoreContainer}>
          <button 
            className={exploreStyles.loadMoreBtn}
            onClick={() => setDisplayLimit(prev => prev + 12)}
          >
            Load More ({filterAndSortStrategies().length - displayLimit} remaining)
          </button>
          <button 
            className={exploreStyles.showAllBtn}
            onClick={() => setDisplayLimit(filterAndSortStrategies().length)}
          >
            Show All
          </button>
        </div>
      )}
      
      {filterAndSortStrategies().length === 0 && (
        <div className={exploreStyles.emptyState}>
          <p>No strategies found</p>
          <p className={exploreStyles.emptyHint}>Try different search terms like "trending", "low-risk", "intraday", or "@username"</p>
        </div>
      )}

      {/* Tearsheet Modal */}
      {tearsheet.isOpen && tearsheet.strategy && (
        <div className={exploreStyles.tearsheetModal} onClick={() => setTearsheet({ ...tearsheet, isOpen: false })}>
          <div className={exploreStyles.tearsheetContent} onClick={(e) => e.stopPropagation()}>
            <button className={exploreStyles.tearsheetClose} onClick={() => setTearsheet({ ...tearsheet, isOpen: false })}>×</button>
            <h2 className={exploreStyles.tearsheetTitle}>{tearsheet.strategy.title}</h2>
            
            <div className={exploreStyles.tearsheetMetrics}>
              <div className={exploreStyles.tearsheetMetric}>
                <span className={exploreStyles.tearsheetMetricValue}>{tearsheet.strategy.metrics?.sharpe.toFixed(2)}</span>
                <span className={exploreStyles.tearsheetMetricLabel}>Sharpe Ratio</span>
              </div>
              <div className={exploreStyles.tearsheetMetric}>
                <span className={exploreStyles.tearsheetMetricValue}>{tearsheet.strategy.metrics?.annualReturn.toFixed(1)}%</span>
                <span className={exploreStyles.tearsheetMetricLabel}>Annual Return</span>
              </div>
              <div className={exploreStyles.tearsheetMetric}>
                <span className={exploreStyles.tearsheetMetricValue}>{tearsheet.strategy.metrics?.maxDrawdown.toFixed(1)}%</span>
                <span className={exploreStyles.tearsheetMetricLabel}>Max Drawdown</span>
              </div>
              <div className={exploreStyles.tearsheetMetric}>
                <span className={exploreStyles.tearsheetMetricValue}>{tearsheet.strategy.metrics?.winRate}%</span>
                <span className={exploreStyles.tearsheetMetricLabel}>Win Rate</span>
              </div>
            </div>
            
            <div className={exploreStyles.tearsheetActions}>
              <button 
                className={exploreStyles.tearsheetIconBtn}
                onClick={() => {
                  handleNotebookClick(new MouseEvent('click') as any, tearsheet.strategy);
                  setTearsheet({ ...tearsheet, isOpen: false });
                }}
                title="Open in Notebook"
              >
                <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" style={{ width: '24px', height: '24px' }}>
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
                <span>Research</span>
              </button>
              <button 
                className={exploreStyles.tearsheetIconBtn}
                onClick={() => navigate('/monitor', { state: { strategy: tearsheet.strategy } })}
                title="Deploy Strategy"
              >
                <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" style={{ width: '24px', height: '24px' }}>
                  {/* Rocket/Deploy icon */}
                  <path d="M4.5 16.5c-1.5 1.26-2 5-2 5s3.74-.5 5-2c.71-.84.7-2.13-.09-2.91a2.18 2.18 0 0 0-2.91-.09z"></path>
                  <path d="M12 15l-3-3a22 22 0 0 1 2-3.95A12.88 12.88 0 0 1 22 2c0 2.72-.78 7.5-6 11a22.35 22.35 0 0 1-4 2z"></path>
                  <path d="M9 12H4s.55-3.03 2-4c1.62-1.08 5 0 5 0"></path>
                  <path d="M12 15v5s3.03-.55 4-2c1.08-1.62 0-5 0-5"></path>
                </svg>
                <span>Deploy</span>
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};