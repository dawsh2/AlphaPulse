/**
 * Code Editor Wrapper Component for Development Page
 * Extracted from DevelopPage.tsx - includes tabs, editor actions, and Monaco editor
 */

import React from 'react';
import CodeEditor from '../../CodeEditor/CodeEditor';

interface Tab {
  id: string;
  name: string;
  content: string;
  language?: string;
}

interface CodeEditorWrapperProps {
  tabs: Tab[];
  activeTab: string;
  editorHidden: boolean;
  outputOpen: boolean;
  splitOrientation: 'horizontal' | 'vertical';
  splitSize: number;
  setTabs: (tabs: Tab[]) => void;
  setActiveTab: (tabId: string) => void;
  onTabContentChange: (tabId: string, content: string) => void;
  onOpenTerminal: () => void;
  onCloseEditor: () => void;
  onOpenFiles: () => void;
  onAddOutput: (message: string) => void;
  styles: Record<string, string>;
}

export const CodeEditorWrapper: React.FC<CodeEditorWrapperProps> = ({
  tabs,
  activeTab,
  editorHidden,
  outputOpen,
  splitOrientation,
  setTabs,
  setActiveTab,
  onTabContentChange,
  onOpenTerminal,
  onCloseEditor,
  onOpenFiles,
  onAddOutput,
  styles
}) => {
  const activeTabData = tabs.find(tab => tab.id === activeTab);

  const closeTab = (tabId: string, e: React.MouseEvent) => {
    e.stopPropagation();
    
    const tabIndex = tabs.findIndex(tab => tab.id === tabId);
    const newTabs = tabs.filter(tab => tab.id !== tabId);
    setTabs(newTabs);
    
    // If closing active tab, switch to another
    if (activeTab === tabId && newTabs.length > 0) {
      const newActiveIndex = Math.max(0, tabIndex - 1);
      setActiveTab(newTabs[newActiveIndex].id);
    }
  };

  const saveFile = () => {
    const activeTabData = tabs.find(tab => tab.id === activeTab);
    if (activeTabData) {
      onAddOutput(`Saving ${activeTabData.name}...`);
      // TODO: Implement actual save logic
      setTimeout(() => {
        onAddOutput(`✓ ${activeTabData.name} saved successfully`);
      }, 500);
    }
  };

  return (
    <div className={`${styles.editorContainer} ${outputOpen && splitOrientation === 'vertical' ? styles.splitVertical : ''}`}>
      {!editorHidden && (
        <div className={styles.tabsContainer}>
        <div className={styles.tabs}>
          {tabs.map(tab => (
            <div
              key={tab.id}
              className={`${styles.tab} ${activeTab === tab.id ? styles.active : ''}`}
              onClick={() => setActiveTab(tab.id)}
            >
              <span className={styles.tabName}>{tab.name}</span>
              <button 
                className={styles.tabClose}
                onClick={(e) => closeTab(tab.id, e)}
              >
                ×
              </button>
            </div>
          ))}
          <button className={styles.newTabBtn} title="New File">+</button>
        </div>
        <div className={styles.editorActions}>
          <button className={styles.actionButton} onClick={saveFile} title="Save File">
            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z"></path>
              <polyline points="17 21 17 13 7 13 7 21"></polyline>
              <polyline points="7 3 7 8 15 8"></polyline>
            </svg>
          </button>
          <button className={styles.actionButton} onClick={onOpenTerminal} title="Open Terminal">
            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <polyline points="4 17 10 11 4 5"></polyline>
              <line x1="12" y1="19" x2="20" y2="19"></line>
            </svg>
          </button>
          <button className={styles.actionButton} onClick={onCloseEditor} title="Close Editor">
            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <path d="M18 6L6 18M6 6l12 12"></path>
            </svg>
          </button>
        </div>
      </div>
      )}

      {!editorHidden && (
        <div 
          className={`${styles.editorWrapper} ${outputOpen ? (splitOrientation === 'horizontal' ? styles.splitHorizontal : styles.splitVertical) : ''}`}
          style={{
            ...(outputOpen ? { 
              flex: '1 1 auto',
              minWidth: '200px',
              minHeight: '200px'
            } : { flex: '1' })
          }}
        >
          {activeTabData ? (
            <div className={styles.editor}>
              <CodeEditor
                value={activeTabData.content}
                onChange={(newContent) => onTabContentChange(activeTab, newContent)}
                language={activeTabData.language}
              />
            </div>
          ) : (
            <div className={styles.welcome}>
              <h2>AlphaPulse Development</h2>
              <p>Select a file from the sidebar to start coding your trading strategies.</p>
              <button 
                className={styles.openFilesBtn}
                onClick={onOpenFiles}
              >
                Open Files
              </button>
            </div>
          )}
        </div>
      )}
    </div>
  );
};