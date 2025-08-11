/**
 * ChartTileControls Component
 * Floating controls for chart tiling operations
 */

import React from 'react';
import styles from '../../MonitorPage/MonitorPage.module.css';

interface ChartTileControlsProps {
  onSplitHorizontal: () => void;
  onSplitVertical: () => void;
  onClose?: () => void;
  canClose: boolean;
}

export const ChartTileControls: React.FC<ChartTileControlsProps> = ({
  onSplitHorizontal,
  onSplitVertical,
  onClose,
  canClose
}) => {
  return (
    <div className={styles.tileControlsContainer}>
      {/* Split Horizontal Button */}
      <button 
        className={styles.tileButton}
        onClick={onSplitHorizontal}
        title="Split Horizontally"
        aria-label="Split chart horizontally"
      >
        <svg 
          xmlns="http://www.w3.org/2000/svg" 
          viewBox="0 0 24 24" 
          fill="none" 
          stroke="currentColor" 
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
        >
          <rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect>
          <line x1="3" y1="12" x2="21" y2="12"></line>
        </svg>
      </button>

      {/* Split Vertical Button */}
      <button 
        className={styles.tileButton}
        onClick={onSplitVertical}
        title="Split Vertically"
        aria-label="Split chart vertically"
      >
        <svg 
          xmlns="http://www.w3.org/2000/svg" 
          viewBox="0 0 24 24" 
          fill="none" 
          stroke="currentColor" 
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
        >
          <rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect>
          <line x1="12" y1="3" x2="12" y2="21"></line>
        </svg>
      </button>

      {/* Close Button */}
      {canClose && (
        <button 
          className={`${styles.tileButton} ${styles.tileButtonClose}`}
          onClick={onClose}
          title="Close Chart"
          aria-label="Close chart window"
        >
          <svg 
            xmlns="http://www.w3.org/2000/svg" 
            viewBox="0 0 24 24" 
            fill="none" 
            stroke="currentColor" 
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <line x1="18" y1="6" x2="6" y2="18"></line>
            <line x1="6" y1="6" x2="18" y2="18"></line>
          </svg>
        </button>
      )}
    </div>
  );
};