/**
 * SwipeIndicator Component
 * Shows a mobile swipe hint at the bottom of the screen
 * Preserves exact styling from ResearchPage
 */

import React from 'react';

interface SwipeIndicatorProps {
  isVisible: boolean;
  text?: string;
}

export const SwipeIndicator: React.FC<SwipeIndicatorProps> = ({ 
  isVisible, 
  text = 'Swipe up for sidebar' 
}) => {
  if (!isVisible) return null;

  return (
    <div
      style={{
        position: 'fixed',
        bottom: '20px',
        left: '50%',
        transform: 'translateX(-50%)',
        zIndex: 100,
        background: 'var(--color-bg-secondary)',
        border: '2px solid var(--color-text-primary)',
        borderRadius: 'var(--radius-lg)',
        padding: '8px 16px',
        display: 'flex',
        alignItems: 'center',
        gap: '8px',
        opacity: 0.9,
        pointerEvents: 'none',
        animation: 'pulse 2s infinite'
      }}
    >
      <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
        <path d="M12 19V6M5 12l7-7 7 7"/>
      </svg>
      <span style={{ fontSize: '12px', fontWeight: 500 }}>{text}</span>
    </div>
  );
};