/**
 * Research Page Wrapper - For A/B testing refactored vs original
 */

import React from 'react';

// For now, use the original to ensure it still works
import OriginalResearchPage from './ResearchPage';

// Later we can switch to:
// import ResearchPageRefactored from '../components/features/Research/ResearchPageRefactored';

const ResearchPageWrapper: React.FC = () => {
  // Add a small indicator to show which version is being used
  const isRefactored = false;
  
  return (
    <>
      {/* Version indicator - minimal styling to not affect layout */}
      <div style={{
        position: 'fixed',
        top: '70px',
        right: '10px',
        background: isRefactored ? '#4CAF50' : '#FF9800',
        color: 'white',
        padding: '4px 8px',
        borderRadius: '4px',
        fontSize: '10px',
        zIndex: 9999,
        opacity: 0.7
      }}>
        {isRefactored ? 'REFACTORED' : 'ORIGINAL'}
      </div>
      
      <OriginalResearchPage />
    </>
  );
};

export default ResearchPageWrapper;