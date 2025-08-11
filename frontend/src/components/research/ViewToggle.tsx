/**
 * ViewToggle Component - Toggle between Strategies and Data views
 */
import React from 'react';
import styles from './ViewToggle.module.css';

interface ViewToggleProps {
  viewType: 'strategies' | 'data';
  onChange: (type: 'strategies' | 'data') => void;
}

export const ViewToggle: React.FC<ViewToggleProps> = ({ viewType, onChange }) => {
  return (
    <div className={styles.viewToggle}>
      <div className={styles.toggleTrack}>
        <div 
          className={`${styles.toggleSlider} ${viewType === 'data' ? styles.sliderRight : ''}`}
        />
        <button
          className={`${styles.toggleOption} ${viewType === 'strategies' ? styles.active : ''}`}
          onClick={() => onChange('strategies')}
          title="Strategies"
        >
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <path d="M3 3v18h18"/>
            <path d="M18 9l-5 5-3-3-5 5"/>
          </svg>
        </button>
        <button
          className={`${styles.toggleOption} ${viewType === 'data' ? styles.active : ''}`}
          onClick={() => onChange('data')}
          title="Data"
        >
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <ellipse cx="12" cy="5" rx="9" ry="3"/>
            <path d="M21 12c0 1.66-4 3-9 3s-9-1.34-9-3"/>
            <path d="M3 5v14c0 1.66 4 3 9 3s9-1.34 9-3V5"/>
          </svg>
        </button>
      </div>
    </div>
  );
};