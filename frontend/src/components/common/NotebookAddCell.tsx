/**
 * Notebook Add Cell Component
 * Displays the add cell button at the bottom of the notebook
 * Preserves exact styling from original ResearchPage
 */

import React from 'react';

interface NotebookAddCellProps {
  styles: any; // CSS module styles
  onAddCell: (type: 'code' | 'markdown') => void;
}

export const NotebookAddCell: React.FC<NotebookAddCellProps> = ({
  styles,
  onAddCell
}) => {
  return (
    <div className={styles.addCellContainer}>
      <button 
        className={styles.addCellButton}
        onClick={() => onAddCell('code')}
        title="Add new cell"
      >
        <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <line x1="12" y1="5" x2="12" y2="19"></line>
          <line x1="5" y1="12" x2="19" y2="12"></line>
        </svg>
        <span>Add Cell</span>
      </button>
    </div>
  );
};