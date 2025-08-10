/**
 * Shared Sidebar Tabs Component
 * Used by Research, Develop, and other pages with sidebar navigation
 * Preserves exact styling from original pages
 */

import React from 'react';

interface SidebarTab {
  id: string;
  title: string;
  icon: React.ReactNode;
  isActive: boolean;
  onClick: () => void;
}

interface SidebarTabsProps {
  tabs: SidebarTab[];
  className?: string;
  styles: any; // CSS module styles object
}

export const SidebarTabs: React.FC<SidebarTabsProps> = ({ tabs, className = '', styles }) => {
  return (
    <div className={styles.sidebarHeader}>
      <div className={styles.sidebarTabs}>
        {tabs.map(tab => (
          <button
            key={tab.id}
            className={`${styles.sidebarTab} ${tab.isActive ? styles.active : ''}`}
            onClick={tab.onClick}
            title={tab.title}
          >
            {tab.icon}
          </button>
        ))}
      </div>
    </div>
  );
};