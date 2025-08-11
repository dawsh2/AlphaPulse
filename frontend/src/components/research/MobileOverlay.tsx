/**
 * MobileOverlay Component - Semi-transparent overlay for mobile when sidebar is open
 * Extracted from ResearchPage mobile overlay section
 */
import React from 'react';

interface MobileOverlayProps {
  show: boolean;
  onClose: () => void;
}

export const MobileOverlay: React.FC<MobileOverlayProps> = ({ show, onClose }) => {
  if (!show) {
    return null;
  }

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
      onClick={onClose}
    />
  );
};