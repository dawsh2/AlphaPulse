/**
 * ExploreView Component - Main content for the explore view
 * Extracted from ResearchPage renderMainContent explore section
 */
import React from 'react';
import exploreStyles from '../../pages/ExplorePage.module.css';
import { SearchControls } from './SearchControls';
import { ResultsInfo } from './ResultsInfo';
import { LoadMoreButtons } from './LoadMoreButtons';
import { EmptyState } from './EmptyState';
import { TearsheetModal } from './TearsheetModal';
import { ViewToggle } from './ViewToggle';
import { DataDetailsModal } from './DataDetailsModal';

interface ExploreViewProps {
  exploreSearchQuery: string;
  setExploreSearchQuery: (query: string) => void;
  sortBy: string;
  setSortBy: (sort: string) => void;
  sortDropdownOpen: boolean;
  setSortDropdownOpen: (open: boolean) => void;
  searchTerms: string[];
  onTagClick: (tag: string) => void;
  onNewStrategy: () => void;
  displayLimit: number;
  totalResults: number;
  strategies: any[];
  renderStrategyCard: (strategy: any) => React.ReactNode;
  onLoadMore: () => void;
  onShowAll: () => void;
  tearsheet: any;
  setTearsheet: (tearsheet: any) => void;
  onNotebookClick: (strategy: any) => void;
  viewType: 'strategies' | 'data';
  setViewType: (type: 'strategies' | 'data') => void;
  dataCards?: any[];
  renderDataCard?: (data: any) => React.ReactNode;
  dataDetails?: { data: any | null; isOpen: boolean };
  setDataDetails?: (details: { data: any | null; isOpen: boolean }) => void;
  onDataNotebookClick?: (data: any) => void;
}

export const ExploreView: React.FC<ExploreViewProps> = ({
  exploreSearchQuery,
  setExploreSearchQuery,
  sortBy,
  setSortBy,
  sortDropdownOpen,
  setSortDropdownOpen,
  searchTerms,
  onTagClick,
  onNewStrategy,
  displayLimit,
  totalResults,
  strategies,
  renderStrategyCard,
  onLoadMore,
  onShowAll,
  tearsheet,
  setTearsheet,
  onNotebookClick,
  viewType,
  setViewType,
  dataCards = [],
  renderDataCard,
  dataDetails,
  setDataDetails,
  onDataNotebookClick
}) => {
  const isDataView = viewType === 'data';
  const items = isDataView ? dataCards : strategies;
  const renderCard = isDataView ? renderDataCard : renderStrategyCard;
  
  return (
    <div className={exploreStyles.catalogueContainer}>
      <div style={{ 
        display: 'flex', 
        alignItems: 'flex-start', 
        justifyContent: 'space-between',
        marginBottom: '20px',
        maxWidth: '1400px',
        margin: '0 0 20px 0'
      }}>
        <SearchControls
          exploreSearchQuery={exploreSearchQuery}
          setExploreSearchQuery={setExploreSearchQuery}
          sortBy={sortBy}
          setSortBy={setSortBy}
          sortDropdownOpen={sortDropdownOpen}
          setSortDropdownOpen={setSortDropdownOpen}
          searchTerms={searchTerms}
          onTagClick={onTagClick}
          onNewStrategy={onNewStrategy}
          viewType={viewType}
        />
        <ViewToggle viewType={viewType} onChange={setViewType} />
      </div>

      <ResultsInfo
        displayLimit={displayLimit}
        totalResults={items.length}
        searchTerms={searchTerms}
      />

      <div className={exploreStyles.strategyGrid}>
        {items.slice(0, displayLimit).map(renderCard)}
      </div>
      
      <LoadMoreButtons
        totalResults={items.length}
        displayLimit={displayLimit}
        onLoadMore={onLoadMore}
        onShowAll={onShowAll}
      />
      
      <EmptyState show={items.length === 0} />

      {!isDataView && (
        <TearsheetModal
          tearsheet={tearsheet}
          setTearsheet={setTearsheet}
          onNotebookClick={onNotebookClick}
        />
      )}
      
      {isDataView && dataDetails && setDataDetails && onDataNotebookClick && (
        <DataDetailsModal
          dataDetails={dataDetails}
          setDataDetails={setDataDetails}
          onNotebookClick={onDataNotebookClick}
        />
      )}
    </div>
  );
};