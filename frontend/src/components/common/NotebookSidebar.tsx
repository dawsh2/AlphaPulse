/**
 * Notebook Sidebar Component
 * Displays code snippets, templates, and saved notebooks
 * Preserves exact styling from original ResearchPage
 */

import React from 'react';

interface CodeSnippet {
  id: string;
  name: string;
  code: string;
  description?: string;
}

interface NotebookTemplate {
  id: string;
  title: string;
  description: string;
}

interface SavedNotebook {
  id: string;
  name: string;
  lastModified: string;
}

interface NotebookSidebarProps {
  styles: any; // CSS module styles
  codeSnippets: Record<string, CodeSnippet[]>;
  notebookTemplates: NotebookTemplate[];
  savedNotebooks: SavedNotebook[];
  searchQuery: string;
  collapsedCategories: Set<string>;
  onToggleCategory: (category: string) => void;
  onInsertSnippet: (snippet: CodeSnippet) => void;
  onLoadTemplate: (template: NotebookTemplate) => void;
}

export const NotebookSidebar: React.FC<NotebookSidebarProps> = ({
  styles,
  codeSnippets,
  notebookTemplates,
  savedNotebooks,
  searchQuery,
  collapsedCategories,
  onToggleCategory,
  onInsertSnippet,
  onLoadTemplate
}) => {
  return (
    <div className={styles.tabContent}>
      {/* Code Snippets Section */}
      {Object.entries(codeSnippets).map(([category, snippets]) => (
        <div key={category} className={styles.snippetCategory}>
          <div 
            className={`${styles.categoryHeader} ${collapsedCategories.has(category) ? styles.collapsed : ''}`}
            onClick={() => onToggleCategory(category)}
          >
            <span className={styles.categoryArrow}>▼</span>
            <span>{category}</span>
          </div>
          {!collapsedCategories.has(category) && (
            <div className={styles.snippetList}>
              {snippets
                .filter(snippet => 
                  snippet.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
                  snippet.code.toLowerCase().includes(searchQuery.toLowerCase())
                )
                .map(snippet => (
                  <div 
                    key={snippet.id} 
                    className={styles.snippetItem}
                    onClick={() => onInsertSnippet(snippet)}
                  >
                    <div>
                      <div className={styles.snippetName}>{snippet.name}</div>
                      {snippet.description && (
                        <div className={styles.snippetDesc}>{snippet.description}</div>
                      )}
                    </div>
                    <span className={styles.insertIcon}>+</span>
                  </div>
                ))}
            </div>
          )}
        </div>
      ))}
      
      {/* Templates Section */}
      <div className={styles.templateCategory}>
        <div className={`${styles.categoryHeader} ${collapsedCategories.has('Templates') ? styles.collapsed : ''}`} onClick={() => onToggleCategory('Templates')}>
          <span className={styles.categoryArrow}>▼</span>
          <span>Analysis Templates</span>
        </div>
        {!collapsedCategories.has('Templates') && (
          <div className={styles.templateList}>
            {notebookTemplates.map(template => (
              <div 
                key={template.id} 
                className={styles.templateItem}
                onClick={() => onLoadTemplate(template)}
              >
                <div className={styles.templateName}>{template.title}</div>
                <div className={styles.templateDesc}>{template.description}</div>
              </div>
            ))}
          </div>
        )}
      </div>
      
      {/* Saved Notebooks Section */}
      <div className={styles.notebookBrowser}>
        <div className={styles.notebookCategory}>
          <div className={`${styles.categoryHeader} ${collapsedCategories.has('Saved Notebooks') ? styles.collapsed : ''}`} onClick={() => onToggleCategory('Saved Notebooks')}>
            <span className={styles.categoryArrow}>▼</span>
            <span>Saved Notebooks</span>
          </div>
          {!collapsedCategories.has('Saved Notebooks') && (
            <div className={styles.notebookList}>
              {savedNotebooks.map(notebook => (
                <div key={notebook.id} className={styles.notebookItem}>
                  <div className={styles.notebookName}>{notebook.name}</div>
                  <div className={styles.notebookDate}>Modified: {notebook.lastModified}</div>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
};