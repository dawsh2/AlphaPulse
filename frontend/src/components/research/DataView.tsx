/**
 * DataView Component - Main content for the data view
 * Extracted from ResearchPage renderMainContent data section
 */
import React from 'react';
import styles from '../../pages/ResearchPage.module.css';
import { SqlQueryInterface } from './SqlQueryInterface';
import { DataEmptyState } from './DataEmptyState';

export const DataView: React.FC = () => {
  return (
    <div className={styles.dataViewerContainer}>
      <div className={styles.dataTableContainer}>
        <div className={styles.dataTableHeader}>
          <div className={styles.tableInfo}>
            <span className={styles.tableName}>Select a dataset from the sidebar to view</span>
            <span className={styles.tableStats}></span>
          </div>
        </div>
        
        <SqlQueryInterface />
        
        <div className={styles.dataTableWrapper}>
          <DataEmptyState />
        </div>
      </div>
    </div>
  );
};