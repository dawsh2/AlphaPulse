/**
 * Data Viewer Component
 * Displays data table and SQL query interface
 * Preserves exact styling from original ResearchPage
 */

import React from 'react';

interface DataViewerProps {
  styles: any; // CSS module styles
}

export const DataViewer: React.FC<DataViewerProps> = ({
  styles
}) => {
  return (
    <div className={styles.dataViewerContainer}>
      <div className={styles.dataTableContainer}>
        <div className={styles.dataTableHeader}>
          <div className={styles.tableInfo}>
            <span className={styles.tableName}>Select a dataset from the sidebar to view</span>
            <span className={styles.tableStats}></span>
          </div>
        </div>
        
        <div className={styles.sqlEditor}>
          <textarea
            className={styles.sqlInput}
            placeholder="-- Enter SQL query here (e.g., SELECT * FROM read_parquet('data.parquet') LIMIT 100)"
            rows={3}
          />
          <button className={styles.runQueryBtn}>
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <polygon points="5 3 19 12 5 21 5 3"></polygon>
            </svg>
            Run Query
          </button>
        </div>
        
        <div className={styles.dataTableWrapper}>
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
        </div>
      </div>
    </div>
  );
};