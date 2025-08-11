/**
 * SqlQueryInterface Component - Simple SQL query input interface
 * Extracted from ResearchPage data view section
 */
import React from 'react';
import styles from '../../pages/ResearchPage.module.css';

export const SqlQueryInterface: React.FC = () => {
  return (
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
  );
};