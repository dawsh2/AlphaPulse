/**
 * DataDetailsModal Component - Modal for showing detailed data source information
 * Similar to TearsheetModal but for data sources
 */
import React from 'react';
import exploreStyles from '../../pages/ExplorePage.module.css';

interface DataCard {
  id: string;
  title: string;
  description: string;
  color: string;
  tags: string[];
  provider?: string;
  frequency?: string;
  coverage?: string;
  dataType?: 'market' | 'economic' | 'alternative' | 'custom';
  metrics?: {
    dataPoints: string;
    dateRange: string;
    updateFreq: string;
  };
}

interface DataDetailsModalProps {
  dataDetails: { data: DataCard | null; isOpen: boolean };
  setDataDetails: (details: { data: DataCard | null; isOpen: boolean }) => void;
  onNotebookClick: (data: DataCard) => void;
}

export const DataDetailsModal: React.FC<DataDetailsModalProps> = ({
  dataDetails,
  setDataDetails,
  onNotebookClick
}) => {
  if (!dataDetails.isOpen || !dataDetails.data) return null;
  
  const data = dataDetails.data;
  
  return (
    <div className={exploreStyles.tearsheetModal} onClick={() => setDataDetails({ data: null, isOpen: false })}>
      <div className={exploreStyles.tearsheetContent} onClick={(e) => e.stopPropagation()}>
        <button className={exploreStyles.tearsheetClose} onClick={() => setDataDetails({ data: null, isOpen: false })}>×</button>
        <h2 className={exploreStyles.tearsheetTitle}>{data.title}</h2>
        
        <div className={exploreStyles.tearsheetMetrics}>
          {data.metrics && (
            <>
              <div className={exploreStyles.tearsheetMetric}>
                <span className={exploreStyles.tearsheetMetricValue}>{data.metrics.dataPoints}</span>
                <span className={exploreStyles.tearsheetMetricLabel}>Data Points</span>
              </div>
              <div className={exploreStyles.tearsheetMetric}>
                <span className={exploreStyles.tearsheetMetricValue}>{data.metrics.dateRange}</span>
                <span className={exploreStyles.tearsheetMetricLabel}>Date Range</span>
              </div>
              <div className={exploreStyles.tearsheetMetric}>
                <span className={exploreStyles.tearsheetMetricValue}>{data.metrics.updateFreq}</span>
                <span className={exploreStyles.tearsheetMetricLabel}>Update Frequency</span>
              </div>
              <div className={exploreStyles.tearsheetMetric}>
                <span className={exploreStyles.tearsheetMetricValue}>{data.coverage || 'Global'}</span>
                <span className={exploreStyles.tearsheetMetricLabel}>Coverage</span>
              </div>
            </>
          )}
        </div>
        
        <div className={exploreStyles.tearsheetActions}>
          <button 
            className={exploreStyles.tearsheetIconBtn}
            onClick={() => {
              onNotebookClick(data);
              setDataDetails({ data: null, isOpen: false });
            }}
            title="Open in Notebook"
          >
            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" style={{ width: '24px', height: '24px' }}>
              {/* Spiral binding */}
              <circle cx="4" cy="4" r="1.5"></circle>
              <circle cx="4" cy="8" r="1.5"></circle>
              <circle cx="4" cy="12" r="1.5"></circle>
              <circle cx="4" cy="16" r="1.5"></circle>
              <circle cx="4" cy="20" r="1.5"></circle>
              {/* Notebook pages */}
              <rect x="7" y="2" width="14" height="20" rx="1"></rect>
              <line x1="11" y1="6" x2="17" y2="6"></line>
              <line x1="11" y1="10" x2="17" y2="10"></line>
              <line x1="11" y1="14" x2="17" y2="14"></line>
              <line x1="11" y1="18" x2="17" y2="18"></line>
            </svg>
            <span>Analyze</span>
          </button>
          <button 
            className={exploreStyles.tearsheetIconBtn}
            onClick={() => {
              // Download/Export functionality
              console.log('Export data:', data.title);
              setDataDetails({ data: null, isOpen: false });
            }}
            title="Export Dataset"
          >
            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" style={{ width: '24px', height: '24px' }}>
              {/* Download icon */}
              <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path>
              <polyline points="7 10 12 15 17 10"></polyline>
              <line x1="12" y1="15" x2="12" y2="3"></line>
            </svg>
            <span>Export</span>
          </button>
        </div>
      </div>
    </div>
  );
};