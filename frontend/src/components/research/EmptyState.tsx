/**
 * EmptyState Component - Message shown when no strategies match the search criteria
 * Extracted from ResearchPage explore view empty state section
 */
import React from 'react';
import exploreStyles from '../../pages/ExplorePage.module.css';

interface EmptyStateProps {
  show: boolean;
}

export const EmptyState: React.FC<EmptyStateProps> = ({ show }) => {
  if (!show) {
    return null;
  }

  return (
    <div className={exploreStyles.emptyState} style={{ paddingLeft: '20px', paddingTop: '40px' }}>
      <p>No strategies found</p>
    </div>
  );
};