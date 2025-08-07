import React, { ReactNode } from 'react';
import { Navigation } from '../Navigation/Navigation';
import { GlobalAIChat } from '../common/GlobalAIChat';
import styles from './Layout.module.css';

interface LayoutProps {
  children: ReactNode;
}

export const Layout: React.FC<LayoutProps> = ({ children }) => {
  return (
    <div className={styles.appContainer}>
      <Navigation />
      <main className={styles.mainContent}>
        {children}
      </main>
      <GlobalAIChat />
    </div>
  );
};