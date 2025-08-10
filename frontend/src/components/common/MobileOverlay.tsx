/**
 * Mobile Overlay Component
 * Extracted from ResearchPage - preserves exact styling
 */

import React from 'react';

interface MobileOverlayProps {
  isVisible: boolean;
  onClick: () => void;
}

export const MobileOverlay: React.FC<MobileOverlayProps> = ({ isVisible, onClick }) => {
  if (!isVisible) return null;
  
  // EXACT styling from original
  return (
    <div
      style={{
        position: 'fixed',
        top: 0,
        left: 0,
        right: 0,
        bottom: 0,
        background: 'rgba(0, 0, 0, 0.5)',
        zIndex: 199,
        backdropFilter: 'blur(2px)'
      }}
      onClick={onClick}
    />
  );
};

interface SwipeIndicatorProps {
  isVisible: boolean;
  text?: string;
}

export const SwipeIndicator: React.FC<SwipeIndicatorProps> = ({ 
  isVisible, 
  text = 'Swipe up for snippets' 
}) => {
  if (!isVisible) return null;
  
  // EXACT styling from original
  return (
    <div
      style={{
        position: 'fixed',
        bottom: '20px',
        left: '50%',
        transform: 'translateX(-50%)',
        background: 'rgba(255, 255, 255, 0.2)',
        borderRadius: '20px',
        padding: '10px 20px',
        fontSize: '12px',
        color: 'white',
        zIndex: 100,
        pointerEvents: 'none'
      }}
    >
      {text}
    </div>
  );
};