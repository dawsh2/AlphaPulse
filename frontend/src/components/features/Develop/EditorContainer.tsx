/**
 * Editor Container component with tabs and actions for the development environment
 */

import React from 'react';
import CodeEditor from '../../CodeEditor/CodeEditor';
import styles from './Develop.module.css';

interface Tab {
  id: string;
  name: string;
  content: string;
  language?: string;
}

interface EditorContainerProps {
  tabs: Tab[];
  activeTab: string;
  onTabSelect: (tabId: string) => void;
  onTabClose: (tabId: string) => void;
  onContentChange: (tabId: string, content: string) => void;
  onSave: () => void;
  onRun: () => void;
  splitOrientation: 'horizontal' | 'vertical';
  outputOpen: boolean;
  editorHidden: boolean;
  onToggleEditor: () => void;
  onOpenTerminal: () => void;
  splitSize: number;
  className?: string;
}

export const EditorContainer: React.FC<EditorContainerProps> = ({
  tabs,
  activeTab,
  onTabSelect,
  onTabClose,
  onContentChange,
  onSave,
  onRun,
  splitOrientation,
  outputOpen,
  editorHidden,
  onToggleEditor,
  onOpenTerminal,
  splitSize,
  className = ''
}) => {
  const activeTabData = tabs.find(tab => tab.id === activeTab);

  return (
    <div className={`${styles.editorContainer} ${className} ${outputOpen && splitOrientation === 'vertical' ? styles.splitVertical : ''}`}>
      {!editorHidden && (
        <>
          <div className={styles.tabsContainer}>
            <div className={styles.tabs}>
              {tabs.map(tab => (
                <div
                  key={tab.id}
                  className={`${styles.tab} ${activeTab === tab.id ? styles.active : ''}`}
                  onClick={() => onTabSelect(tab.id)}
                >
                  <span className={styles.tabName}>{tab.name}</span>
                  <button 
                    className={styles.tabClose}
                    onClick={(e) => {
                      e.stopPropagation();
                      onTabClose(tab.id);
                    }}
                  >
                    ×
                  </button>
                </div>
              ))}
            </div>
            <div className={styles.editorActions}>
              <button className={styles.actionButton} onClick={onSave} title="Save File">
                <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                  <path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z"></path>
                  <polyline points="17 21 17 13 7 13 7 21"></polyline>
                  <polyline points="7 3 7 8 15 8"></polyline>
                </svg>
              </button>
              <button className={styles.actionButton} onClick={onRun} title="Run Code">
                <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                  <polygon points="5 3 19 12 5 21 5 3"></polygon>
                </svg>
              </button>
              <button className={styles.actionButton} onClick={onOpenTerminal} title="Open Terminal">
                <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                  <polyline points="4 17 10 11 4 5"></polyline>
                  <line x1="12" y1="19" x2="20" y2="19"></line>
                </svg>
              </button>
              <button className={styles.actionButton} onClick={onToggleEditor} title="Close Editor">
                <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                  <path d="M18 6L6 18M6 6l12 12"></path>
                </svg>
              </button>
            </div>
          </div>
          <div 
            className={`${styles.editorWrapper} ${outputOpen ? (splitOrientation === 'horizontal' ? styles.splitHorizontal : styles.splitVertical) : ''}`}
            style={{
              ...(outputOpen ? { 
                flex: '1 1 auto',
                minHeight: splitOrientation === 'horizontal' ? '300px' : 'auto',
                minWidth: splitOrientation === 'vertical' ? '400px' : 'auto'
              } : {})
            }}
          >
            {activeTabData ? (
              <div className={styles.editor}>
                <CodeEditor
                  value={activeTabData.content}
                  onChange={(newContent) => onContentChange(activeTabData.id, newContent)}
                  language={activeTabData.language || 'python'}
                />
              </div>
            ) : (
              <div className={styles.welcome}>
                <h2>AlphaPulse Development</h2>
                <p>Select a file from the sidebar to start coding your trading strategies.</p>
              </div>
            )}
          </div>
        </>
      )}
    </div>
  );
};