/**
 * DataEmptyState Component - Empty state display for the data viewer
 * Extracted from ResearchPage data viewer empty state section
 */
import React from 'react';
import styles from '../../pages/ResearchPage.module.css';

export const DataEmptyState: React.FC = () => {
  return (
    <div className={styles.emptyDataState}>
      <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" opacity="0.3">
        <rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect>
        <line x1="3" y1="9" x2="21" y2="9"></line>
        <line x1="9" y1="21" x2="9" y2="9"></line>
        <line x1="15" y1="21" x2="15" y2="9"></line>
      </svg>
      <p>No data loaded</p>
      <p className={styles.dataHint}>Select a dataset or run a SQL query to view data</p>
    </div>
  );
};