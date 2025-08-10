/**
 * Custom hook for filtering and sorting strategies
 * Extracted from ResearchPage to make it reusable
 */

import { useMemo } from 'react';
import type { Strategy } from '../data/strategies';

export type SortBy = 'new' | 'sharpe' | 'returns' | 'name' | 'winrate';

interface UseStrategyFilteringProps {
  strategies: Strategy[];
  searchQuery: string;
  searchTerms: string[];
  sortBy: SortBy;
}

export const useStrategyFiltering = ({
  strategies,
  searchQuery,
  searchTerms,
  sortBy
}: UseStrategyFilteringProps) => {
  
  const filteredAndSorted = useMemo(() => {
    let filtered = strategies;

    // Multi-tag filter
    const allSearchTerms = [...searchTerms];
    if (searchQuery.trim()) {
      allSearchTerms.push(...searchQuery.toLowerCase().split(' ').filter(term => term.length > 0));
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
          return strategies.indexOf(b) - strategies.indexOf(a);
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
  }, [strategies, searchQuery, searchTerms, sortBy]);

  return filteredAndSorted;
};

/**
 * Helper function to get random subset of tags and shuffle them
 * Uses strategy ID as seed for consistent randomization
 */
export const getRandomTags = (tags: string[], strategyId: string) => {
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