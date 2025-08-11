/**
 * SidebarTabs Component - Navigation tabs for the research sidebar
 * Extracted from ResearchPage sidebar tabs section
 */
import React from 'react';
import styles from '../../pages/ResearchPage.module.css';

type MainView = 'explore' | 'notebook' | 'builder';
type SidebarTab = 'builder' | 'notebooks';

interface SidebarTabsProps {
  mainView: MainView;
  setMainView: (view: MainView) => void;
  handleTabSwitch: (tab: SidebarTab) => void;
}

export const SidebarTabs: React.FC<SidebarTabsProps> = ({
  mainView,
  setMainView,
  handleTabSwitch,
}) => {
  return (
    <div className={styles.sidebarTabs}>
      <button 
        className={`${styles.sidebarTab} ${mainView === 'builder' ? styles.active : ''}`}
        onClick={() => handleTabSwitch('builder')}
        title="StrategyWorkbench"
      >
        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" style={{ width: '24px', height: '24px' }}>
          {/* Wrench only */}
          <path d="M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z"></path>
        </svg>
      </button>
      <button 
        className={`${styles.sidebarTab} ${mainView === 'notebook' ? styles.active : ''}`}
        onClick={() => handleTabSwitch('notebooks')}
        title="Notebooks"
      >
        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" style={{ width: '24px', height: '24px' }}>
          {/* Spiral binding */}
          <circle cx="4" cy="4" r="1.5"></circle>
          <circle cx="4" cy="8" r="1.5"></circle>
          <circle cx="4" cy="12" r="1.5"></circle>
          <circle cx="4" cy="16" r="1.5"></circle>
          <circle cx="4" cy="20" r="1.5"></circle>
          {/* Notebook pages */}
          <rect x="7" y="2" width="14" height="20" rx="1"></rect>
          {/* Lines on pages */}
          <line x1="10" y1="7" x2="18" y2="7"></line>
          <line x1="10" y1="11" x2="18" y2="11"></line>
          <line x1="10" y1="15" x2="16" y2="15"></line>
        </svg>
      </button>
      <button 
        className={`${styles.sidebarTab} ${mainView === 'explore' ? styles.active : ''}`}
        onClick={() => setMainView('explore')}
        title="Explore"
      >
        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" style={{ width: '24px', height: '24px' }}>
          {/* Magnifying glass */}
          <circle cx="11" cy="11" r="8"></circle>
          <path d="m21 21-4.35-4.35"></path>
        </svg>
      </button>
    </div>
  );
};