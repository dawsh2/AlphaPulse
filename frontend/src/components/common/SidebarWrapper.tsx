/**
 * Sidebar Wrapper Component
 * Just the container structure - content is passed as children
 * Preserves EXACT styling from original ResearchPage
 */

import React from 'react';
import styles from '../../pages/ResearchPage.module.css';

interface SidebarWrapperProps {
  children: React.ReactNode;
  isOpen: boolean;
  isMobile: boolean;
}

export const SidebarWrapper: React.FC<SidebarWrapperProps> = ({
  children,
  isOpen,
  isMobile
}) => {
  // EXACT styling from original ResearchPage sidebar
  return (
    <div 
      className={styles.snippetsSidebar}
      style={{
        transform: isMobile ? (isOpen ? 'translateY(0)' : 'translateY(calc(100% - 60px))') : 'none',
        position: isMobile ? 'fixed' : 'relative',
        bottom: isMobile ? 0 : 'auto',
        left: isMobile ? 0 : 'auto',
        right: isMobile ? 0 : 'auto',
        height: isMobile ? '70vh' : '100%',
        zIndex: isMobile ? 200 : 'auto',
        borderRadius: isMobile ? '20px 20px 0 0' : '0',
        boxShadow: isMobile ? '0 -4px 20px rgba(0,0,0,0.1)' : 'none'
      }}
    >
      {children}
    </div>
  );
};