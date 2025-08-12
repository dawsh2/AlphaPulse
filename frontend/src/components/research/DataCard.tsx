/**
 * DataCard Component - Individual data source card for the explore view
 * Matches StrategyCard styling exactly
 */
import React from 'react';
import exploreStyles from '../../pages/ExplorePage.module.css';

interface DataCard {
  id: string;
  title: string;
  description: string;
  color: string;
  tags: string[];
  provider?: string;
  frequency?: string;
  coverage?: string;
  records?: string;
  dataType?: 'market' | 'economic' | 'alternative' | 'custom';
  metrics?: {
    dataPoints?: string;
    dateRange?: string;
    updateFreq?: string;
    reliability?: string;
    latency?: string;
    coverage?: string;
    frequency?: string;
  };
}

interface DataCardProps {
  data: DataCard;
  isHovered: boolean;
  onMouseEnter: () => void;
  onMouseLeave: () => void;
  onDataSelect: () => void;
  onTagClick: (tag: string) => void;
  onNotebookClick: (e: React.MouseEvent) => void;
  searchTerms: string[];
}

// Helper function to get random subset of tags and shuffle them
const getRandomTags = (tags: string[], dataId: string) => {
  // Use data ID as seed for consistent randomization per data source
  const seed = dataId.split('').reduce((acc, char) => acc + char.charCodeAt(0), 0);
  const shuffled = [...tags].sort(() => {
    const random = Math.sin(seed) * 10000;
    return random - Math.floor(random) < 0.5 ? -1 : 1;
  });
  
  // Random number of tags between 2 and 4
  const numTags = 2 + (seed % 3);
  return shuffled.slice(0, Math.min(numTags, tags.length));
};

export const DataCardComponent: React.FC<DataCardProps> = ({
  data,
  isHovered,
  onMouseEnter,
  onMouseLeave,
  onDataSelect,
  onTagClick,
  onNotebookClick,
  searchTerms
}) => {
  const displayTags = getRandomTags(data.tags, data.id);
  const seed = data.id.split('').reduce((acc, char) => acc + char.charCodeAt(0), 0);
  
  return (
    <div
      key={data.id}
      className={`${exploreStyles.strategyCard} ${exploreStyles[data.color]}`}
      onClick={onDataSelect}
      onMouseEnter={onMouseEnter}
      onMouseLeave={onMouseLeave}
      style={{ cursor: 'pointer' }}
    >
      <div className={exploreStyles.cardContent}>
        <h3 className={exploreStyles.strategyTitle}>{data.title}</h3>
        {data.provider && (
          <div className={exploreStyles.creatorInfo}>
            <span className={exploreStyles.creatorLabel}>by</span>
            <button 
              className={exploreStyles.creatorName}
              onClick={(e) => {
                e.stopPropagation();
                onTagClick(`@${data.provider}`);
              }}
              title={`Search for data sources by @${data.provider}`}
            >
              @{data.provider}
            </button>
          </div>
        )}
        
        {data.metrics && (
          <div className={exploreStyles.compactMetrics}>
            <div className={exploreStyles.primaryMetric}>
              <span className={exploreStyles.primaryValue}>{data.metrics.dataPoints}</span>
              <span className={exploreStyles.primaryLabel}>Data Points</span>
            </div>
            <div className={exploreStyles.secondaryMetrics}>
              <span className={exploreStyles.secondaryMetric}>{data.metrics.dateRange}</span>
              <span className={exploreStyles.secondaryMetric}>{data.metrics.updateFreq}</span>
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
                onTagClick(tag);
              }}
            >
              {tag}
            </button>
          ))}
        </div>
      </div>
      
      {isHovered && (
        <div className={exploreStyles.hoverOverlay}>
          <button 
            className={exploreStyles.overlayBtn}
            onClick={onNotebookClick}
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
            onClick={(e) => {
              e.stopPropagation();
              console.log('Connect to data source:', data.id);
            }}
            title="Connect"
          >
            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" style={{ width: '20px', height: '20px' }}>
              <path d="M20 12h-6l-2 5-4-10-2 5H4"/>
            </svg>
          </button>
        </div>
      )}
    </div>
  );
};