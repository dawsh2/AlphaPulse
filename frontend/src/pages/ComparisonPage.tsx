/**
 * Comparison Page - View original vs refactored side by side or toggle
 */

import React, { useState } from 'react';
import OriginalResearchPage from './ResearchPage';
import { DevelopPage as OriginalDevelopPage } from './DevelopPage';
import RefactoredResearchPage from '../components/features/Research/ResearchPageIncremental';
import { DevelopPage as RefactoredDevelopPage } from '../components/features/Develop/DevelopPageWorking';

const ComparisonPage: React.FC = () => {
  const [view, setView] = useState<'original' | 'refactored' | 'split'>('original');
  const [page, setPage] = useState<'research' | 'develop'>('research');

  const renderContent = () => {
    if (view === 'split') {
      return (
        <div style={{ display: 'flex', height: 'calc(100vh - 100px)' }}>
          <div style={{ flex: 1, borderRight: '2px solid #333', overflow: 'auto' }}>
            <div style={{
              position: 'sticky',
              top: 0,
              background: '#FF9800',
              color: 'white',
              padding: '4px',
              textAlign: 'center',
              fontSize: '12px',
              zIndex: 100
            }}>
              ORIGINAL
            </div>
            {page === 'research' ? <OriginalResearchPage /> : <OriginalDevelopPage />}
          </div>
          <div style={{ flex: 1, overflow: 'auto' }}>
            <div style={{
              position: 'sticky',
              top: 0,
              background: '#4CAF50',
              color: 'white',
              padding: '4px',
              textAlign: 'center',
              fontSize: '12px',
              zIndex: 100
            }}>
              REFACTORED
            </div>
            {page === 'research' ? <RefactoredResearchPage /> : <RefactoredDevelopPage />}
          </div>
        </div>
      );
    }

    return (
      <>
        <div style={{
          position: 'fixed',
          top: '70px',
          right: '10px',
          background: view === 'refactored' ? '#4CAF50' : '#FF9800',
          color: 'white',
          padding: '4px 8px',
          borderRadius: '4px',
          fontSize: '10px',
          zIndex: 9999,
          opacity: 0.7
        }}>
          {view.toUpperCase()}
        </div>
        {view === 'original' ? (
          page === 'research' ? <OriginalResearchPage /> : <OriginalDevelopPage />
        ) : (
          page === 'research' ? <RefactoredResearchPage /> : <RefactoredDevelopPage />
        )}
      </>
    );
  };

  return (
    <div>
      {/* Control Bar */}
      <div style={{
        position: 'fixed',
        top: 0,
        left: 0,
        right: 0,
        height: '50px',
        background: '#1a1a1a',
        borderBottom: '2px solid #333',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        gap: '20px',
        zIndex: 10000
      }}>
        <div style={{ display: 'flex', gap: '10px' }}>
          <label style={{ color: 'white', fontSize: '14px' }}>Page:</label>
          <button
            onClick={() => setPage('research')}
            style={{
              padding: '5px 15px',
              background: page === 'research' ? '#007ACC' : '#333',
              color: 'white',
              border: 'none',
              borderRadius: '4px',
              cursor: 'pointer'
            }}
          >
            Research
          </button>
          <button
            onClick={() => setPage('develop')}
            style={{
              padding: '5px 15px',
              background: page === 'develop' ? '#007ACC' : '#333',
              color: 'white',
              border: 'none',
              borderRadius: '4px',
              cursor: 'pointer'
            }}
          >
            Develop
          </button>
        </div>

        <div style={{ display: 'flex', gap: '10px' }}>
          <label style={{ color: 'white', fontSize: '14px' }}>View:</label>
          <button
            onClick={() => setView('original')}
            style={{
              padding: '5px 15px',
              background: view === 'original' ? '#FF9800' : '#333',
              color: 'white',
              border: 'none',
              borderRadius: '4px',
              cursor: 'pointer'
            }}
          >
            Original
          </button>
          <button
            onClick={() => setView('refactored')}
            style={{
              padding: '5px 15px',
              background: view === 'refactored' ? '#4CAF50' : '#333',
              color: 'white',
              border: 'none',
              borderRadius: '4px',
              cursor: 'pointer'
            }}
          >
            Refactored
          </button>
          <button
            onClick={() => setView('split')}
            style={{
              padding: '5px 15px',
              background: view === 'split' ? '#9C27B0' : '#333',
              color: 'white',
              border: 'none',
              borderRadius: '4px',
              cursor: 'pointer'
            }}
          >
            Split View
          </button>
        </div>
      </div>

      {/* Content */}
      <div style={{ marginTop: '50px' }}>
        {renderContent()}
      </div>
    </div>
  );
};

export default ComparisonPage;