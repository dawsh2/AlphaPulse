/**
 * SavedNotebooksList Component - Displays list of saved notebooks
 * Extracted from ResearchPage saved notebooks section
 */
import React from 'react';
import styles from '../../pages/ResearchPage.module.css';

interface SavedNotebook {
  id: string;
  name: string;
  lastModified: string;
  cells: any[];
}

interface SavedNotebooksListProps {
  notebooks: SavedNotebook[];
}

export const SavedNotebooksList: React.FC<SavedNotebooksListProps> = ({
  notebooks
}) => {
  return (
    <div className={styles.notebookList}>
      {notebooks.map(notebook => (
        <div key={notebook.id} className={styles.notebookItem}>
          <div className={styles.notebookName}>{notebook.name}</div>
          <div className={styles.notebookDate}>Modified: {notebook.lastModified}</div>
        </div>
      ))}
    </div>
  );
};