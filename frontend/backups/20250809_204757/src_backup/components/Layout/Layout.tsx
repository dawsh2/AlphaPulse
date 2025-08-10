import React, { type ReactNode } from 'react';
import { Navigation } from '../Navigation/Navigation';
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
    </div>
  );
};