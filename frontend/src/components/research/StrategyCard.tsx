/**
 * StrategyCard Component - Individual strategy card display
 * Extracted from ResearchPage renderStrategyCard function
 */
import React from 'react';
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

interface StrategyCardProps {
  strategy: Strategy;
  isHovered: boolean;
  onMouseEnter: () => void;
  onMouseLeave: () => void;
  onStrategySelect: () => void;
  onTagClick: (tag: string) => void;
  onNotebookClick: (e: React.MouseEvent) => void;
  onDeployClick: (e: React.MouseEvent) => void;
  searchTerms: string[];
}

// Helper function to get random subset of tags and shuffle them
const getRandomTags = (tags: string[], strategyId: string) => {
  // Use strategy ID as seed for consistent randomization per strategy
  const seed = strategyId.split('').reduce((acc, char) => acc + char.charCodeAt(0), 0);
  const shuffled = [...tags].sort(() => {
    const random = Math.sin(seed) * 10000;
    return random - Math.floor(random) < 0.5 ? -1 : 1;
  });
  
  // Random number of tags between 2 and 4
  const numTags = 2 + (seed % 3);
  return shuffled.slice(0, Math.min(numTags, tags.length));
};

export const StrategyCard: React.FC<StrategyCardProps> = ({
  strategy,
  isHovered,
  onMouseEnter,
  onMouseLeave,
  onStrategySelect,
  onTagClick,
  onNotebookClick,
  onDeployClick,
  searchTerms,
}) => {
  const displayTags = getRandomTags(strategy.tags, strategy.id);
  const seed = strategy.id.split('').reduce((acc, char) => acc + char.charCodeAt(0), 0);
  
  return (
    <div
      key={strategy.id}
      className={`${exploreStyles.strategyCard} ${exploreStyles[strategy.color]}`}
      onClick={onStrategySelect}
      onMouseEnter={onMouseEnter}
      onMouseLeave={onMouseLeave}
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
                onTagClick(`@${strategy.creator}`);
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
                onTagClick(tag);
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
            onClick={onDeployClick}
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